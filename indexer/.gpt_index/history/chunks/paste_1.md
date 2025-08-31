# GPT Paste Chunk 1

> Generated: 2025-08-11T02:45:04.041548910+00:00  
> Files: 13  •  ~Tokens: 1755

## `Cargo.toml` [toml]
- sha1: `a806cac41ef28eba2e96a3ede73637a55efc1636` • size: 278 • mtime: 1754880027
**Summary:** Cargo manifest / workspace config.
```toml
[package]
name = "indexer"
version = "0.1.2"
edition = "2021"
resolver = "1"
[dependencies]
anyhow = "1.0.98"
chrono = { version = "0.4.41", features = ["serde"] }
ignore = "0.4.23"
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.142"
sha1 = "0.10.6"
```

## `src/chunker.rs` [rust]
- sha1: `7904e6102dff3130fbe08b0d956e56645dd89f8a` • size: 6879 • mtime: 1754877462
**Summary:** Splits index into paste-friendly markdown chunks.
```rust
Build markdown "paste chunks" for LLMs from a JSONL index (streaming, robust).
Backwards-compatible with existing outputs like `chunks/paste_1.md`.  // README/outputs match:contentReference[oaicite:2]{index=2}:contentReference[oaicite:3]{index=3}
use anyhow::{
use chrono::Utc;
use std::{
use crate::{
/// Build markdown "paste chunks" for LLMs from a JSONL index (streaming, robust).
/// Backwards-compatible with existing outputs like `chunks/paste_1.md`.  // README/outputs match:contentReference[oaicite:2]{index=2}:contentReference[oaicite:3]{index=3}
pub fn chunk_index_for_gpt(index_path: &Path, out_prefix: &str, token_cap: usize) -> Result<()> {
/// Conservative fallback token estimator if index line forgot to set it.
fn estimate_tokens_fallback(s: &str) -> usize {
/// Write one chunk file with rich header + per-file sections.
fn write_chunk(
/// Normalize fence language to something Markdown renderers recognize.
fn fence_lang<'a>(lang: &'a str) -> &'a str {
/// Trim monstrous snippets but keep code fences valid.
fn trim_snippet(s: &str, max_chars: usize) -> String {
```

## `src/commands.rs` [rust]
- sha1: `6c47fa00bff495bea8d8fe8bbecc379bfab3f3d1` • size: 7874 • mtime: 1754877473
**Summary:** Resolve all standard output paths under .gpt_index for the current working dir.
```rust
Resolve all standard output paths under .gpt_index for the current working dir.
use anyhow::{
use std::{
use crate::{
pub fn run_cli() -> Result<()> {
/// Resolve all standard output paths under .gpt_index for the current working dir.
fn resolve_paths() -> Result<ResolvedPaths> {
struct ResolvedPaths {
#[allow(dead_code)]
fn index_root(is_reindex: bool) -> Result<()> {
fn index_subdir() -> Result<()> {
fn generate_tree() -> Result<()> {
fn generate_map() -> Result<()> {
/// Support: `indexer chunk` or `indexer chunk --cap=12000`
fn chunk_index(arg: Option<&str>) -> Result<()> {
fn parse_cap(arg: Option<&str>) -> Option<usize> {
fn ensure_index_exists(p: &Path) -> Result<()> {
fn print_help() {
```

## `src/diff.rs` [rust]
- sha1: `9e018a005de45413ebd8d1a91879cde64c453a59` • size: 4952 • mtime: 1754877524
**Summary:** Compute a structured diff between two index snapshots.
```rust
Compute a structured diff between two index snapshots.
- Detects adds/removes/modifies
- Detects renames by matching `sha1` across different paths
- Emits compact items (path + minimal fields) and a summary
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use crate::{
/// Compute a structured diff between two index snapshots.
/// - Detects adds/removes/modifies
/// - Detects renames by matching `sha1` across different paths
/// - Emits compact items (path + minimal fields) and a summary
pub fn diff_indexes(old: &[FileIntentEntry], new: &[FileIntentEntry]) -> Value {
/// Minimal JSON for a file to keep diff payloads lean.
fn json_min(e: &FileIntentEntry) -> Value {
```

## `src/file_intent_entry.rs` [rust]
- sha1: `5e27c6a32b3bce34dda1e3b288e9f499bcfe9572` • size: 1798 • mtime: 1754878269
**Summary:** "bin" | "lib" | "test" | "doc" | "config" | "script" | "ui" | "core"
```rust
"bin" | "lib" | "test" | "doc" | "config" | "script" | "ui" | "core"
use serde::{
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)] // ← lets us read older JSONL that lacks the new fields
pub struct FileIntentEntry {
/// "bin" | "lib" | "test" | "doc" | "config" | "script" | "ui" | "core"
/// Best‑effort module path (lang‑aware), e.g. `scan`, `foo::bar`, or `pkg.module`
/// Cheap import edges (regex‑free skim)
/// Cheap public surface (fn/struct/trait/def/class)
/// Line counts for quick size/churn heuristics
/// Top-level directory (e.g. "src")
/// True if file lives in noisy infra dirs
impl Default for FileIntentEntry {
fn default() -> Self {
```

## `src/helpers.rs` [rust]
- sha1: `6c772aeddf6ad7305093f56673ad9edb361e32f6` • size: 7616 • mtime: 1754877653
**Summary:** pub fn infer_role(path: &str, lang: &str, snippet: &str) -> String {
```rust
pub fn infer_role(path: &str, lang: &str, snippet: &str) -> String {
pub fn infer_module_id(path: &str, lang: &str) -> String {
pub fn rust_module_id(p: &str) -> String {
pub fn python_module_id(p: &str) -> String {
pub fn skim_symbols(snippet: &str, lang: &str) -> (Vec<String>, Vec<String>) {
pub fn skim_rust(s: &str) -> (Vec<String>, Vec<String>) {
pub fn skim_python(s: &str) -> (Vec<String>, Vec<String>) {
pub fn skim_js_ts(s: &str) -> (Vec<String>, Vec<String>) {
pub fn sig_ident(line: &str, prefix: &str) -> String {
pub fn dedup(mut v: Vec<String>) -> Vec<String> {
```

## `src/intent.rs` [rust]
- sha1: `399ec043bddcb7ef843153b3b9f348537c834918` • size: 7310 • mtime: 1754877687
**Summary:** pub fn guess_summary(path: &str, snippet: &str, lang: &str) -> String {
```rust
pub fn guess_summary(path: &str, snippet: &str, lang: &str) -> String {
#[inline]
fn s(msg: &str) -> String { msg.to_string() }
#[inline]
fn contains(hay: &str, needle: &str) -> bool { hay.contains(needle) }
#[inline]
fn ends_with(hay: &str, suffix: &str) -> bool { hay.ends_with(suffix) }
#[inline]
fn starts_with(hay: &str, prefix: &str) -> bool { hay.starts_with(prefix) }
fn any_in(hay: &str, needles: &[&str]) -> bool {
/// Extracts a succinct doc summary from module/file docs or Markdown headings.
fn extract_doc_summary(s: &str) -> Option<String> {
fn first_non_empty_line(s: &str) -> Option<String> {
```

## `src/main.rs` [rust]
- sha1: `f7866327e665c256559dd4689bef35b67fb3b787` • size: 281 • mtime: 1754877045
**Summary:** Entrypoint for this Rust binary.
```rust
pub mod chunker;
pub mod commands;
pub mod diff;
pub mod intent;
pub mod map_view;
pub mod scan;
pub mod snippet;
pub mod tree_view;
pub mod file_intent_entry;
pub mod util;
pub mod helpers;
use anyhow::Result;
fn main() -> Result<()> {
```

## `src/map_view.rs` [rust]
- sha1: `6a61dedcd7c9801187e015f83a8b54024ed7d373` • size: 6217 • mtime: 1754878465
**Summary:** Builds hierarchical project map (markdown).
```rust
Build a hierarchical, skim-friendly project map from a JSONL index.
- Groups by top-level directory
- Summaries are clamped to a single tight line
- Per-directory caps (with "+N more…" footer) keep output readable
- Skips obvious noise directories (configurable below)
use std::{
use crate::{
/// Build a hierarchical, skim-friendly project map from a JSONL index.
/// - Groups by top-level directory
/// - Summaries are clamped to a single tight line
/// - Per-directory caps (with "+N more…" footer) keep output readable
/// - Skips obvious noise directories (configurable below)
pub fn build_map_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
#[allow(unused_variables)]
#[derive(Clone)]
struct EntryLite {
fn split_top(path: &str) -> (String, String) {
fn clamp_summary(s: &str) -> String {
fn truncate_ellipsis(s: &str, max: usize) -> String {
fn top_k_tags(freq: &BTreeMap<String, usize>, k: usize) -> (String, usize) {
```

## `src/scan.rs` [rust]
- sha1: `9dfded355948de4e04b152b80a5a8b398a725d5c` • size: 7901 • mtime: 1754879348
**Summary:** Project indexer: walk, hash, snippet, summarize.
```rust
use anyhow::{
use ignore::{
use sha1::{
use std::{
use crate::{
pub fn scan_and_write_index(root: &Path, out: &Path) -> Result<Vec<FileIntentEntry>> {
pub fn index_project(root: &Path) -> Result<Vec<FileIntentEntry>> {
pub fn estimate_tokens(s: &str) -> usize {
pub fn read_index(path: &Path) -> Result<Vec<FileIntentEntry>> {
fn normalize_rel(root: &Path, path: &Path) -> String {
fn is_probably_binary(path: &Path) -> Result<bool> {
fn detect_lang(path: &Path) -> Option<String> {
fn ext_to_lang(ext: &str) -> String {
```

## `src/snippet.rs` [rust]
- sha1: `8ddcf14ebaa646f76cf76fe090d4e86d9040e738` • size: 8285 • mtime: 1754878501
**Summary:** ------------------------- scoring + helpers -------------------------
```rust
------------------------- scoring + helpers -------------------------
pub fn extract_relevant_snippet(content: &str, lang: &str) -> String {
fn score_line(l: &str, lang: &str) -> u8 {
fn score_rust(l: &str, ll: &str) -> u8 {
if ll.contains("todo") || ll.contains("fixme") { return 2; }
fn score_python(l: &str, ll: &str) -> u8 {
if ll.contains("todo") || ll.contains("fixme") { return 2; }
fn score_js_ts(l: &str, _ll: &str) -> u8 {
fn score_go(l: &str, _ll: &str) -> u8 {
fn score_config(l: &str, _ll: &str) -> u8 {
fn score_md(l: &str, _ll: &str) -> u8 {
fn score_generic(l: &str, _ll: &str) -> u8 {
fn leading_doc_block(s: &str, lang: &str) -> Option<Vec<String>> {
fn strip_rust_doc(t: &str) -> &str {
fn normalize_doc(lines: Vec<String>) -> Vec<String> {
fn push_lines(out: &mut Vec<String>, lines: Vec<String>) {
fn join(lines: &[String]) -> String {
```

## `src/tree_view.rs` [rust]
- sha1: `82d7e5ea0e6865320ee2c06aef01ddcc7504cd18` • size: 5042 • mtime: 1754878914
**Summary:** Builds project directory tree (markdown).
```rust
Build a hierarchical markdown tree from the JSONL index.
Output format (example):
- src/
- commands.rs — Run CLI entrypoints [rust]
- tree_view.rs — Builds the directory tree [rust]
- README.md — Project overview [md]
use anyhow::Context;
use std::{
use std::fs::File;
use std::io::{
use std::path::{
use crate::{
/// Build a hierarchical markdown tree from the JSONL index.
///
/// Output format (example):
/// - src/
///   - commands.rs — Run CLI entrypoints [rust]
///   - tree_view.rs — Builds the directory tree [rust]
/// - README.md — Project overview [md]
pub fn build_tree_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
fn render_dir(
fn norm(p: &Path) -> String {
fn join(base: &str, tail: &str) -> String {
fn indent(out: &mut File, depth: usize) -> std::io::Result<()> {
fn to_io<E: std::fmt::Display>(e: E) -> std::io::Error {
```

## `src/util.rs` [rust]
- sha1: `e7d9986e170e5c795db1e466e71b2fe2f6e060fa` • size: 5205 • mtime: 1754878944
**Summary:** Filesystem / IO utilities.
```rust
Best-effort current directory name, canonicalized, safe for filenames.
Fallbacks to env vars or "project" instead of erroring.
use std::{
/// Best-effort current directory name, canonicalized, safe for filenames.
/// Fallbacks to env vars or "project" instead of erroring.
pub fn get_dir_name() -> std::io::Result<String> {
/// RFC3339 (sortable) + a compact stamp string.
pub fn now_timestamp() -> String {
use chrono::{Local, SecondsFormat};
/// RFC3339 (sortable) + a compact, filesystem-safe stamp.
pub fn now_ts_compact() -> String {
use chrono::{Local, Datelike, Timelike};
/// Modified time → UNIX seconds. Falls back to created() if needed.
pub fn to_unix_epoch(meta: &Metadata) -> String {
use std::time::{SystemTime, UNIX_EPOCH};
fn secs(t: SystemTime) -> Option<String> {
/// Heuristic tagger: case-insensitive signals from path + language.
/// Adds structural tags (dir:..., ext:...) to help downstream filtering.
pub fn infer_tags(path: &str, lang: &str) -> Vec<String> {
use std::path::Component;
fn project_name_from_path(p: &Path) -> String {
fn slugify(s: &str) -> String {
fn normalize_lang<'a>(lang: &'a str) -> &'a str {
fn dedup_preserve_order(mut v: Vec<String>) -> Vec<String> {
use std::collections::HashSet;
```

