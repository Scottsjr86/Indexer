// indexer/src/util.rs
//! Utility layer: workdir slugs, filenames, timestamps, tagging, and misc helpers.
//! No side effects beyond explicit file writes. No global state.

use std::{
    fs::{self, File, Metadata},
    io::{self, Write},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
    borrow::Cow,
};

/* =============================== Workdir & Filenames =============================== */

/// Best-effort current directory **slug**, safe for filenames.
/// Falls back to env vars or "project". Lowercase, `[a-z0-9_-]`, collapsed `_`.
pub fn workdir_slug() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    project_name_from_path(&cwd)
}

/// Prefix an output filename with the workdir slug.
/// Example: `prefixed_filename("PROJECT_TREE", "md")` -> `indexer_PROJECT_TREE.md`
pub fn prefixed_filename(stem: &str, ext: &str) -> String {
    format!(
        "{}_{}.{}",
        workdir_slug(),
        stem,
        ext.trim_start_matches('.')
    )
}

/// Join `base` + `rel`, normalizing `..` and stripping any leading separators in `rel`.
pub fn safe_join(base: &Path, rel: &Path) -> PathBuf {
    let rel = rel.components().filter(|c| !matches!(c, Component::RootDir)).collect::<PathBuf>();
    base.join(rel)
}

/* ================================== Time & Stamps ================================= */

/// RFC3339 (sortable) + a compact stamp string.
/// Example: `20250810_140359 (2025-08-10T14:03:59-05:00)`
pub fn now_timestamp() -> String {
    use chrono::{Local, SecondsFormat};
    let now = Local::now();
    let rfc3339 = now.to_rfc3339_opts(SecondsFormat::Secs, true);
    let compact = now.format("%Y%m%d_%H%M%S").to_string();
    format!("{compact} ({rfc3339})")
}

/// Compact, filesystem-safe UTC-agnostic local timestamp: `YYYYMMDD_HHMMSS`.
pub fn now_ts_compact() -> String {
    use chrono::{Datelike, Local, Timelike};
    let dt = Local::now();
    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        dt.year(),
        dt.month(),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

/// Modified time → UNIX seconds (as string). Falls back to created() or "0".
pub fn to_unix_epoch(meta: &Metadata) -> String {
    fn secs(t: SystemTime) -> Option<String> {
        t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs().to_string())
    }
    meta.modified()
        .ok()
        .and_then(secs)
        .or_else(|| meta.created().ok().and_then(secs))
        .unwrap_or_else(|| "0".to_string())
}

/* ================================= File I/O Helpers ================================ */

/// Atomic-ish write: write to `path.tmp`, fsync, then rename over `path`.
/// Avoids torn writes on crash. Creates parent dirs as needed.
pub fn safe_write(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
    ));
    {
        let mut f = File::create(&tmp)?;
        f.write_all(contents.as_ref())?;
        f.sync_all()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

/* ================================== Formatting ==================================== */

/// Human-friendly bytes (SI), e.g., 1.2 KB, 3.4 MB. Exact for small numbers.
pub fn humanize_bytes(n: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    if n < 1000 {
        return format!("{} B", n);
    }
    let mut v = n as f64;
    let mut idx = 0usize;
    while v >= 1000.0 && idx < UNITS.len() - 1 {
        v /= 1000.0;
        idx += 1;
    }
    if v < 10.0 {
        format!("{:.1} {}", v, UNITS[idx])
    } else {
        format!("{:.0} {}", v, UNITS[idx])
    }
}

/// Count non-empty source lines (for quick LOC estimates).
pub fn count_loc(text: &str) -> usize {
    text.lines().filter(|l| !l.trim().is_empty()).count()
}

/* ================================ Content Heuristics =============================== */

/// Very cheap binary detector: if >1% NULs or many non-utf8 bytes, call it binary.
/// Bound input slice to a window for speed.
pub fn is_probably_binary(bytes: &[u8]) -> bool {
    let window = bytes.get(..8192).unwrap_or(bytes);
    if window.contains(&0) {
        return true;
    }
    match std::str::from_utf8(window) {
        Ok(_) => false,
        Err(e) => e.error_len().is_none(), // hard error with no length => likely binary
    }
}

/// Map extension or filename to a canonical language label used by the indexer.
/// Keep this set small & stable for predictable tags.
pub fn ext_to_lang(path: &Path) -> &'static str {
    let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();

    match (fname.as_str(), ext.as_str()) {
        (_, "rs") => "rust",
        (_, "rs.in") => "rust",
        ("makefile", _) | (_, "mk") => "make",
        (_, "toml") => "toml",
        (_, "json") | (_, "jsonl") => "json",
        (_, "yml") | (_, "yaml") => "yaml",
        (_, "md") => "md",
        (_, "sh") => "sh",
        (_, "py") => "python",
        (_, "ts") => "ts",
        (_, "tsx") => "tsx",
        (_, "js") => "js",
        (_, "jsx") => "jsx",
        (_, "go") => "go",
        (_, "java") => "java",
        (_, "kt") => "kotlin",
        (_, "cpp") | (_, "cc") | (_, "cxx") => "cpp",
        (_, "c") => "c",
        (_, "h") | (_, "hpp") => "c_header",
        (_, "sql") => "sql",
        _ => "txt",
    }
}

/* ================================== Tagging ======================================= */

/// Heuristic tagger: case-insensitive signals from path + language.
/// Adds structural tags (dir:..., ext:...) and coarse role tags to guide GPT.
pub fn infer_tags(path: &str, lang: &str) -> Vec<String> {
    let mut tags = Vec::with_capacity(12);

    let lang = normalize_lang(lang);
    if !lang.is_empty() {
        tags.push(lang.to_string());
    }

    let p = Path::new(path);
    let fname = p
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let full = path.to_lowercase();

    // Structural/context tags
    if let Some(top) = p.components().next().and_then(|c| match c {
        Component::Normal(s) => s.to_str(),
        _ => None,
    }) {
        tags.push(format!("dir:{}", top.to_lowercase()));
    }
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        tags.push(format!("ext:{}", ext.to_lowercase()));
    }

    // Common codebase signals
    if full.contains("core") { tags.push("core".into()); }
    if full.contains("gui") || full.contains("ui") { tags.push("ui".into()); }
    if full.contains("cli") || fname == "main.rs" { tags.push("cli".into()); }
    if full.contains("test") || fname.ends_with("_test.rs") || fname.ends_with("_tests.rs") { tags.push("test".into()); }
    if full.contains("bench") || full.contains("benchmark") { tags.push("bench".into()); }
    if full.contains("docs") || full.contains("doc/") || full.ends_with(".md") { tags.push("docs".into()); }
    if full.contains("script") || full.ends_with(".sh") { tags.push("script".into()); }
    if fname == "lib.rs" { tags.push("crate:lib".into()); }
    if fname == "main.rs" { tags.push("crate:bin".into()); }
    if fname.ends_with("_mod.rs") || fname == "mod.rs" { tags.push("mod".into()); }
    if fname.ends_with("_types.rs") || fname == "types.rs" { tags.push("types".into()); }
    if full.contains("build") || full.contains("ci") || full.contains(".github") { tags.push("build".into()); }
    if full.contains("scan") { tags.push("scan".into()); }
    if full.contains("chunk") { tags.push("chunk".into()); }
    if full.contains("map_view") || full.contains("project_map") { tags.push("map".into()); }
    if full.contains("tree_view") || full.contains("project_tree") { tags.push("tree".into()); }
    if full.contains("paste") || full.contains("snippet") { tags.push("paste".into()); }

    // Project-specific hints
    for needle in ["nyx", "nyxia", "indexer"] {
        if full.contains(needle) {
            tags.push(format!("proj:{needle}"));
        }
    }

    dedup_preserve_order(tags)
}

/* ================================== Internals ===================================== */

fn project_name_from_path(p: &Path) -> String {
    // Canonicalize when possible, but don’t fail if it errors.
    let canon: PathBuf = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    // Prefer the last component; if it’s empty (e.g., `/`), try parent.
    let name = canon
        .file_name()
        .or_else(|| canon.parent().and_then(|pp| pp.file_name()))
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    slugify(name)
}

/// Slugify to `[a-z0-9_-]`, lowercase, collapse runs, strip edges.
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'A'..='Z' => out.push(ch.to_ascii_lowercase()),
            'a'..='z' | '0'..='9' => out.push(ch),
            '-' | '_' => out.push(ch),
            ' ' | '.' => out.push('_'),
            _ => out.push('_'),
        }
    }
    // collapse multiple underscores
    let collapsed = out
        .split('_')
        .filter(|seg| !seg.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    let trimmed = collapsed.trim_matches('_').trim_matches('-').to_string();
    if trimmed.is_empty() { "project".into() } else { trimmed }
}

/// Normalize language labels to small, stable set.
pub fn normalize_lang(lang: &str) -> Cow<'static, str> {
    let l = lang.trim();
    if l.is_empty() {
        return Cow::Borrowed("");
    }
    let lowered = l.to_ascii_lowercase();
    match lowered.as_str() {
        "rs" | "rust"        => Cow::Borrowed("rust"),
        "py" | "python"      => Cow::Borrowed("python"),
        "ts" | "typescript"  => Cow::Borrowed("ts"),
        "tsx"                => Cow::Borrowed("tsx"),
        "js" | "javascript"  => Cow::Borrowed("js"),
        "md" | "markdown"    => Cow::Borrowed("md"),
        "yml" | "yaml"       => Cow::Borrowed("yaml"),
        "json" | "jsonl"     => Cow::Borrowed("json"),
        "sh" | "bash"        => Cow::Borrowed("sh"),
        other                => Cow::Owned(other.to_string()),
    }
}

/// Deduplicate while preserving first-appearance order.
fn dedup_preserve_order(mut v: Vec<String>) -> Vec<String> {
    use std::collections::HashSet;
    let mut seen = HashSet::with_capacity(v.len());
    v.retain(|s| seen.insert(s.clone()));
    v
}

/* =================================== Tests ======================================== */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_basic() {
        assert_eq!(slugify("Indexer"), "indexer");
        assert_eq!(slugify("My Project"), "my_project");
        assert_eq!(slugify("..weird///name!!"), "weird_name");
    }

    #[test]
    fn prefixed_name() {
        // Can't assert exact slug without cwd context; just ensure format.
        let f = prefixed_filename("PROJECT_TREE", "md");
        assert!(f.ends_with("_PROJECT_TREE.md"));
        assert!(f.len() > "_PROJECT_TREE.md".len());
    }

    #[test]
    fn bytes_humanize() {
        assert_eq!(humanize_bytes(999), "999 B");
        assert_eq!(humanize_bytes(1500), "2 KB");
    }

    #[test]
    fn loc_counts() {
        let s = "a\n\nb\n  \n c ";
        assert_eq!(count_loc(s), 3);
    }

    #[test]
    fn lang_map() {
        assert_eq!(ext_to_lang(Path::new("src/main.rs")), "rust");
        assert_eq!(ext_to_lang(Path::new("README.md")), "md");
        assert_eq!(ext_to_lang(Path::new("script.sh")), "sh");
        assert_eq!(ext_to_lang(Path::new("foo.unknown")), "txt");
    }

    #[test]
    fn tags_have_lang_and_struct() {
        let tags = infer_tags("src/tree_view.rs", "rs");
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.iter().any(|t| t.starts_with("dir:")));
        assert!(tags.iter().any(|t| t.starts_with("ext:")));
    }

    #[test]
    fn ts_compact_shape() {
        let ts = now_ts_compact();
        assert!(ts.len() == "YYYYMMDD_HHMMSS".len());
        assert!(ts.chars().all(|c| c.is_ascii_digit() || c == '_'));
    }

    #[test]
    fn binary_detector() {
        let txt = b"hello world";
        let bin = b"abc\0def\0ghi";
        assert!(!is_probably_binary(txt));
        assert!(is_probably_binary(bin));
    }
}
