//! types_view.rs — renders "Project Types" grouped by source file, showing
//! structs/enums with field/variant names verbatim. Includes attributes on fields.
//!
//! Accepts JSON array or JSONL index files. Only `.rs` or `lang=="rust"` entries are parsed.
//!
//! Add to Cargo.toml:
//! ```toml
//! [dependencies]
//! anyhow = "1"
//! serde = { version = "1", features = ["derive"] }
//! serde_json = "1"
//! syn = { version = "2", features = ["full", "extra-traits", "printing"] }
//! quote = "1"
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use syn::{visit::Visit, Attribute, Fields, Item, ItemEnum, ItemStruct};

use crate::map_view::indent;

#[derive(Debug, Deserialize)]
struct FileIntentEntryMini {
    path: String,
    #[allow(dead_code)]
    lang: Option<String>,
}

pub fn build_types_from_index(index_path: &Path, output_path: &Path) -> io::Result<()> {
    let text = fs::read_to_string(index_path)?;

    // Accept JSON array or JSONL
    let mut entries: Vec<FileIntentEntryMini> = Vec::new();
    let mut loaded = false;
    if let Ok(v) = serde_json::from_str::<Vec<FileIntentEntryMini>>(&text) {
        entries = v;
        loaded = true;
    }
    if !loaded {
        for (lineno, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() { continue; }
            match serde_json::from_str::<FileIntentEntryMini>(line) {
                Ok(e) => entries.push(e),
                Err(err) => eprintln!("[types_view] skip line {}: {}", lineno + 1, err),
            }
        }
    }

    let project_root = project_root_from_index(index_path);

    let mut per_file: BTreeMap<PathBuf, Vec<Decl>> = BTreeMap::new();

    for e in entries {
        let path = resolve_path(&project_root, &e.path);
        let is_rust_ext = path.extension().and_then(|s| s.to_str()) == Some("rs");
        let is_rust_tag = e.lang.as_deref() == Some("rust");
        if !(is_rust_ext || is_rust_tag) || !path.exists() {
            continue;
        }
        let file_src = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Ok(ast) = syn::parse_file(&file_src) {
            let mut v = TypeCollector::default();
            v.visit_file(&ast);
            if !v.out.is_empty() {
                per_file.entry(to_rel(&project_root, &path)).or_default().extend(v.out);
            }
        }
    }

    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut out = fs::File::create(output_path)?;
    writeln!(out, "# Project Types")?;
    writeln!(out)?;
    writeln!(out, "*Project structs/enums by module. Field and variant names shown verbatim.*")?;
    writeln!(out)?;

    for (path, decls) in per_file {
        writeln!(out, "# {}", path.display())?;
        writeln!(out)?;
        for d in decls {
            match d {
                Decl::Struct(s) => {
                    // render a struct
                    let vis = if s.public { "pub " } else { "" };
                    writeln!(out, "{}struct {} {{", vis, s.name)?;

                    // ⬇️ This for-loop was missing, which is why `f` was “not found”
                    for f in s.fields {
                        for a in f.attrs {
                            writeln!(out, "{}", a)?;
                        }
                        let vis = if f.public { "pub " } else { "" };
                        match &f.name {
                            Some(name) => writeln!(out, "{}{}{}: {},", indent(1), vis, name, f.ty)?,
                            None       => writeln!(out, "{}{}{},",      indent(1), vis, f.ty)?, // tuple field
                        }
                    }

                    writeln!(out, "}}")?;
                    writeln!(out, "")?;
                }
                Decl::Enum(e) => {
                    let vis = if e.public { "pub " } else { "" };
                    write!(out, "{}enum {} {{ ", vis, e.name)?;
                    for (i, v) in e.variants.iter().enumerate() {
                        if i > 0 { write!(out, ", ")?; }
                        write!(out, "{}", v)?;
                    }
                    writeln!(out, ", }}")?;
                    writeln!(out)?;
                }
            }
        }
    }

    Ok(())
}

fn resolve_path(root: &Path, p: &str) -> PathBuf { let pb = PathBuf::from(p); if pb.is_absolute() { pb } else { root.join(pb) } }
fn to_rel(root: &Path, p: &Path) -> PathBuf { pathdiff::diff_paths(p, root).unwrap_or_else(|| p.to_path_buf()) }

#[derive(Default)]
pub struct TypeCollector { pub out: Vec<Decl>, }

impl<'ast> syn::visit::Visit<'ast> for TypeCollector {
    fn visit_item(&mut self, i: &'ast Item) {
        match i {
            Item::Struct(s) => self.push_struct(s),
            Item::Enum(e) => self.push_enum(e),
            Item::Mod(m) => {
                if let Some((_brace, items)) = &m.content {
                    for it in items { self.visit_item(it); }
                }
            }
            _ => {}
        }
    }
}

impl TypeCollector {
    fn push_struct(&mut self, s: &ItemStruct) {
        let mut fields_out = Vec::new();
        match &s.fields {
            Fields::Named(named) => {
                for f in &named.named {
                    let attrs  = render_attrs(&f.attrs);
                    let ty     = norm_tokens(&f.ty);
                    let public = matches!(f.vis, syn::Visibility::Public(_));
                    let name   = f.ident.as_ref().map(|id| id.to_string()); // <-- NEW
                    fields_out.push(FieldDecl { attrs, public, name, ty }); // <-- name included
                }
            }
            Fields::Unnamed(unnamed) => {
                for f in &unnamed.unnamed {
                    let attrs  = render_attrs(&f.attrs);
                    let ty     = norm_tokens(&f.ty);
                    let public = matches!(f.vis, syn::Visibility::Public(_));
                    let name   = None; // <-- NEW
                    fields_out.push(FieldDecl { attrs, public, name, ty });
                }
            }
            Fields::Unit => {}
        }
        let public = matches!(s.vis, syn::Visibility::Public(_));
        self.out.push(Decl::Struct(StructDecl { name: s.ident.to_string(), public, fields: fields_out }));
    }

    fn push_enum(&mut self, e: &ItemEnum) {
        let public = matches!(e.vis, syn::Visibility::Public(_));
        let variants = e.variants.iter().map(|v| v.ident.to_string()).collect::<Vec<_>>();
        self.out.push(Decl::Enum(EnumDecl { name: e.ident.to_string(), public, variants }));
    }
}

#[derive(Debug)] 
pub enum Decl { 
    Struct(StructDecl), 
    Enum(EnumDecl) 
}
#[derive(Debug)] 
pub struct StructDecl { 
    pub name: String, 
    pub public: bool, 
    pub fields: Vec<FieldDecl>, 
}
#[derive(Debug)] 
pub struct FieldDecl { 
    pub attrs: Vec<String>, 
    pub public: bool,
    pub name: Option<String>, 
    pub ty: String, 
}
#[derive(Debug)] 
pub struct EnumDecl { 
    pub name: String, 
    pub public: bool, 
    pub variants: Vec<String>, 
}

fn render_attrs(attrs: &[Attribute]) -> Vec<String> {
    attrs.iter().filter_map(|a| {
        let path = a.path().segments.iter().map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::");
        if ["serde", "allow", "derive"].contains(&path.as_str()) {
            Some(format!("#[{}]", norm_tokens(a.meta.clone())))
        } else { None }
    }).collect()
}

fn norm_tokens<T: quote::ToTokens>(t: T) -> String {
    let s = t.into_token_stream().to_string();
    normalize_token_string(&s)
}

fn normalize_token_string(s: &str) -> String {
    let mut out = s.to_string();
    for (a, b) in [
        (" < ", "<"), (" > ", ">"), (" ( ", "("), (" ) ", ")"),
        (" [ ", "["), (" ] ", "]"), (" , ", ", "), (" : : ", "::"),
        (" & '", "&'"), (" & ", " &"), (" :: ", "::"), (" = > ", "=>"),
        (" | ", "|"), (" ;", ";"),
    ] { out = out.replace(a, b); }
    out = out.replace(" ,", ",").replace(" :", ":");
    out
}

fn project_root_from_index(index_path: &Path) -> PathBuf {
    // .gpt_index/indexes/<slug>.jsonl  ->  project root is three parents up
    index_path
        .parent()  // indexes/
        .and_then(|p| p.parent())  // .gpt_index/
        .and_then(|p| p.parent())  // <project root>
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
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
                (Some(a), Some(b)) if a == b => { ita.next(); itb.next(); }
                _ => break,
            }
        }
        for _ in itb { comps.push(Component::ParentDir); }
        comps.extend(ita);
        let mut p = PathBuf::new();
        for c in comps { p.push(c.as_os_str()); }
        Some(p)
    }
}