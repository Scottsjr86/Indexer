This is what the types_view produced

# Project Types

_Public structs/enums by module. Field and variant names shown verbatim._

> Modules: 2  •  Decls: 3

## module: file_intent_entry

pub enum Role {
  Bin, Lib, Test, Doc, Config, Script, Ui, Core, Other,
}

pub struct FileIntentEntry {
  pub lines_nonblank: usize
  pub lines_total: usize
  pub noise: bool
  pub size: usize
  pub token_estimate: usize
}


## module: scan

pub struct ScanOptions {
  pub follow_symlinks: bool
  pub include_docs_and_configs: bool
  pub max_file_bytes: u64
  pub sniff_bytes: usize
  pub snippet_bytes: usize
  pub split_html_embeds: bool
}


This is what i want it to produce


# Project Types

_Project structs/enums by module. Field and variant names shown verbatim._

# indexer/src/chunker.rs

struct FileIntentEntry {
    pub path: String,
    pub lang: String,
    pub sha1: String,
    pub size: usize,
    pub last_modified: String,
    pub summary: Option<String>,
    pub snippet: String,
    #[serde(default)]
    pub token_estimate: usize,
}

# indexer/src/commands.rs

struct ResolvedPaths {
    cwd: PathBuf,
    dir_name: String,
    #[allow(dead_code)]
    index_dir: PathBuf,
    maps_dir: PathBuf,
    types_dir: PathBuf,
    functions_dir: PathBuf,
    chunks_dir: PathBuf,
    #[allow(dead_code)]
    indexes_dir: PathBuf,
    history_full: PathBuf,
    history_diff: PathBuf,
    index_file: PathBuf,
}

# indexer/src/custom_view.rs

struct Section {
    category: String,
    verbatim: bool,
    lang: String,
    render: String, 
}

# indexer/src/file_intent_entry.rs

pub enum Role {
    Bin, Lib, Test, Doc, Config, Script, Ui, Core, Other,
}

pub struct FileIntentEntry {
    #[serde(deserialize_with = "de_string_from_any")] pub path: String,
    #[serde(deserialize_with = "de_string_from_any")] pub lang: String,
    #[serde(deserialize_with = "de_string_from_any")] pub sha1: String,
    pub size: usize,
    #[serde(deserialize_with = "de_string_from_any")] pub last_modified: String,
    #[serde(deserialize_with = "de_string_from_any")] pub snippet: String,
    #[serde(deserialize_with = "de_vec_string_from_any")] pub tags: Vec<String>,
    #[serde(default, deserialize_with = "de_opt_string_from_any")] pub summary: Option<String>,
    pub token_estimate: usize,

    
    #[serde(deserialize_with = "de_string_from_any")] pub role: String,
    #[serde(deserialize_with = "de_string_from_any")] pub module: String,
    #[serde(deserialize_with = "de_vec_string_from_any")] pub imports: Vec<String>,
    #[serde(deserialize_with = "de_vec_string_from_any")] pub exports: Vec<String>,
    pub lines_total: usize,
    pub lines_nonblank: usize,
    #[serde(deserialize_with = "de_string_from_any")] pub rel_dir: String,
    pub noise: bool,
}

# indexer/src/functions_view.rs

struct FnDecl {    
    signature: String,
}

# indexer/src/scan.rs

pub struct ScanOptions {    
    pub max_file_bytes: u64,
    pub sniff_bytes: usize,
    pub snippet_bytes: usize,
    pub follow_symlinks: bool,
    pub include_docs_and_configs: bool,
    pub split_html_embeds: bool,
}

struct HtmlBlock<'a> {
    lang: &'static str,
    body: &'a str,
    kind: BlockKind,
}

enum BlockKind { Script, Style }

# indexer/src/snippet.rs

enum TypeKind { Struct, Enum }

struct TypeDecl {
    kind: TypeKind,
    vis: String,    
    name: String,
    body_lines: Vec<String>,
}

