//! custom_view.rs — "custom index blocks" extracted from your source files.
//!
//! Block grammar:
//!   //--functions public
//!   $ # Project Functions
//!   $ *Functions and methods by module. Signatures are shown verbatim (one line).*
//!   //--end
//!
//! Categories: types|functions (aliases: type, structs, enums, fn, fns).
//! `$` lines are emitted verbatim. Generated content for the file follows.
//!
//! Add to Cargo.toml deps used by this module:
//! anyhow, serde, serde_json, syn, quote

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use syn::visit::Visit;

#[derive(Debug, Deserialize)]
struct FileIntentEntryMini {
    path: String,
    #[allow(dead_code)]
    lang: Option<String>,
}

#[derive(Debug, Clone)]
struct Section {
    category: String,
    verbatim: bool,
    _lang: String,
    render: String,
}

pub fn build_custom_from_index(index_path: &Path, output_path: &Path) -> io::Result<()> {
    let text = fs::read_to_string(index_path)?;

    // Accept JSON array or JSONL
    let mut entries: Vec<FileIntentEntryMini> = Vec::new();
    if let Ok(v) = serde_json::from_str::<Vec<FileIntentEntryMini>>(&text) {
        entries = v;
    } else {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            if let Ok(e) = serde_json::from_str::<FileIntentEntryMini>(line) {
                entries.push(e);
            }
        }
    }

    let project_root = index_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

    let mut out = String::new();

    for e in entries {
        let abs = resolve_path(&project_root, &e.path);
        if abs.extension().and_then(|s| s.to_str()) != Some("rs") || !abs.exists() { continue; }
        let lang = "rust".to_string();
        let src = match fs::read_to_string(&abs) { Ok(s) => s, Err(_) => continue };
        let sections = scan_custom_regions(&src, &lang);
        if sections.is_empty() { continue; }

        let rel = to_rel(&project_root, &abs);
        out.push_str(&format!("# {}\n\n", rel.display()));
        for s in sections {
            let cat = normalize_category(&s.category);
            out.push_str(&category_heading(&cat));
            out.push('\n');

            if s.verbatim && !s.render.is_empty() {
                out.push_str(&s.render);
                if !s.render.ends_with('\n') { out.push('\n'); }
            }

            let gen = match cat.as_str() {
                "types" => types_for_file(&abs).unwrap_or_default(),
                "functions" => functions_for_file(&abs).unwrap_or_default(),
                _ => String::new(),
            };
            out.push_str(&gen);
            out.push('\n');
        }
        out.push('\n');
    }

    fs::write(output_path, out)?;
    Ok(())
}

fn scan_custom_regions(text: &str, lang: &str) -> Vec<Section> {
    let mut out = Vec::new();
    let mut lines = text.lines().enumerate().peekable();
    while let Some((_i, line)) = lines.next() {
        let l = line.trim_start();
        if l.starts_with("//--") {
            let rest = l.trim_start_matches("//--");
            if rest.eq_ignore_ascii_case("end") { continue; }
            let mut parts = rest.split_whitespace();
            let category = parts.next().unwrap_or("").to_string();
            let _filters: Vec<String> = parts.map(|s| s.to_string()).collect();

            let mut buf = String::new();
            let mut verbatim = false;
            while let Some((_, ln)) = lines.peek() {
                let trimmed = ln.trim_start();
                if trimmed.starts_with("//--end") { lines.next(); break; }
                let (_j, cur) = lines.next().unwrap();
                if cur.trim_start().starts_with('$') {
                    verbatim = true;
                    let idx = cur.find('$').unwrap_or(0);
                    buf.push_str(&cur[idx + 1..]);
                    buf.push('\n');
                }
            }
            out.push(Section { category, verbatim, _lang: lang.to_string(), render: buf });
        }
    }
    out
}

fn normalize_category(c: &str) -> String {
    match c.to_ascii_lowercase().as_str() {
        "type" | "types" | "structs" | "enums" => "types".into(),
        "fn" | "fns" | "function" | "functions" => "functions".into(),
        other => other.into(),
    }
}

fn category_heading(c: &str) -> String {
    match c {
        "types" => "## types\n".into(),
        "functions" => "## functions\n".into(),
        _ => format!("## {}\n", c),
    }
}

fn types_for_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let src = fs::read_to_string(path)?;
    let ast = syn::parse_file(&src)?;
    let mut v = types_view_like::TypeCollector::default();
    v.visit_file(&ast);
    let mut out = String::new();
    for d in v.out {
        match d {
            types_view_like::Decl::Struct(s) => {
                // render a struct
                let vis = if s.public { "pub " } else { "" };
                out.push_str(&format!("{}struct {} {{\n", vis, s.name));

                // ⬇️ Loop over fields so `f` is in scope for the match
                for f in s.fields {
                    for a in f.attrs {
                        out.push_str(&format!("{}\n", a));
                    }
                    let vis = if f.public { "pub " } else { "" };
                    match &f.name {
                        Some(name) => out.push_str(&format!("    {}{}: {},\n", vis, name, f.ty)),
                        None       => out.push_str(&format!("    {}{},\n",      vis, f.ty)), // tuple field
                    }
                }

                out.push_str("}\n\n");
            }
            types_view_like::Decl::Enum(e) => {
                let vis = if e.public { "pub " } else { "" };
                out.push_str(&format!("{}enum {} {{ ", vis, e.name));
                for (i, v) in e.variants.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(v);
                }
                out.push_str(", }\n\n");
            }
        }
    }
    Ok(out)
}

fn functions_for_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let src = fs::read_to_string(path)?;
    let ast = syn::parse_file(&src)?;
    let mut v = functions_view_like::FnCollector::default();
    v.visit_file(&ast);

    let mut groups: functions_view_like::Groups = Default::default();
    groups.extend(v.out.into_iter());

    let mut out = String::new();
    if !groups.public.is_empty() {
        out.push_str("### public\n\n");
        for s in &groups.public { out.push_str(s); out.push('\n'); }
        out.push('\n');
    }
    if !groups.internal.is_empty() {
        out.push_str("### internal\n\n");
        for s in &groups.internal { out.push_str(s); out.push('\n'); }
        out.push('\n');
    }
    if !groups.tests.is_empty() {
        out.push_str("### tests\n\n");
        for s in &groups.tests { out.push_str(s); out.push('\n'); }
        out.push('\n');
    }
    Ok(out)
}

// ------ local helpers (path ops) ------
fn resolve_path(root: &Path, p: &str) -> PathBuf { let pb = PathBuf::from(p); if pb.is_absolute() { pb } else { root.join(pb) } }
fn to_rel(root: &Path, p: &Path) -> PathBuf { pathdiff::diff_paths(p, root).unwrap_or_else(|| p.to_path_buf()) }
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

// ---- "like" collectors so we avoid circular crate refs ----
mod types_view_like {
    use syn::{visit::Visit, Fields, Item, ItemEnum, ItemStruct};

    #[derive(Default)]
    pub struct TypeCollector { 
        pub out: Vec<Decl>, 
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

    impl<'ast> Visit<'ast> for TypeCollector {
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
                        let attrs = super::render_attrs(&f.attrs);
                        let ty = super::norm_tokens(&f.ty);
                        let public = matches!(f.vis, syn::Visibility::Public(_));
                        // named
                        let name = f.ident.as_ref().map(|id| id.to_string());
                        fields_out.push(FieldDecl { attrs, public, name, ty });
                    }
                }
                Fields::Unnamed(unnamed) => {
                    for f in &unnamed.unnamed {
                        let attrs = super::render_attrs(&f.attrs);
                        let ty = super::norm_tokens(&f.ty);
                        let public = matches!(f.vis, syn::Visibility::Public(_));
                        // unnamed
                        let name = None;
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
}

mod functions_view_like {
    use syn::{visit::Visit, ImplItem, Item, ItemFn, ItemImpl};

    #[derive(Default)]
    pub struct FnCollector { pub out: Vec<(Kind, String)>, pub in_test_mod: bool, }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Kind { Public, Internal, Test, }

    #[derive(Default)]
    pub struct Groups { pub public: Vec<String>, pub internal: Vec<String>, pub tests: Vec<String>, }

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
            if let Item::Mod(m) = i {
                let was_test = self.in_test_mod;
                let now_test = m.attrs.iter().any(|a| a.path().is_ident("cfg") && quote::ToTokens::to_token_stream(&a.meta).to_string().contains("test"));
                if now_test { self.in_test_mod = true; }
                if let Some((_brace, items)) = &m.content {
                    for it in items { self.visit_item(it); }
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
            let sig = super::norm_sig(&f.sig);
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
            let ty_name = match &*imp.self_ty {
                syn::Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default(),
                _ => String::new(),
            };
            for item in &imp.items {
                if let ImplItem::Fn(m) = item {
                    let mut sig = super::norm_sig(&m.sig);
                    if !ty_name.is_empty() {
                        sig = sig.replacen("fn ", &format!("{}::fn ", ty_name), 1);
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
}

// shared token normalizers
fn render_attrs(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs.iter().filter_map(|a| {
        let path = a.path().segments.iter().map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::");
        if ["serde", "allow", "derive"].contains(&path.as_str()) {
            Some(format!("#[{}]", norm_tokens(a.meta.clone())))
        } else { None }
    }).collect()
}
fn norm_tokens<T: quote::ToTokens>(t: T) -> String { let s = t.into_token_stream().to_string(); normalize_token_string(&s) }
fn norm_sig(sig: &syn::Signature) -> String { let s = quote::quote!(#sig).to_string(); normalize_token_string(&s) }
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