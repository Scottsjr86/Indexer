// indexer/src/intent.rs
// Purely offline, high-signal, context/intent summary engine.
// No allocations beyond what's necessary for scanning, no external deps.

/// Max characters of `snippet` we scan for intent signals and doc extraction.
/// Keep small for speed; we bias toward the top-of-file semantics.
const MAX_SCAN_BYTES: usize = 32 * 1024;

/// Public entrypoint: return a short, high-signal, human/GPT friendly summary
/// for a given file path, snippet, and language label.
pub fn guess_summary(path: &str, snippet: &str, lang: &str) -> String {
    // Normalize path (Windows-safe) and lowercased mirrors.
    let p = normalize_path(path);
    let pl = p.to_ascii_lowercase();

    // Trim snippet to a reasonable scan window.
    let scan = trim_window(snippet, MAX_SCAN_BYTES);
    let sl = scan.to_ascii_lowercase();

    // --- 0) Project-specific short-circuits (our own modules) ---
    if pl.ends_with("map_view.rs")    { return s("Builds semantic project map (markdown)."); }
    if pl.ends_with("tree_view.rs")   { return s("Builds structural project tree (markdown)."); }
    if pl.ends_with("chunker.rs")     { return s("Splits indexed files into GPT-ready paste chunks."); }
    if pl.ends_with("scan.rs")        { return s("Repo scanner: walk, hash, detect, snippet, summarize."); }
    if pl.ends_with("snippet.rs")     { return s("PASTE emitter: model-optimized prompt pack."); }
    if pl.ends_with("helpers.rs")     { return s("Formatting and shared helper utilities."); }
    if pl.ends_with("commands.rs")    { return s("CLI subcommands wiring and user-facing flows."); }
    if pl.ends_with("intent.rs")      { return s("Intent classifier: offline file purpose inference."); }

    // --- 1) Ultra-specific: config / docs / CI ---
    if is_cargo_toml(&pl)                { return s("Cargo manifest / workspace configuration."); }
    if ends_with(&pl, "package.json")    { return s("Node package manifest (scripts/deps)."); }
    if ends_with(&pl, "pyproject.toml")  { return s("Python project configuration (build/deps/tooling)."); }
    if ends_with(&pl, "requirements.txt"){ return s("Python dependencies locklist."); }
    if is_docker_related(&pl)            { return s("Container build definition (Dockerfile)."); }
    if ends_with(&pl, "makefile") || ends_with(&pl, ".mk") { return s("Make build targets and automation."); }
    if is_readme(&pl)                    { return s("Project README / documentation."); }
    if is_license(&pl)                   { return s("Project license."); }
    if is_ci_yaml(&pl)                   { return s("CI pipeline/workflow configuration."); }
    if ends_with_any(&pl, &[".yml", ".yaml"]) { return s("YAML configuration file."); }
    if ends_with(&pl, ".toml")           { return s("TOML configuration file."); }
    if ends_with(&pl, ".env")            { return s("Environment variables file."); }

    // --- 2) Entrypoints ---
    if is_rust_bin_entry(&pl, &sl)       { return s("Entrypoint for this Rust binary."); }
    if is_python_entry(lang, &sl)        { return s("Python script entrypoint."); }
    if ends_with(&pl, "lib.rs")          { return s("Root library file for this Rust crate."); }

    // --- 3) Tests ---
    if is_test_file(&pl, &sl)            { return s("Test module or spec suite."); }

    // --- 4) Module aggregators / layout ---
    if ends_with(&pl, "mod.rs") || sl.trim_start().starts_with("mod ") {
        return s("Module definition / namespace aggregator.");
    }

    // --- 5) Feature heuristics (fast, high-signal) ---
    if any_in(&pl, &["/ui", "/panel", "/editor", "/view", "/component", "/widget", "/screen", "/page"]) {
        return s("User interface / presentation layer.");
    }
    if any_in(&pl, &["/core", "/engine", "/domain", "/model", "/service"]) {
        return s("Core domain logic / engine layer.");
    }
    if sl.contains("use clap") || sl.contains("use structopt") || sl.contains("use argh") || contains(&pl, "/cli") {
        return s("Command-line interface.");
    }
    if is_httpish(&sl, &pl) {
        return s("HTTP server / routing.");
    }
    if is_dblike(&sl, &pl) {
        return s("Database access / persistence layer.");
    }
    if is_concurrency(&sl) {
        return s("Concurrency / async orchestration.");
    }
    if is_fsio(&sl, &pl) {
        return s("Filesystem / IO utilities.");
    }

    // --- 6) Language-specific nudges ---
    if eq_ic(lang, "rust") && contains(&pl, "/types") {
        return s("Type definitions / data models.");
    }
    if eq_ic(lang, "rust") && contains(&pl, "/util") {
        return s("Utility helpers for the crate.");
    }

    // --- 7) Doc comment or Markdown heading extraction (better fallback) ---
    if let Some(doc) = extract_doc_summary(scan) {
        return doc;
    }

    // --- 8) Last resort: first non-empty code line (trimmed) ---
    first_non_empty_line(scan).unwrap_or_else(|| "No summary available (offline mode).".into())
}

/* ----------------------------- helper layer ----------------------------- */

#[inline]
fn s(msg: &str) -> String { msg.to_string() }

#[inline]
fn contains(hay: &str, needle: &str) -> bool { hay.contains(needle) }

#[inline]
fn ends_with(hay: &str, suffix: &str) -> bool { hay.ends_with(suffix) }

#[inline]
fn _starts_with(hay: &str, prefix: &str) -> bool { hay.starts_with(prefix) }

#[inline]
fn ends_with_any(hay: &str, suffixes: &[&str]) -> bool {
    suffixes.iter().any(|s| hay.ends_with(s))
}

#[inline]
fn any_in(hay: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| hay.contains(n))
}

#[inline]
fn eq_ic(a: &str, b: &str) -> bool { a.eq_ignore_ascii_case(b) }

#[inline]
fn normalize_path(p: &str) -> String { p.replace('\\', "/") }

#[inline]
fn trim_window(s: &str, max: usize) -> &str {
    if s.len() > max { &s[..max] } else { s }
}

/* ----------- specific detectors (keep tiny & branch-predictable) ----------- */

fn is_cargo_toml(pl: &str) -> bool {
    ends_with(pl, "cargo.toml")
}

fn is_docker_related(pl: &str) -> bool {
    ends_with(pl, "dockerfile") || contains(pl, "/docker/")
}

fn is_readme(pl: &str) -> bool {
    ends_with(pl, "readme.md") || ends_with(pl, "readme")
}

fn is_license(pl: &str) -> bool {
    ends_with(pl, "license") || ends_with(pl, "license.md")
}

fn is_ci_yaml(pl: &str) -> bool {
    contains(pl, ".github/workflows/") || contains(pl, "/.gitlab-ci") || contains(pl, "/.circleci/")
}

fn is_rust_bin_entry(pl: &str, sl: &str) -> bool {
    ends_with(pl, "src/main.rs") || contains(pl, "/bin/") || sl.contains("fn main(")
}

fn is_python_entry(lang: &str, sl: &str) -> bool {
    eq_ic(lang, "python") && sl.contains("if __name__ == '__main__'")
}

fn is_test_file(pl: &str, sl: &str) -> bool {
    contains(pl, "/tests") ||
    contains(pl, "/test")  ||
    ends_with(pl, "_test.rs") ||
    ends_with(pl, ".spec.ts") || ends_with(pl, ".spec.js") ||
    sl.contains("#[test]") || sl.contains("pytest")
}

fn is_httpish(sl: &str, pl: &str) -> bool {
    sl.contains("axum::") || sl.contains("actix") || sl.contains("rocket::") || sl.contains("warp::")
        || (sl.contains("router") && any_in(pl, &["/http", "/server", "/api"]))
}

fn is_dblike(sl: &str, pl: &str) -> bool {
    sl.contains("sqlx::") || sl.contains("diesel::") || sl.contains("postgres")
        || sl.contains("mongodb") || sl.contains("redis")
        || any_in(pl, &["/db", "/repo", "/repository", "/persistence"])
}

fn is_concurrency(sl: &str) -> bool {
    sl.contains("tokio::") || sl.contains("async fn") || sl.contains("std::sync")
        || sl.contains("mpsc") || sl.contains("spawn(")
}

fn is_fsio(sl: &str, pl: &str) -> bool {
    sl.contains("std::fs") || sl.contains("std::io") || any_in(pl, &["/io", "/fs"])
}

/* ----------- documentation/headline extraction with fence awareness ----------- */

/// Extract a succinct doc summary from module/file docs or Markdown headings.
/// - Prefers Rust doc comments (`//!`, `///`) near top of file.
/// - Falls back to first Markdown H1 or first non-empty prose line.
/// - Skips code blocks fenced with ``` to avoid grabbing random code.
pub fn extract_doc_summary(s: &str) -> Option<String> {
    // 0) Cheap pass for Rust doc comments at top
    for line in s.lines().take(256) { // early cap for speed
        let t = line.trim_start();
        if t.starts_with("//!") || t.starts_with("///") {
            let msg = t.trim_start_matches("//!")
                        .trim_start_matches("///")
                        .trim();
            if !msg.is_empty() { return Some(msg.to_string()); }
            continue;
        }
        // bail quickly if we hit non-comment code
        if !t.is_empty() && !t.starts_with("//") && !t.starts_with("#!") {
            break;
        }
    }

    // 1) Markdown fence-aware skim: first H1 or first non-empty non-fenced prose
    let mut in_fence = false;
    for line in s.lines().take(512) {
        let t = line.trim();
        if t.starts_with("```") { in_fence = !in_fence; continue; }
        if in_fence { continue; }

        if t.starts_with("# ") {
            let msg = t.trim_start_matches('#').trim();
            if !msg.is_empty() { return Some(msg.to_string()); }
        }

        // First non-empty, non-fence, non-directive prose
        if !t.is_empty() && !t.starts_with("#!") && !t.starts_with("//") && !t.starts_with("/*") {
            // Avoid commonly noisy single tokens
            if t.len() > 2 { return Some(t.to_string()); }
        }
    }

    // 2) As a last doc-like attempt: first line that "looks like" a sentence.
    for line in s.lines().take(256) {
        let t = line.trim();
        if t.len() > 6 && (t.ends_with('.') || t.contains(' ')) {
            return Some(t.to_string());
        }
    }

    None
}

fn first_non_empty_line(s: &str) -> Option<String> {
    s.lines().find_map(|l| {
        let t = l.trim();
        if t.is_empty() { None } else { Some(t.to_string()) }
    })
}

/* ----------------------------------- tests ----------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_readme() {
        let sum = guess_summary("README.md", "# Hello\n\nstuff", "md");
        assert!(sum.to_lowercase().contains("readme"));
    }

    #[test]
    fn detects_rust_main() {
        let sum = guess_summary("src/main.rs", "fn main(){println!(\"hi\");}", "rust");
        assert!(sum.to_lowercase().contains("entrypoint"));
    }

    #[test]
    fn detects_test_file() {
        let sum = guess_summary("src/foo_test.rs", "#[test]\nfn t(){}", "rust");
        assert!(sum.to_lowercase().contains("test"));
    }

    #[test]
    fn extract_prefers_rust_docs() {
        let doc = extract_doc_summary("//! Cool module\nfn main(){}").unwrap();
        assert!(doc.contains("Cool module"));
    }

    #[test]
    fn extract_skips_fenced_code() {
        let md = "Intro\n```rust\nfn main(){}\n```\n# Heading\n";
        let doc = extract_doc_summary(md).unwrap();
        assert_eq!(doc, "Heading");
    }
}
