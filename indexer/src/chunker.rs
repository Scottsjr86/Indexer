// indexer/src/chunker.rs

use anyhow::{
    Context, 
    Result
};
use chrono::Utc;
use std::{
    cmp,
    fs::{self, 
        File},
    io::{
        BufRead, 
        BufReader, 
        Write},
    path::Path,
};
use crate::{
    file_intent_entry::FileIntentEntry
};


/// Build markdown "paste chunks" for LLMs from a JSONL index (streaming, robust).
/// Backwards-compatible with existing outputs like `chunks/paste_1.md`.  // README/outputs match:contentReference[oaicite:2]{index=2}:contentReference[oaicite:3]{index=3}
pub fn chunk_index_for_gpt(index_path: &Path, out_prefix: &str, token_cap: usize) -> Result<()> {
    // Hard floor so we never infinite-loop on tiny caps.
    let token_cap = token_cap.max(256);

    // Ensure parent dir exists if user passed a prefix like ".gpt_index/chunks/paste_"
    if let Some(parent) = Path::new(out_prefix).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating chunks parent dir: {}", parent.display()))?;
        }
    }

    // Stream read the JSONL index (one FileIntentEntry per line).  // JSONL contract from your indexer:contentReference[oaicite:4]{index=4}
    let file = File::open(index_path)
        .with_context(|| format!("opening index at {}", index_path.display()))?;
    let reader = BufReader::new(file);

    // Collect to allow optional deterministic sort by path; still O(n) memory but you can toggle off.
    // If you want strict streaming (no buffering), set `SORT_BEFORE_CHUNK` to false.
    const SORT_BEFORE_CHUNK: bool = true;
    let mut entries: Vec<FileIntentEntry> = Vec::new();

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
                // Defensive clamps
                if entry.token_estimate == 0 {
                    entry.token_estimate = estimate_tokens_fallback(&entry.snippet);
                }
                entries.push(entry);
            }
            Err(e) => {
                eprintln!("[chunker] warn: bad JSONL at line {}: {}", i + 1, e);
                continue;
            }
        }
    }

    if SORT_BEFORE_CHUNK {
        entries.sort_by(|a, b| a.path.cmp(&b.path));
    }

    // Chunk loop
    let mut chunk_idx: usize = 1;
    let mut cur_tokens: usize = 0;
    let mut cur: Vec<FileIntentEntry> = Vec::new();

    // Practical guard: avoid pathological giant chunks even if token_estimate undercounts.
    const MAX_FILES_PER_CHUNK: usize = 120;

    for entry in entries.into_iter() {
        let entry_tokens = cmp::max(1, entry.token_estimate);

        let would_overflow = cur_tokens + entry_tokens > token_cap;
        let too_many_files = cur.len() >= MAX_FILES_PER_CHUNK;

        if (!cur.is_empty()) && (would_overflow || too_many_files) {
            write_chunk(out_prefix, chunk_idx, &cur, cur_tokens)?;
            chunk_idx += 1;
            cur.clear();
            cur_tokens = 0;
        }

        cur_tokens = cur_tokens.saturating_add(entry_tokens);
        cur.push(entry);
    }

    if !cur.is_empty() {
        write_chunk(out_prefix, chunk_idx, &cur, cur_tokens)?;
    }

    Ok(())
}

/// Conservative fallback token estimator if index line forgot to set it.
fn estimate_tokens_fallback(s: &str) -> usize {
    // Rough: ~1 token per 4 chars; minimum 12 to avoid zero.
    let chars = s.len();
    let est = (chars / 4).max(12);
    est
}

/// Write one chunk file with rich header + per-file sections.
fn write_chunk(
    out_prefix: &str,
    idx: usize,
    files: &[FileIntentEntry],
    total_tokens: usize,
) -> Result<()> {
    let path = format!("{}{}.md", out_prefix, idx);
    let mut out = File::create(&path).with_context(|| format!("create {}", path))?;

    // Header with metadata (stable & skim-friendly).
    writeln!(out, "# GPT Paste Chunk {}\n", idx)?;
    writeln!(
        out,
        "> Generated: {}  \n> Files: {}  •  ~Tokens: {}",
        Utc::now().to_rfc3339(),
        files.len(),
        total_tokens
    )?;
    writeln!(out)?;

    for f in files {
        // File header: keep quick context for LLM and humans.
        writeln!(out, "## `{}` [{}]", f.path, f.lang)?;
        writeln!(
            out,
            "- sha1: `{}` • size: {} • mtime: {}",
            f.sha1, f.size, f.last_modified
        )?;

        if let Some(sum) = &f.summary {
            if !sum.trim().is_empty() {
                writeln!(out, "**Summary:** {}", sum.trim())?;
            }
        }

        // Fences: sanitize language, trim absurdly long snippets defensively.
        let fence_lang = fence_lang(&f.lang);
        let snippet = trim_snippet(&f.snippet, 32_000); // hard cap ~32k chars per file section
        writeln!(out, "```{}\n{}\n```\n", fence_lang, snippet)?;
    }

    Ok(())
}

/// Normalize fence language to something Markdown renderers recognize.
// Normalize fence language to something Markdown renderers recognize.
// Returns either a 'static literal or a slice of the input (no alloc).
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
           || l.eq_ignore_ascii_case("cxx") || l.eq_ignore_ascii_case("hpp") {
        "cpp"
    } else if l.eq_ignore_ascii_case("java") {
        "java"
    } else if l.eq_ignore_ascii_case("md") || l.eq_ignore_ascii_case("markdown") {
        "md"
    } else {
        // Fallback: hand back the original (trimmed) slice — same lifetime as input.
        l
    }
}


/// Trim monstrous snippets but keep code fences valid.
fn trim_snippet(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    let mut out = String::with_capacity(max_chars + 64);
    out.push_str(&s[..max_chars]);
    out.push_str("\n// … [truncated]");
    out
}
