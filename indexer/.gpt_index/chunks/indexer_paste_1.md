# GPT Paste Chunk 1

> generated: 2025-09-07T04:15:00.668309060+00:00
> files: 16  •  parts: 16  •  ~tokens: 7273

## `Cargo.toml` [toml]

- sha1: `94d3cf096ec6da466e69690f1b39e989c3c91a70` • size: 360 • mtime: 1757214441
**Summary:** Cargo manifest / workspace configuration.
```toml
[package]
name = "indexer"
version = "0.2.1"
edition = "2021"
resolver = "1"
[dependencies]
anyhow = "1.0.98"
chrono = { version = "0.4.41", features = ["serde"] }
hex = "0.4.3"
ignore = "0.4.23"
memchr = "2.7.5"
quote = "1.0.40"
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.142"
sha1 = "0.10.6"
syn = "2.0.106"
walkdir = "2.5.0"
```

## `src/chunker.rs` [rust]

- sha1: `85a081ed6c4e8b24efaeebb10eeb90c18ba7fac4` • size: 12604 • mtime: 1757173442
**Summary:** Splits indexed files into GPT-ready paste chunks.
```rust
Chunk builder: converts a JSONL index (FileIntentEntry per line) into
GPT-ready paste chunks, enforcing token caps and splitting large files.
//! Chunk builder: converts a JSONL index (FileIntentEntry per line) into
//! GPT-ready paste chunks, enforcing token caps and splitting large files.
use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;
use std::{
    cmp,
#[derive(Debug, Deserialize, Clone)]
struct FileIntentEntry {
    pub path: String,
    #[serde(default)]
    pub token_estimate: usize,
/// Build markdown "paste chunks" for LLMs from a JSONL index.
/// - `index_path`: path to JSONL with one FileIntentEntry per line
/// - `out_prefix`: prefix for output files, e.g. ".gpt/chunks/paste_"
/// - `token_cap`: desired approximate token cap per chunk (min 256)
pub fn chunk_index_for_gpt(index_path: &Path, out_prefix: &str, token_cap: usize) -> Result<()> {
    let token_cap = token_cap.max(256);
fn load_entries(index_path: &Path) -> Result<Vec<FileIntentEntry>> {
    let file = File::open(index_path)?;
#[derive(Debug, Clone)]
struct Part {
    path: String,
fn split_entry_into_parts(
    e: &FileIntentEntry,
fn write_chunk(out_prefix: &str, idx: usize, parts: &[Part]) -> Result<()> {
    let path = format!("{}{}.md", out_prefix, idx);
fn render_file_section(out: &mut File, parts: &[Part]) -> Result<()> {
    if parts.is_empty() {
fn estimate_tokens_fallback(s: &str) -> usize {
    let chars = s.len();
fn fence_lang<'a>(lang: &'a str) -> &'a str {
    let l = lang.trim();
fn count_unique_files(parts: &[Part]) -> usize {
    let mut n = 0usize;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_estimator_floor() {
        assert!(estimate_tokens_fallback("") >= 12);
    #[test]
    fn split_small_is_single_part() {
        let e = FileIntentEntry {
    #[test]
    fn split_large_makes_multiple_parts() {
        let mut body = String::new();
```

## `src/commands.rs` [rust]

- sha1: `b7e2df39b6a12f8a5aa8e544a4cf468b5c5d9763` • size: 15285 • mtime: 1757218023
**Summary:** CLI subcommands wiring and user-facing flows.
```rust
Resolve all standard output paths under .gpt_index for the current working dir.
use anyhow::{anyhow, Context, Result};
use std::{
    env,
use crate::{
    chunker,
pub fn run_cli() -> Result<()> {
    let args: Vec<String> = env::args().collect();
fn is_help_flag(s: &str) -> bool {
    matches!(s, "--help" | "-h" | "help")
fn print_version() {
    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
/// Resolve all standard output paths under .gpt_index for the current working dir.
fn resolve_paths() -> Result<ResolvedPaths> {
    let cwd = std::env::current_dir().context("failed to get current_dir")?;
struct ResolvedPaths {
    cwd: PathBuf,
    #[allow(dead_code)]
    index_dir: PathBuf,
    #[allow(dead_code)]
    indexes_dir: PathBuf,
fn index_root(is_reindex: bool) -> Result<()> {
    let p = resolve_paths()?;
fn index_subdir() -> Result<()> {
    let cwd = std::env::current_dir().context("get current_dir")?;
fn generate_map() -> Result<()> {
    let p = resolve_paths()?;
fn generate_types() -> Result<()> {
    let p = resolve_paths()?;
fn generate_functions() -> Result<()> {
    let p = resolve_paths()?;
#[allow(dead_code)]
fn generate_custom() -> Result<()> {
    let paths = resolve_paths()?;
/// Support: `indexer chunk` or `indexer chunk --cap=12000`
/// Also supports: `indexer help chunk` | `indexer chunk --help`
fn chunk_index(arg: Option<&str>) -> Result<()> {
    // Accept `--help` passed as the sole arg: `indexer chunk --help`
fn parse_cap(arg: Option<&str>) -> Option<usize> {
    let a = arg?;
fn ensure_index_exists(p: &Path) -> Result<()> {
    if p.exists() {
fn print_help_dispatch(sub: Option<&str>) -> Result<()> {
    match sub {
fn print_help_main() {
    println!(
fn print_help_init() {
    println!(
fn print_help_reindex() {
    println!(
fn print_help_sub() {
    println!(
fn print_help_map() {
    println!(
fn print_help_types() {
    println!(
```

## `src/custom_view.rs` [rust]

- sha1: `eed2ca05f0f7c03dd5e8b4a42ded7cfd48c660ff` • size: 14944 • mtime: 1757217535
**Summary:** Filesystem / IO utilities.
```rust
custom_view.rs — "custom index blocks" extracted from your source files.
Block grammar:
//--functions public
$ # Project Functions
$ *Functions and methods by module. Signatures are shown verbatim (one line).*
//--end
Categories: types|functions (aliases: type, structs, enums, fn, fns).
`$` lines are emitted verbatim. Generated content for the file follows.
Add to Cargo.toml deps used by this module:
anyhow, serde, serde_json, syn, quote
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
#[derive(Debug, Clone)]
struct Section {
    category: String,
pub fn build_custom_from_index(index_path: &Path, output_path: &Path) -> io::Result<()> {
    let text = fs::read_to_string(index_path)?;
fn scan_custom_regions(text: &str, lang: &str) -> Vec<Section> {
    let mut out = Vec::new();
fn normalize_category(c: &str) -> String {
    match c.to_ascii_lowercase().as_str() {
fn category_heading(c: &str) -> String {
    match c {
fn types_for_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let src = fs::read_to_string(path)?;
fn functions_for_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let src = fs::read_to_string(path)?;
fn resolve_path(root: &Path, p: &str) -> PathBuf { let pb = PathBuf::from(p); if pb.is_absolute() { pb } else { root.join(pb) } }
fn to_rel(root: &Path, p: &Path) -> PathBuf { pathdiff::diff_paths(p, root).unwrap_or_else(|| p.to_path_buf()) }
mod pathdiff {
    use std::path::{Component, Path, PathBuf};
    pub fn diff_paths(path: &Path, base: &Path) -> Option<PathBuf> {
```

## `src/diff.rs` [rust]

- sha1: `3485f6de849368d852d5d512f3d6915156c3759e` • size: 8311 • mtime: 1757173096
**Summary:** Compute a structured diff between two index snapshots.
```rust
Compute a structured diff between two index snapshots.
- Adds / Removes / Modifies (by sha1 change or signal deltas)
- Renames (one-to-one sha1 match where old path disappeared and new path appeared)
- Stable, path-sorted output for deterministic diffs
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, BTreeSet};
use crate::file_intent_entry::{FileIntentEntry};
/// Compute a structured diff between two index snapshots.
/// - Adds / Removes / Modifies (by sha1 change or signal deltas)
/// - Renames (one-to-one sha1 match where old path disappeared and new path appeared)
/// - Stable, path-sorted output for deterministic diffs
pub fn diff_indexes(old: &[FileIntentEntry], new: &[FileIntentEntry]) -> Value {
    // Index by path
/// Minimal JSON for a file to keep diff payloads lean.
fn json_min(e: &FileIntentEntry) -> Value {
    json!({
/// Focused delta payload for a modified file (or signal-only change).
fn json_delta(path: &str, before: &FileIntentEntry, after: &FileIntentEntry) -> Value {
    json!({
/// Return true if non-content “signals” changed (lang/role/module/lines/tags).
fn signals_changed(a: &FileIntentEntry, b: &FileIntentEntry) -> bool {
    a.lang != b.lang
fn tags_added(before: &[String], after: &[String]) -> Value {
    let b: BTreeSet<&str> = before.iter().map(|s| s.as_str()).collect();
fn tags_removed(before: &[String], after: &[String]) -> Value {
    let b: BTreeSet<&str> = before.iter().map(|s| s.as_str()).collect();
```

## `src/file_intent_entry.rs` [rust]

- sha1: `e80bcbd9063064861a146765bac1309363887b28` • size: 13845 • mtime: 1757205826
**Summary:** File-level intent record: what is this file, what does it export, and how should GPT treat it?
```rust
File-level intent record: what is this file, what does it export, and how should GPT treat it?
Backward-compat:
- `#[serde(default)]` keeps old JSONL readable (missing new fields).
- `role` accepts legacy string values (case-insensitive); unknown -> Role::Other.
Zero extra deps beyond `serde`.
//! File-level intent record: what is this file, what does it export, and how should GPT treat it?
//!
//! Backward-compat:
//! - `#[serde(default)]` keeps old JSONL readable (missing new fields).
//! - `role` accepts legacy string values (case-insensitive); unknown -> Role::Other.
//!
//! Zero extra deps beyond `serde`.
use serde::{
    de::{
use std::fmt;
/// Coarse role for retrieval/ranking. Keep small & stable.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Bin, Lib, Test, Doc, Config, Script, Ui, Core, Other,
impl Role {
    pub fn from_str_ic<S: AsRef<str>>(s: S) -> Self {
        match s.as_ref().to_ascii_lowercase().as_str() {
    pub fn as_str(self) -> &'static str {
        match self {
impl From<String> for Role {
    fn from(s: String) -> Self { Role::from_str_ic(&s) }
}
impl From<&str> for Role {
    fn from(s: &str) -> Self { Role::from_str_ic(s) }
}
impl Default for Role {
    fn default() -> Self { Role::Other }
}
impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Role::*;
        let s = match self {
/// Primary record emitted per file. This is your JSONL unit.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(default)]
pub struct FileIntentEntry {
    #[serde(deserialize_with = "de_string_from_any")] pub path: String,
    #[serde(deserialize_with = "de_string_from_any")] pub lang: String,
    #[serde(deserialize_with = "de_string_from_any")] pub sha1: String,
    pub size: usize,
```

## `src/functions_view.rs` [rust]

- sha1: `f79ce89c1fc0c2082785ce38cd698934c475ea59` • size: 9900 • mtime: 1757218368
**Summary:** Filesystem / IO utilities.
```rust
functions_view.rs — renders "Project Functions" grouped by file and
split into Public / Internal / Tests sections. Method names are prefixed
with `Type::` when inside impl blocks. Signatures are one-line, verbatim.
Add to Cargo.toml:
```toml
[dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
walkdir = "2"
syn = { version = "2", features = ["full", "extra-traits", "printing"] }
quote = "1"
```
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
pub fn build_functions_from_index(index_path: &Path, output_path: &Path) -> io::Result<()> {
    // Load entries from JSONL (one object per line), but also accept a JSON array fallback.
#[derive(Default)]
pub struct FnCollector {
    pub out: Vec<(Kind, String)>,
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Public,
#[derive(Default)]
pub struct Groups {
    pub public: Vec<String>,
impl Groups {
    pub fn extend(&mut self, it: impl Iterator<Item = (Kind, String)>) {
        for (k, s) in it {
```

## `src/helpers.rs` [rust]

- sha1: `16962d868fa03556c2d65811baee0dcaee8657ae` • size: 13382 • mtime: 1757173100
**Summary:** Formatting and shared helper utilities.
```rust
Heuristics and light parsers used across scan + intent.
- Role inference (typed) from path/lang/snippet
- Module ID derivation (language-aware)
- Cheap import/export skimming (no regex/AST)
- Small utilities (dedup, ident capture)
//! Heuristics and light parsers used across scan + intent.
//! - Role inference (typed) from path/lang/snippet
//! - Module ID derivation (language-aware)
//! - Cheap import/export skimming (no regex/AST)
//! - Small utilities (dedup, ident capture)
use crate::file_intent_entry::Role;
/// Infer a coarse role for the file: bin/lib/test/doc/config/script/ui/core
/// Returns a typed `Role`. Pure, allocation-free (except small to_lowercase() temps).
pub fn infer_role(path: &str, lang: &str, snippet: &str) -> Role {
    let p = path.replace('\\', "/").to_ascii_lowercase();
/// Best-effort module id (path → module), language-aware. Returns stable identifiers.
pub fn infer_module_id(path: &str, lang: &str) -> String {
    let p = path.replace('\\', "/").trim_matches('/').to_string();
/// Rust:
/// - src/lib.rs        -> crate
/// - src/main.rs       -> bin
/// - src/bin/foo.rs    -> bin::foo
/// - src/foo/bar.rs    -> foo::bar
/// - src/foo/mod.rs    -> foo
pub fn rust_module_id(p: &str) -> String {
    if p.ends_with("src/lib.rs") { return "crate".into(); }
/// Python: strip extension, convert / to .
/// tests/foo_test.py -> tests.foo_test
pub fn python_module_id(p: &str) -> String {
    let stem = p.strip_suffix(".py").unwrap_or(p);
/// Web (ts/js): strip extension; treat directories as namespaces with `::`
pub fn web_module_id(p: &str) -> String {
    let stem = p.rsplit_once('.').map(|(a, _)| a).unwrap_or(p);
/// Generic: strip extension; use `::` as separator.
pub fn generic_module_id(p: &str) -> String {
    let stem = p.rsplit_once('.').map(|(a, _)| a).unwrap_or(p);
/// Extract imports/exports cheaply from the snippet (no regex/AST).
/// Returns (imports, exports). Deduplicated, order-preserving (first occurrence).
pub fn skim_symbols(snippet: &str, lang: &str) -> (Vec<String>, Vec<String>) {
    match lang.to_ascii_lowercase().as_str() {
pub fn skim_rust(s: &str) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
pub fn skim_python(s: &str) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
pub fn skim_js_ts(s: &str) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
```

## `src/intent.rs` [rust]

- sha1: `5aa79496bd3b4478e0821d49d0e18eee5b5edd14` • size: 11014 • mtime: 1757173101
**Summary:** Intent classifier: offline file purpose inference.
```rust
Max characters of `snippet` we scan for intent signals and doc extraction.
Keep small for speed; we bias toward the top-of-file semantics.
/// Max characters of `snippet` we scan for intent signals and doc extraction.
/// Keep small for speed; we bias toward the top-of-file semantics.
const MAX_SCAN_BYTES: usize = 32 * 1024;
/// Public entrypoint: return a short, high-signal, human/GPT friendly summary
/// for a given file path, snippet, and language label.
pub fn guess_summary(path: &str, snippet: &str, lang: &str) -> String {
    // Normalize path (Windows-safe) and lowercased mirrors.
#[inline]
fn s(msg: &str) -> String { msg.to_string() }
#[inline]
fn contains(hay: &str, needle: &str) -> bool { hay.contains(needle) }
#[inline]
fn ends_with(hay: &str, suffix: &str) -> bool { hay.ends_with(suffix) }
#[inline]
fn _starts_with(hay: &str, prefix: &str) -> bool { hay.starts_with(prefix) }
#[inline]
fn ends_with_any(hay: &str, suffixes: &[&str]) -> bool {
    suffixes.iter().any(|s| hay.ends_with(s))
#[inline]
fn any_in(hay: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| hay.contains(n))
#[inline]
fn eq_ic(a: &str, b: &str) -> bool { a.eq_ignore_ascii_case(b) }
#[inline]
fn normalize_path(p: &str) -> String { p.replace('\\', "/") }
#[inline]
fn trim_window(s: &str, max: usize) -> &str {
    if s.len() > max { &s[..max] } else { s }
fn is_cargo_toml(pl: &str) -> bool {
    ends_with(pl, "cargo.toml")
fn is_docker_related(pl: &str) -> bool {
    ends_with(pl, "dockerfile") || contains(pl, "/docker/")
fn is_readme(pl: &str) -> bool {
    ends_with(pl, "readme.md") || ends_with(pl, "readme")
fn is_license(pl: &str) -> bool {
    ends_with(pl, "license") || ends_with(pl, "license.md")
fn is_ci_yaml(pl: &str) -> bool {
    contains(pl, ".github/workflows/") || contains(pl, "/.gitlab-ci") || contains(pl, "/.circleci/")
fn is_rust_bin_entry(pl: &str, sl: &str) -> bool {
    ends_with(pl, "src/main.rs") || contains(pl, "/bin/") || sl.contains("fn main(")
fn is_python_entry(lang: &str, sl: &str) -> bool {
    eq_ic(lang, "python") && sl.contains("if __name__ == '__main__'")
fn is_test_file(pl: &str, sl: &str) -> bool {
    contains(pl, "/tests") ||
fn is_httpish(sl: &str, pl: &str) -> bool {
    sl.contains("axum::") || sl.contains("actix") || sl.contains("rocket::") || sl.contains("warp::")
fn is_dblike(sl: &str, pl: &str) -> bool {
    sl.contains("sqlx::") || sl.contains("diesel::") || sl.contains("postgres")
```

## `src/lib.rs` [rust]

- sha1: `a92e08731f60e5a053125e0755808209de3c6f2c` • size: 313 • mtime: 1757114377
**Summary:** Root library file for this Rust crate.
```rust
pub mod util;
pub mod helpers;
pub mod file_intent_entry;
pub mod snippet;
pub mod intent;
pub mod scan;
pub mod chunker;
pub mod types_view;
pub mod diff;
pub mod map_view;
pub mod commands;
pub mod functions_view;
pub mod custom_view;
```

## `src/main.rs` [rust]

- sha1: `770ea35b67ba7b8b50e8e65df557a93503aa9fbe` • size: 97 • mtime: 1757173103
**Summary:** Entrypoint for this Rust binary.
```rust
use anyhow::Result;
fn main() -> Result<()> {
    indexer::commands::run_cli()
```

## `src/map_view.rs` [rust]

- sha1: `90e3c2f12c33af8118aaeae79d7e40fc91602420` • size: 9160 • mtime: 1757173104
**Summary:** Builds semantic project map (markdown).
```rust
Combined Project Map (with tree-lite appendix)
- Top section: tag-rich grouped catalog by top-level dir (old MAP).
- Appendix: compact hierarchical tree (old TREE), same output file.
Output path example: `.gpt_index/maps/<slug>_PROJECT_MAP.md`
//! Combined Project Map (with tree-lite appendix)
//!
//! - Top section: tag-rich grouped catalog by top-level dir (old MAP).
//! - Appendix: compact hierarchical tree (old TREE), same output file.
//!
//! Output path example: `.gpt_index/maps/<slug>_PROJECT_MAP.md`
use std::{
    collections::{BTreeMap, BTreeSet},
use crate::file_intent_entry::FileIntentEntry;
use crate::util;
/// Public entrypoint: build the combined MAP (+ tree-lite) into `output_path`.
pub fn build_map_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
    // Ensure parent dir exists
#[derive(Clone)]
struct EntryLite {
    path: String,   // relative to top-level group
fn load_entries(index_path: &Path) -> std::io::Result<Vec<FileIntentEntry>> {
    let f = File::open(index_path)?;
/// Return (top_level_dir, remainder_relative_path).
/// For paths with no '/', group = "Cargo.toml" dir-like marker (".") and rel="filename".
fn split_top(path: &str) -> (String, String) {
    let pb = PathBuf::from(path);
fn clamp_summary(s: &str) -> String {
    truncate_ellipsis(s.trim(), 140)
fn truncate_ellipsis(s: &str, max: usize) -> String {
    if s.len() <= max {
fn normalize_tags(tags: &[String]) -> Vec<String> {
    // keep order, dedup
fn top_k_tags(freq: &BTreeMap<String, usize>, k: usize) -> (String, usize) {
    let mut v: Vec<(&str, usize)> = freq.iter().map(|(k, v)| (k.as_str(), *v)).collect();
#[derive(Default)]
struct DirNode {
    // name is implied by map key; this holds children and file entries
#[derive(Clone)]
struct TreeFile {
    name: String,  // filename only
/// Build a directory tree rooted at "" from entries.
fn build_tree(entries: &[FileIntentEntry]) -> DirNode {
    let mut root = DirNode::default();
    fn sort_node(n: &mut DirNode) {
        n.files.sort_by(|a, b| a.name.cmp(&b.name));
/// Render the tree to markdown (indented bullet list).
fn render_tree(out: &mut File, node: &DirNode, base: &str, depth: usize) -> std::io::Result<()> {
    // render current dir header only if depth==0 (root) or base not empty
fn indent(depth: usize) -> String {
    let mut s = String::new();
```

## `src/scan.rs` [rust]

- sha1: `9907c30fe8a61cd76da366d9de85369389f075b4` • size: 16951 • mtime: 1757173105
**Summary:** Repo scanner: walk, hash, detect, snippet, summarize.
```rust
Repo scanner: walks the tree, applies ignores, detects language, splits polyglot
containers (HTML), extracts snippets/metadata, and writes a JSONL index.
Distinctions vs. old version:
- Uses util::ext_to_lang + shebang for better lang map (Rust, Python, Java, HTML, CSS, JS/TS, etc.).
- Skips binaries via util::is_probably_binary.
- Splits HTML into virtual sub-entries for <script> (js/ts) and <style> (css).
- Deterministic sorting; safer writing via util::safe_write.
- Tunable limits via ScanOptions.
- Extra tags (role/module/imports/exports) preserved.
//! Repo scanner: walks the tree, applies ignores, detects language, splits polyglot
//! containers (HTML), extracts snippets/metadata, and writes a JSONL index.
//!
//! Distinctions vs. old version:
//! - Uses util::ext_to_lang + shebang for better lang map (Rust, Python, Java, HTML, CSS, JS/TS, etc.).
//! - Skips binaries via util::is_probably_binary.
//! - Splits HTML into virtual sub-entries for <script> (js/ts) and <style> (css).
//! - Deterministic sorting; safer writing via util::safe_write.
//! - Tunable limits via ScanOptions.
//! - Extra tags (role/module/imports/exports) preserved.
use anyhow::{Context, Result};
use ignore::{gitignore::GitignoreBuilder, WalkBuilder};
use sha1::{Digest, Sha1};
use std::{
    fs,
use crate::{
    file_intent_entry::FileIntentEntry,
/// Scan configuration knobs.
#[derive(Clone, Debug)]
pub struct ScanOptions {
    /// Hard cap per file (bytes). Files larger than this are skipped.
    pub max_file_bytes: u64,
    /// Head bytes for binary sniff.
    pub sniff_bytes: usize,
    /// Snippet source window (bytes).
    pub snippet_bytes: usize,
    /// Follow symlinks?
    pub follow_symlinks: bool,
    /// Include common config/doc types (json/toml/yaml/md)?
    pub include_docs_and_configs: bool,
    /// If true, split HTML into sub-entries for <script>/<style>.
    pub split_html_embeds: bool,
impl Default for ScanOptions {
    fn default() -> Self {
        Self {
/// Scan repo and write JSONL index file at `out`.
pub fn scan_and_write_index(root: &Path, out: &Path) -> Result<Vec<FileIntentEntry>> {
    let mut entries = index_project_with_opts(root, &ScanOptions::default())?;
/// Default indexer with sane options.
pub fn index_project(root: &Path) -> Result<Vec<FileIntentEntry>> {
    index_project_with_opts(root, &ScanOptions::default())
/// Full-control indexer.
pub fn index_project_with_opts(root: &Path, opts: &ScanOptions) -> Result<Vec<FileIntentEntry>> {
```

## `src/snippet.rs` [rust]

- sha1: `32ed897b8389a97ab4239ac4bb7470f7b8bb3c79` • size: 14093 • mtime: 1757173106
**Summary:** PASTE emitter: model-optimized prompt pack.
```rust
Extract a compact, high-signal snippet optimized for GPT ingestion.
Strategy:
1) Capture top-of-file docs/comments (language aware).
2) Score lines by language; keep highest-signal lines in original order,
with a sliver of context after each.
3) Hard caps and dedup to stay within MAX_KEEP_LINES.
/// Extract a compact, high-signal snippet optimized for GPT ingestion.
/// Strategy:
/// 1) Capture top-of-file docs/comments (language aware).
/// 2) Score lines by language; keep highest-signal lines in original order,
///    with a sliver of context after each.
/// 3) Hard caps and dedup to stay within MAX_KEEP_LINES.
pub fn extract_relevant_snippet(content: &str, lang: &str) -> String {
    // Window the content for speed.
fn score_line(l: &str, lang: &str) -> u8 {
    let ll = l.to_ascii_lowercase();
fn score_rust(l: &str, ll: &str) -> u8 {
    if l.starts_with("///") || l.starts_with("//!") { return 9; }                     // docs
    if ll.contains("todo") || ll.contains("fixme") { return 2; }
    0
fn score_python(l: &str, ll: &str) -> u8 {
    if l.starts_with("\"\"\"") || l.starts_with("'''") || l.starts_with("#!") || l.starts_with("# ") { return 9; } // docs/shebang
    if ll.contains("todo") || ll.contains("fixme") { return 2; }
    0
fn score_js_ts(l: &str, _ll: &str) -> u8 {
    if l.starts_with("/**") || l.starts_with("* ") || l.starts_with("//") { return 8; }       // docs/comments
fn score_go(l: &str, _ll: &str) -> u8 {
    if l.starts_with("//") { return 7; }
fn score_config(l: &str, _ll: &str) -> u8 {
    if l.starts_with('[') || l.contains(": ") || l.contains(" = ") { return 5; }
fn score_md(l: &str, _ll: &str) -> u8 {
    if l.starts_with("# ") || l.starts_with("## ") { return 8; }
fn score_generic(l: &str, _ll: &str) -> u8 {
    if l.starts_with("//") || l.starts_with("#") || l.starts_with("--") { return 6; }         // comments
fn leading_doc_block(s: &str, lang: &str) -> Option<Vec<String>> {
    match lang.to_ascii_lowercase().as_str() {
fn leading_rust_docs(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
fn leading_python_docs(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
fn leading_js_docs(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
fn leading_md_head(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
fn leading_generic_head(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
fn normalize_doc_opt(v: Vec<String>) -> Option<Vec<String>> {
    if v.is_empty() { return None; }
fn normalize_doc(lines: Vec<String>) -> Vec<String> {
    let mut v = Vec::new();
fn push_lines(out: &mut Vec<String>, lines: Vec<String>) {
    for l in lines {
fn join(lines: &[String]) -> String {
    lines.join("\n")
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rust_doc_capture() {
        let s = "//! Top module docs\n/// more\nfn main(){}\n";
```

## `src/types_view.rs` [rust]

- sha1: `f0b65256dbb43026ac0d1aae1bb61cbd8ff67827` • size: 8580 • mtime: 1757218451
**Summary:** Filesystem / IO utilities.
```rust
types_view.rs — renders "Project Types" grouped by source file, showing
structs/enums with field/variant names verbatim. Includes attributes on fields.
Accepts JSON array or JSONL index files. Only `.rs` or `lang=="rust"` entries are parsed.
Add to Cargo.toml:
```toml
[dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
syn = { version = "2", features = ["full", "extra-traits", "printing"] }
quote = "1"
```
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
#[derive(Debug, Deserialize)]
struct FileIntentEntryMini {
    path: String,
    #[allow(dead_code)]
    lang: Option<String>,
pub fn build_types_from_index(index_path: &Path, output_path: &Path) -> io::Result<()> {
    let text = fs::read_to_string(index_path)?;
fn resolve_path(root: &Path, p: &str) -> PathBuf { let pb = PathBuf::from(p); if pb.is_absolute() { pb } else { root.join(pb) } }
fn to_rel(root: &Path, p: &Path) -> PathBuf { pathdiff::diff_paths(p, root).unwrap_or_else(|| p.to_path_buf()) }
#[derive(Default)]
pub struct TypeCollector { pub out: Vec<Decl>, }
    fn visit_item(&mut self, i: &'ast Item) {
        match i {
impl TypeCollector {
    fn push_struct(&mut self, s: &ItemStruct) {
        let mut fields_out = Vec::new();
    fn push_enum(&mut self, e: &ItemEnum) {
        let public = matches!(e.vis, syn::Visibility::Public(_));
#[derive(Debug)] pub enum Decl { Struct(StructDecl), Enum(EnumDecl) }
#[derive(Debug)] pub struct StructDecl { pub name: String, pub public: bool, pub fields: Vec<FieldDecl>, }
```

## `src/util.rs` [rust]

- sha1: `c8332ac8d5f779544bbcc1cdfc4c83375d61e16a` • size: 12625 • mtime: 1757173109
**Summary:** Utility helpers for the crate.
```rust
Utility layer: workdir slugs, filenames, timestamps, tagging, and misc helpers.
No side effects beyond explicit file writes. No global state.
//! Utility layer: workdir slugs, filenames, timestamps, tagging, and misc helpers.
//! No side effects beyond explicit file writes. No global state.
use std::{
    fs::{self, File, Metadata},
/// Best-effort current directory **slug**, safe for filenames.
/// Falls back to env vars or "project". Lowercase, `[a-z0-9_-]`, collapsed `_`.
pub fn workdir_slug() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
/// Prefix an output filename with the workdir slug.
/// Example: `prefixed_filename("PROJECT_TREE", "md")` -> `indexer_PROJECT_TREE.md`
pub fn prefixed_filename(stem: &str, ext: &str) -> String {
    format!(
/// Join `base` + `rel`, normalizing `..` and stripping any leading separators in `rel`.
pub fn safe_join(base: &Path, rel: &Path) -> PathBuf {
    let rel = rel.components().filter(|c| !matches!(c, Component::RootDir)).collect::<PathBuf>();
/// RFC3339 (sortable) + a compact stamp string.
/// Example: `20250810_140359 (2025-08-10T14:03:59-05:00)`
pub fn now_timestamp() -> String {
    use chrono::{Local, SecondsFormat};
    let now = Local::now();
/// Compact, filesystem-safe UTC-agnostic local timestamp: `YYYYMMDD_HHMMSS`.
pub fn now_ts_compact() -> String {
    use chrono::{Datelike, Local, Timelike};
    let dt = Local::now();
/// Modified time → UNIX seconds (as string). Falls back to created() or "0".
pub fn to_unix_epoch(meta: &Metadata) -> String {
    fn secs(t: SystemTime) -> Option<String> {
        t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs().to_string())
/// Atomic-ish write: write to `path.tmp`, fsync, then rename over `path`.
/// Avoids torn writes on crash. Creates parent dirs as needed.
pub fn safe_write(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    if let Some(parent) = path.parent() {
/// Human-friendly bytes (SI), e.g., 1.2 KB, 3.4 MB. Exact for small numbers.
pub fn humanize_bytes(n: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
/// Count non-empty source lines (for quick LOC estimates).
pub fn count_loc(text: &str) -> usize {
    text.lines().filter(|l| !l.trim().is_empty()).count()
/// Very cheap binary detector: if >1% NULs or many non-utf8 bytes, call it binary.
/// Bound input slice to a window for speed.
pub fn is_probably_binary(bytes: &[u8]) -> bool {
    let window = bytes.get(..8192).unwrap_or(bytes);
```

