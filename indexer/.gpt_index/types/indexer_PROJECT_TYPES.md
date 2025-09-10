# Project Types

*Project structs/enums by module. Field and variant names shown verbatim.*

# src/chunker.rs

struct FileIntentEntry {
  pub path: String,
  pub lang: String,
  pub sha1: String,
  pub size: usize,
  pub last_modified: String,
  pub summary: Option<String >,
  pub snippet: String,
#[serde (default)]
  pub token_estimate: usize,
}

struct Part {
  path: String,
  lang: String,
  sha1: String,
  size: String,
  last_modified: String,
  summary: Option<String >,
  part_idx: usize,
  part_total: usize,
  body: String,
  token_estimate: usize,
}

# src/custom_view.rs

struct FileIntentEntryMini {
  path: String,
#[allow (dead_code)]
  lang: Option<String >,
}

struct Section {
  category: String,
  verbatim: bool,
  _lang: String,
  render: String,
}

pub struct TypeCollector {
  pub out: Vec<Decl >,
}

pub enum Decl { Struct, Enum, }

pub struct StructDecl {
  pub name: String,
  pub public: bool,
  pub fields: Vec<FieldDecl >,
}

pub struct FieldDecl {
  pub attrs: Vec<String >,
  pub public: bool,
  pub name: Option<String >,
  pub ty: String,
}

pub struct EnumDecl {
  pub name: String,
  pub public: bool,
  pub variants: Vec<String >,
}

pub struct FnCollector {
  pub out: Vec<(Kind, String) >,
  pub in_test_mod: bool,
}

pub enum Kind { Public, Internal, Test, }

pub struct Groups {
  pub public: Vec<String >,
  pub internal: Vec<String >,
  pub tests: Vec<String >,
}

# src/file_intent_entry.rs

pub enum Role { Bin, Lib, Test, Doc, Config, Script, Ui, Core, Other, }

pub struct FileIntentEntry {
#[serde (deserialize_with = "de_string_from_any")]
  pub path: String,
#[serde (deserialize_with = "de_string_from_any")]
  pub lang: String,
#[serde (deserialize_with = "de_string_from_any")]
  pub sha1: String,
  pub size: usize,
#[serde (deserialize_with = "de_string_from_any")]
  pub last_modified: String,
#[serde (deserialize_with = "de_string_from_any")]
  pub snippet: String,
#[serde (deserialize_with = "de_vec_string_from_any")]
  pub tags: Vec<String >,
#[serde (default, deserialize_with = "de_opt_string_from_any")]
  pub summary: Option<String >,
  pub token_estimate: usize,
#[serde (deserialize_with = "de_string_from_any")]
  pub role: String,
#[serde (deserialize_with = "de_string_from_any")]
  pub module: String,
#[serde (deserialize_with = "de_vec_string_from_any")]
  pub imports: Vec<String >,
#[serde (deserialize_with = "de_vec_string_from_any")]
  pub exports: Vec<String >,
  pub lines_total: usize,
  pub lines_nonblank: usize,
#[serde (deserialize_with = "de_string_from_any")]
  pub rel_dir: String,
  pub noise: bool,
}

# src/functions_view.rs

struct FileIntentEntryMini {
  path: String,
#[allow (dead_code)]
  lang: Option<String >,
}

pub struct FnCollector {
  pub out: Vec<(Kind, String) >,
  pub in_test_mod: bool,
}

pub enum Kind { Public, Internal, Test, }

pub struct Groups {
  pub public: Vec<String >,
  pub internal: Vec<String >,
  pub tests: Vec<String >,
}

# src/index_v3.rs

pub struct IndexPack {
  format: & 'static str,
  version: & 'static str,
  hash_algo: & 'static str,
  pack_id: String,
  created_utc: String,
  lang: LangMeta,
  rules: Rules,
  files: Vec<FileEntry >,
}

struct LangMeta {
  primary: & 'static str,
  dialect: & 'static str,
}

struct Rules {
  mode: & 'static str,
  patch_contract: PatchContract,
}

struct PatchContract {
  diff_format: & 'static str,
  limit_scope_to_verified_anchors: bool,
}

struct FileEntry {
  path: String,
  language: String,
  size_bytes: usize,
  line_count: usize,
  encoding: & 'static str,
  eol: & 'static str,
  file_sha256: String,
  chunks: ChunkSet,
  anchors: Vec<Anchor >,
}

struct ChunkSet {
  chunk_size_bytes: usize,
  merkle_root: String,
  list: Vec<Chunk >,
}

struct Chunk {
  index: usize,
  offset: usize,
  length: usize,
  sha256: String,
}

struct Anchor {
  kind: & 'static str,
  name: String,
  visibility: String,
  signature: Option<String >,
  range: Range,
  slice_sha256: String,
  verbatim_b64: String,
  schema: Option<Schema >,
}

struct Range {
  start_line: usize,
  end_line: usize,
}

struct Schema {
  fields: Option<Vec<Field>>,
  variants: Option<Vec<String>>,
  params: Option<Vec<(String, String)>>,
  returns: Option<String >,
}

struct Field {
  name: String,
  ty: String,
  public: bool,
}

# src/map_view.rs

struct EntryLite {
  path: String,
  lang: String,
  summary: String,
  tags: Vec<String >,
}

struct DirNode {
  subdirs: BTreeMap<String, DirNode >,
  files: Vec<TreeFile >,
}

struct TreeFile {
  name: String,
  lang: String,
  size: usize,
  summary: String,
}

# src/scan.rs

pub struct ScanOptions {
  pub max_file_bytes: u64,
  pub sniff_bytes: usize,
  pub snippet_bytes: usize,
  pub follow_symlinks: bool,
  pub include_docs_and_configs: bool,
  pub split_html_embeds: bool,
}

struct HtmlBlock {
  lang: & 'static str,
  body: & 'a str,
  kind: BlockKind,
}

enum BlockKind { Script, Style, }

# src/types_view.rs

struct FileIntentEntryMini {
  path: String,
#[allow (dead_code)]
  lang: Option<String >,
}

pub struct TypeCollector {
  pub out: Vec<Decl >,
}

pub enum Decl { Struct, Enum, }

pub struct StructDecl {
  pub name: String,
  pub public: bool,
  pub fields: Vec<FieldDecl >,
}

pub struct FieldDecl {
  pub attrs: Vec<String >,
  pub public: bool,
  pub name: Option<String >,
  pub ty: String,
}

pub struct EnumDecl {
  pub name: String,
  pub public: bool,
  pub variants: Vec<String >,
}

