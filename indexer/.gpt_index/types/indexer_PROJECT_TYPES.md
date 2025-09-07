# Project Types

*Project structs/enums by module. Field and variant names shown verbatim.*

# src/chunker.rs

struct FileIntentEntry {
    pub : String,
    pub : String,
    pub : String,
    pub : usize,
    pub : String,
    pub : Option<String >,
    pub : String,
#[serde (default)]
    pub : usize,
}

struct Part {
    : String,
    : String,
    : String,
    : String,
    : String,
    : Option<String >,
    : usize,
    : usize,
    : String,
    : usize,
}

# src/commands.rs

struct ResolvedPaths {
    : PathBuf,
    : String,
#[allow (dead_code)]
    : PathBuf,
    : PathBuf,
    : PathBuf,
    : PathBuf,
    : PathBuf,
#[allow (dead_code)]
    : PathBuf,
    : PathBuf,
    : PathBuf,
    : PathBuf,
}

# src/custom_view.rs

struct FileIntentEntryMini {
    : String,
#[allow (dead_code)]
    : Option<String >,
}

struct Section {
    : String,
    : bool,
    : String,
    : String,
}

pub struct TypeCollector {
    pub : Vec<Decl >,
}

pub enum Decl { Struct, Enum, }

pub struct StructDecl {
    pub : String,
    pub : bool,
    pub : Vec<FieldDecl >,
}

pub struct FieldDecl {
    pub : Vec<String >,
    pub : bool,
    pub : String,
}

pub struct EnumDecl {
    pub : String,
    pub : bool,
    pub : Vec<String >,
}

pub struct FnCollector {
    pub : Vec<(Kind, String) >,
    pub : bool,
}

pub enum Kind { Public, Internal, Test, }

pub struct Groups {
    pub : Vec<String >,
    pub : Vec<String >,
    pub : Vec<String >,
}

# src/file_intent_entry.rs

pub enum Role { Bin, Lib, Test, Doc, Config, Script, Ui, Core, Other, }

pub struct FileIntentEntry {
#[serde (deserialize_with = "de_string_from_any")]
    pub : String,
#[serde (deserialize_with = "de_string_from_any")]
    pub : String,
#[serde (deserialize_with = "de_string_from_any")]
    pub : String,
    pub : usize,
#[serde (deserialize_with = "de_string_from_any")]
    pub : String,
#[serde (deserialize_with = "de_string_from_any")]
    pub : String,
#[serde (deserialize_with = "de_vec_string_from_any")]
    pub : Vec<String >,
#[serde (default, deserialize_with = "de_opt_string_from_any")]
    pub : Option<String >,
    pub : usize,
#[serde (deserialize_with = "de_string_from_any")]
    pub : String,
#[serde (deserialize_with = "de_string_from_any")]
    pub : String,
#[serde (deserialize_with = "de_vec_string_from_any")]
    pub : Vec<String >,
#[serde (deserialize_with = "de_vec_string_from_any")]
    pub : Vec<String >,
    pub : usize,
    pub : usize,
#[serde (deserialize_with = "de_string_from_any")]
    pub : String,
    pub : bool,
}

# src/functions_view.rs

struct FileIntentEntryMini {
    : String,
#[allow (dead_code)]
    : Option<String >,
}

pub struct FnCollector {
    pub : Vec<(Kind, String) >,
    pub : bool,
}

pub enum Kind { Public, Internal, Test, }

pub struct Groups {
    pub : Vec<String >,
    pub : Vec<String >,
    pub : Vec<String >,
}

# src/map_view.rs

struct EntryLite {
    : String,
    : String,
    : String,
    : Vec<String >,
}

struct DirNode {
    : BTreeMap<String, DirNode >,
    : Vec<TreeFile >,
}

struct TreeFile {
    : String,
    : String,
    : usize,
    : String,
}

# src/scan.rs

pub struct ScanOptions {
    pub : u64,
    pub : usize,
    pub : usize,
    pub : bool,
    pub : bool,
    pub : bool,
}

struct HtmlBlock {
    : & 'static str,
    : & 'a str,
    : BlockKind,
}

enum BlockKind { Script, Style, }

# src/types_view.rs

struct FileIntentEntryMini {
    : String,
#[allow (dead_code)]
    : Option<String >,
}

pub struct TypeCollector {
    pub : Vec<Decl >,
}

pub enum Decl { Struct, Enum, }

pub struct StructDecl {
    pub : String,
    pub : bool,
    pub : Vec<FieldDecl >,
}

pub struct FieldDecl {
    pub : Vec<String >,
    pub : bool,
    pub : String,
}

pub struct EnumDecl {
    pub : String,
    pub : bool,
    pub : Vec<String >,
}

