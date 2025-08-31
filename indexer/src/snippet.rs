// indexer/src/snippet.rs

const MAX_SCAN_BYTES: usize = 32 * 1024;  // fast head window
const MAX_SCAN_LINES: usize = 600;        // don't read the world
const MAX_KEEP_LINES: usize = 60;         // final snippet size
const MAX_INTERESTING_SEEN: usize = 400;  // bail early if repo is spicy

pub fn extract_relevant_snippet(content: &str, lang: &str) -> String {
    // Window the content for speed
    let head = if content.len() > MAX_SCAN_BYTES {
        &content[..MAX_SCAN_BYTES]
    } else {
        content
    };

    // Try to grab a leading doc block first (cheap & high signal)
    let mut out: Vec<String> = Vec::with_capacity(MAX_KEEP_LINES);
    if let Some(doc) = leading_doc_block(head, lang) {
        push_lines(&mut out, doc);
        if out.len() >= MAX_KEEP_LINES {
            return join(&out);
        }
    }

    // Scan lines with language-aware scoring
    let mut seen_interesting = 0usize;
    let mut kept: Vec<(usize, String)> = Vec::with_capacity(MAX_KEEP_LINES * 2);

    for (idx, raw) in head.lines().take(MAX_SCAN_LINES).enumerate() {
        let line = raw.trim();
        if line.is_empty() { continue; }

        let score = score_line(line, lang);
        if score == 0 { continue; }

        // Keep as (original order idx, text)
        kept.push((idx, line.to_string()));
        seen_interesting += 1;

        if kept.len() >= MAX_KEEP_LINES { break; }
        if seen_interesting >= MAX_INTERESTING_SEEN { break; }
    }

    // If nothing scored, fallback to the head of file (cleaned)
    if kept.is_empty() && out.is_empty() {
        return head
            .lines()
            .take(40)
            .map(|l| l.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n");
    }

    // Merge: doc block (already pushed) + interesting lines in original order (dedup consecutive)
    kept.sort_by_key(|(i, _)| *i);
    for (_, l) in kept {
        if out.last().map(|p| p == &l).unwrap_or(false) { continue; }
        out.push(l);
        if out.len() >= MAX_KEEP_LINES { break; }
    }

    join(&out)
}

/* ------------------------- scoring + helpers ------------------------- */

fn score_line(l: &str, lang: &str) -> u8 {
    let ll = l.to_ascii_lowercase();
    match lang.to_ascii_lowercase().as_str() {
        "rust" => score_rust(l, &ll),
        "python" => score_python(l, &ll),
        "typescript" | "ts" | "javascript" | "js" => score_js_ts(l, &ll),
        "go" => score_go(l, &ll),
        "toml" | "yaml" | "yml" | "json" => score_config(l, &ll),
        "markdown" | "md" => score_md(l, &ll),
        _ => score_generic(l, &ll),
    }
}

fn score_rust(l: &str, ll: &str) -> u8 {
    if l.starts_with("///") || l.starts_with("//!") { return 9; }    // doc
    if l.starts_with("use ") || l.starts_with("extern crate") { return 3; } // imports
    if l.starts_with("pub fn ") || l.starts_with("pub struct ")
        || l.starts_with("pub enum ") || l.starts_with("pub trait ")
        || l.starts_with("pub mod ") { return 8; }                   // public API
    if l.starts_with("fn ") || l.starts_with("struct ") || l.starts_with("enum ") || l.starts_with("impl ") { return 6; }
    if l.starts_with("#[") { return 4; }                              // attributes/tests/etc.
    if ll.contains("todo") || ll.contains("fixme") { return 2; }
    0
}

fn score_python(l: &str, ll: &str) -> u8 {
    if l.starts_with("\"\"\"") || l.starts_with("#!") || l.starts_with("# ") { return 9; } // doc/shebang
    if l.starts_with("def ") || l.starts_with("class ") { return 8; }      // API surface
    if l.starts_with("import ") || l.starts_with("from ") { return 3; }    // imports
    if ll.starts_with("if __name__ == '__main__'") { return 7; }           // entrypoint
    if ll.contains("todo") || ll.contains("fixme") { return 2; }
    0
}

fn score_js_ts(l: &str, _ll: &str) -> u8 {
    if l.starts_with("//") || l.starts_with("/**") || l.starts_with("* ") { return 8; }
    if l.starts_with("export ") { return 8; }
    if l.starts_with("import ") { return 3; }
    if l.starts_with("function ") || l.contains("=>") { return 5; }
    0
}

fn score_go(l: &str, _ll: &str) -> u8 {
    if l.starts_with("//") { return 7; }
    if l.starts_with("package ") { return 5; }
    if l.starts_with("import ") { return 3; }
    if l.starts_with("func ") { return 6; }
    0
}

fn score_config(l: &str, _ll: &str) -> u8 {
    if l.starts_with('[') || l.contains(": ") || l.contains(" = ") { return 5; }
    0
}

fn score_md(l: &str, _ll: &str) -> u8 {
    if l.starts_with("# ") || l.starts_with("## ") { return 8; }
    0
}

fn score_generic(l: &str, _ll: &str) -> u8 {
    if l.starts_with("//") || l.starts_with("#") { return 6; } // comments
    if l.contains("class ") || l.starts_with("def ") || l.starts_with("fn ") { return 5; }
    if l.starts_with("import ") || l.starts_with("using ") { return 3; }
    0
}

fn leading_doc_block(s: &str, lang: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut started = false;

    match lang.to_ascii_lowercase().as_str() {
        "rust" => {
            for line in s.lines().take(80) {
                let t = line.trim_start();
                if t.starts_with("//!") || t.starts_with("///") {
                    started = true;
                    out.push(strip_rust_doc(t).to_string());
                } else if started && (t.starts_with("//") || t.is_empty()) {
                    // allow blank or comment continuations
                    continue;
                } else if started {
                    break;
                } else if t.starts_with("/*") && t.contains("*/") {
                    // single-line block comment
                    out.push(t.trim_matches('/').trim_matches('*').trim().to_string());
                    break;
                }
            }
        }
        "python" => {
            let mut triple = false;
            for line in s.lines().take(120) {
                let t = line.trim_start();
                if t.starts_with("\"\"\"") {
                    started = true;
                    triple = !triple;
                    if !triple { break; }
                    continue;
                }
                if t.starts_with("#!") || t.starts_with("# ") {
                    started = true;
                    out.push(t.trim_start_matches("#!").trim_start_matches('#').trim().to_string());
                    continue;
                }
                if started {
                    if triple {
                        if t.starts_with("\"\"\"") { break; }
                        out.push(t.to_string());
                    } else if t.starts_with("#") || t.is_empty() {
                        // continue header comments
                        continue;
                    } else {
                        break;
                    }
                }
            }
        }
        _ => {
            // Markdown / generic: take top heading paragraph
            for line in s.lines().take(40) {
                let t = line.trim();
                if t.starts_with("# ") || t.starts_with("//") || t.starts_with("/*") || t.starts_with("--") {
                    started = true;
                    out.push(t.trim_start_matches("# ").trim().to_string());
                } else if started && !t.is_empty() {
                    out.push(t.to_string());
                    break;
                } else if started {
                    break;
                }
            }
        }
    }

    if out.is_empty() { None } else { Some(normalize_doc(out)) }
}

fn strip_rust_doc(t: &str) -> &str {
    t.trim_start_matches("///").trim_start_matches("//!").trim()
}

fn normalize_doc(lines: Vec<String>) -> Vec<String> {
    let mut v = Vec::new();
    for l in lines {
        let t = l.trim();
        if t.is_empty() { continue; }
        v.push(t.to_string());
        if v.len() >= (MAX_KEEP_LINES / 3) { break; } // doc doesn't hog the whole snippet
    }
    v
}

fn push_lines(out: &mut Vec<String>, lines: Vec<String>) {
    for l in lines {
        if out.len() >= MAX_KEEP_LINES { break; }
        if out.last().map(|p| p == &l).unwrap_or(false) { continue; }
        out.push(l);
    }
}

fn join(lines: &[String]) -> String {
    lines.join("\n")
}
