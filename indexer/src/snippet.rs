// indexer/src/snippet.rs

// Scan/keep limits — tuned for high signal without dragging the whole file.
const MAX_SCAN_BYTES: usize = 32 * 1024;  // only read the head window
const MAX_SCAN_LINES: usize = 800;        // and line-cap within that window
const MAX_KEEP_LINES: usize = 60;         // final snippet budget
const MAX_INTERESTING_SEEN: usize = 400;  // early stop if file is super "interesting"
const CONTEXT_AFTER: usize = 1;           // keep a tiny bit of context after hot lines

/// Extract a compact, high-signal snippet optimized for GPT ingestion.
/// Strategy:
/// 1) Capture top-of-file docs/comments (language aware).
/// 2) Score lines by language; keep highest-signal lines in original order,
///    with a sliver of context after each.
/// 3) Hard caps and dedup to stay within MAX_KEEP_LINES.
pub fn extract_relevant_snippet(content: &str, lang: &str) -> String {
    // Window the content for speed.
    let head = if content.len() > MAX_SCAN_BYTES {
        &content[..MAX_SCAN_BYTES]
    } else {
        content
    };

    // Try a leading doc/comment block first.
    let mut out: Vec<String> = Vec::with_capacity(MAX_KEEP_LINES);
    if let Some(doc) = leading_doc_block(head, lang) {
        push_lines(&mut out, doc);
        if out.len() >= MAX_KEEP_LINES {
            return join(&out);
        }
    }

    // Second pass: score lines and keep interesting ones with a touch of context.
    let mut kept: Vec<(usize, String)> = Vec::with_capacity(MAX_KEEP_LINES * 2);
    let mut seen_interesting = 0usize;

    let lines: Vec<&str> = head.lines().take(MAX_SCAN_LINES).collect();
    let mut i = 0usize;
    while i < lines.len() && kept.len() < MAX_KEEP_LINES && seen_interesting < MAX_INTERESTING_SEEN {
        let raw = lines[i];
        let line = raw.trim();
        if !line.is_empty() {
            let score = score_line(line, lang);
            if score > 0 {
                // Keep this line (original order + trimmed end to keep indentation)
                kept.push((i, raw.trim_end().to_string()));
                // Minimal context after a scored line (helps keep signatures / braces)
                for k in 1..=CONTEXT_AFTER {
                    if i + k < lines.len() {
                        let ctx = lines[i + k].trim_end();
                        if !ctx.is_empty() {
                            kept.push((i + k, ctx.to_string()));
                        }
                    }
                }
                seen_interesting += 1;
            }
        }
        i += 1;
    }

    // If nothing scored and we didn't have doc lines, return a clean head slice.
    if kept.is_empty() && out.is_empty() {
        return lines
            .into_iter()
            .take(MAX_KEEP_LINES.min(40))
            .map(|l| l.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n");
    }

    // Merge doc block + interesting lines in original order with simple dedup.
    kept.sort_by_key(|(idx, _)| *idx);
    for (_, l) in kept {
        if out.len() >= MAX_KEEP_LINES { break; }
        if out.last().map(|p| p == &l).unwrap_or(false) { continue; }
        out.push(l);
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
    if l.starts_with("///") || l.starts_with("//!") { return 9; }                     // docs
    if l.starts_with("pub ") {
        if l.starts_with("pub use ") { return 5; }                                    // re-export
        if l.starts_with("pub fn ") || l.starts_with("pub struct ")
            || l.starts_with("pub enum ") || l.starts_with("pub trait ")
            || l.starts_with("pub mod ") { return 8; }                                // public API
    }
    if l.starts_with("use ") || l.starts_with("extern crate") { return 3; }           // imports
    if l.starts_with("fn ") || l.starts_with("struct ") || l.starts_with("enum ")
        || l.starts_with("impl ") || l.starts_with("type ") { return 6; }             // internal API
    if l.starts_with("#[") { return 4; }                                              // attributes/tests/etc.
    if ll.contains("todo") || ll.contains("fixme") { return 2; }
    0
}

fn score_python(l: &str, ll: &str) -> u8 {
    if l.starts_with("\"\"\"") || l.starts_with("'''") || l.starts_with("#!") || l.starts_with("# ") { return 9; } // docs/shebang
    if l.starts_with("def ") || l.starts_with("class ") { return 8; }                         // API surface
    if l.starts_with("import ") || l.starts_with("from ") { return 3; }                       // imports
    if ll.starts_with("if __name__ == '__main__'") { return 7; }                              // entrypoint
    if ll.contains("todo") || ll.contains("fixme") { return 2; }
    0
}

fn score_js_ts(l: &str, _ll: &str) -> u8 {
    if l.starts_with("/**") || l.starts_with("* ") || l.starts_with("//") { return 8; }       // docs/comments
    if l.starts_with("export ") { return 8; }                                                 // public surface
    if l.starts_with("import ") { return 3; }                                                 // imports
    if l.starts_with("function ") || l.contains("=>") { return 5; }                           // functions
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
    if l.starts_with("//") || l.starts_with("#") || l.starts_with("--") { return 6; }         // comments
    if l.contains("class ") || l.starts_with("def ") || l.starts_with("fn ") { return 5; }    // API-ish
    if l.starts_with("import ") || l.starts_with("using ") { return 3; }                      // imports
    0
}

/* ------------------------ leading doc-block detection ------------------------ */

fn leading_doc_block(s: &str, lang: &str) -> Option<Vec<String>> {
    match lang.to_ascii_lowercase().as_str() {
        "rust"   => leading_rust_docs(s),
        "python" => leading_python_docs(s),
        "typescript" | "ts" | "javascript" | "js" => leading_js_docs(s),
        "markdown" | "md" => leading_md_head(s),
        _ => leading_generic_head(s),
    }
}

fn leading_rust_docs(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut started = false;
    for line in s.lines().take(120) {
        let t = line.trim_start();
        if t.starts_with("//!") || t.starts_with("///") {
            started = true;
            out.push(t.trim_start_matches("//!").trim_start_matches("///").trim().to_string());
            continue;
        }
        if started {
            // allow blank or comment continuation lines
            if t.starts_with("//") || t.is_empty() { continue; }
            break;
        }
        // catch an immediate single-line block comment header: /* ... */
        if t.starts_with("/*") && t.contains("*/") {
            let inner = t.trim_start_matches("/*").trim_end_matches("*/").trim();
            if !inner.is_empty() { out.push(inner.to_string()); }
            break;
        }
    }
    normalize_doc_opt(out)
}

fn leading_python_docs(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut in_triple = false;
    let mut triple_quote: Option<&str> = None;
    for line in s.lines().take(160) {
        let t = line.trim_start();
        if !in_triple && (t.starts_with("\"\"\"") || t.starts_with("'''")) {
            in_triple = true;
            triple_quote = Some(if t.starts_with("\"\"\"") { "\"\"\"" } else { "'''" });
            // same-line open/close
            if t.ends_with(triple_quote.unwrap()) && t.len() > 6 {
                let inner = t.trim_matches('"').trim_matches('\'').trim();
                if !inner.is_empty() { out.push(inner.to_string()); }
                // in_triple = false;  // <-- remove this line
                break;
            }
            continue;
        }
        if in_triple {
            if let Some(q) = triple_quote {
                if t.contains(q) {
                    // closing line (ignore trailing)
                    let inner = t.split(q).next().unwrap_or("").trim();
                    if !inner.is_empty() { out.push(inner.to_string()); }
                    break;
                } else {
                    out.push(t.to_string());
                }
            }
            continue;
        }
        if t.starts_with("#!") || t.starts_with("# ") {
            out.push(t.trim_start_matches("#!").trim_start_matches('#').trim().to_string());
            continue;
        }
        if !out.is_empty() {
            if !t.starts_with("#") && !t.is_empty() { break; }
        } else if !t.is_empty() {
            break;
        }
    }
    normalize_doc_opt(out)
}

fn leading_js_docs(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in s.lines().take(160) {
        let t = line.trim_start();
        if !in_block && t.starts_with("/**") {
            in_block = true;
            // if it's /** ... */ on the same line
            if t.contains("*/") {
                let inner = t.trim_start_matches("/**").trim_end_matches("*/").trim().trim_start_matches('*').trim();
                if !inner.is_empty() { out.push(inner.to_string()); }
                break;
            }
            continue;
        }
        if in_block {
            if t.contains("*/") {
                let inner = t.trim_end_matches("*/").trim().trim_start_matches('*').trim();
                if !inner.is_empty() { out.push(inner.to_string()); }
                break;
            }
            let inner = t.trim_start_matches('*').trim();
            if !inner.is_empty() { out.push(inner.to_string()); }
            continue;
        }
        if t.starts_with("// ") || t.starts_with("//\t") {
            out.push(t.trim_start_matches("//").trim().to_string());
            continue;
        }
        if !out.is_empty() && !t.starts_with("//") && !t.is_empty() {
            break;
        } else if out.is_empty() && !t.is_empty() && !t.starts_with("//") && !t.starts_with("/*") {
            break;
        }
    }
    normalize_doc_opt(out)
}

fn leading_md_head(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for line in s.lines().take(60) {
        let t = line.trim();
        if t.starts_with("# ") {
            out.push(t.trim_start_matches("# ").trim().to_string());
            continue;
        }
        if !out.is_empty() {
            if t.is_empty() { break; }
            out.push(t.to_string());
            break;
        }
        if !t.is_empty() && !t.starts_with("#") {
            break;
        }
    }
    normalize_doc_opt(out)
}

fn leading_generic_head(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for line in s.lines().take(40) {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with("# ") || t.starts_with("--") {
            out.push(t.trim_start_matches("//").trim_start_matches("--").trim().to_string());
            continue;
        }
        if !t.is_empty() && !out.is_empty() {
            out.push(t.to_string());
            break;
        }
        if !out.is_empty() && t.is_empty() {
            break;
        }
        if !t.is_empty() && out.is_empty() {
            break;
        }
    }
    normalize_doc_opt(out)
}

fn normalize_doc_opt(v: Vec<String>) -> Option<Vec<String>> {
    if v.is_empty() { return None; }
    Some(normalize_doc(v))
}

/* ------------------------------ small utilities ------------------------------ */

fn normalize_doc(lines: Vec<String>) -> Vec<String> {
    let mut v = Vec::new();
    for l in lines {
        let t = l.trim();
        if t.is_empty() { continue; }
        v.push(t.to_string());
        if v.len() >= (MAX_KEEP_LINES / 3) { break; } // docs shouldn’t hog the snippet
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

/* ----------------------------------- tests ----------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_doc_capture() {
        let s = "//! Top module docs\n/// more\nfn main(){}\n";
        let snip = extract_relevant_snippet(s, "rust");
        assert!(snip.contains("Top module docs"));
        assert!(snip.contains("more"));
    }

    #[test]
    fn py_triple_quote() {
        let s = r#"
"""Module summary
Goes here."""
def f(): pass
"#;
        let snip = extract_relevant_snippet(s, "python");
        assert!(snip.contains("Module summary"));
    }

    #[test]
    fn js_block_doc() {
        let s = "/** Hello */\nexport function x(){}\n";
        let snip = extract_relevant_snippet(s, "ts");
        assert!(snip.contains("Hello"));
        assert!(snip.contains("export function"));
    }

    #[test]
    fn fallback_head() {
        let s = "line1\n\nline2\n";
        let snip = extract_relevant_snippet(s, "txt");
        assert!(snip.contains("line1"));
    }
}
