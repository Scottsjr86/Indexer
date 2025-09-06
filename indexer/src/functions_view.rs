//! Project Functions view
//!
//! Renders a catalog of **public free functions and inherent/trait methods**,
//! grouped by module (helpers::infer_module_id). We avoid heavy parsing by
//! scanning text with brace/paren-aware logic and keeping signatures verbatim.
//!
//! Output: `.gpt_index/functions/<slug>_PROJECT_FUNCTIONS.md`

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::Path,
};

use crate::{file_intent_entry::FileIntentEntry, helpers};

/// Public entry: build functions doc from a JSONL index.
/// * `index_path`  - JSONL with one FileIntentEntry per line
/// * `output_path` - markdown file to write
pub fn build_functions_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let entries = load_entries(index_path)?;
    let rust_files: Vec<_> = entries
        .iter()
        .filter(|e| e.lang.eq_ignore_ascii_case("rust"))
        .collect();

    let mut by_module: BTreeMap<String, Vec<FnDecl>> = BTreeMap::new();

    for e in rust_files {
        let path = Path::new(&e.path);
        let module = helpers::infer_module_id(&e.path, &e.lang);
        let content = fs::read_to_string(path).unwrap_or_default();
        let mut decls = scan_rust_functions(&content);

        dedup_decls(&mut decls);

        if decls.is_empty() {
            eprintln!("[functions] note: no public fns in {}", e.path);
            continue;
        }
        by_module.entry(module).or_default().extend(decls);
    }

    // Render
    let mut out = File::create(output_path)?;
    writeln!(out, "# Project Functions")?;
    writeln!(out)?;
    writeln!(
        out,
        "_Public free functions and methods by module. Signatures are shown verbatim (one line)._"
    )?;
    writeln!(out)?;

    let total_modules = by_module.len();
    let total_decls: usize = by_module.values().map(|v| v.len()).sum();
    writeln!(out, "> Modules: {}  •  Decls: {}", total_modules, total_decls)?;
    writeln!(out)?;

    for (module, decls) in by_module {
        writeln!(out, "## module: {}", module)?;
        writeln!(out)?;
        for d in decls {
            writeln!(out, "```rust")?;
            writeln!(out, "{}", d.signature)?;
            writeln!(out, "```")?;
            writeln!(out)?;
        }
    }

    Ok(())
}

/* ---------------- Parsing ---------------- */

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FnDecl {
    /// One-line, cleaned signature (e.g., `pub fn foo<T>(x: i32) -> Option<T>`).
    signature: String,
}

/// Dedup identical signatures (across re-exports, traits, etc.)
fn dedup_decls(v: &mut Vec<FnDecl>) {
    let mut seen = BTreeSet::new();
    v.retain(|d| seen.insert(d.signature.clone()));
}

/// Minimal scanner for PUBLIC functions/methods in Rust, including:
/// - `pub fn name(..)`
/// - `pub(crate) fn name(..)`
/// - `impl Type { pub fn method(..) ... }` (we scan whole file for `pub fn`)
///
/// We capture the signature from `pub` through the end of the signature:
/// up to the opening `{` (exclusive) or semicolon `;` (trait default/no-body).
fn scan_rust_functions(s: &str) -> Vec<FnDecl> {
    let bytes = s.as_bytes();
    let mut decls = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        // Find "pub"
        if let Some(p) = find_word(bytes, i, b"pub") {
            // After pub, accept optional `(…)` visibility and whitespace
            let mut j = p + 3;
            j = skip_ws(bytes, j);
            if j < bytes.len() && bytes[j] == b'(' {
                if let Some(k) = find_matching(bytes, j, b'(', b')') {
                    j = k + 1;
                } else {
                    i = p + 3; // malformed; advance past pub
                    continue;
                }
            }
            j = skip_ws(bytes, j);

            // Expect `fn`
            if !is_word_at(bytes, j, b"fn") {
                i = p + 3;
                continue;
            }
            // Move past `fn`
            j += 2;
            j = skip_ws(bytes, j);

            // Capture identifier
            let (_name_start, name_end) = match capture_ident(bytes, j) {
                Some(t) => t,
                None => {
                    i = p + 3;
                    continue;
                }
            };
            j = name_end;

            // Optional generics <...> immediately after name
            if j < bytes.len() && bytes[j] == b'<' {
                if let Some(k) = find_matching(bytes, j, b'<', b'>') {
                    j = k + 1;
                } else {
                    i = p + 3;
                    continue;
                }
            }
            j = skip_ws(bytes, j);

            // Params required
            if j >= bytes.len() || bytes[j] != b'(' {
                i = p + 3;
                continue;
            }
            let params_end = match find_matching(bytes, j, b'(', b')') {
                Some(k) => k,
                None => {
                    i = p + 3;
                    continue;
                }
            };
            j = params_end + 1;

            // Optional return type: `-> ...` (stop at `{` or `where` or `;`)
            let mut sig_end = j;
            j = skip_ws(bytes, j);

            if j + 2 <= bytes.len() && &bytes[j..j + 2] == b"->" {
                j += 2;
                // advance through return type expr; stop before `{` or `where` or `;`
                let mut k = j;
                while k < bytes.len() {
                    if bytes[k] == b'{' || bytes[k] == b';' { break; }
                    if is_word_at(bytes, k, b"where") { break; }
                    // bracket-awareness inside return types
                    if bytes[k] == b'<' {
                        if let Some(m) = find_matching(bytes, k, b'<', b'>') {
                            k = m + 1; continue;
                        }
                    }
                    if bytes[k] == b'(' {
                        if let Some(m) = find_matching(bytes, k, b'(', b')') {
                            k = m + 1; continue;
                        }
                    }
                    k += 1;
                }
                sig_end = k;
            }

            // Optional where-clause: `where ...` (stop at `{` or `;`)
            j = skip_ws(bytes, j);
            if is_word_at(bytes, j, b"where") {
                j += "where".len();
                let mut k = j;
                while k < bytes.len() {
                    if bytes[k] == b'{' || bytes[k] == b';' { break; }
                    // allow simple bracket matching inside where bounds
                    if bytes[k] == b'<' {
                        if let Some(m) = find_matching(bytes, k, b'<', b'>') {
                            k = m + 1; continue;
                        }
                    }
                    if bytes[k] == b'(' {
                        if let Some(m) = find_matching(bytes, k, b'(', b')') {
                            k = m + 1; continue;
                        }
                    }
                    k += 1;
                }
                sig_end = k;
            }

            // Final terminator: either `{` (has body) or `;` (no body here)
            // We don't include `{`/`;` in the signature line.
            let raw_sig = &s[p..sig_end];
            let signature = one_line_whitespace(raw_sig);

            // Only keep truly public (we matched `pub`).
            decls.push(FnDecl { signature });

            // Advance
            i = sig_end.saturating_add(1);
            continue;
        }

        break; // no more "pub"
    }

    decls
}

/* ---------------- Small scanning helpers ---------------- */

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() { i += 1; }
    i
}

/// True if `word` appears starting at `i` and is delimited as a word boundary on both sides.
fn is_word_at(bytes: &[u8], i: usize, word: &[u8]) -> bool {
    if i + word.len() > bytes.len() { return false; }
    if &bytes[i..i + word.len()] != word { return false; }
    let left_ok = i == 0 || !is_ident_char(byte_at(bytes, i.saturating_sub(1)));
    let right_idx = i + word.len();
    let right_ok = right_idx >= bytes.len() || !is_ident_char(byte_at(bytes, right_idx));
    left_ok && right_ok
}

fn byte_at(bytes: &[u8], i: usize) -> u8 {
    if i >= bytes.len() { 0 } else { bytes[i] }
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Return Some((start, end)) of an identifier starting at `i`
fn capture_ident(bytes: &[u8], mut i: usize) -> Option<(usize, usize)> {
    i = skip_ws(bytes, i);
    let start = i;
    while i < bytes.len() && is_ident_char(bytes[i]) { i += 1; }
    if i > start { Some((start, i)) } else { None }
}

/// Find the matching closing delimiter for nested pairs like (), {}, [] or <>.
/// Returns the index of the matching closer. Assumes bytes[start] == open.
fn find_matching(bytes: &[u8], start: usize, open: u8, close: u8) -> Option<usize> {
    if start >= bytes.len() || bytes[start] != open { return None; }
    let mut depth: i32 = 0;
    let mut i = start;
    while i < bytes.len() {
        let c = bytes[i];
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 { return Some(i); }
        } else if c == b'"' || c == b'\'' {
            if let Some(n) = skip_string_like(bytes, i) { i = n; continue; }
        }
        i += 1;
    }
    None
}

fn skip_string_like(bytes: &[u8], start: usize) -> Option<usize> {
    let quote = bytes[start];
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2; // skip escape
            continue;
        }
        if bytes[i] == quote { return Some(i); }
        i += 1;
    }
    None
}

/// Collapse whitespace/newlines to a single space and trim -> **owned String**
fn one_line_whitespace(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/* ---------------- JSONL loader ---------------- */

fn load_entries(index_path: &Path) -> std::io::Result<Vec<FileIntentEntry>> {
    let f = File::open(index_path)?;
    let br = BufReader::new(f);
    let mut v = Vec::new();

    for (i, line) in br.lines().enumerate() {
        let line = match line {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[functions] warn: bad JSONL at line {}: {}", i + 1, e);
                continue;
            }
        };
        match serde_json::from_str::<FileIntentEntry>(&line) {
            Ok(e) => v.push(e),
            Err(e) => {
                eprintln!("[functions] warn: bad JSONL at line {}: {}", i + 1, e);
                continue;
            }
        }
    }
    Ok(v)
}

// --- helpers: word-boundary search -----------------------------------------

#[allow(dead_code)]
#[inline]
fn is_ident_start(b: u8) -> bool {
    (b'A'..=b'Z').contains(&b) || (b'a'..=b'z').contains(&b) || b == b'_'
}

#[allow(dead_code)]
#[inline]
fn is_ident_continue(b: u8) -> bool {
    is_ident_start(b) || (b'0'..=b'9').contains(&b)
}

/// Scan forward from `from` to find `needle` at a word boundary.
fn find_word(hay: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() { return None; }
    let mut i = from;
    while i + needle.len() <= hay.len() {
        // cheap first-byte prefilter
        if hay[i] == needle[0] && is_word_at(hay, i, needle) {
            return Some(i);
        }
        i += 1;
    }
    None
}
