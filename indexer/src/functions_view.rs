//! functions_view.rs — renders "Project Functions" grouped by file and
//! split into Public / Internal / Tests sections. Method names are prefixed
//! with `Type::` when inside impl blocks. Signatures are one-line, verbatim.
//!
//! Add to Cargo.toml:
//! ```toml
//! [dependencies]
//! anyhow = "1"
//! serde = { version = "1", features = ["derive"] }
//! serde_json = "1"
//! walkdir = "2"
//! syn = { version = "2", features = ["full", "extra-traits", "printing"] }
//! quote = "1"
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use syn::{visit::Visit, ImplItem, Item, ItemFn, ItemImpl};

#[derive(Debug, Deserialize)]
struct FileIntentEntryMini {
    path: String,
    #[allow(dead_code)]
    lang: Option<String>,
}

pub fn build_functions_from_index(index_path: &Path, output_path: &Path) -> io::Result<()> {
    // Load entries from JSONL (one object per line), but also accept a JSON array fallback.
    let text = fs::read_to_string(index_path)?;

    let mut entries: Vec<FileIntentEntryMini> = Vec::new();
    let mut loaded = false;

    // Try JSON array first
    if let Ok(v) = serde_json::from_str::<Vec<FileIntentEntryMini>>(&text) {
        entries = v;
        loaded = true;
    }
    if !loaded {
        // Fallback: JSONL stream
        for (lineno, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() { continue; }
            match serde_json::from_str::<FileIntentEntryMini>(line) {
                Ok(e) => entries.push(e),
                Err(err) => {
                    // Ignore malformed lines, but keep going.
                    eprintln!("[functions_view] skip line {}: {}", lineno + 1, err);
                }
            }
        }
    }

    // Resolve project root from index location
    let project_root = index_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let mut per_file: BTreeMap<PathBuf, Groups> = BTreeMap::new();

    for e in entries {
        let path = resolve_path(&project_root, &e.path);
        // Accept only Rust sources by extension, or if lang == Some("rust").
        let is_rust_ext = path.extension().and_then(|s| s.to_str()) == Some("rs");
        let is_rust_tag = e.lang.as_deref() == Some("rust");
        if !(is_rust_ext || is_rust_tag) || !path.exists() {
            continue;
        }
        let file_src = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let ast = match syn::parse_file(&file_src) {
            Ok(ast) => ast,
            Err(_) => continue,
        };
        let mut v = FnCollector::default();
        v.visit_file(&ast);
        if !v.out.is_empty() {
            per_file
                .entry(to_rel(&project_root, &path))
                .or_default()
                .extend(v.out.into_iter());
        }
    }

    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut out = fs::File::create(output_path)?;
    writeln!(out, "# Project Functions")?;
    writeln!(out, "")?;
    writeln!(out, "*Functions and methods by module. Signatures are shown verbatim (one line).*")?;
    writeln!(out, "")?;

    for (path, groups) in per_file {
        writeln!(out, "# {}", path.display())?;
        writeln!(out, "")?;

        if !groups.public.is_empty() {
            writeln!(out, "## public")?;
            writeln!(out, "")?;
            for s in &groups.public {
                writeln!(out, "{}", s)?;
            }
            writeln!(out, "")?;
        }

        if !groups.internal.is_empty() {
            writeln!(out, "## internal")?;
            writeln!(out, "")?;
            for s in &groups.internal {
                writeln!(out, "{}", s)?;
            }
            writeln!(out, "")?;
        }

        if !groups.tests.is_empty() {
            writeln!(out, "## tests")?;
            writeln!(out, "")?;
            for s in &groups.tests {
                writeln!(out, "{}", s)?;
            }
            writeln!(out, "")?;
        }
    }

    Ok(())
}

#[derive(Default)]
pub struct FnCollector {
    pub out: Vec<(Kind, String)>,
    pub in_test_mod: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Public,
    Internal,
    Test,
}

#[derive(Default)]
pub struct Groups {
    pub public: Vec<String>,
    pub internal: Vec<String>,
    pub tests: Vec<String>,
}

impl Groups {
    pub fn extend(&mut self, it: impl Iterator<Item = (Kind, String)>) {
        for (k, s) in it {
            match k {
                Kind::Public => self.public.push(s),
                Kind::Internal => self.internal.push(s),
                Kind::Test => self.tests.push(s),
            }
        }
    }
}

impl<'ast> Visit<'ast> for FnCollector {
    fn visit_item(&mut self, i: &'ast Item) {
        // Track entering a #[cfg(test)] mod to catch test helpers too.
        if let Item::Mod(m) = i {
            let was_test = self.in_test_mod;
            let now_test = m
                .attrs
                .iter()
                .any(|a| a.path().is_ident("cfg") && quote::ToTokens::to_token_stream(&a.meta).to_string().contains("test"));
            if now_test {
                self.in_test_mod = true;
            }
            // Recurse into inline module content, if any.
            if let Some((_brace, items)) = &m.content {
                for it in items {
                    self.visit_item(it);
                }
            }
            self.in_test_mod = was_test;
            return;
        }

        match i {
            Item::Fn(f) => self.push_free_fn(f),
            Item::Impl(imp) => self.push_impl(imp),
            _ => {}
        }
    }
}

impl FnCollector {
    fn push_free_fn(&mut self, f: &ItemFn) {
        let is_test_attr = f.attrs.iter().any(|a| a.path().is_ident("test"));
        let sig = norm_sig(&f.sig);
        let rendered = format!("{} {{", sig);
        let kind = if is_test_attr || self.in_test_mod {
            Kind::Test
        } else if matches!(f.vis, syn::Visibility::Public(_)) {
            Kind::Public
        } else {
            Kind::Internal
        };
        self.out.push((kind, rendered));
    }

    fn push_impl(&mut self, imp: &ItemImpl) {
        // Identify the self type (e.g., FileIntentEntry).
        let ty_name = match &*imp.self_ty {
            syn::Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
            _ => String::new(),
        };
        // Iterate impl items for methods.
        for item in &imp.items {
            if let ImplItem::Fn(m) = item {
                let mut sig = norm_sig(&m.sig);
                // Prefix with Type::
                if !ty_name.is_empty() {
                    sig = sig.replacen("fn ", &format!("{}::fn ", ty_name), 1);
                    // Public methods: change "Type::fn" to "Type::pub fn" when vis is pub
                    if matches!(m.vis, syn::Visibility::Public(_)) {
                        sig = sig.replacen(&format!("{}::fn", ty_name), &format!("{}::pub fn", ty_name), 1);
                    }
                }
                let is_test = m.attrs.iter().any(|a| a.path().is_ident("test")) || self.in_test_mod;
                let kind = if is_test {
                    Kind::Test
                } else if matches!(m.vis, syn::Visibility::Public(_)) {
                    Kind::Public
                } else {
                    Kind::Internal
                };
                let rendered = format!("{} {{", sig);
                self.out.push((kind, rendered));
            }
        }
    }
}

fn norm_sig(sig: &syn::Signature) -> String {
    let s = quote::quote!(#sig).to_string();
    normalize_token_string(&s)
}

fn resolve_path(root: &Path, p: &str) -> PathBuf {
    let pb = PathBuf::from(p);
    if pb.is_absolute() {
        pb
    } else {
        root.join(pb)
    }
}

fn to_rel(root: &Path, p: &Path) -> PathBuf {
    pathdiff::diff_paths(p, root).unwrap_or_else(|| p.to_path_buf())
}

fn normalize_token_string(s: &str) -> String {
    let mut out = s.to_string();
    for (a, b) in [
        (" < ", "<"),
        (" > ", ">"),
        (" ( ", "("),
        (" ) ", ")"),
        (" [ ", "["),
        (" ] ", "]"),
        (" , ", ", "),
        (" : : ", "::"),
        (" & '", "&'"),
        (" & ", " &"),
        (" :: ", "::"),
        (" = > ", "=>"),
        (" | ", "|"),
        (" ;", ";"),
    ] {
        out = out.replace(a, b);
    }
    out = out.replace(" ,", ",");
    out = out.replace(" :", ":");
    out
}

// tiny single-file dep to compute relative paths
mod pathdiff {
    use std::path::{Component, Path, PathBuf};

    pub fn diff_paths(path: &Path, base: &Path) -> Option<PathBuf> {
        let mut ita = path.components();
        let mut itb = base.components();
        let mut comps: Vec<Component> = Vec::new();
        loop {
            match (ita.clone().next(), itb.clone().next()) {
                (Some(a), Some(b)) if a == b => {
                    ita.next();
                    itb.next();
                }
                _ => break,
            }
        }
        for _ in itb {
            comps.push(Component::ParentDir);
        }
        comps.extend(ita);
        let mut p = PathBuf::new();
        for c in comps {
            p.push(c.as_os_str());
        }
        Some(p)
    }
}