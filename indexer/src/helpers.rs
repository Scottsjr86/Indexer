// indexer/src/helpers.rs
//! Heuristics and light parsers used across scan + intent.
//! - Role inference (typed) from path/lang/snippet
//! - Module ID derivation (language-aware)
//! - Cheap import/export skimming (no regex/AST)
//! - Small utilities (dedup, ident capture)

use crate::file_intent_entry::Role;

/* =============================== Role inference =============================== */

/// Infer a coarse role for the file: bin/lib/test/doc/config/script/ui/core
/// Returns a typed `Role`. Pure, allocation-free (except small to_lowercase() temps).
pub fn infer_role(path: &str, lang: &str, snippet: &str) -> Role {
    let p = path.replace('\\', "/").to_ascii_lowercase();
    let l = lang.to_ascii_lowercase();
    let s = snippet.to_ascii_lowercase();

    // --- tests ---
    if p.contains("/tests/") || p.ends_with("/tests") || p.contains("/test/")
        || p.ends_with("_test.rs") || p.ends_with("_tests.rs")
        || p.ends_with(".spec.ts") || p.ends_with(".spec.js")
        || s.contains("#[test]") || s.contains("pytest")
    {
        return Role::Test;
    }

    // --- entrypoints / bins ---
    if p.ends_with("src/main.rs") || p.contains("/src/bin/")
        || s.contains("fn main(")
        || (l == "python" && s.contains("if __name__ == '__main__'"))
    {
        return Role::Bin;
    }

    // --- docs / configs ---
    if p.ends_with(".md") || p.contains("/docs/") || p.ends_with("/docs") || p.ends_with("readme") || p.ends_with("readme.md") {
        return Role::Doc;
    }
    if matches!(l.as_str(), "toml" | "yaml" | "yml" | "json")
        || p.ends_with(".env")
        || p.contains(".github/workflows/") || p.contains("/.gitlab-ci") || p.contains("/.circleci/")
    {
        return Role::Config;
    }

    // --- scripts ---
    if p.ends_with(".sh")
        || s.starts_with("#!/bin/bash")
        || s.starts_with("#!/usr/bin/env bash")
        || s.starts_with("#!/usr/bin/env sh")
    {
        return Role::Script;
    }

    // --- ui-ish ---
    if p.contains("/ui") || p.contains("/panel") || p.contains("/editor") || p.contains("/view")
        || p.contains("/component") || p.contains("/widget") || p.contains("/screen") || p.contains("/page")
    {
        return Role::Ui;
    }

    // --- crate / core hints ---
    if p.ends_with("lib.rs") {
        return Role::Lib;
    }
    if p.contains("/core/") || p.contains("core_") || p.contains("_core")
        || p.contains("/engine/") || p.contains("engine_") || p.contains("_engine")
    {
        return Role::Core;
    }

    // default
    Role::Lib
}

/* ============================= Module identification ============================= */

/// Best-effort module id (path → module), language-aware. Returns stable identifiers.
pub fn infer_module_id(path: &str, lang: &str) -> String {
    let p = path.replace('\\', "/").trim_matches('/').to_string();
    match lang.to_ascii_lowercase().as_str() {
        "rust"   => rust_module_id(&p),
        "python" => python_module_id(&p),
        "ts" | "typescript" | "js" | "javascript" => web_module_id(&p),
        _ => generic_module_id(&p),
    }
}

/// Rust:
/// - src/lib.rs        -> crate
/// - src/main.rs       -> bin
/// - src/bin/foo.rs    -> bin::foo
/// - src/foo/bar.rs    -> foo::bar
/// - src/foo/mod.rs    -> foo
pub fn rust_module_id(p: &str) -> String {
    if p.ends_with("src/lib.rs") { return "crate".into(); }
    if p.ends_with("src/main.rs") { return "bin".into(); }
    if let Some(rest) = p.strip_prefix("src/bin/") {
        let name = rest.strip_suffix(".rs").unwrap_or(rest);
        return format!("bin::{}", name.replace('/', "::"));
    }
    if let Some(rest) = p.strip_prefix("src/") {
        if rest.ends_with("/mod.rs") {
            let modpath = rest.trim_end_matches("/mod.rs");
            return modpath.replace('/', "::");
        }
        let stem = rest.strip_suffix(".rs").unwrap_or(rest);
        return stem.replace('/', "::");
    }
    generic_module_id(p)
}

/// Python: strip extension, convert / to .
/// tests/foo_test.py -> tests.foo_test
pub fn python_module_id(p: &str) -> String {
    let stem = p.strip_suffix(".py").unwrap_or(p);
    stem.replace('/', ".")
}

/// Web (ts/js): strip extension; treat directories as namespaces with `::`
pub fn web_module_id(p: &str) -> String {
    let stem = p.rsplit_once('.').map(|(a, _)| a).unwrap_or(p);
    stem.replace('/', "::")
}

/// Generic: strip extension; use `::` as separator.
pub fn generic_module_id(p: &str) -> String {
    let stem = p.rsplit_once('.').map(|(a, _)| a).unwrap_or(p);
    stem.replace('/', "::")
}

/* ============================ Symbol skimming (cheap) ============================ */

/// Extract imports/exports cheaply from the snippet (no regex/AST).
/// Returns (imports, exports). Deduplicated, order-preserving (first occurrence).
pub fn skim_symbols(snippet: &str, lang: &str) -> (Vec<String>, Vec<String>) {
    match lang.to_ascii_lowercase().as_str() {
        "rust" => skim_rust(snippet),
        "python" => skim_python(snippet),
        "typescript" | "ts" | "javascript" | "js" => skim_js_ts(snippet),
        _ => (Vec::new(), Vec::new()),
    }
}

pub fn skim_rust(s: &str) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    for raw in s.lines() {
        let l = trim_inline_comment(raw.trim());
        if l.starts_with("use ") {
            // use foo::bar::{Baz, Qux as Q};
            let body = l.trim_start_matches("use ").trim_end_matches(';').trim();
            if !body.is_empty() { imports.push(body.to_string()); }
        } else if l.starts_with("pub ") {
            // pub(crate) fn ...  / pub struct ...  / pub enum ...  / pub trait ...  / pub mod ...
            let li = l.trim_start_matches("pub ").trim_start();
            if li.starts_with("fn ")      { exports.push(sig_ident(li, "fn ")); }
            else if li.starts_with("struct ") { exports.push(sig_ident(li, "struct ")); }
            else if li.starts_with("enum ")   { exports.push(sig_ident(li, "enum ")); }
            else if li.starts_with("trait ")  { exports.push(sig_ident(li, "trait ")); }
            else if li.starts_with("mod ")    { exports.push(sig_ident(li, "mod ")); }
            // ignore `pub use` (it’s re-export; import already captured)
        }
    }
    (dedup_preserve_order(imports), dedup_preserve_order(exports))
}

pub fn skim_python(s: &str) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    for raw in s.lines() {
        let l = trim_hash_comment(raw.trim());
        if l.starts_with("import ") {
            // import a, b as c -> "a", "b"
            let rest = l.trim_start_matches("import ").trim();
            for part in rest.split(',') {
                let tok = part.trim().split_whitespace().next().unwrap_or("");
                if !tok.is_empty() { imports.push(tok.to_string()); }
            }
        } else if l.starts_with("from ") {
            // from x.y import z, t as u -> "x.y.z", "x.y.t"
            let rest = l.trim_start_matches("from ").trim();
            if let Some((pkg, rhs)) = rest.split_once(" import ") {
                for part in rhs.split(',') {
                    let item = part.trim().split_whitespace().next().unwrap_or("");
                    if !item.is_empty() { imports.push(format!("{}.{}", pkg.trim(), item)); }
                }
            }
        }
        // exports: top-level defs/classes (simple heuristic)
        if l.starts_with("def ") {
            exports.push(sig_ident(l, "def "));
        } else if l.starts_with("class ") {
            exports.push(sig_ident(l, "class "));
        }
    }
    (dedup_preserve_order(imports), dedup_preserve_order(exports))
}

pub fn skim_js_ts(s: &str) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    for raw in s.lines() {
        let l = trim_js_comment(raw.trim());
        if l.starts_with("import ") {
            // import x from 'y';  import {a,b} from "y";  import * as ns from "y";
            if let Some((_, from)) = l.split_once(" from ") {
                let pkg = from.trim().trim_end_matches(';').trim();
                let pkg = pkg.trim_matches(['"', '\'', '`'].as_ref());
                if !pkg.is_empty() { imports.push(pkg.to_string()); }
            } else {
                // Side-effect import: import 'zone.js';
                if let Some(q) = l.trim_start_matches("import ").trim().trim_end_matches(';').trim().strip_prefix(|c| c=='\''||c=='"'||c=='`') {
                    let pkg = q.trim_end_matches(|c| c=='\''||c=='"'||c=='`');
                    if !pkg.is_empty() { imports.push(pkg.to_string()); }
                }
            }
        }
        if l.starts_with("export ") {
            // export function Foo / export class Bar / export const baz / export let x / export type Thing
            let le = l.trim_start_matches("export ").trim_start();
            if le.starts_with("function ") {
                exports.push(sig_ident(le, "function "));
            } else if le.starts_with("class ") {
                exports.push(sig_ident(le, "class "));
            } else if le.starts_with("const ") {
                exports.push(sig_ident(le, "const "));
            } else if le.starts_with("let ") {
                exports.push(sig_ident(le, "let "));
            } else if le.starts_with("type ") {
                exports.push(sig_ident(le, "type "));
            } else if le.starts_with("interface ") {
                exports.push(sig_ident(le, "interface "));
            }
        }
    }
    (dedup_preserve_order(imports), dedup_preserve_order(exports))
}

/* ================================== Utilities ================================== */

/// Capture identifier token after `prefix` (until `(`, `{`, `<`, `:`, `=`, whitespace).
pub fn sig_ident(line_after_prefix: &str, prefix: &str) -> String {
    let rest = line_after_prefix.trim_start_matches(prefix).trim();
    let mut out = String::new();
    for ch in rest.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ':' { out.push(ch); continue; }
        break;
    }
    if out.is_empty() {
        rest.split_whitespace().next().unwrap_or(rest).to_string()
    } else {
        out
    }
}

fn trim_inline_comment(l: &str) -> &str {
    // Rust: strip trailing // ... (naive; good enough for skim)
    if let Some((code, _cmt)) = l.split_once("//") { code.trim_end() } else { l }
}

fn trim_hash_comment(l: &str) -> &str {
    if let Some((code, _cmt)) = l.split_once('#') { code.trim_end() } else { l }
}

fn trim_js_comment(l: &str) -> &str {
    if let Some((code, _)) = l.split_once("//") { return code.trim_end(); }
    l
}

/// Deduplicate while preserving first occurrence order.
pub fn dedup_preserve_order(v: Vec<String>) -> Vec<String> {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::with_capacity(v.len());
    let mut out = Vec::with_capacity(v.len());
    for s in v {
        if seen.insert(s.clone()) {
            out.push(s);
        }
    }
    out
}

/* ===================================== Tests ===================================== */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_basic() {
        assert_eq!(infer_role("src/main.rs", "rust", "fn main(){}") as u8, Role::Bin as u8);
        assert_eq!(infer_role("src/lib.rs", "rust", "") as u8, Role::Lib as u8);
        assert_eq!(infer_role("tests/foo.rs", "rust", "#[test] fn t(){}") as u8, Role::Test as u8);
        assert_eq!(infer_role("docs/README.md", "md", "") as u8, Role::Doc as u8);
        assert_eq!(infer_role(".github/workflows/ci.yml", "yaml", "") as u8, Role::Config as u8);
    }

    #[test]
    fn rust_module_paths() {
        assert_eq!(rust_module_id("src/lib.rs"), "crate");
        assert_eq!(rust_module_id("src/main.rs"), "bin");
        assert_eq!(rust_module_id("src/bin/foo.rs"), "bin::foo");
        assert_eq!(rust_module_id("src/a/b.rs"), "a::b");
        assert_eq!(rust_module_id("src/a/mod.rs"), "a");
    }

    #[test]
    fn python_mods() {
        assert_eq!(python_module_id("a/b/c.py"), "a.b.c");
    }

    #[test]
    fn skim_rust_symbols() {
        let (im, ex) = skim_rust("use crate::foo::Bar;\npub struct X {}\npub fn go() {}\n");
        assert!(im.iter().any(|s| s.contains("crate::foo::Bar")));
        assert!(ex.contains(&"X".to_string()));
        assert!(ex.contains(&"go".to_string()));
    }

    #[test]
    fn skim_python_symbols() {
        let (im, ex) = skim_python("from a.b import c, d as e\nimport x, y as z\ndef f(): pass\nclass K: pass\n");
        assert!(im.contains(&"a.b.c".to_string()) && im.contains(&"a.b.d".to_string()));
        assert!(im.contains(&"x".to_string()));
        assert!(ex.contains(&"f".to_string()) && ex.contains(&"K".to_string()));
    }

    #[test]
    fn skim_ts_symbols() {
        let (im, ex) = skim_js_ts("import { a,b } from \"pkg\";\nexport class Foo {}\nexport const bar = 1;\n");
        assert!(im.contains(&"pkg".to_string()));
        assert!(ex.contains(&"Foo".to_string()) && ex.contains(&"bar".to_string()));
    }

    #[test]
    fn dedup_preserves_first() {
        let out = dedup_preserve_order(vec!["a".into(),"b".into(),"a".into()]);
        assert_eq!(out, vec!["a","b"]);
    }
}
