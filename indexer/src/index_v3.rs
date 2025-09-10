// indexer/src/index_v3.rs
use std::{fs, path::{Path}};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::scan::read_index;
use syn::{Item, ItemStruct, ItemEnum, ItemImpl, ImplItem, ItemFn};

const CHUNK: usize = 16 * 1024;

#[derive(Serialize)]
pub struct IndexPack {
  format: &'static str,
  version: &'static str,
  hash_algo: &'static str,
  pack_id: String,
  created_utc: String,
  lang: LangMeta,
  rules: Rules,
  files: Vec<FileEntry>,
}

#[derive(Serialize)]
struct LangMeta { primary: &'static str, dialect: &'static str }
#[derive(Serialize)]
struct Rules {
  mode: &'static str,
  patch_contract: PatchContract,
}
#[derive(Serialize)]
struct PatchContract {
  diff_format: &'static str,
  limit_scope_to_verified_anchors: bool,
}

#[derive(Serialize)]
struct FileEntry {
  path: String,
  language: String,
  size_bytes: usize,
  line_count: usize,
  encoding: &'static str,
  eol: &'static str,
  file_sha256: String,
  chunks: ChunkSet,
  anchors: Vec<Anchor>,
}
#[derive(Serialize)]
struct ChunkSet {
  chunk_size_bytes: usize,
  merkle_root: String,
  list: Vec<Chunk>,
}
#[derive(Serialize)]
struct Chunk { index: usize, offset: usize, length: usize, sha256: String }

#[derive(Serialize)]
struct Anchor {
  kind: &'static str,
  name: String,
  visibility: String,
  signature: Option<String>,
  range: Range,
  slice_sha256: String,
  verbatim_b64: String,
  schema: Option<Schema>,
}
#[derive(Serialize)]
struct Range { start_line: usize, end_line: usize }
#[derive(Serialize)]
struct Schema {
  fields: Option<Vec<Field>>,
  variants: Option<Vec<String>>,
  params: Option<Vec<(String, String)>>,
  returns: Option<String>,
}
#[derive(Serialize)]
struct Field { name: String, ty: String, public: bool }

pub fn build_index_v3(index_path: &Path, project_root: &Path, out_path: &Path) -> Result<()> {
  let entries = read_index(index_path).context("read_index")?; // JSONL or JSON array
  let mut files = Vec::new();
  for e in entries {
    let abs = project_root.join(&e.path);
    let src = fs::read_to_string(&abs)
      .with_context(|| format!("read {}", abs.display()))?;
    let bytes = src.as_bytes();
    let file_sha256 = hex256(bytes);
    let size_bytes = bytes.len();
    let line_count = src.lines().count();
    let (chunks, merkle_root) = chunk_and_merkle(bytes);
    let anchors = if e.lang.eq_ignore_ascii_case("rust") || e.path.ends_with(".rs") {
      extract_rust_anchors(&src)?
    } else { Vec::new() };
    files.push(FileEntry {
      path: e.path.clone(),
      language: e.lang.clone(),
      size_bytes,
      line_count,
      encoding: "utf-8",
      eol: "lf",
      file_sha256,
      chunks: ChunkSet { chunk_size_bytes: CHUNK, merkle_root, list: chunks },
      anchors,
    });
  }
  let pack = IndexPack {
    format: "LLM-CODE-INDEX",
    version: "3.0",
    hash_algo: "sha256",
    pack_id: crate::util::prefixed_filename("PACK", "uuid"), // placeholder slug
    created_utc: crate::util::now_timestamp(),
    lang: LangMeta { primary: "rust", dialect: "edition2021" },
    rules: Rules {
      mode: "strict",
      patch_contract: PatchContract { diff_format: "unified", limit_scope_to_verified_anchors: true },
    },
    files,
  };
  let json = serde_json::to_string_pretty(&pack)?;
  crate::util::safe_write(out_path, json)?;
  Ok(())
}

fn hex256(data: impl AsRef<[u8]>) -> String {
  let mut h = Sha256::new(); h.update(data.as_ref()); hex::encode(h.finalize())
}
fn chunk_and_merkle(bytes: &[u8]) -> (Vec<Chunk>, String) {
  let mut list = Vec::new();
  let mut digests = Vec::new();
  let mut i = 0usize; let mut off = 0usize;
  while off < bytes.len() {
    let end = (off + CHUNK).min(bytes.len());
    let slice = &bytes[off..end];
    let d = hex256(slice);
    list.push(Chunk { index: i, offset: off, length: slice.len(), sha256: d.clone() });
    digests.push(hex::decode(d).unwrap());
    off = end; i += 1;
  }
  // simple pairwise Merkle over raw digests (sha256)
  let mut layer = digests;
  while layer.len() > 1 {
    let mut next = Vec::new();
    for pair in layer.chunks(2) {
      let combined = if pair.len() == 2 { [pair[0].as_slice(), pair[1].as_slice()].concat() } else { pair[0].clone() };
      next.push(Sha256::digest(&combined).to_vec());
    }
    layer = next;
  }
  let root = layer.first().map(|d| hex::encode(d)).unwrap_or_else(|| hex256(&[]));
  (list, root)
}

fn extract_rust_anchors(src: &str) -> Result<Vec<Anchor>> {
  let file = syn::parse_file(src).context("parse rust")?;
  let mut out = Vec::new();
  for item in file.items {
    match item {
      Item::Struct(s) => out.push(struct_anchor(src, s)?),
      Item::Enum(e)   => out.push(enum_anchor(src, e)?),
      Item::Impl(i)   => out.extend(impl_anchors(src, i)?),
      Item::Fn(f)     => out.push(fn_anchor(src, f)?),
      _ => {}
    }
  }
  Ok(out)
}

fn struct_anchor(src: &str, s: ItemStruct) -> Result<Anchor> {
  let name = s.ident.to_string();
  let (start, end) = find_balanced_block(src, "struct", &name).context("struct slice")?;
  let slice = &src[start..end];
  let fields = match &s.fields {
    syn::Fields::Named(named) => named.named.iter().map(|f| Field {
      name: f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default(),
      ty: crate::types_view::norm_tokens(&f.ty), // you already normalize tokens in views :contentReference[oaicite:8]{index=8}
      public: matches!(f.vis, syn::Visibility::Public(_)),
    }).collect::<Vec<_>>(),
    _ => vec![],
  };
  Ok(Anchor {
    kind: "struct",
    name,
    visibility: if matches!(s.vis, syn::Visibility::Public(_)) { "pub".into() } else { "priv".into() },
    signature: None,
    range: line_range(src, start, end),
    slice_sha256: hex256(slice),
    verbatim_b64: B64.encode(slice),
    schema: Some(Schema { fields: Some(fields), variants: None, params: None, returns: None }),
  })
}
fn enum_anchor(src: &str, e: ItemEnum) -> Result<Anchor> {
  let name = e.ident.to_string();
  let (start, end) = find_balanced_block(src, "enum", &name).context("enum slice")?;
  let slice = &src[start..end];
  let variants = e.variants.iter().map(|v| v.ident.to_string()).collect::<Vec<_>>();
  Ok(Anchor {
    kind: "enum",
    name,
    visibility: if matches!(e.vis, syn::Visibility::Public(_)) { "pub".into() } else { "priv".into() },
    signature: None,
    range: line_range(src, start, end),
    slice_sha256: hex256(slice),
    verbatim_b64: B64.encode(slice),
    schema: Some(Schema { fields: None, variants: Some(variants), params: None, returns: None }),
  })
}
fn impl_anchors(src: &str, i: ItemImpl) -> Result<Vec<Anchor>> {
  let mut out = Vec::new();
  for item in i.items {
    if let ImplItem::Fn(f) = item {
      let is_pub = matches!(f.vis, syn::Visibility::Public(_));
      out.push(fn_anchor_sig(src, &f.sig, is_pub)?);
    }
  }
  Ok(out)
}
fn fn_anchor(src: &str, f: ItemFn) -> Result<Anchor> {
  let is_pub = matches!(f.vis, syn::Visibility::Public(_));
  fn_anchor_sig(src, &f.sig, is_pub)
}

/// Build an anchor from a function/method signature (works for free fns and impl methods).
fn fn_anchor_sig(src: &str, sig: &syn::Signature, is_pub: bool) -> Result<Anchor> {
  let name = sig.ident.to_string();
  let (start, end) = find_balanced_block(src, "fn", &name).context("fn slice")?;
  let slice = &src[start..end];
  let ret = match &sig.output {
    syn::ReturnType::Default => "()".into(),
    syn::ReturnType::Type(_, t) => crate::types_view::norm_tokens(&**t),
  };
  Ok(Anchor {
    kind: "fn",
    name,
    visibility: if is_pub { "pub".into() } else { "priv".into() },
    signature: Some(crate::functions_view::norm_sig(sig)),
    range: line_range(src, start, end),
    slice_sha256: hex256(slice),
    verbatim_b64: B64.encode(slice),
    // Keep params None; the normalized signature already carries full arg info.
    schema: Some(Schema { fields: None, variants: None, params: None, returns: Some(ret) }),
  })
}

fn line_range(src: &str, start: usize, end: usize) -> Range {
  #[inline]
  fn line_of(src: &str, pos: usize) -> usize {
    // Count '\n' strictly before pos → 1-based line number.
    src[..pos].as_bytes().iter().filter(|&&b| b == b'\n').count() + 1
  }
  Range {
    start_line: line_of(src, start),
    end_line:   line_of(src, end),
  }
}

/// Find `keyword ident { ...balanced... }` and return [start,end) byte offsets.
fn find_balanced_block(src: &str, kw: &str, ident: &str) -> Result<(usize, usize)> {
  let needle = format!("{kw} {ident}");
  let start_kw = src.find(&needle)
    .with_context(|| format!("cannot find '{needle}'"))?;
  // scan forward to the first '{' that is NOT in a string/comment/char
  let mut i = start_kw;
  let bytes = src.as_bytes();
  let len = bytes.len();
  let mut line_comment = false;
  let mut block_comment: i32 = 0;
  let mut in_str = false;
  let mut in_char = false;
  let mut raw_hashes: Option<usize> = None; // Some(N) => in raw string with N '#'
  while i < len {
    let c = bytes[i];
    let next = |j: usize| if j + 1 < len { bytes[j + 1] } else { 0 };
    // end line comment
    if line_comment {
      if c == b'\n' { line_comment = false; }
      i += 1; continue;
    }
    // end block comment
    if block_comment > 0 {
      if c == b'/' && i > 0 && bytes[i - 1] == b'*' { block_comment -= 1; }
      else if c == b'*' && next(i) == b'/' { /* handled on next iter */ }
      // start nested
      if c == b'/' && next(i) == b'*' { block_comment += 1; i += 1; }
      i += 1; continue;
    }
    // end normal string
    if in_str {
      if let Some(n) = raw_hashes {
        // raw string: look for quote followed by n '#'
        if c == b'"' {
          let mut k = 0usize;
          while i + 1 + k < len && bytes[i + 1 + k] == b'#' { k += 1; }
          if k == n { in_str = false; raw_hashes = None; i += k; }
        }
      } else {
        if c == b'\\' { i += 2; continue; } // escape
        if c == b'"' { in_str = false; }
      }
      i += 1; continue;
    }
    // end char literal
    if in_char {
      if c == b'\\' { i += 2; continue; }
      if c == b'\'' { in_char = false; }
      i += 1; continue;
    }
    // detect starts
    if c == b'/' && next(i) == b'/' { line_comment = true; i += 2; continue; }
    if c == b'/' && next(i) == b'*' { block_comment += 1; i += 2; continue; }
    if c == b'r' {
      // raw string prefix: r###"
      let mut j = i + 1;
      let mut hashes = 0usize;
      while j < len && bytes[j] == b'#' { hashes += 1; j += 1; }
      if j < len && bytes[j] == b'"' {
        in_str = true; raw_hashes = Some(hashes); i = j + 1; continue;
      }
    }
    if c == b'"' { in_str = true; raw_hashes = None; i += 1; continue; }
    if c == b'\'' { in_char = true; i += 1; continue; }
    if c == b'{' { // found the body start
      let start_brace = i;
      // now consume balanced braces with the same lexer rules
      let mut depth: i32 = 0;
      let mut j = start_brace;
      let mut lc = false;
      let mut bc: i32 = 0;
      let mut s = false;
      let mut ch = false;
      let mut rs: Option<usize> = None;
      while j < len {
        let d = bytes[j];
        let nxt = |t: usize| if t + 1 < len { bytes[t + 1] } else { 0 };
        if lc { if d == b'\n' { lc = false; } j += 1; continue; }
        if bc > 0 {
          if d == b'/' && nxt(j) == b'*' { bc += 1; j += 2; continue; }
          if d == b'*' && nxt(j) == b'/' { bc -= 1; j += 2; continue; }
          j += 1; continue;
        }
        if s {
          if let Some(n) = rs {
            if d == b'"' {
              let mut k = 0usize; while j + 1 + k < len && bytes[j + 1 + k] == b'#' { k += 1; }
              if k == n { s = false; rs = None; j += 1 + k; continue; }
            }
            j += 1; continue;
          } else {
            if d == b'\\' { j += 2; continue; }
            if d == b'"' { s = false; j += 1; continue; }
            j += 1; continue;
          }
        }
        if ch {
          if d == b'\\' { j += 2; continue; }
          if d == b'\'' { ch = false; j += 1; continue; }
          j += 1; continue;
        }
        if d == b'/' && nxt(j) == b'/' { lc = true; j += 2; continue; }
        if d == b'/' && nxt(j) == b'*' { bc += 1; j += 2; continue; }
        if d == b'r' {
          let mut t = j + 1; let mut h = 0usize;
          while t < len && bytes[t] == b'#' { h += 1; t += 1; }
          if t < len && bytes[t] == b'"' { s = true; rs = Some(h); j = t + 1; continue; }
        }
        if d == b'"' { s = true; rs = None; j += 1; continue; }
        if d == b'\'' { ch = true; j += 1; continue; }
        if d == b'{' { depth += 1; j += 1; continue; }
        if d == b'}' {
          depth -= 1; j += 1;
          if depth == 0 { return Ok((start_kw, j)); }
          continue;
        }
        j += 1;
      }
      anyhow::bail!("unbalanced braces for {kw} {ident}");
    }
    i += 1;
  }
  anyhow::bail!("no body '{{' found for {kw} {ident}")
}
