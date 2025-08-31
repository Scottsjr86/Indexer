// indexer/src/tree_view.rs

use anyhow::Context;
use std::{
    collections::{
        BTreeMap, 
        BTreeSet
    }
};
use std::fs::File;
use std::io::{
    BufRead, 
    BufReader, 
    Write
};
use std::path::{
    Path
};
use crate::{
    file_intent_entry::FileIntentEntry
};

/// Build a hierarchical markdown tree from the JSONL index.
///
/// Output format (example):
/// - src/
///   - commands.rs — Run CLI entrypoints [rust]
///   - tree_view.rs — Builds the directory tree [rust]
/// - README.md — Project overview [md]
pub fn build_tree_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
    // ---- load index ----
    let f = File::open(index_path)?;
    let rdr = BufReader::new(f);
    let mut files: Vec<FileIntentEntry> = Vec::new();
    for line in rdr.lines() {
        let entry: FileIntentEntry = serde_json::from_str(&line?)
            .context("Failed to parse FileIntentEntry from index line")
            .map_err(to_io)?;
        files.push(entry);
    }

    // ---- build dir graph: dir -> {child_dirs}, dir -> [file_indices] ----
    // We keep files in a single Vec and store indices to avoid cloning big snippets.
    let mut files_in_dir: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut children_dirs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    // ensure root exists
    children_dirs.entry(String::new()).or_default();

    for (idx, fe) in files.iter().enumerate() {
        let rel = Path::new(&fe.path);
        let parent = rel.parent().map(|p| norm(p)).unwrap_or_else(String::new);

        // register file under its parent dir
        files_in_dir.entry(parent.clone()).or_default().push(idx);

        // register all intermediate directories so the graph is complete
        let mut acc = String::new(); // "" for root
        if let Some(dir) = rel.parent() {
            for comp in dir.components() {
                let next = join(&acc, &comp.as_os_str().to_string_lossy());
                children_dirs.entry(acc.clone()).or_default().insert(next.clone());
                children_dirs.entry(next.clone()).or_default(); // ensure node exists
                acc = next;
            }
        }
    }

    // sort files in each dir by file name
    for (_dir, idxs) in files_in_dir.iter_mut() {
        idxs.sort_by(|&a, &b| {
            let pa = Path::new(&files[a].path).file_name().unwrap().to_string_lossy();
            let pb = Path::new(&files[b].path).file_name().unwrap().to_string_lossy();
            pa.cmp(&pb)
        });
    }

    // ---- render markdown ----
    let mut out = File::create(output_path)?;
    writeln!(out, "# Project File Tree\n")?;

    fn render_dir(
        out: &mut File,
        here: &str,
        depth: usize,
        children_dirs: &BTreeMap<String, BTreeSet<String>>,
        files_in_dir: &BTreeMap<String, Vec<usize>>,
        files: &[FileIntentEntry],
    ) -> std::io::Result<()> {
        // print current directory header (skip printing root line, only its children)
        if !here.is_empty() {
            indent(out, depth)?;
            writeln!(out, "- **{}/**", if here.is_empty() { "/" } else { &here })?;
        }

        // files directly under this directory
        if let Some(idxs) = files_in_dir.get(here) {
            for &i in idxs {
                let fe = &files[i];
                indent(out, depth + if here.is_empty() { 0 } else { 1 })?;
                let fname = Path::new(&fe.path).file_name().unwrap().to_string_lossy();
                let tags = if fe.tags.is_empty() {
                    String::new()
                } else {
                    format!(" _[{}]_", fe.tags.join(", "))
                };
                let sum = fe.summary.as_deref().unwrap_or("");
                if sum.is_empty() {
                    writeln!(out, "- `{}`{}", fname, tags)?;
                } else {
                    writeln!(out, "- `{}`{} — {}", fname, tags, sum)?;
                }
            }
        }

        // recurse into child directories (sorted by normalized path)
        if let Some(kids) = children_dirs.get(here) {
            for child in kids {
                render_dir(out, child, depth + if here.is_empty() { 0 } else { 1 }, children_dirs, files_in_dir, files)?;
            }
        }

        Ok(())
    }

    render_dir(&mut out, "", 0, &children_dirs, &files_in_dir, &files)?;
    Ok(())
}

// ---------- helpers ----------
fn norm(p: &Path) -> String {
    if p.as_os_str().is_empty() { return String::new(); }
    p.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn join(base: &str, tail: &str) -> String {
    if base.is_empty() { tail.to_string() } else { format!("{}/{}", base, tail) }
}

fn indent(out: &mut File, depth: usize) -> std::io::Result<()> {
    for _ in 0..depth { write!(out, "  ")?; }
    Ok(())
}

fn to_io<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
}
