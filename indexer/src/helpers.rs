// indexer/src/helpers.rs
// Infer a coarse "role" for the file: bin/lib/test/doc/config/script/ui/core


pub fn infer_role(path: &str, lang: &str, snippet: &str) -> String {
    let p = path.to_ascii_lowercase();
    let l = lang.to_ascii_lowercase();
    let s = snippet.to_ascii_lowercase();

    // tests
    if p.contains("/tests") || p.contains("/test") ||
       p.ends_with("_test.rs") || p.ends_with("_tests.rs") ||
       s.contains("#[test]") || s.contains("pytest") {
        return "test".into();
    }

    // entrypoints / bins
    if p.ends_with("src/main.rs") || s.contains("fn main(") { return "bin".into(); }
    if p.contains("/src/bin/") { return "bin".into(); }

    // docs / configs
    if p.ends_with(".md") || p.contains("/docs") { return "doc".into(); }
    if matches!(l.as_str(), "toml" | "yaml" | "yml" | "json") { return "config".into(); }

    // scripts
    if p.ends_with(".sh") || s.starts_with("#!/bin/bash") || s.starts_with("#!/usr/bin/env bash") {
        return "script".into();
    }

    // ui-ish
    if p.contains("/ui") || p.contains("panel") || p.contains("editor") || p.contains("view") {
        return "ui".into();
    }

    // crate role hints
    if p.ends_with("lib.rs") { return "lib".into(); }
    if p.contains("core") || p.contains("engine") { return "core".into(); }

    // default: code/lib
    "lib".into()
}

// Best-effort module id (path → module), language-aware.
pub fn infer_module_id(path: &str, lang: &str) -> String {
    let p = path.trim_matches('/');
    match lang.to_ascii_lowercase().as_str() {
        "rust" => rust_module_id(p),
        "python" => python_module_id(p),
        _ => {
            // generic: strip extension, use slashes as separators
            let stem = p.trim_end_matches('/');
            let stem = stem.rsplit_once('.').map(|(a, _)| a).unwrap_or(stem);
            stem.replace('/', "::")
        }
    }
}

pub fn rust_module_id(p: &str) -> String {
    // Common cases:
    // src/lib.rs        -> crate
    // src/main.rs       -> bin
    // src/bin/foo.rs    -> bin::foo
    // src/foo/bar.rs    -> foo::bar
    // src/foo/mod.rs    -> foo
    if p.ends_with("src/lib.rs") { return "crate".into(); }
    if p.ends_with("src/main.rs") { return "bin".into(); }
    if let Some(rest) = p.strip_prefix("src/bin/") {
        let name = rest.strip_suffix(".rs").unwrap_or(rest);
        return format!("bin::{}", name);
    }
    if let Some(rest) = p.strip_prefix("src/") {
        if rest.ends_with("/mod.rs") {
            let modpath = rest.trim_end_matches("/mod.rs");
            return modpath.replace('/', "::");
        }
        let stem = rest.strip_suffix(".rs").unwrap_or(rest);
        return stem.replace('/', "::");
    }
    // Fallback: strip extension, use :: separators
    let stem = p.rsplit_once('.').map(|(a, _)| a).unwrap_or(p);
    stem.replace('/', "::")
}

pub fn python_module_id(p: &str) -> String {
    // Normalize: strip extension, convert / to .
    // tests/foo_test.py -> tests.foo_test
    let stem = p.strip_suffix(".py").unwrap_or(p);
    stem.replace('/', ".")
}

// Extract imports/exports cheaply from the snippet (no AST).
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
        let l = raw.trim();
        if l.starts_with("use ") {
            // take `use foo::bar` → "foo::bar"
            let body = l.trim_start_matches("use ").trim_end_matches(';').trim();
            if !body.is_empty() { imports.push(body.to_string()); }
        }
        if l.starts_with("pub ") {
            // pub fn/struct/enum/trait/mod <Name>
            if l.starts_with("pub fn ") {
                exports.push(sig_ident(l, "pub fn "));
            } else if l.starts_with("pub struct ") {
                exports.push(sig_ident(l, "pub struct "));
            } else if l.starts_with("pub enum ") {
                exports.push(sig_ident(l, "pub enum "));
            } else if l.starts_with("pub trait ") {
                exports.push(sig_ident(l, "pub trait "));
            } else if l.starts_with("pub mod ") {
                exports.push(sig_ident(l, "pub mod "));
            }
        }
    }
    (dedup(imports), dedup(exports))
}

pub fn skim_python(s: &str) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    for raw in s.lines() {
        let l = raw.trim();
        if l.starts_with("import ") {
            // import a, b as c → "a", "b"
            let rest = l.trim_start_matches("import ").trim();
            for part in rest.split(',') {
                let tok = part.trim().split_whitespace().next().unwrap_or("");
                if !tok.is_empty() { imports.push(tok.to_string()); }
            }
        } else if l.starts_with("from ") {
            // from x.y import z → "x.y.z"
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
    (dedup(imports), dedup(exports))
}

pub fn skim_js_ts(s: &str) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    for raw in s.lines() {
        let l = raw.trim();
        if l.starts_with("import ") {
            // import x from 'y' → 'y'
            if let Some((_, from)) = l.split_once(" from ") {
                let pkg = from.trim().trim_matches(&['"', '\'', ';'][..]).to_string();
                if !pkg.is_empty() { imports.push(pkg); }
            }
        }
        if l.starts_with("export ") {
            // export function Foo / export class Bar / export const baz
            if l.starts_with("export function ") {
                exports.push(sig_ident(l, "export function "));
            } else if l.starts_with("export class ") {
                exports.push(sig_ident(l, "export class "));
            } else if l.starts_with("export const ") {
                exports.push(sig_ident(l, "export const "));
            } else if l.starts_with("export let ") {
                exports.push(sig_ident(l, "export let "));
            }
        }
    }
    (dedup(imports), dedup(exports))
}

pub fn sig_ident(line: &str, prefix: &str) -> String {
    // Grab identifier token right after the prefix (until '(', '{', '<', ':' or whitespace)
    let rest = line.trim_start_matches(prefix).trim();
    let mut out = String::new();
    for ch in rest.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ':' { out.push(ch); continue; }
        break;
    }
    if out.is_empty() { rest.split_whitespace().next().unwrap_or(rest).to_string() } else { out }
}

pub fn dedup(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v.dedup();
    v
}
