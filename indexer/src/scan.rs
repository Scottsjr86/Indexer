// indexer/src/scan.rs
//! Repo scanner: walks the tree, applies ignores, detects language, splits polyglot
//! containers (HTML), extracts snippets/metadata, and writes a JSONL index.
//!
//! Distinctions vs. old version:
//! - Uses util::ext_to_lang + shebang for better lang map (Rust, Python, Java, HTML, CSS, JS/TS, etc.).
//! - Skips binaries via util::is_probably_binary.
//! - Splits HTML into virtual sub-entries for <script> (js/ts) and <style> (css).
//! - Deterministic sorting; safer writing via util::safe_write.
//! - Tunable limits via ScanOptions.
//! - Extra tags (role/module/imports/exports) preserved.

use anyhow::{Context, Result};
use ignore::{gitignore::GitignoreBuilder, WalkBuilder};
use sha1::{Digest, Sha1};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path},
};

use crate::{
    file_intent_entry::FileIntentEntry,
    helpers::{infer_module_id, infer_role, skim_symbols},
    intent,
    snippet,
    util, // ext_to_lang, is_probably_binary, infer_tags, to_unix_epoch, safe_write
};

/* ================================= Config ================================= */

/// Scan configuration knobs.
#[derive(Clone, Debug)]
pub struct ScanOptions {
    /// Hard cap per file (bytes). Files larger than this are skipped.
    pub max_file_bytes: u64,
    /// Head bytes for binary sniff.
    pub sniff_bytes: usize,
    /// Snippet source window (bytes).
    pub snippet_bytes: usize,
    /// Follow symlinks?
    pub follow_symlinks: bool,
    /// Include common config/doc types (json/toml/yaml/md)?
    pub include_docs_and_configs: bool,
    /// If true, split HTML into sub-entries for <script>/<style>.
    pub split_html_embeds: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_file_bytes: 512_000, // ~0.5MB
            sniff_bytes: 4096,
            snippet_bytes: 32 * 1024,
            follow_symlinks: false,
            include_docs_and_configs: true,
            split_html_embeds: true,
        }
    }
}

/* ============================= Public API ============================== */

/// Scan repo and write JSONL index file at `out`.
pub fn scan_and_write_index(root: &Path, out: &Path) -> Result<Vec<FileIntentEntry>> {
    let mut entries = index_project_with_opts(root, &ScanOptions::default())?;
    // Deterministic output: sort by path, then lang, then module-ish fields
    entries.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.lang.cmp(&b.lang))
            .then(a.module.cmp(&b.module))
    });

    // Write atomically
    let mut buf = Vec::with_capacity(entries.len() * 256);
    for entry in &entries {
        writeln!(&mut buf, "{}", serde_json::to_string(entry)?)?;
    }
    util::safe_write(out, buf)?;
    Ok(entries)
}

/// Default indexer with sane options.
pub fn index_project(root: &Path) -> Result<Vec<FileIntentEntry>> {
    index_project_with_opts(root, &ScanOptions::default())
}

/// Full-control indexer.
pub fn index_project_with_opts(root: &Path, opts: &ScanOptions) -> Result<Vec<FileIntentEntry>> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let matcher = build_ignore_matcher(&root)?;
    let walker = {
        let mut w = WalkBuilder::new(&root);
        w.standard_filters(true);
        w.follow_links(opts.follow_symlinks);
        w.build()
    };

    let mut out = Vec::<FileIntentEntry>::new();

    for dent in walker.filter_map(|e| e.ok()) {
        let path = dent.path();
        if !path.is_file() {
            continue;
        }
        let rel_path = normalize_rel(&root, path);

        // Respect ignore rules
        if matcher.matched(&rel_path, false).is_ignore() {
            continue;
        }

        // Skip obvious noise dirs (defense-in-depth; standard_filters already helps)
        if is_noise_path(&rel_path) {
            continue;
        }

        // Metadata / size gate
        let meta = dent.metadata()?;
        let size = meta.len();
        if size == 0 || size > opts.max_file_bytes {
            continue;
        }

        // Binary sniff
        if file_is_probably_binary(path, opts.sniff_bytes)? {
            continue;
        }

        // Detect language
        let lang = detect_lang(path)?;
        if lang_is_doc_or_config(&lang) && !opts.include_docs_and_configs {
            continue;
        }
        if lang == "txt" {
            // extremely generic — typically skip unless user opts in
            continue;
        }

        // Read content (UTF-8 only; non-UTF-8 we treat as binary-like for our purposes)
        let content = read_utf8(path).with_context(|| format!("read {}", path.display()))?;
        if content.is_empty() {
            continue;
        }

        // SHA1 across the full content buffer
        let sha1 = sha1_hex(content.as_bytes());

        // Snippet window
        let snip_src = slice_prefix(&content, opts.snippet_bytes);
        let mut base_snip = snippet::extract_relevant_snippet(snip_src, &lang);

        // HTML splitting: produce sub-entries for <script> / <style>
        if opts.split_html_embeds && (lang == "html" || lang == "htm") {
            let mut any_split = false;
            for (i, block) in extract_html_blocks(&content).into_iter().enumerate() {
                any_split = true;
                let virt_path = format!("{}#{}", &rel_path, block.id_for_path(i + 1));
                let virt_lang = block.lang.to_string();
                let snip = snippet::extract_relevant_snippet(
                    slice_prefix(block.body, opts.snippet_bytes),
                    &virt_lang,
                );

                let (entry, _) = build_entry(
                    &virt_path,
                    &virt_lang,
                    &sha1,               // keep same full-file sha; sub-blocks reference parent file bytes
                    size as usize,
                    &meta,
                    &snip,
                    &content,            // use full content for line counts to keep math simple
                    Some((&rel_path, &lang)),
                );

                out.push(entry);
            }

            // Also include a parent HTML entry with a compact summary/snippet of structure
            if any_split {
                base_snip = html_structure_preview(&content, opts.snippet_bytes);
            }
        }

        // Base file entry (for non-html or additionally for html)
        let (entry, skip_base) = build_entry(
            &rel_path,
            &lang,
            &sha1,
            size as usize,
            &meta,
            &base_snip,
            &content,
            None,
        );
        if !skip_base {
            out.push(entry);
        }
    }

    Ok(out)
}

/* ============================== Core helpers =============================== */

fn build_ignore_matcher(root: &Path) -> Result<ignore::gitignore::Gitignore> {
    let mut gitignore = GitignoreBuilder::new(root);
    if root.join(".gitignore").exists() {
        gitignore.add(".gitignore");
    }
    if root.join(".gptignore").exists() {
        gitignore.add(".gptignore");
    }
    Ok(gitignore.build()?)
}

fn normalize_rel(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.to_string_lossy().replace('\\', "/")
}

fn is_noise_path(rel: &str) -> bool {
    // quick path-based noise check
    const NOISE_DIRS: [&str; 10] = [
        "target", "node_modules", ".git", ".github", ".idea", ".vscode",
        ".venv", "__pycache__", "dist", "build",
    ];
    if let Some(dir) = rel.split('/').next() {
        NOISE_DIRS.contains(&dir)
    } else {
        false
    }
}

fn file_is_probably_binary(path: &Path, sniff: usize) -> Result<bool> {
    // use util::is_probably_binary but honor sniff window
    let mut f = fs::File::open(path)?;
    let mut buf = vec![0u8; sniff];
    let n = f.read(&mut buf)?;
    Ok(util::is_probably_binary(&buf[..n]))
}

fn read_utf8(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn sha1_hex(bytes: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

fn slice_prefix<'a>(s: &'a str, max: usize) -> &'a str {
    if s.len() <= max { s } else { &s[..max] }
}

fn detect_lang(path: &Path) -> Result<String> {
    // 1) extension
    let ext_lang = util::ext_to_lang(path);
    if ext_lang != "txt" {
        return Ok(ext_lang.to_string());
    }

    // 2) shebang
    if let Ok(mut file) = fs::File::open(path) {
        let mut first = String::new();
        let _ = BufReader::new(&mut file).read_line(&mut first);
        let l = first.trim_start();
        if l.starts_with("#!") {
            if l.contains("python") { return Ok("python".into()); }
            if l.contains("bash") || l.contains("sh") { return Ok("sh".into()); }
            if l.contains("node") { return Ok("js".into()); }
        }
    }

    Ok(ext_lang.to_string()) // likely "txt"
}

fn lang_is_doc_or_config(lang: &str) -> bool {
    matches!(lang, "md" | "json" | "toml" | "yaml" | "yml")
}

fn build_entry(
    rel_path: &str,
    lang: &str,
    sha1: &str,
    size: usize,
    meta: &fs::Metadata,
    snip: &str,
    full_content: &str,
    _parent: Option<(&str, &str)>, // (parent_path, parent_lang) for virtual children
) -> (FileIntentEntry, bool) {
    let tags = util::infer_tags(rel_path, lang);
    let summary = Some(intent::guess_summary(rel_path, snip, lang));
    let token_estimate = estimate_tokens(snip);
    let last_modified = util::to_unix_epoch(meta);

    let role = infer_role(rel_path, lang, snip);
    let module = infer_module_id(rel_path, lang);
    let (imports, exports) = skim_symbols(snip, lang);

    let (lines_total, lines_nonblank) = {
        let t = full_content.lines().count();
        let nb = full_content.lines().filter(|l| !l.trim().is_empty()).count();
        (t, nb)
    };

    let rel_dir = rel_path.split('/').next().unwrap_or(".").to_string();
    let noise = is_noise_path(rel_path);

    // For virtual child entries (e.g., HTML blocks), we still emit the base.
    // No need to skip base here; caller decides when to also push the parent.
    let skip_base = false;

    (
        FileIntentEntry {
            path: rel_path.to_string(),
            lang: lang.to_string(),
            sha1: sha1.to_string(),
            size,
            last_modified,
            snippet: snip.to_string(),
            tags,
            summary,
            token_estimate,

            role,
            module,
            imports,
            exports,
            lines_total,
            lines_nonblank,
            rel_dir,
            noise,
        },
        skip_base,
    )
}

/* ========================= Token Estimation ========================= */

/// Very basic token estimator (customize as needed)
pub fn estimate_tokens(s: &str) -> usize {
    // 1 token ≈ 0.75 words (rough GPT rule of thumb)
    ((s.split_whitespace().count() as f64) / 0.75).ceil() as usize
}

/* ============================ JSONL Reader =========================== */

/// Read back index for diffing/history.
pub fn read_index(path: &Path) -> Result<Vec<FileIntentEntry>> {
    let f = fs::File::open(path)
        .with_context(|| format!("open index {}", path.display()))?;
    let rdr = BufReader::new(f);
    let mut entries = Vec::new();
    for (i, line) in rdr.lines().enumerate() {
        let line = line.with_context(|| format!("read jsonl line {}", i + 1))?;
        let entry: FileIntentEntry =
            serde_json::from_str(&line).with_context(|| format!("parse jsonl line {}", i + 1))?;
        entries.push(entry);
    }
    Ok(entries)
}

/* ======================= Polyglot: HTML splitting ======================= */

#[derive(Debug, Clone)]
struct HtmlBlock<'a> {
    lang: &'static str, // "js" | "ts" | "css"
    body: &'a str,
    kind: BlockKind,
}
#[derive(Debug, Clone, Copy)]
enum BlockKind { Script, Style }

impl HtmlBlock<'_> {
    fn id_for_path(&self, idx1: usize) -> String {
        match self.kind {
            BlockKind::Script => format!("script-{idx1}"),
            BlockKind::Style  => format!("style-{idx1}"),
        }
    }
}

/// Extract <script> and <style> blocks, guessing JS/TS for scripts via type attr.
fn extract_html_blocks(content: &str) -> Vec<HtmlBlock<'_>> {
    let mut out = Vec::new();
    let lower = content.to_ascii_lowercase();

    // naive but fast scanning; avoids heavy HTML parsers
    let bytes = content.as_bytes();
    let mut i = 0usize;
    while let Some(open_pos) = find_ci(&lower.as_bytes(), b"<script", i) {
        // find '>' of the opening tag
        if let Some(tag_end) = find_byte(bytes, b'>', open_pos) {
            // inspect type attribute in the opening tag slice
            let tag_slice = &lower.as_bytes()[open_pos..=tag_end.min(lower.len() - 1)];
            let lang = if memfind(tag_slice, b"type=\"module\"") || memfind(tag_slice, b"type='module'") {
                "js"
            } else if memfind(tag_slice, b"type=\"text/typescript\"") || memfind(tag_slice, b"type='text/typescript'") {
                "ts"
            } else {
                "js"
            };
            // find closing tag
            if let Some(close_pos) = find_ci(&lower.as_bytes(), b"</script>", tag_end + 1) {
                let body_start = tag_end.saturating_add(1);
                let body_end = close_pos;
                if body_start < body_end && body_end <= content.len() {
                    out.push(HtmlBlock { lang, body: &content[body_start..body_end], kind: BlockKind::Script });
                }
                i = close_pos + "</script>".len();
                continue;
            }
            i = tag_end + 1;
        } else {
            break;
        }
    }

    // styles
    let mut j = 0usize;
    while let Some(open_pos) = find_ci(&lower.as_bytes(), b"<style", j) {
        if let Some(tag_end) = find_byte(bytes, b'>', open_pos) {
            if let Some(close_pos) = find_ci(&lower.as_bytes(), b"</style>", tag_end + 1) {
                let body_start = tag_end.saturating_add(1);
                let body_end = close_pos;
                if body_start < body_end && body_end <= content.len() {
                    out.push(HtmlBlock { lang: "css", body: &content[body_start..body_end], kind: BlockKind::Style });
                }
                j = close_pos + "</style>".len();
                continue;
            }
            j = tag_end + 1;
        } else {
            break;
        }
    }

    out
}

/// Very small HTML structure preview for parent entry.
fn html_structure_preview(content: &str, limit_bytes: usize) -> String {
    let mut lines: Vec<String> = Vec::with_capacity(64);
    let mut scripts = 0usize;
    let mut styles = 0usize;
    for line in content.lines().take(400) {
        let l = line.trim();
        if l.starts_with("<script") { scripts += 1; }
        if l.starts_with("<style")  { styles += 1; }
        if l.starts_with("<head")   { lines.push("<head>…".into()); }
        if l.starts_with("<body")   { lines.push("<body>…".into()); }
        if l.starts_with("<main")   { lines.push("<main>…".into()); }
        if lines.len() >= 12 { break; }
    }
    let mut out = String::new();
    if !lines.is_empty() {
        out.push_str("Structure: ");
        out.push_str(&lines.join(", "));
        out.push('\n');
    }
    out.push_str(&format!("Embeds: <script> x{}, <style> x{}\n", scripts, styles));
    let tail = slice_prefix(content, limit_bytes.saturating_sub(out.len()));
    if !tail.is_empty() {
        out.push_str("---\n");
        out.push_str(tail);
    }
    out
}

/* ========================= tiny byte search helpers ========================= */

fn find_ci(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    // hay is already lowercase for our usage; needle should be lowercase too.
    memchr_slice(hay, needle, from)
}
fn find_byte(hay: &[u8], byte: u8, from: usize) -> Option<usize> {
    hay.get(from..).and_then(|s| memchr::memchr(byte, s)).map(|i| from + i)
}
fn memfind(hay: &[u8], needle: &[u8]) -> bool {
    memchr_slice(hay, needle, 0).is_some()
}
fn memchr_slice(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from >= hay.len() { return None; }
    let last = hay.len() - needle.len();
    let first = *needle.first().unwrap();
    let mut idx = from;
    while idx <= last {
        match memchr::memchr(first, &hay[idx..=last]) {
            None => return None,
            Some(off) => {
                idx += off;
                if hay[idx..idx + needle.len()].eq(needle) {
                    return Some(idx);
                }
                idx += 1;
            }
        }
    }
    None
}
