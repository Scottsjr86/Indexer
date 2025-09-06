// indexer/src/chunker.rs
//! Chunk builder: converts a JSONL index (FileIntentEntry per line) into
//! GPT-ready paste chunks, enforcing token caps and splitting large files.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;
use std::{
    cmp,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::Path,
};

#[derive(Debug, Deserialize, Clone)]
struct FileIntentEntry {
    pub path: String,
    pub lang: String,
    pub sha1: String,
    pub size: usize,
    pub last_modified: String,
    pub summary: Option<String>,
    pub snippet: String,
    #[serde(default)]
    pub token_estimate: usize,
}

/// Build markdown "paste chunks" for LLMs from a JSONL index.
/// - `index_path`: path to JSONL with one FileIntentEntry per line
/// - `out_prefix`: prefix for output files, e.g. ".gpt/chunks/paste_"
/// - `token_cap`: desired approximate token cap per chunk (min 256)
pub fn chunk_index_for_gpt(index_path: &Path, out_prefix: &str, token_cap: usize) -> Result<()> {
    let token_cap = token_cap.max(256);
    const MAX_FILES_PER_CHUNK: usize = 120;
    const MAX_SECTION_CHARS: usize = 32_000;
    const TARGET_SECTION_TOKENS: usize = 800;

    if let Some(parent) = Path::new(out_prefix).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating chunks parent dir: {}", parent.display()))?;
        }
    }

    let mut entries = load_entries(index_path)
        .with_context(|| format!("reading index at {}", index_path.display()))?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let mut expanded: Vec<Part> = Vec::new();
    for e in entries.into_iter() {
        let parts = split_entry_into_parts(&e, TARGET_SECTION_TOKENS, MAX_SECTION_CHARS);
        expanded.extend(parts);
    }
    expanded.sort_by(|a, b| {
        let o = a.path.cmp(&b.path);
        if o == std::cmp::Ordering::Equal {
            a.part_idx.cmp(&b.part_idx)
        } else {
            o
        }
    });

    let mut chunk_idx = 1usize;
    let mut cur_tokens = 0usize;
    let mut cur_files = 0usize;
    let mut cur_vec: Vec<Part> = Vec::new();

    for part in expanded.into_iter() {
        let part_tokens = cmp::max(1, part.token_estimate);

        // new file if last part in the current chunk has a different path
        let will_add_new_file = match cur_vec.last() {
            Some(last) => last.path != part.path,
            None => true,
        };

        let would_overflow = cur_tokens + part_tokens > token_cap;
        let too_many_files = will_add_new_file && cur_files >= MAX_FILES_PER_CHUNK;

        if !cur_vec.is_empty() && (would_overflow || too_many_files) {
            write_chunk(out_prefix, chunk_idx, &cur_vec)?;
            chunk_idx += 1;
            cur_vec.clear();
            cur_tokens = 0;
            cur_files = 0;
        }

        cur_tokens = cur_tokens.saturating_add(part_tokens);
        if will_add_new_file {
            cur_files += 1;
        }
        cur_vec.push(part);
    }

    if !cur_vec.is_empty() {
        write_chunk(out_prefix, chunk_idx, &cur_vec)?;
    }

    Ok(())
}

/* ================================ Loading & Splitting ================================ */

fn load_entries(index_path: &Path) -> Result<Vec<FileIntentEntry>> {
    let file = File::open(index_path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[chunker] warn: failed to read line {}: {}", i + 1, e);
                continue;
            }
        };
        match serde_json::from_str::<FileIntentEntry>(&line) {
            Ok(mut entry) => {
                if entry.token_estimate == 0 {
                    entry.token_estimate = estimate_tokens_fallback(&entry.snippet);
                }
                out.push(entry);
            }
            Err(e) => {
                eprintln!("[chunker] warn: bad JSONL at line {}: {}", i + 1, e);
                continue;
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
struct Part {
    path: String,
    lang: String,
    sha1: String,
    size: String,
    last_modified: String,
    summary: Option<String>,
    part_idx: usize,
    part_total: usize,
    body: String,
    token_estimate: usize,
}

fn split_entry_into_parts(
    e: &FileIntentEntry,
    target_tokens: usize,
    hard_char_cap: usize,
) -> Vec<Part> {
    let snippet = if e.snippet.len() > hard_char_cap {
        let mut s = String::with_capacity(hard_char_cap + 64);
        s.push_str(&e.snippet[..hard_char_cap]);
        s.push_str("\n// … [truncated]");
        s
    } else {
        e.snippet.clone()
    };

    let total_est = estimate_tokens_fallback(&snippet);
    if total_est <= target_tokens {
        return vec![Part {
            path: e.path.clone(),
            lang: e.lang.clone(),
            sha1: e.sha1.clone(),
            size: e.size.to_string(),
            last_modified: e.last_modified.clone(),
            summary: e.summary.clone(),
            part_idx: 1,
            part_total: 1,
            body: snippet,
            token_estimate: total_est,
        }];
    }

    let mut parts = Vec::new();
    let mut acc = String::new();
    let mut idx = 1usize;

    for line in snippet.lines() {
        if !acc.is_empty() {
            acc.push('\n');
        }
        acc.push_str(line);

        let acc_tokens = estimate_tokens_fallback(&acc);
        if acc_tokens >= target_tokens {
            parts.push(Part {
                path: e.path.clone(),
                lang: e.lang.clone(),
                sha1: e.sha1.clone(),
                size: e.size.to_string(),
                last_modified: e.last_modified.clone(),
                summary: None, // summary on first part only
                part_idx: idx,
                part_total: 0, // fill later
                body: acc.clone(),
                token_estimate: acc_tokens,
            });
            acc.clear();
            idx += 1;
        }
    }

    if !acc.is_empty() {
        let token_estimate = estimate_tokens_fallback(&acc);
        parts.push(Part {
            path: e.path.clone(),
            lang: e.lang.clone(),
            sha1: e.sha1.clone(),
            size: e.size.to_string(),
            last_modified: e.last_modified.clone(),
            summary: None,
            part_idx: idx,
            part_total: 0,
            body: acc,
            token_estimate,
        });
    }

    if let Some(first) = parts.first_mut() {
        first.summary = e.summary.clone();
    }

    let total = parts.len();
    for p in &mut parts {
        p.part_total = total;
    }
    parts
}

/* ================================== Rendering ====================================== */

fn write_chunk(out_prefix: &str, idx: usize, parts: &[Part]) -> Result<()> {
    let path = format!("{}{}.md", out_prefix, idx);
    let mut out = File::create(&path).with_context(|| format!("create {}", path))?;

    let total_parts = parts.len();
    let total_files = count_unique_files(parts);
    let approx_tokens: usize = parts.iter().map(|p| cmp::max(1, p.token_estimate)).sum();

    writeln!(out, "# GPT Paste Chunk {}\n", idx)?;
    writeln!(out, "> generated: {}", Utc::now().to_rfc3339())?;
    writeln!(out, "> files: {}  •  parts: {}  •  ~tokens: {}", total_files, total_parts, approx_tokens)?;
    writeln!(out)?;

    let mut i = 0usize;
    while i < parts.len() {
        let start = i;
        let path_here = &parts[i].path;
        while i < parts.len() && parts[i].path == *path_here {
            i += 1;
        }
        render_file_section(&mut out, &parts[start..i])?;
    }

    Ok(())
}

fn render_file_section(out: &mut File, parts: &[Part]) -> Result<()> {
    if parts.is_empty() {
        return Ok(());
    }
    let meta = &parts[0];
    let multi = parts.len() > 1 || meta.part_total > 1;
    let title = if multi {
        format!("`{}` [{}] ({} parts)", meta.path, meta.lang, meta.part_total)
    } else {
        format!("`{}` [{}]", meta.path, meta.lang)
    };
    writeln!(out, "## {}\n", title)?;
    writeln!(
        out,
        "- sha1: `{}` • size: {} • mtime: {}",
        meta.sha1, meta.size, meta.last_modified
    )?;

    if let Some(sum) = &meta.summary {
        if !sum.trim().is_empty() {
            writeln!(out, "**Summary:** {}", sum.trim())?;
        }
    }

    for p in parts {
        if multi {
            writeln!(out, "\n**Part {}/{}**", p.part_idx, p.part_total)?;
        }
        let fence = fence_lang(&p.lang);
        writeln!(out, "```{}\n{}\n```", fence, p.body)?;
    }

    writeln!(out)?;
    Ok(())
}

/* ================================== Heuristics ===================================== */

fn estimate_tokens_fallback(s: &str) -> usize {
    let chars = s.len();
    (chars / 4).max(12)
}

fn fence_lang<'a>(lang: &'a str) -> &'a str {
    let l = lang.trim();
    if l.is_empty() {
        ""
    } else if l.eq_ignore_ascii_case("rs") || l.eq_ignore_ascii_case("rust") {
        "rust"
    } else if l.eq_ignore_ascii_case("ts") || l.eq_ignore_ascii_case("typescript") {
        "ts"
    } else if l.eq_ignore_ascii_case("js") || l.eq_ignore_ascii_case("javascript") {
        "javascript"
    } else if l.eq_ignore_ascii_case("py") || l.eq_ignore_ascii_case("python") {
        "python"
    } else if l.eq_ignore_ascii_case("go") || l.eq_ignore_ascii_case("golang") {
        "go"
    } else if l.eq_ignore_ascii_case("sh") || l.eq_ignore_ascii_case("bash") || l.eq_ignore_ascii_case("zsh") {
        "bash"
    } else if l.eq_ignore_ascii_case("c") || l.eq_ignore_ascii_case("h") {
        "c"
    } else if l.eq_ignore_ascii_case("cpp") || l.eq_ignore_ascii_case("cc")
        || l.eq_ignore_ascii_case("cxx") || l.eq_ignore_ascii_case("hpp")
    {
        "cpp"
    } else if l.eq_ignore_ascii_case("java") {
        "java"
    } else if l.eq_ignore_ascii_case("md") || l.eq_ignore_ascii_case("markdown") {
        "md"
    } else if l.eq_ignore_ascii_case("toml") {
        "toml"
    } else if l.eq_ignore_ascii_case("yaml") || l.eq_ignore_ascii_case("yml") {
        "yaml"
    } else if l.eq_ignore_ascii_case("json") || l.eq_ignore_ascii_case("jsonl") {
        "json"
    } else {
        l
    }
}

/* ================================== Utilities ====================================== */

fn count_unique_files(parts: &[Part]) -> usize {
    let mut n = 0usize;
    let mut last: Option<&str> = None;
    for p in parts {
        if last.map(|s| s != p.path.as_str()).unwrap_or(true) {
            n += 1;
            last = Some(&p.path);
        }
    }
    n
}

/* ===================================== Tests ======================================= */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_estimator_floor() {
        assert!(estimate_tokens_fallback("") >= 12);
        assert!(estimate_tokens_fallback("abcd") >= 12);
        assert!(estimate_tokens_fallback(&"x".repeat(400)) >= 100);
    }

    #[test]
    fn split_small_is_single_part() {
        let e = FileIntentEntry {
            path: "src/main.rs".into(),
            lang: "rs".into(),
            sha1: "deadbeef".into(),
            size: 2 * 1024, // 2048 bytes
            last_modified: "1234567890".into(),
            summary: Some("entrypoint".into()),
            snippet: "fn main() {}".into(),
            token_estimate: 0,
        };
        let parts = split_entry_into_parts(&e, 800, 32_000);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].part_idx, 1);
        assert_eq!(parts[0].part_total, 1);
        assert!(parts[0].summary.is_some());
    }

    #[test]
    fn split_large_makes_multiple_parts() {
        let mut body = String::new();
        for _ in 0..5000 {
            body.push_str("let x = 1;\n");
        }
        let e = FileIntentEntry {
            path: "src/lib.rs".into(),
            lang: "rust".into(),
            sha1: "cafebabe".into(),
            size: 120 * 1024, // 122_880 bytes
            last_modified: "1234567890".into(),
            summary: Some("lib".into()),
            snippet: body,
            token_estimate: 0,
        };
        let parts = split_entry_into_parts(&e, 800, 32_000);
        assert!(parts.len() > 1);
        assert!(parts[0].summary.is_some());
        assert!(parts.iter().skip(1).all(|p| p.summary.is_none()));
        assert!(parts.iter().all(|p| p.part_total == parts.len()));
    }
}
