// indexer/src/intent.rs 
// Purely offline, high-signal, context/intent summary engine.
//
const MAX_SCAN_BYTES: usize = 32 * 1024;

pub fn guess_summary(path: &str, snippet: &str, lang: &str) -> String {
    let p = path.replace('\\', "/"); // normalize windows
    let pl = p.to_ascii_lowercase();

    // Trim snippet to a reasonable scan window
    let scan = if snippet.len() > MAX_SCAN_BYTES {
        &snippet[..MAX_SCAN_BYTES]
    } else {
        snippet
    };
    let sl = scan.to_ascii_lowercase();


    // File-specific clarifications for our own tools (preempt generic heuristics)
    if pl.ends_with("map_view.rs")  { return s("Builds hierarchical project map (markdown)."); }
    if pl.ends_with("tree_view.rs") { return s("Builds project directory tree (markdown)."); }
    if pl.ends_with("chunker.rs")   { return s("Splits index into paste-friendly markdown chunks."); }
    if pl.ends_with("scan.rs")      { return s("Project indexer: walk, hash, snippet, summarize."); }

    // --- 1) Ultra-specific: config / docs / CI ---
    if ends_with(&pl, "cargo.toml") { return s("Cargo manifest / workspace config."); }
    if ends_with(&pl, "package.json") { return s("Node package manifest (scripts/deps)."); }
    if ends_with(&pl, "pyproject.toml") { return s("Python project config (build/dep/tooling)."); }
    if ends_with(&pl, "requirements.txt") { return s("Python dependencies locklist."); }
    if ends_with(&pl, "dockerfile") || contains(&pl, "/docker/") { return s("Container build definition (Dockerfile)."); }
    if ends_with(&pl, "makefile") { return s("Make build targets and automation."); }
    if ends_with(&pl, "readme.md") || ends_with(&pl, "readme") { return s("Project README / documentation."); }
    if ends_with(&pl, "license") || ends_with(&pl, "license.md") { return s("Project license."); }
    if ends_with(&pl, ".yml") || ends_with(&pl, ".yaml") {
        if contains(&pl, ".github/workflows/") || contains(&pl, "/.gitlab-ci") || contains(&pl, "/.circleci/") {
            return s("CI pipeline/workflow configuration.");
        }
        return s("YAML configuration file.");
    }
    if ends_with(&pl, ".toml") { return s("TOML configuration file."); }
    if ends_with(&pl, ".env") { return s("Environment variables."); }

    // --- 2) Entrypoints ---
    if ends_with(&pl, "src/main.rs") || contains(&pl, "/bin/") || sl.contains("fn main(") {
        return s("Entrypoint for this Rust binary.");
    }
    if lang.eq_ignore_ascii_case("python") && sl.contains("if __name__ == '__main__'") {
        return s("Python script entrypoint.");
    }
    if ends_with(&pl, "lib.rs") { return s("Root library file for this Rust crate."); }

    // --- 3) Tests ---
    if contains(&pl, "/tests") || contains(&pl, "/test") ||
       ends_with(&pl, "_test.rs") || ends_with(&pl, ".spec.ts") || ends_with(&pl, ".spec.js") ||
       sl.contains("#[test]") || sl.contains("pytest") {
        return s("Test module or spec suite.");
    }

    // --- 4) Module aggregators / layout ---
    if ends_with(&pl, "mod.rs") || starts_with(&sl, "mod ") {
        return s("Module definition / namespace aggregator.");
    }

    // --- 5) Feature heuristics (fast, high-signal) ---
    // UI / presentation
    if any_in(&pl, &["/ui", "/panel", "/editor", "/view", "/component", "/widget", "/screen", "/page"]) {
        return s("User interface / presentation layer.");
    }
    // Core domain / engine
    if any_in(&pl, &["/core", "/engine", "/domain", "/model", "/service"]) {
        return s("Core domain logic / engine layer.");
    }
    // CLI
    if sl.contains("use clap") || sl.contains("use structopt") || sl.contains("use argh")
        || contains(&pl, "/cli") {
        return s("Command-line interface.");
    }
    // HTTP server / routing
    if sl.contains("axum::") || sl.contains("actix") || sl.contains("rocket::") || sl.contains("warp::")
        || sl.contains("router") && any_in(&pl, &["/http", "/server", "/api"]) {
        return s("HTTP server / routing.");
    }
    // Database / persistence
    if sl.contains("sqlx::") || sl.contains("diesel::") || sl.contains("postgres")
        || sl.contains("mongodb") || sl.contains("redis")
        || any_in(&pl, &["/db", "/repo", "/repository", "/persistence"]) {
        return s("Database access / persistence layer.");
    }
    // Concurrency / async infra
    if sl.contains("tokio::") || sl.contains("async fn") || sl.contains("std::sync")
        || sl.contains("mpsc") || sl.contains("spawn(") {
        return s("Concurrency / async orchestration.");
    }
    // Filesystem / IO
    if sl.contains("std::fs") || sl.contains("std::io") || any_in(&pl, &["/io", "/fs"]) {
        return s("Filesystem / IO utilities.");
    }

    // --- 6) Language-specific nudges ---
    if lang.eq_ignore_ascii_case("rust") && contains(&pl, "/types") {
        return s("Type definitions / data models.");
    }
    if lang.eq_ignore_ascii_case("rust") && contains(&pl, "/util") {
        return s("Utility helpers for the crate.");
    }

    // --- 7) Doc comment or Markdown heading extraction (better fallback) ---
    if let Some(doc) = extract_doc_summary(scan) {
        return doc;
    }

    // --- 8) Last resort: first non-empty code line ---
    first_non_empty_line(scan)
        .unwrap_or_else(|| "No summary available (offline mode).".into())
}

/* ----------------------------- helpers ----------------------------- */

#[inline]
fn s(msg: &str) -> String { msg.to_string() }

#[inline]
fn contains(hay: &str, needle: &str) -> bool { hay.contains(needle) }

#[inline]
fn ends_with(hay: &str, suffix: &str) -> bool { hay.ends_with(suffix) }

#[inline]
fn starts_with(hay: &str, prefix: &str) -> bool { hay.starts_with(prefix) }

fn any_in(hay: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| hay.contains(n))
}

/// Extracts a succinct doc summary from module/file docs or Markdown headings.
fn extract_doc_summary(s: &str) -> Option<String> {
    // 1) Rust module docs (`//!`) or item docs (`///`)
    for line in s.lines() {
        let t = line.trim_start();
        if t.starts_with("//!") || t.starts_with("///") {
            let msg = t.trim_start_matches("//!")
                        .trim_start_matches("///")
                        .trim();
            if !msg.is_empty() { return Some(msg.to_string()); }
        }
        // stop scanning early if we hit non-comment code quickly
        if !t.is_empty() && !t.starts_with("//") && !t.starts_with("#!") { break; }
    }

    // 2) Markdown first heading / paragraph (README or md docs in snippet)
    let lines = s.lines();
    // Prefer a top-level heading
    if let Some(h1) = lines.clone().find(|l| l.trim_start().starts_with("# ")) {
        let msg = h1.trim_start().trim_start_matches('#').trim();
        if !msg.is_empty() { return Some(msg.to_string()); }
    }
    // Or the first non-empty prose line
    for l in lines {
        let t = l.trim();
        if !t.is_empty() && !t.starts_with("```") && !t.starts_with("#!") && !t.starts_with("//") {
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
