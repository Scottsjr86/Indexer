// indexer/src/map_view.rs

use std::{
    collections::{
        BTreeMap, 
        BTreeSet
    },
    fs::{self, 
        File
    },
    io::{
        BufRead, 
        BufReader, 
        Write
    },
    path::{
        Path, 
        PathBuf
    },
};
use crate::{
    file_intent_entry::FileIntentEntry
};

/// Build a hierarchical, skim-friendly project map from a JSONL index.
/// - Groups by top-level directory
/// - Summaries are clamped to a single tight line
/// - Per-directory caps (with "+N more…" footer) keep output readable
/// - Skips obvious noise directories (configurable below)
pub fn build_map_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
    // Ensure parent dir exists
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    // ===== Ingest (streaming) =====
    let f = File::open(index_path)?;
    let rdr = BufReader::new(f);

    // Group by top-level directory
    let mut groups: BTreeMap<String, Vec<EntryLite>> = BTreeMap::new();
    let mut all_tags: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;
    #[allow(unused_variables)]
    for (i, line) in rdr.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue, // skip bad read
        };
        let entry: FileIntentEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue, // skip malformed JSONL line
        };
        total += 1;

        let path_str = entry.path.clone();
        let (group, rel) = split_top(&path_str);
        let mut tags = entry.tags.clone();
        tags.sort();
        tags.dedup();

        // record tag freqs for header rollup
        for t in &tags {
            *all_tags.entry(t.to_string()).or_insert(0) += 1;
        }

        let sum = clamp_summary(entry.summary.as_deref().unwrap_or_default());

        groups
            .entry(group)
            .or_default()
            .push(EntryLite { path: rel, lang: entry.lang, summary: sum, tags });
    }

    // Sort each group deterministically
    for v in groups.values_mut() {
        v.sort_by(|a, b| a.path.cmp(&b.path));
    }

    // ===== Emit =====
    let mut out = File::create(output_path)?;
    let (rollup_line, tag_count) = top_k_tags(&all_tags, 12);
    writeln!(out, "# Project File Map\n")?;
    writeln!(
        out,
        "> Files: {}  •  Groups: {}  •  Tag varieties: {}",
        total,
        groups.len(),
        tag_count
    )?;
    if !rollup_line.is_empty() {
        writeln!(out, "> Tags: {}\n", rollup_line)?;
    } else {
        writeln!(out)?;
    }

    // Noise groups (don’t spam output). Add/remove to taste.
    let noise_groups: BTreeSet<&'static str> = [
        "target", "node_modules", ".git", ".github", ".idea", ".vscode",
        ".cargo", ".venv", "venv", "dist", "build", "out",
    ].into_iter().collect();

    // Per-group listing cap
    const LIST_CAP: usize = 120;

    for (group, entries) in groups {
        if noise_groups.contains(group.as_str()) {
            // Skip noisy infra entirely; uncomment if you want a collapsed line instead.
            // writeln!(out, "## `{}/` (skipped noisy infra, {} files)\n", group, entries.len())?;
            continue;
        }
        writeln!(out, "## `{}/`  _(files: {})_\n", group, entries.len())?;

        // Optional sub‑section tag rollup per group
        let mut gtags: BTreeMap<String, usize> = BTreeMap::new();
        for e in &entries {
            for t in &e.tags {
                *gtags.entry(t.clone()).or_insert(0) += 1;
            }
        }
        let (groll, _) = top_k_tags(&gtags, 8);
        if !groll.is_empty() {
            writeln!(out, "> Tags: {}\n", groll)?;
        }

        let mut shown = 0usize;
        for e in entries.iter().take(LIST_CAP) {
            let tag_str = if e.tags.is_empty() {
                String::new()
            } else {
                format!(" _[{}]_", e.tags.join(", "))
            };
            let sum_suffix = if e.summary.is_empty() {
                String::new()
            } else {
                format!(" — {}", e.summary)
            };

            // Show relative path inside the group
            writeln!(out, "- `{}` [{}]{}{}", e.path, e.lang, tag_str, sum_suffix)?;
            shown += 1;
        }

        if shown < entries.len() {
            writeln!(out, "\n_… {} more in `{}/` (use tree view or open the index for full list)_\n",
                entries.len() - shown, group)?;
        } else {
            writeln!(out)?;
        }
    }

    Ok(())
}

/* ----------------------------- helpers ----------------------------- */

#[derive(Clone)]
struct EntryLite {
    path: String,   // relative path inside the group
    lang: String,
    summary: String,
    tags: Vec<String>,
}

fn split_top(path: &str) -> (String, String) {
    let pb = PathBuf::from(path);
    let mut comps = pb.components();

    let first = comps.next();
    match first {
        Some(c) => {
            let top = c.as_os_str().to_string_lossy().to_string();
            let rel = comps.as_path().to_string_lossy().to_string();
            if rel.is_empty() { (top, String::from(".")) } else { (top, rel) }
        }
        None => (String::from("."), String::from(".")),
    }
}

fn clamp_summary(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() { return String::new(); }
    // collapse whitespace + clamp to a single, short sentence
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_ellipsis(&s, 140)
}

fn truncate_ellipsis(s: &str, max: usize) -> String {
    if s.len() <= max { return s.to_string(); }
    let mut out = s[..max].to_string();
    out.push_str("…");
    out
}

fn top_k_tags(freq: &BTreeMap<String, usize>, k: usize) -> (String, usize) {
    let mut v: Vec<(&str, usize)> = freq.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let shown = v.iter().take(k).map(|(t, n)| format!("`{}`({})", t, n)).collect::<Vec<_>>().join(", ");
    (shown, freq.len())
}
