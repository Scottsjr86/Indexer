// indexer/src/index_v3.rs
use std::{fs, path::{Path}, env};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Serialize;
use sha2::{Digest, Sha256};
use proc_macro2::Span;
use crate::scan::read_index;
use syn::{
  ImplItem, ImplItemFn, Item, ItemEnum, ItemFn, ItemImpl, ItemStruct
};

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
  // Prefer span-based bounds:
  //   start := `struct` keyword span start
  //   end   := for {..} use close brace; for (..), use paren close (or semicolon if present); for unit, use semicolon
  let start_off = span_start_offset(src, s.struct_token.span);
  let mut end_off: Option<usize> = None;
  match &s.fields {
    syn::Fields::Named(named) => {
      if let (Some(close), Some(_open)) = (
        span_start_offset(src, named.brace_token.span.close()),
        span_start_offset(src, named.brace_token.span.open()),
      ) {
        end_off = Some(close.saturating_add(1));
      }
    }
    syn::Fields::Unnamed(unnamed) => {
      let close_paren = span_start_offset(src, unnamed.paren_token.span.close());
      let semi = s.semi_token.as_ref().and_then(|t| span_start_offset(src, t.span));
      end_off = match (semi, close_paren) {
        (Some(semi), _) => Some(semi.saturating_add(1)), // include ';'
        (None, Some(cp)) => Some(cp.saturating_add(1)),  // up to ')'
        _ => None,
      };
    }
    syn::Fields::Unit => {
      let semi = s.semi_token.as_ref().and_then(|t| span_start_offset(src, t.span));
      if let Some(semi) = semi {
        end_off = Some(semi.saturating_add(1));
      }
    }
  }
  let (start, end) = match (start_off, end_off) {
    (Some(st), Some(en)) => (st, en),
    _ => {
      // Fallback to robust token search (handles braces; unit/tuple will still succeed if present)
      find_balanced_block(src, "struct", &name).context("struct slice (fallback)")?
    }
  };
  let slice = &src[start..end];
  let fields = match &s.fields {
    syn::Fields::Named(named) => named.named.iter().map(|f| Field {
      name: f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default(),
      ty: crate::types_view::norm_tokens(&f.ty),
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
  // Span-first: `enum` keyword to closing brace
  if let (Some(start), Some(close)) = (
    span_start_offset(src, e.enum_token.span),
    span_start_offset(src, e.brace_token.span.close()),
  ) {
    let end = close.saturating_add(1);
    let slice = &src[start..end];
    let variants = e.variants.iter().map(|v| v.ident.to_string()).collect::<Vec<_>>();
    return Ok(Anchor {
      kind: "enum",
      name,
      visibility: if matches!(e.vis, syn::Visibility::Public(_)) { "pub".into() } else { "priv".into() },
      signature: None,
      range: line_range(src, start, end),
      slice_sha256: hex256(slice),
      verbatim_b64: B64.encode(slice),
      schema: Some(Schema { fields: None, variants: Some(variants), params: None, returns: None }),
    });
  }
  // Fallback
  let (start, end) = find_balanced_block(src, "enum", &name).context("enum slice (fallback)")?;
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
    if let ImplItem::Fn(f) = item { out.push(fn_anchor_from_impl(src, &f)?); }
  }
  Ok(out)
}
fn fn_anchor(src: &str, f: ItemFn) -> Result<Anchor> {
  // Prefer precise span-based bounds:
  // start at `fn` token span, end at closing brace span (+1 to include '}')
  if let (Some(open_off), Some(close_off)) = (
      span_start_offset(src, f.block.brace_token.span.open()),
      span_start_offset(src, f.block.brace_token.span.close())
  ) {
    let fn_off = span_start_offset(src, f.sig.fn_token.span).unwrap_or(open_off.saturating_sub(128));
    let end_after_close = close_off.saturating_add(1);
    let slice = &src[fn_off..end_after_close];
    let is_pub = matches!(f.vis, syn::Visibility::Public(_));
    let ret = match &f.sig.output {
      syn::ReturnType::Default => "()".into(),
      syn::ReturnType::Type(_, t) => crate::types_view::norm_tokens(&**t),
    };
    return Ok(Anchor {
      kind: "fn",
      name: f.sig.ident.to_string(),
      visibility: if is_pub { "pub".into() } else { "priv".into() },
      signature: Some(crate::functions_view::norm_sig(&f.sig)),
      range: line_range(src, fn_off, end_after_close),
      slice_sha256: hex256(slice),
      verbatim_b64: B64.encode(slice),
      schema: Some(Schema { fields: None, variants: None, params: None, returns: Some(ret) }),
    });
  }
  // Fallback: signature-driven finder
  let is_pub = matches!(f.vis, syn::Visibility::Public(_));
  fn_anchor_sig(src, &f.sig, is_pub)
}

fn fn_anchor_from_impl(src: &str, f: &ImplItemFn) -> Result<Anchor> {
  // Same strategy for impl methods: use open/close brace spans.
  if let (Some(open_off), Some(close_off)) = (
      span_start_offset(src, f.block.brace_token.span.open()),
      span_start_offset(src, f.block.brace_token.span.close())
  ) {
    let fn_off = span_start_offset(src, f.sig.fn_token.span).unwrap_or(open_off.saturating_sub(128));
    let end_after_close = close_off.saturating_add(1);
    let slice = &src[fn_off..end_after_close];
    let is_pub = matches!(f.vis, syn::Visibility::Public(_));
    let ret = match &f.sig.output {
      syn::ReturnType::Default => "()".into(),
      syn::ReturnType::Type(_, t) => crate::types_view::norm_tokens(&**t),
    };
    return Ok(Anchor {
      kind: "fn",
      name: f.sig.ident.to_string(),
      visibility: if is_pub { "pub".into() } else { "priv".into() },
      signature: Some(crate::functions_view::norm_sig(&f.sig)),
      range: line_range(src, fn_off, end_after_close),
      slice_sha256: hex256(slice),
      verbatim_b64: B64.encode(slice),
      schema: Some(Schema { fields: None, variants: None, params: None, returns: Some(ret) }),
    });
  }
  // Fallback: signature-driven finder
  let is_pub = matches!(f.vis, syn::Visibility::Public(_));
  fn_anchor_sig(src, &f.sig, is_pub)
}

/// Build an anchor from a function/method signature (works for free fns and impl methods).
fn fn_anchor_sig(src: &str, sig: &syn::Signature, is_pub: bool) -> Result<Anchor> {
  let name = sig.ident.to_string();
  // Prefer span-based start; fall back to global token search.
  let start_hint = span_start_offset(src, sig.fn_token.span);
  let (start, end) = match start_hint {
    Some(off) => {
      match find_body_bounds_from(src, off) {
        Ok(se) => se,
        Err(e) => {
          if env::var("INDEXER_DEBUG").is_ok() {
            eprintln!("[v3] span-based search failed for fn {} at off {}: {}", name, off, e);
          }
          find_balanced_block(src, "fn", &name)?
        }
      }
    }
    None => find_balanced_block(src, "fn", &name)?,
  };
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

/// Convert a Span start (line/col) into a byte offset in `src`.
fn span_start_offset(src: &str, sp: Span) -> Option<usize> {
  let loc = sp.start();
  Some(offset_from_line_col(src, loc.line, loc.column))
}

fn offset_from_line_col(src: &str, line_1based: usize, col_0based: usize) -> usize {
  let mut off = 0usize;
  let mut line = 1usize;
  for l in src.split_inclusive('\n') {
    if line == line_1based {
      return off + col_0based.min(l.len());
    }
    off += l.len();
    line += 1;
  }
  // If file has no trailing newline and line points past EOF, clamp to end.
  off
}

/// Given a starting offset (ideally at the `fn` token), find `{...}` body bounds.
/// Returns (start_of_fn_token, end_after_closing_brace).
fn find_body_bounds_from(src: &str, start_from: usize) -> Result<(usize, usize)> {
  let bytes = src.as_bytes();
  let len = bytes.len();
  let mut i = start_from;
  // Walk to first '{' or ';' in code context, skipping trivia.
  loop {
    i = skip_ws_and_comments(bytes, i);
    if i >= len { break; }
    let c = bytes[i];
    if c == b'{' {
      let (end_brace, _) = consume_balanced_block(bytes, i)?;
      return Ok((start_from, end_brace));
    }
    if c == b';' {
      // trait/extern signature-only
      anyhow::bail!("signature-only (no body) after span");
    }
    // Skip strings/chars that can appear in signature tails (`where` with doc attrs nearby).
    if c == b'"' {
      // normal string
      i += 1;
      while i < len {
        if bytes[i] == b'\\' { i += 2; continue; }
        if bytes[i] == b'"' { i += 1; break; }
        i += 1;
      }
      continue;
    }
    if c == b'r' {
      // raw string r###"
      let mut t = i + 1; let mut h = 0usize;
      while t < len && bytes[t] == b'#' { h += 1; t += 1; }
      if t < len && bytes[t] == b'"' {
        t += 1;
        loop {
          if t >= len { break; }
          if bytes[t] == b'"' {
            let mut m = 0usize; while t + 1 + m < len && bytes[t + 1 + m] == b'#' { m += 1; }
            if m == h { i = t + 1 + m; break; }
          }
          t += 1;
        }
        continue;
      }
    }
    if c == b'\'' {
      // char
      i += 1;
      if i < len && bytes[i] == b'\\' { i += 2; }
      if i < len { i += 1; }
      continue;
    }
    // Generic advance through signature tokens.
    i += 1;
  }
  anyhow::bail!("no body '{{' after span")
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

fn find_balanced_block(src: &str, kw: &str, ident: &str) -> Result<(usize, usize)> {
  let bytes = src.as_bytes();
  let len = bytes.len();
  let mut i = 0usize;

  // scanning state
  let mut lc = false;        // line comment
  let mut bc: i32 = 0;       // block comment depth
  let mut s = false;         // in string
  let mut ch = false;        // in char
  let mut rs: Option<usize> = None; // raw string hashes

  while i < len {
    let c = bytes[i];
    let next = |j: usize| if j + 1 < len { bytes[j + 1] } else { 0 };

    // skip comments/strings/chars
    if lc { if c == b'\n' { lc = false; } i += 1; continue; }
    if bc > 0 {
      if c == b'/' && next(i) == b'*' { bc += 1; i += 2; continue; }
      if c == b'*' && next(i) == b'/' { bc -= 1; i += 2; continue; }
      i += 1; continue;
    }
    if s {
      if let Some(n) = rs {
        if c == b'"' {
          let mut k = 0usize; while i + 1 + k < len && bytes[i + 1 + k] == b'#' { k += 1; }
          if k == n { s = false; rs = None; i += 1 + k; continue; }
        }
        i += 1; continue;
      } else {
        if c == b'\\' { i += 2; continue; }
        if c == b'"' { s = false; i += 1; continue; }
        i += 1; continue;
      }
    }
    if ch {
      if c == b'\\' { i += 2; continue; }
      if c == b'\'' { ch = false; i += 1; continue; }
      i += 1; continue;
    }

    // detect trivia & raw string start
    if c == b'/' && next(i) == b'/' { lc = true; i += 2; continue; }
    if c == b'/' && next(i) == b'*' { bc += 1; i += 2; continue; }
    if c == b'r' {
      // raw string r###"
      let mut j = i + 1; let mut h = 0usize;
      while j < len && bytes[j] == b'#' { h += 1; j += 1; }
      if j < len && bytes[j] == b'"' { s = true; rs = Some(h); i = j + 1; continue; }
    }
    if c == b'"' { s = true; rs = None; i += 1; continue; }
    if c == b'\'' { ch = true; i += 1; continue; }

    // check for the keyword token at i (code context only)
    if is_token_at(bytes, i, kw) {
      // position after keyword
      let mut j = i + kw.len();
      j = skip_ws_and_comments(bytes, j);
      // parse identifier
      let (name, name_end) = parse_ident(bytes, j);
      if name.as_deref() == Some(ident) {
        // From here, scan to first '{' or ';' in code context.
        let mut k = name_end;
        // allow generics / where / args etc., skipping trivia
        loop {
          k = skip_ws_and_comments(bytes, k);
          if k >= len { break; }
          let d = bytes[k];
          if d == b'{' {
            // consume balanced body starting at k
            let (end_brace, _) = consume_balanced_block(bytes, k)?;
            return Ok((i, end_brace));
          }
          if d == b';' {
            // signature-only (trait/extern). No body.
            anyhow::bail!("signature-only (no body) for {kw} {ident}");
          }
          // step through tokens safely: handle strings/chars/comments that can appear in sig (rare but safe)
          if d == b'/' && k + 1 < len && bytes[k + 1] == b'/' {
            while k < len && bytes[k] != b'\n' { k += 1; }
            continue;
          }
          if d == b'/' && k + 1 < len && bytes[k + 1] == b'*' {
            // skip block comment
            k += 2; let mut depth = 1i32;
            while k < len && depth > 0 {
              if k + 1 < len && bytes[k] == b'/' && bytes[k + 1] == b'*' { depth += 1; k += 2; continue; }
              if k + 1 < len && bytes[k] == b'*' && bytes[k + 1] == b'/' { depth -= 1; k += 2; continue; }
              k += 1;
            }
            continue;
          }
          if d == b'"' {
            // skip normal string
            k += 1;
            while k < len {
              if bytes[k] == b'\\' { k += 2; continue; }
              if bytes[k] == b'"' { k += 1; break; }
              k += 1;
            }
            continue;
          }
          if d == b'r' {
            // skip raw string
            let mut t = k + 1; let mut h = 0usize;
            while t < len && bytes[t] == b'#' { h += 1; t += 1; }
            if t < len && bytes[t] == b'"' {
              t += 1; // after opening quote
              loop {
                if t >= len { break; }
                if bytes[t] == b'"' {
                  let mut m = 0usize; while t + 1 + m < len && bytes[t + 1 + m] == b'#' { m += 1; }
                  if m == h { k = t + 1 + m; break; }
                }
                t += 1;
              }
              continue;
            }
          }
          if d == b'\'' {
            // skip char
            k += 1;
            if k < len && bytes[k] == b'\\' { k += 2; }
            if k < len { k += 1; }
            continue;
          }
          // generic advance
          k += 1;
        }
        anyhow::bail!("no body '{{' found for {kw} {ident}");
      }
      // not our ident → continue scanning after this keyword
    }
    i += 1;
  }
  anyhow::bail!("cannot find token '{kw} {ident}'")
}

#[inline]
fn is_ident_start(b: u8) -> bool {
  (b'A'..=b'Z').contains(&b) || (b'a'..=b'z').contains(&b) || b == b'_'
}
#[inline]
fn is_ident_continue(b: u8) -> bool {
  is_ident_start(b) || (b'0'..=b'9').contains(&b)
}
#[inline]
fn is_token_at(bytes: &[u8], i: usize, kw: &str) -> bool {
  let k = kw.as_bytes();
  if i + k.len() > bytes.len() { return false; }
  if &bytes[i..i + k.len()] != k { return false; }
  // boundary checks (prev not ident, next not ident)
  let prev_ok = i == 0 || !is_ident_continue(bytes[i - 1]);
  let next_ok = i + k.len() == bytes.len() || !is_ident_continue(bytes[i + k.len()]);
  prev_ok && next_ok
}
#[inline]
fn parse_ident(bytes: &[u8], mut j: usize) -> (Option<String>, usize) {
  let len = bytes.len();
  if j >= len || !is_ident_start(bytes[j]) { return (None, j); }
  let start = j; j += 1;
  while j < len && is_ident_continue(bytes[j]) { j += 1; }
  (Some(String::from_utf8_lossy(&bytes[start..j]).into_owned()), j)
}
#[inline]
fn skip_ws_and_comments(bytes: &[u8], mut j: usize) -> usize {
  let len = bytes.len();
  loop {
    while j < len && matches!(bytes[j], b' ' | b'\t' | b'\n' | b'\r') { j += 1; }
    if j + 1 < len && bytes[j] == b'/' && bytes[j + 1] == b'/' {
      while j < len && bytes[j] != b'\n' { j += 1; }
      continue;
    }
    if j + 1 < len && bytes[j] == b'/' && bytes[j + 1] == b'*' {
      j += 2; let mut depth = 1i32;
      while j < len && depth > 0 {
        if j + 1 < len && bytes[j] == b'/' && bytes[j + 1] == b'*' { depth += 1; j += 2; continue; }
        if j + 1 < len && bytes[j] == b'*' && bytes[j + 1] == b'/' { depth -= 1; j += 2; continue; }
        j += 1;
      }
      continue;
    }
    break;
  }
  j
}
#[inline]
fn consume_balanced_block(bytes: &[u8], start_brace: usize) -> Result<(usize, usize)> {
  let len = bytes.len();
  let mut j = start_brace;
  let mut depth: i32 = 0;
  // local scanners
  let mut lc = false; let mut bc: i32 = 0; let mut s = false; let mut ch = false; let mut rs: Option<usize> = None;
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
    if ch { if d == b'\\' { j += 2; continue; } if d == b'\'' { ch = false; j += 1; continue; } j += 1; continue; }
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
      if depth == 0 { return Ok((j, depth as usize)); }
      continue;
    }
    j += 1;
  }
  anyhow::bail!("unbalanced braces while consuming block")
}
