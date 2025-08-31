// indexer/src/util.rs

use std::{
    fs::Metadata, 
    path::{
        Path, 
        PathBuf
    }
};

/// Best-effort current directory name, canonicalized, safe for filenames.
/// Fallbacks to env vars or "project" instead of erroring.
pub fn get_dir_name() -> std::io::Result<String> {
    let cwd = std::env::current_dir()?;
    Ok(project_name_from_path(&cwd))
}

/// RFC3339 (sortable) + a compact stamp string.
pub fn now_timestamp() -> String {
    use chrono::{Local, SecondsFormat};
    let now = Local::now();
    // e.g., 2025-08-10T14:03:59-05:00 | 20250810_140359
    let rfc3339 = now.to_rfc3339_opts(SecondsFormat::Secs, true);
    let compact = now.format("%Y%m%d_%H%M%S").to_string();
    format!("{compact} ({rfc3339})")
}

/// RFC3339 (sortable) + a compact, filesystem-safe stamp.
pub fn now_ts_compact() -> String {
    use chrono::{Local, Datelike, Timelike};
    let dt = Local::now();
    format!("{:04}{:02}{:02}_{:02}{:02}{:02}",
        dt.year(), dt.month(), dt.day(), dt.hour(), dt.minute(), dt.second())
}


/// Modified time → UNIX seconds. Falls back to created() if needed.
pub fn to_unix_epoch(meta: &Metadata) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    fn secs(t: SystemTime) -> Option<String> {
        t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs().to_string())
    }
    meta.modified()
        .ok()
        .and_then(secs)
        .or_else(|| meta.created().ok().and_then(secs))
        .unwrap_or_else(|| "0".to_string())
}

/// Heuristic tagger: case-insensitive signals from path + language.
/// Adds structural tags (dir:..., ext:...) to help downstream filtering.
pub fn infer_tags(path: &str, lang: &str) -> Vec<String> {
    let mut tags = Vec::with_capacity(8);
    let lang = normalize_lang(lang);
    tags.push(lang.to_string());

    let p = Path::new(path);
    let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    let full = path.to_lowercase();

    // Structural/context tags
    if let Some(top) = p.components().next().and_then(|c| {
        use std::path::Component;
        match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        }
    }) {
        tags.push(format!("dir:{}", top.to_lowercase()));
    }
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        tags.push(format!("ext:{}", ext.to_lowercase()));
    }

    // Common codebase signals
    if full.contains("core") { tags.push("core".into()); }
    if full.contains("gui") || full.contains("ui") { tags.push("ui".into()); }
    if full.contains("test") || fname.ends_with("_test.rs") || fname.ends_with("_tests.rs") { tags.push("test".into()); }
    if full.contains("bench") || full.contains("benchmark") { tags.push("bench".into()); }
    if full.contains("docs") || full.contains("doc/") || full.ends_with(".md") { tags.push("docs".into()); }
    if full.contains("script") || full.ends_with(".sh") { tags.push("script".into()); }
    if fname == "lib.rs" { tags.push("crate:lib".into()); }
    if fname == "main.rs" { tags.push("crate:bin".into()); }
    if fname.ends_with("_mod.rs") || fname == "mod.rs" { tags.push("mod".into()); }
    if fname.ends_with("_types.rs") || fname == "types.rs" { tags.push("types".into()); }
    if full.contains("build") || full.contains("ci") || full.contains(".github") { tags.push("build".into()); }

    // Project-specific hints
    for needle in ["nyx", "nyxia", "indexer"] {
        if full.contains(needle) { tags.push(format!("proj:{needle}")); }
    }

    dedup_preserve_order(tags)
}

/* --------------------------- helpers --------------------------- */

fn project_name_from_path(p: &Path) -> String {
    // canonicalize when possible, but don’t fail the whole call if it errors
    let canon: PathBuf = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    canon
        .file_name()
        .or_else(|| canon.parent().and_then(|pp| pp.file_name()))
        .and_then(|s| s.to_str())
        .map(|s| slugify(s))
        .unwrap_or_else(|| "project".into())
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' => out.push(ch),
            '-' | '_' | '.' => out.push(ch),
            ' ' => out.push('_'),
            _ => out.push('-'),
        }
    }
    out.trim_matches(['-', '_']).to_string()
}

fn normalize_lang<'a>(lang: &'a str) -> &'a str {
    let l = lang.trim();
    if l.is_empty() {
        return l;
    }
    if l.eq_ignore_ascii_case("rs") || l.eq_ignore_ascii_case("rust") {
        "rust"
    } else if l.eq_ignore_ascii_case("py") || l.eq_ignore_ascii_case("python") {
        "python"
    } else if l.eq_ignore_ascii_case("ts") || l.eq_ignore_ascii_case("typescript") {
        "ts"
    } else if l.eq_ignore_ascii_case("js") || l.eq_ignore_ascii_case("javascript") {
        "js"
    } else {
        // return a slice of the input, not a temp
        l
    }
}


fn dedup_preserve_order(mut v: Vec<String>) -> Vec<String> {
    use std::collections::HashSet;
    let mut seen = HashSet::with_capacity(v.len());
    v.retain(|s| seen.insert(s.clone()));
    v
}
