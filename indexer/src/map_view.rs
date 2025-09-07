//! Combined Project Map (with tree-lite appendix)
//!
//! - Top section: tag-rich grouped catalog by top-level dir (old MAP).
//! - Appendix: compact hierarchical tree (old TREE), same output file.
//!
//! Output path example: `.gpt_index/maps/<slug>_PROJECT_MAP.md`

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use crate::file_intent_entry::FileIntentEntry;
use crate::util;

/// Public entrypoint: build the combined MAP (+ tree-lite) into `output_path`.
pub fn build_map_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
    // Ensure parent dir exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let entries = load_entries(index_path)?;

    // ---- GROUPED (MAP) ----
    let mut groups: BTreeMap<String, Vec<EntryLite>> = BTreeMap::new();
    let mut tag_freq_global: BTreeMap<String, usize> = BTreeMap::new();

    for e in &entries {
        let (group, rel) = split_top(&e.path);
        let tags = normalize_tags(&e.tags);
        for t in &tags {
            *tag_freq_global.entry(t.clone()).or_insert(0) += 1;
        }
        groups.entry(group).or_default().push(EntryLite {
            path: rel,
            lang: e.lang.clone(),
            summary: clamp_summary(e.summary.as_deref().unwrap_or("")),
            tags,
        });
    }

    // Compute global counts
    let files_total = entries.len();
    let groups_total = groups.len();
    let tag_k = top_k_tags(&tag_freq_global, 14); // present up to 14 tag buckets

    // ---- TREE-LITE (build a directory tree) ----
    let dir_tree = build_tree(&entries);

    // ---- Render combined doc ----
    let mut out = File::create(output_path)?;
    writeln!(out, "# indexer Project Map (combined)")?;
    writeln!(out)?;
    writeln!(
        out,
        "_Legend: grouped by top-level dir • each line = `rel/path [lang] — summary` • tags shown when present_"
    )?;
    writeln!(out)?;
    writeln!(
        out,
        "> Files: {}  •  Groups: {}  •  Tag varieties: {}",
        files_total,
        groups_total,
        tag_freq_global.len()
    )?;
    writeln!(out, "> Tags: {}", tag_k.0)?;
    writeln!(out)?;

    // ---- RENDER: MAP groups ----
    for (group, mut list) in groups {
        // sort by path within group
        list.sort_by(|a, b| a.path.cmp(&b.path));

        let files_n = list.len();
        let mut local_tags: BTreeMap<String, usize> = BTreeMap::new();
        for e in &list {
            for t in &e.tags {
                *local_tags.entry(t.clone()).or_insert(0) += 1;
            }
        }
        let tag_line = top_k_tags(&local_tags, 10).0;

        writeln!(out, "## `{}/`  _(files: {})_", group, files_n)?;
        if !tag_line.is_empty() {
            writeln!(out, "")?;
            writeln!(out, "> Tags: {}", tag_line)?;
        }
        writeln!(out, "")?;
        for e in &list {
            if e.tags.is_empty() {
                writeln!(out, "- `{}` [{}] — {}", e.path, e.lang, e.summary)?;
            } else {
                writeln!(
                    out,
                    "- `{}` [{}] _[{}]_ — {}",
                    e.path,
                    e.lang,
                    e.tags.join(", "),
                    e.summary
                )?;
            }
        }
        writeln!(out)?;
    }

    // ---- RENDER: TREE-LITE appendix ----
    writeln!(out, "---")?;
    writeln!(out, "")?;
    writeln!(out, "## Appendix: Directory Tree")?;
    writeln!(out, "")?;
    writeln!(out, "> Compact, skimmable tree with size/lang and one-line summary")?;
    writeln!(out, "")?;

    // root children in sorted order
    render_tree(&mut out, &dir_tree, "", 0)?;

    Ok(())
}

/* ============================
   Internals
   ============================ */

#[derive(Clone)]
struct EntryLite {
    path: String,   // relative to top-level group
    lang: String,
    summary: String,
    tags: Vec<String>,
}

fn load_entries(index_path: &Path) -> std::io::Result<Vec<FileIntentEntry>> {
    let f = File::open(index_path)?;
    let br = BufReader::new(f);
    let mut v = Vec::new();

    for (i, line) in br.lines().enumerate() {
        let line = match line {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[map] warn: bad JSONL at line {}: {}", i + 1, e);
                continue;
            }
        };
        match serde_json::from_str::<FileIntentEntry>(&line) {
            Ok(e) => v.push(e),
            Err(e) => {
                eprintln!("[map] warn: bad JSONL at line {}: {}", i + 1, e);
                continue;
            }
        }
    }
    Ok(v)
}

/// Return (top_level_dir, remainder_relative_path).
/// For paths with no '/', group = "Cargo.toml" dir-like marker (".") and rel="filename".
fn split_top(path: &str) -> (String, String) {
    let pb = PathBuf::from(path);
    let comp: Vec<_> = pb.components().collect();
    if comp.len() <= 1 {
        (String::from("Cargo.toml"), String::from("."))
    } else {
        let group = comp[0].as_os_str().to_string_lossy().to_string();
        let rel = pb
            .iter()
            .skip(1)
            .collect::<PathBuf>()
            .to_string_lossy()
            .to_string();
        (group, if rel.is_empty() { ".".into() } else { rel })
    }
}

fn clamp_summary(s: &str) -> String {
    truncate_ellipsis(s.trim(), 140)
}

fn truncate_ellipsis(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut t = s[..max].to_string();
    t.push('…');
    t
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    // keep order, dedup
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for t in tags {
        if seen.insert(t.to_string()) {
            out.push(t.to_string());
        }
    }
    out
}

fn top_k_tags(freq: &BTreeMap<String, usize>, k: usize) -> (String, usize) {
    let mut v: Vec<(&str, usize)> = freq.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let mut items = Vec::new();
    for (i, (tag, n)) in v.iter().enumerate() {
        if i >= k {
            break;
        }
        items.push(format!("`{}`({})", tag, n));
    }
    (items.join(", "), v.len())
}

/* -------- TREE-LITE builder -------- */

#[derive(Default)]
struct DirNode {
    // name is implied by map key; this holds children and file entries
    subdirs: BTreeMap<String, DirNode>,
    files: Vec<TreeFile>, // sorted by filename
}

#[derive(Clone)]
struct TreeFile {
    name: String,  // filename only
    lang: String,
    size: usize,
    summary: String,
    
    
}

/// Build a directory tree rooted at "" from entries.
fn build_tree(entries: &[FileIntentEntry]) -> DirNode {
    let mut root = DirNode::default();
    for e in entries {
        let p = Path::new(&e.path);
        let mut cur = &mut root;

        if let Some(parent) = p.parent() {
            for comp in parent {
                let key = comp.to_string_lossy().to_string();
                cur = cur.subdirs.entry(key).or_default();
            }
        }

        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        cur.files.push(TreeFile {
            name,
            lang: e.lang.clone(),
            size: e.size,
            summary: clamp_summary(e.summary.as_deref().unwrap_or("")),                        
        });
    }

    // sort files for each node
    fn sort_node(n: &mut DirNode) {
        n.files.sort_by(|a, b| a.name.cmp(&b.name));
        for (_k, v) in n.subdirs.iter_mut() {
            sort_node(v);
        }
    }
    let mut root_mut = root;
    sort_node(&mut root_mut);
    root_mut
}

/// Render the tree to markdown (indented bullet list).
fn render_tree(out: &mut File, node: &DirNode, base: &str, depth: usize) -> std::io::Result<()> {
    // render current dir header only if depth==0 (root) or base not empty
    if depth == 0 && !base.is_empty() {
        writeln!(out, "- **{}/**", base)?;
    }

    // subdirectories first
    for (name, child) in &node.subdirs {
        let path = if base.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", base, name)
        };

        // count totals: number of immediate files in child
        let files_n = child.files.len();
        let subdirs_n = child.subdirs.len();
        writeln!(
            out,
            "{}- **{}/** — {} dirs, {} files",
            indent(depth),
            path,
            subdirs_n,
            files_n
        )?;
        render_tree(out, child, &path, depth + 1)?;
    }

    // files in this dir
    for f in &node.files {
        writeln!(
            out,
            "{}- `{}` — {} • {} • {}",
            indent(depth + 1),
            f.name,
            f.lang,
            util::humanize_bytes(f.size as u64),
            f.summary
        )?;
    }

    Ok(())
}

pub fn indent(depth: usize) -> String {
    let mut s = String::new();
    for _ in 0..depth {
        s.push_str("  ");
    }
    s
}
