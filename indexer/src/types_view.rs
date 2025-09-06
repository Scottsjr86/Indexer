//! Project Types view
//!
//! Renders a catalog of public type shapes (structs & enums), grouped by module.
//! - No heavy parsing deps; brace-aware skim works for conventional Rust.
//! - Fields/variants are kept verbatim (trimmed), avoiding hallucinated names.
//! - Grouping uses helpers::infer_module_id(path, lang).
//!
//! Output: `.gpt_index/types/<slug>_PROJECT_TYPES.md`

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path},
};

use crate::{file_intent_entry::FileIntentEntry, helpers};

/// Public entry: build types doc from a JSONL index.
/// Files are read relative to the current working directory.
///
/// # Arguments
/// * `index_path`  - JSONL with one FileIntentEntry per line
/// * `output_path` - markdown file to write
pub fn build_types_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let entries = load_entries(index_path)?;

    // Resolve repo root from the index path:
    //   .gpt_index/indexes/<slug>.jsonl  => repo_root = index_path/../../
    let repo_root = index_path
        .parent()          // indexes/
        .and_then(|p| p.parent()) // .gpt_index/
        .and_then(|p| p.parent()) // <repo root>
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Collect Rust files only
    let rust_files: Vec<_> = entries
        .iter()
        .filter(|e| e.lang.eq_ignore_ascii_case("rust"))
        .collect();

    // Parse decls per file, group by module
    let mut by_module: BTreeMap<String, Vec<TypeDecl>> = BTreeMap::new();
        for e in rust_files {
        // Prefer absolute path resolved from repo_root
        let abs = repo_root.join(&e.path);
        let path = if abs.exists() { abs.as_path() } else { Path::new(&e.path) };
        let module = helpers::infer_module_id(&e.path, &e.lang);
        let content = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("[types] warn: could not read {}: {}", path.display(), err);
                String::new()
            }
        };
        // Remove comments & strings so examples in docs don’t trip the parser.
        let clean = strip_comments_and_strings(&content);
        let mut decls = scan_rust_types(&clean);
        if decls.is_empty() && !content.is_empty() && e.path.ends_with(".rs") {
            eprintln!("[types] note: no decls in {}", e.path);
        }

        // De-dup identical decl headers (in case of re-exports / macro doubles)
        dedup_decls(&mut decls);
        // Ignore files that only yield “example” empty decls (e.g., X/.. from docs)
        decls.retain(|d| !(d.name.len() <= 1 && d.body_lines.is_empty()));
        if !decls.is_empty() {
            by_module.entry(module).or_default().extend(decls);
        }
    }

    // Render
    let mut out = File::create(output_path)?;
    writeln!(out, "# Project Types")?;
    writeln!(out)?;
    writeln!(
        out,
        "_Public structs/enums by module. Field and variant names shown verbatim._"
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
            render_decl(&mut out, &d)?;
            writeln!(out)?;
        }
        writeln!(out)?;
    }

    Ok(())
}

/* ---------------- Parsing ---------------- */

#[derive(Clone, Debug, PartialEq, Eq)]
enum TypeKind { Struct, Enum }

#[derive(Clone, Debug)]
struct TypeDecl {
    kind: TypeKind,
    vis: String,       // "pub", "pub(crate)", etc.
    name: String,
    body_lines: Vec<String>, // for struct fields or enum variants (verbatim, trimmed)
}

/// Minimal, brace-aware skim for "pub struct X { .. }" and "pub enum X { .. }"
fn scan_rust_types(s: &str) -> Vec<TypeDecl> {    
    let mut decls = Vec::new();
    let mut i = 0usize;
    let bytes = s.as_bytes();
    let len = bytes.len();
    

    while let Some((start, vis, kind, name)) = find_next_head(s, i) {
        // Find matching top-level braces for this decl
        if let Some((body_start, body_end)) = find_brace_block(s, start) {
            let body = &s[body_start..body_end];
            let body_lines = collect_body_lines(body, matches!(kind, TypeKind::Struct));
            decls.push(TypeDecl { kind, vis, name, body_lines });
            i = body_end + 1;
        } else {
            // No block; advance to avoid infinite loop
            i = start + 1;
        }
        if i >= len { break; }
    }

    decls
}

/// Find next "pub ... (struct|enum) Name {" head, returning (index, vis, kind, name).
fn find_next_head(s: &str, from: usize) -> Option<(usize, String, TypeKind, String)> {
    // Scan for "pub" and let `next_keyword` handle whitespace and `pub(..)` forms.
    let mut idx = from;
    while let Some(off) = s[idx..].find("pub") {
        let pos = idx + off;
        if let Some((kw, kind)) = next_keyword(&s[pos + 3..]) {
            // visibility text is "pub[...]" up to the keyword start
            let vis = s[pos..pos + 3 + kw.0].trim().to_string();
            if let Some((name, head_end)) = next_ident(&s[pos + 3 + kw.0 + kw.1..]) {
                // require there to be a '{' after the head (allows generics/where)
                let rest = &s[pos + 3 + kw.0 + kw.1 + head_end..];
                if rest.contains('{') {
                    return Some((pos, vis, kind, name.to_string()));
                }
            }
        }
        idx = pos + 3;
        if idx >= s.len() { break; }
    }
    None
}

/// From a slice starting just after "pub", find ("struct" or "enum") and byte offsets:
/// returns ((bytes_before_kw, bytes_of_kw), kind)
fn next_keyword(s: &str) -> Option<((usize, usize), TypeKind)> {
    // scan whitespace/comments lightly
    let mut i = 0usize;
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() { i += 1; }
    // optional "(crate)" etc.
    if s[i..].starts_with('(') {
        if let Some(endp) = s[i..].find(')') { i += endp + 1; }
    }
    let j = i;
    if s[j..].starts_with("struct") {
        return Some(((j, "struct".len()), TypeKind::Struct));
    }
    if s[j..].starts_with("enum") {
        return Some(((j, "enum".len()), TypeKind::Enum));
    }
    None
}

/// Parse a Rust identifier; return (ident, bytes_consumed) from the given start.
fn next_ident(s: &str) -> Option<(&str, usize)> {
    let mut i = 0usize;
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() { i += 1; }
    let start = i;
    while i < s.len() {
        let c = s.as_bytes()[i];
        let ok = c.is_ascii_alphanumeric() || c == b'_' ;
        if !ok { break; }
        i += 1;
    }
    if i > start {
        Some((&s[start..i], i))
    } else { None }
}

/// Given position of the decl head, find the top-level `{ ... }` block that follows.
fn find_brace_block(s: &str, head_start: usize) -> Option<(usize, usize)> {
    // Find first '{' after head_start
    let mut i = s[head_start..].find('{')? + head_start;
    let mut depth = 0i32;
    let mut in_str: Option<char> = None;
    let bytes = s.as_bytes();

    let mut start = None;
    while i < s.len() {
        let c = bytes[i] as char;
        // crude string skipping
        if in_str.is_none() && (c == '"' || c == '\'') {
            in_str = Some(c);
            i += 1;
            continue;
        }
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            }
            i += 1;
            continue;
        }

        if c == '{' {
            depth += 1;
            if depth == 1 && start.is_none() {
                start = Some(i + 1);
            }
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                let st = start?;
                return Some((st, i)); // exclusive end
            }
        }
        i += 1;
    }
    None
}

/// Split the body lines of a struct/enum block into neat one-liners.
/// - For structs: keep lines with field-like `pub foo: Type` or `foo: Type` (we prefer `pub`).
/// - For enums: keep each top-level variant line (trim trailing comma).
fn collect_body_lines(body: &str, is_struct: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();

    // We gather top-level commas/newlines as separators for enum variants.
    for raw in body.lines() {
        let mut l = raw.trim().to_string();
        // skip pure attribute lines
        if l.starts_with("#[") { continue; }
        // track braces for tuple/struct variants; only commit at depth 0
        for ch in l.chars() {
            if ch == '{' || ch == '(' { depth += 1; }
            if ch == '}' || ch == ')' { depth -= 1; }
        }

        if is_struct {
            // Only keep public fields, drop private to avoid leaking internals.
            // Remove any leading inline attributes like `#[serde(...)]`
            while l.starts_with("#[") {
                if let Some(p) = l.find(']') { l = l[p+1..].trim_start().to_string(); } else { break; }
            }
            let ls = l.trim_start();
            if ls.starts_with("pub ") && ls.contains(':') {
                if l.ends_with(',') { l.pop(); }
                out.push(l);
            }

        } else {
            // Enum: accumulate until a top-level comma or we’re back at depth 0 on newline
            cur.push_str(raw.trim());
            if l.ends_with(',') && depth == 0 {
                if cur.ends_with(',') { cur.pop(); }
                out.push(cur.trim().to_string());
                cur.clear();
            } else {
                cur.push(' ');
            }
        }
    }
    if !is_struct {
        let t = cur.trim();
        if !t.is_empty() { out.push(t.to_string()); }
    }

    // Clean duplicates / empties
    let mut seen = BTreeSet::new();
    out.retain(|s| !s.is_empty() && seen.insert(s.clone()));
    out
}

fn dedup_decls(v: &mut Vec<TypeDecl>) {
    let mut seen = BTreeSet::new();
    v.retain(|d| {
        let key = format!("{:?}::{}::{}", d.kind, d.vis, d.name);
        seen.insert(key)
    });
}

/* ---------------- Rendering ---------------- */

fn render_decl(out: &mut File, d: &TypeDecl) -> std::io::Result<()> {
    match d.kind {
        TypeKind::Struct => {
            writeln!(out, "pub struct {} {{", d.name)?;
            // Render in stable order.
            let mut fields = d.body_lines.clone();
            fields.sort();
            if fields.is_empty() {
                writeln!(out, "  /* non-public or no fields */")?;
            }
            for f in fields {
                writeln!(out, "  {}", f)?;
            }
            writeln!(out, "}}")?;
        }
        TypeKind::Enum => {
            writeln!(out, "pub enum {} {{", d.name)?;
            for v in &d.body_lines {
                writeln!(out, "  {},", v)?;
            }
            writeln!(out, "}}")?;
        }
    }
    Ok(())
}

/* ---------------- Sanitization ---------------- */

/// Very lightweight pass to blank out comments and string-literals so
/// example code in docs doesn’t look like real declarations.
fn strip_comments_and_strings(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    let b = s.as_bytes();
    let mut in_sl_comment = false;
    let mut in_ml_comment = false;
    let mut in_str: Option<char> = None;
    while i < b.len() {
        let c = b[i] as char;
        // end single-line comment
        if in_sl_comment {
            if c == '\n' {
                in_sl_comment = false;
                out.push(c);
            } else {
                out.push(' ');
            }
            i += 1;
            continue;
        }
        // end multi-line comment
        if in_ml_comment {
            if c == '*' && i + 1 < b.len() && b[i+1] as char == '/' {
                in_ml_comment = false; i += 2; out.push_str("  "); continue;
            }
            out.push(' ');
            i += 1;
            continue;
        }
        // in string?
        if let Some(q) = in_str {
            if c == '\\' { // skip escaped
                out.push(' '); i += 2; continue;
            }
            if c == q { in_str = None; }
            out.push(' ');
            i += 1;
            continue;
        }
        // start of comment?
        if c == '/' && i + 1 < b.len() {
            let n = b[i+1] as char;
            if n == '/' { in_sl_comment = true; out.push_str("  "); i += 2; continue; }
            if n == '*' { in_ml_comment = true; out.push_str("  "); i += 2; continue; }
        }
        // start of string?
        if c == '"' || c == '\'' {
            in_str = Some(c);
            out.push(' ');
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
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
                eprintln!("[types] warn: bad JSONL at line {}: {}", i + 1, e);
                continue;
            }
        };
        match serde_json::from_str::<FileIntentEntry>(&line) {
            Ok(e) => v.push(e),
            Err(e) => {
                eprintln!("[types] warn: bad JSONL at line {}: {}", i + 1, e);
                continue;
            }
        }
    }
    Ok(v)
}
