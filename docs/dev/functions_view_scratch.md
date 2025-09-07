This is a snippet of what functions_view produced via "indexer init":

# Project Functions

_Public free functions and methods by module. Signatures are shown verbatim (one line)._

> Modules: 13  •  Decls: 54

## module: chunker

```rust
pub fn chunk_index_for_gpt(index_path: &Path, out_prefix: &str, token_cap: usize) -> Result<()>
```

## module: commands

```rust
pub fn run_cli() -> Result<()>
```

## module: custom_view

```rust
pub fn build_custom_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()>
```

## module: diff

```rust
pub fn diff_indexes(old: &[FileIntentEntry], new: &[FileIntentEntry]) -> Value
```
continues for all modules...


This is an example of what I want produced via hand picking:

# Project Functions

_Functions and methods by module. Signatures are shown verbatim (one line)._

# indexer/src/chunker.rs
## public
pub fn chunk_index_for_gpt(index_path: &Path, out_prefix: &str, token_cap: usize) -> Result<()> {
## internal
fn load_entries(index_path: &Path) -> Result<Vec<FileIntentEntry>> {
fn split_entry_into_parts(e: &FileIntentEntry, target_tokens: usize, hard_char_cap: usize,) -> Vec<Part>{
fn write_chunk(out_prefix: &str, idx: usize, parts: &[Part]) -> Result<()> {
fn render_file_section(out: &mut File, parts: &[Part]) -> Result<()> {
fn estimate_tokens_fallback(s: &str) -> usize {
fn fence_lang<'a>(lang: &'a str) -> &'a str {
fn count_unique_files(parts: &[Part]) -> usize {
## tests
fn token_estimator_floor() {
fn split_small_is_single_part() {
fn split_large_makes_multiple_parts() {

# indexer/src/commands.rs
## public
pub fn run_cli() -> Result<()> {
## internal
fn print_version() {
fn resolve_paths()
fn index_root(is_reindex: bool) -> Result<()> {
fn index_subdir() -> Result<()> {
fn generate_map() -> Result<()> {
fn generate_types() -> Result<()> {
fn generate_functions() -> Result<()> {
fn chunk_index(arg: Option<&str>) -> Result<()> {
fn parse_cap(arg: Option<&str>) -> Option<usize> {
fn ensure_index_exists(p: &Path) -> Result<()> {
fn print_help() {

# indexer/src/custom_view.rs
## public
pub fn build_custom_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
## internal
fn scan_custom_regions(text: &str, lang: &str) -> Vec<Section> {
fn clamp_markdown_block(s: &str, max_chars: usize) -> String {
fn normalize_category(c: &str) -> String {
fn category_heading(c: &str) -> String {
fn fence_lang(lang: &str) -> &str {
fn load_entries(index_path: &Path) -> std::io::Result<Vec<FileIntentEntry>> {

# indexer/src/diffs.rs
## public
pub fn diff_indexes(old: &[FileIntentEntry], new: &[FileIntentEntry]) -> Value {
## internal
fn json_min(e: &FileIntentEntry) -> Value {
fn json_delta(path: &str, before: &FileIntentEntry, after: &FileIntentEntry) -> Value {
fn signals_changed(a: &FileIntentEntry, b: &FileIntentEntry) -> bool {
fn tags_added(before: &[String], after: &[String]) -> Value {
fn tags_removed(before: &[String], after: &[String]) -> Value {

# indexer/src/file_intent_entry.rs
## public
Role::pub fn from_str_ic<S: AsRef<str>>(s: S) -> Self {
Role::pub fn as_str(self) -> &'static str {
FileIntentEntry::pub fn role_enum(&self) -> Role {
FileIntentEntry::pub fn set_role_enum(&mut self, r: Role) {
## internal
FileIntentEntry::fn default() -> Self {
## tests

file_intent_entry.rs is full of exceptions and need to be organived so that it shows what fn's are wrapped in what and which stand alone.
