// indexer/src/tree_view.rs

use anyhow::Context;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader, Write},
    path::{Path},
};

use crate::{
    file_intent_entry::FileIntentEntry,
    util::{workdir_slug, humanize_bytes},
};

/// Build a hierarchical markdown tree from the JSONL index.
///
/// Output shape:
/// # <project> File Tree
/// - src/ — 2 dirs, 5 files
///   - commands.rs — CLI wiring [role: bin • lang: rust • 3.2 KB]
///   - tree_view.rs — Builds the directory tree [role: lib • lang: rust]
/// - README.md — Project overview [role: doc • lang: md]
pub fn build_tree_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
    // ---- load index ----
    let f = File::open(index_path)?;
    let rdr = BufReader::new(f);
    let mut files: Vec<FileIntentEntry> = Vec::new();
    for (i, line) in rdr.lines().enumerate() {
        let entry: FileIntentEntry = serde_json::from_str(&line?)
            .with_context(|| format!("Failed to parse FileIntentEntry at line {}", i + 1))
            .map_err(to_io)?;
        files.push(entry);
    }

    // ---- build dir graph: dir -> {child_dirs}, dir -> [file_indices] ----
    // keep single Vec<FileIntentEntry> and refer by indices
    let mut files_in_dir: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut children_dirs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    children_dirs.entry(String::new()).or_default(); // ensure root

    for (idx, fe) in files.iter().enumerate() {
        let rel = Path::new(&fe.path);
        let parent = rel.parent().map(norm).unwrap_or_else(String::new);
        files_in_dir.entry(parent.clone()).or_default().push(idx);

        // register intermediate directories
        let mut acc = String::new(); // "" == root
        if let Some(dir) = rel.parent() {
            for comp in dir.components() {
                let tail = comp.as_os_str().to_string_lossy();
                let next = join(&acc, &tail);
                children_dirs.entry(acc.clone()).or_default().insert(next.clone());
                children_dirs.entry(next.clone()).or_default(); // ensure node exists
                acc = next;
            }
        }
    }

    // sort files within each dir by filename (stable)
    for idxs in files_in_dir.values_mut() {
        idxs.sort_by(|&a, &b| {
            let pa = Path::new(&files[a].path).file_name().unwrap().to_string_lossy();
            let pb = Path::new(&files[b].path).file_name().unwrap().to_string_lossy();
            pa.cmp(&pb)
        });
    }

    // ---- render markdown ----
    let _out = File::create(output_path)?;
    let project_name = workdir_slug();

    let mut out = File::create(output_path)?;
    writeln!(out, "# {} Project File Tree\n", project_name)?;
    
    render_dir(&mut out, "", 0, &children_dirs, &files_in_dir, &files)?;
    Ok(())
}

/* --------------------------------- render --------------------------------- */

fn render_dir(
    out: &mut File,
    here: &str,
    depth: usize,
    children_dirs: &BTreeMap<String, BTreeSet<String>>,
    files_in_dir: &BTreeMap<String, Vec<usize>>,
    files: &[FileIntentEntry],
) -> std::io::Result<()> {
    // current directory header (skip explicit "root" name)
    if !here.is_empty() {
        let kids = children_dirs.get(here).map(|s| s.len()).unwrap_or(0);
        let files_here = files_in_dir.get(here).map(|v| v.len()).unwrap_or(0);
        indent(out, depth)?;
        writeln!(out, "- **{}/** — {} dirs, {} files", here, kids, files_here)?;
    }

    // files directly under this directory
    if let Some(idxs) = files_in_dir.get(here) {
        for &i in idxs {
            let fe = &files[i];
            let fname = Path::new(&fe.path).file_name().unwrap().to_string_lossy();
            let role = fe.role.to_string();
            let lang = fe.lang.as_str();
            let size = humanize_bytes(fe.size as u64);
            let tags = if fe.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", fe.tags.join(", "))
            };
            let sum = fe.summary.as_deref().unwrap_or("");
            let sum = trim_summary(sum, 140);

            indent(out, depth + if here.is_empty() { 0 } else { 1 })?;
            if sum.is_empty() {
                writeln!(out, "- `{}` — role: {} • lang: {} • {}", fname, role, lang, size)?;
            } else {
                writeln!(out, "- `{}` — {} • role: {} • lang: {} • {}{}", fname, sum, role, lang, size, tags)?;
            }
        }
    }

    // recurse into child directories (sorted)
    if let Some(kids) = children_dirs.get(here) {
        for child in kids {
            render_dir(
                out,
                child,
                depth + if here.is_empty() { 0 } else { 1 },
                children_dirs,
                files_in_dir,
                files,
            )?;
        }
    }

    Ok(())
}

/* --------------------------------- helpers --------------------------------- */

fn norm(p: &Path) -> String {
    if p.as_os_str().is_empty() {
        return String::new();
    }
    p.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn join(base: &str, tail: &str) -> String {
    if base.is_empty() {
        tail.to_string()
    } else {
        format!("{}/{}", base, tail)
    }
}

fn indent(out: &mut File, depth: usize) -> std::io::Result<()> {
    for _ in 0..depth {
        write!(out, "  ")?; // 2 spaces per level
    }
    Ok(())
}

fn to_io<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
}

/// Humanize bytes like 1.2 KB, 3.4 MB (base-1024).
fn _human_bytes(n: usize) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = n as f64;
    let mut idx = 0usize;
    while size >= 1024.0 && idx + 1 < UNITS.len() {
        size /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{} {}", n, UNITS[idx])
    } else {
        format!("{:.1} {}", size, UNITS[idx])
    }
}

/// Trim summary so the tree stays skimmable (no code blocks or huge paragraphs).
fn trim_summary(s: &str, max: usize) -> String {
    let mut t = s.trim();
    if let Some(pos) = t.find('\n') {
        t = &t[..pos]; // first line only
    }
    let t = t.trim();
    if t.len() <= max {
        return t.to_string();
    }
    let mut out = t[..max].to_string();
    out.push('…');
    out
}
