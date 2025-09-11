# Project Functions

*Functions and methods by module. Signatures are shown verbatim (one line).*

# src/chunker.rs

## public

fn chunk_index_for_gpt (index_path: &Path, out_prefix: &str, token_cap: usize) -> Result<() > {

## internal

fn load_entries (index_path: &Path) -> Result<Vec<FileIntentEntry>> {
fn split_entry_into_parts (e: &FileIntentEntry, target_tokens: usize, hard_char_cap: usize,) -> Vec<Part > {
fn write_chunk (out_prefix: &str, idx: usize, parts: &[Part]) -> Result<() > {
fn render_file_section (out: &mut File, parts: &[Part]) -> Result<() > {
fn estimate_tokens_fallback (s: &str) -> usize {
fn fence_lang<'a>(lang:&'a str) ->&'a str {
fn count_unique_files (parts: &[Part]) -> usize {

## tests

fn token_estimator_floor () {
fn split_small_is_single_part () {
fn split_large_makes_multiple_parts () {

# src/custom_view.rs

## public

fn build_custom_from_index (index_path: &Path, output_path: &Path) -> io::Result<() > {
fn diff_paths (path: &Path, base: &Path) -> Option<PathBuf > {
Groups::pub fn extend (& mut self, it: impl Iterator<Item = (Kind, String) >) {

## internal

fn scan_custom_regions (text: &str, lang: &str) -> Vec<Section > {
fn normalize_category (c: &str) -> String {
fn category_heading (c: &str) -> String {
fn types_for_file (path: &Path) -> Result<String, Box<dyn std::error::Error>> {
fn functions_for_file (path: &Path) -> Result<String, Box<dyn std::error::Error>> {
fn resolve_path (root: &Path, p: &str) -> PathBuf {
fn to_rel (root: &Path, p: &Path) -> PathBuf {
TypeCollector::fn visit_item (& mut self, i:&'ast Item) {
TypeCollector::fn push_struct (& mut self, s: &ItemStruct) {
TypeCollector::fn push_enum (& mut self, e: &ItemEnum) {
FnCollector::fn visit_item (& mut self, i:&'ast Item) {
FnCollector::fn push_free_fn (& mut self, f: &ItemFn) {
FnCollector::fn push_impl (& mut self, imp: &ItemImpl) {
fn render_attrs (attrs: &[syn::Attribute]) -> Vec<String > {
fn norm_tokens<T: quote::ToTokens>(t: T) -> String {
fn norm_sig (sig: &syn::Signature) -> String {
fn normalize_token_string (s: &str) -> String {

# src/diff.rs

## public

fn diff_indexes (old: &[FileIntentEntry], new: &[FileIntentEntry]) -> Value {

## internal

fn json_min (e: &FileIntentEntry) -> Value {
fn json_delta (path: &str, before: &FileIntentEntry, after: &FileIntentEntry) -> Value {
fn signals_changed (a: &FileIntentEntry, b: &FileIntentEntry) -> bool {
fn tags_added (before: &[String], after: &[String]) -> Value {
fn tags_removed (before: &[String], after: &[String]) -> Value {

# src/file_intent_entry.rs

## public

Role::pub fn from_str_ic<S: AsRef<str>> (s: S) -> Self {
Role::pub fn as_str (self) ->&'static str {
FileIntentEntry::pub fn role_enum (& self) -> Role {
FileIntentEntry::pub fn set_role_enum (& mut self, r: Role) {
fn de_string_from_any<'de, D>(d: D) -> Result<String, D::Error>where D: Deserializer<'de>, {
fn de_opt_string_from_any<'de, D>(d: D) -> Result<Option<String>, D::Error>where D: Deserializer<'de>, {
fn de_vec_string_from_any<'de, D>(d: D) -> Result<Vec<String>, D::Error>where D: Deserializer<'de>, {
FileIntentEntry::pub fn stable_id (& self) -> String {
FileIntentEntry::pub fn ensure_token_estimate (& mut self) {
FileIntentEntry::pub fn tag (& mut self, t: impl Into<String >) {
FileIntentEntry::pub fn backfill_role (& mut self) {
FileIntentEntry::pub fn compute_line_metrics (& mut self) {
FileIntentEntry::pub fn is_probably_noise (& self) -> bool {

## internal

Role::fn from (s: String) -> Self {
Role::fn from (s: &str) -> Self {
Role::fn default () -> Self {
Role::fn fmt (& self, f: &mut fmt::Formatter<'_ >) -> fmt::Result {
FileIntentEntry::fn default () -> Self {
fn has (v: &[String], needle: &str) -> bool {
fn rough_token_estimate (s: &str) -> usize {

## tests

fn default_is_sane () {
fn stable_id_uses_path_and_sha_prefix () {
fn backfill_role_from_tags_and_path () {
fn line_metrics_count () {
fn ensure_token_estimate_sets_min () {

# src/functions_view.rs

## public

fn build_functions_from_index (index_path: &Path, output_path: &Path) -> io::Result<() > {
Groups::pub fn extend (& mut self, it: impl Iterator<Item = (Kind, String) >) {
fn norm_sig (sig: &syn::Signature) -> String {
fn diff_paths (path: &Path, base: &Path) -> Option<PathBuf > {

## internal

FnCollector::fn visit_item (& mut self, i:&'ast Item) {
FnCollector::fn push_free_fn (& mut self, f: &ItemFn) {
FnCollector::fn push_impl (& mut self, imp: &ItemImpl) {
fn project_root_from_index (index_path: &Path) -> PathBuf {
fn resolve_path (root: &Path, p: &str) -> PathBuf {
fn to_rel (root: &Path, p: &Path) -> PathBuf {
fn normalize_token_string (s: &str) -> String {

# src/helpers.rs

## public

fn infer_role (path: &str, lang: &str, snippet: &str) -> Role {
fn infer_module_id (path: &str, lang: &str) -> String {
fn rust_module_id (p: &str) -> String {
fn python_module_id (p: &str) -> String {
fn web_module_id (p: &str) -> String {
fn generic_module_id (p: &str) -> String {
fn skim_symbols (snippet: &str, lang: &str) -> (Vec<String>, Vec<String >) {
fn skim_rust (s: &str) -> (Vec<String>, Vec<String >) {
fn skim_python (s: &str) -> (Vec<String>, Vec<String >) {
fn skim_js_ts (s: &str) -> (Vec<String>, Vec<String >) {
fn sig_ident (line_after_prefix: &str, prefix: &str) -> String {
fn dedup_preserve_order (v: Vec<String >) -> Vec<String > {

## internal

fn trim_inline_comment (l: &str) -> &str {
fn trim_hash_comment (l: &str) -> &str {
fn trim_js_comment (l: &str) -> &str {

## tests

fn roles_basic () {
fn rust_module_paths () {
fn python_mods () {
fn skim_rust_symbols () {
fn skim_python_symbols () {
fn skim_ts_symbols () {
fn dedup_preserves_first () {

# src/index_v3.rs

## public

fn build_index_v3 (index_path: &Path, project_root: &Path, out_path: &Path) -> Result<() > {

## internal

fn hex256 (data: impl AsRef<[u8] >) -> String {
fn chunk_and_merkle (bytes: &[u8]) -> (Vec<Chunk>, String) {
fn extract_rust_anchors (src: &str) -> Result<Vec<Anchor>> {
fn struct_anchor (src: &str, s: ItemStruct) -> Result<Anchor > {
fn enum_anchor (src: &str, e: ItemEnum) -> Result<Anchor > {
fn impl_anchors (src: &str, i: ItemImpl) -> Result<Vec<Anchor>> {
fn fn_anchor (src: &str, f: ItemFn) -> Result<Anchor > {
fn fn_anchor_from_impl (src: &str, f: &ImplItemFn) -> Result<Anchor > {
fn fn_anchor_sig (src: &str, sig: &syn::Signature, is_pub: bool) -> Result<Anchor > {
fn span_start_offset (src: &str, sp: Span) -> Option<usize > {
fn offset_from_line_col (src: &str, line_1based: usize, col_0based: usize) -> usize {
fn find_body_bounds_from (src: &str, start_from: usize) -> Result<(usize, usize) > {
fn line_range (src: &str, start: usize, end: usize) -> Range {
fn find_balanced_block (src: &str, kw: &str, ident: &str) -> Result<(usize, usize) > {
fn is_ident_start (b: u8) -> bool {
fn is_ident_continue (b: u8) -> bool {
fn is_token_at (bytes: &[u8], i: usize, kw: &str) -> bool {
fn parse_ident (bytes: &[u8], mut j: usize) -> (Option<String>, usize) {
fn skip_ws_and_comments (bytes: &[u8], mut j: usize) -> usize {
fn consume_balanced_block (bytes: &[u8], start_brace: usize) -> Result<(usize, usize) > {

# src/intent.rs

## public

fn guess_summary (path: &str, snippet: &str, lang: &str) -> String {
fn extract_doc_summary (s: &str) -> Option<String > {

## internal

fn s (msg: &str) -> String {
fn contains (hay: &str, needle: &str) -> bool {
fn ends_with (hay: &str, suffix: &str) -> bool {
fn _starts_with (hay: &str, prefix: &str) -> bool {
fn ends_with_any (hay: &str, suffixes: &[& str]) -> bool {
fn any_in (hay: &str, needles: &[& str]) -> bool {
fn eq_ic (a: &str, b: &str) -> bool {
fn normalize_path (p: &str) -> String {
fn trim_window (s: &str, max: usize) -> &str {
fn is_cargo_toml (pl: &str) -> bool {
fn is_docker_related (pl: &str) -> bool {
fn is_readme (pl: &str) -> bool {
fn is_license (pl: &str) -> bool {
fn is_ci_yaml (pl: &str) -> bool {
fn is_rust_bin_entry (pl: &str, sl: &str) -> bool {
fn is_python_entry (lang: &str, sl: &str) -> bool {
fn is_test_file (pl: &str, sl: &str) -> bool {
fn is_httpish (sl: &str, pl: &str) -> bool {
fn is_dblike (sl: &str, pl: &str) -> bool {
fn is_concurrency (sl: &str) -> bool {
fn is_fsio (sl: &str, pl: &str) -> bool {
fn first_non_empty_line (s: &str) -> Option<String > {

## tests

fn detects_readme () {
fn detects_rust_main () {
fn detects_test_file () {
fn extract_prefers_rust_docs () {
fn extract_skips_fenced_code () {

# src/main.rs

## internal

fn main () -> Result<() > {

# src/map_view.rs

## public

fn build_map_from_index (index_path: &Path, output_path: &Path) -> std::io::Result<() > {
fn indent (depth: usize) -> String {

## internal

fn load_entries (index_path: &Path) -> std::io::Result<Vec<FileIntentEntry>> {
fn split_top (path: &str) -> (String, String) {
fn clamp_summary (s: &str) -> String {
fn truncate_ellipsis (s: &str, max: usize) -> String {
fn normalize_tags (tags: &[String]) -> Vec<String > {
fn top_k_tags (freq: &BTreeMap<String, usize>, k: usize) -> (String, usize) {
fn build_tree (entries: &[FileIntentEntry]) -> DirNode {
fn render_tree (out: &mut File, node: &DirNode, base: &str, depth: usize) -> std::io::Result<() > {

# src/scan.rs

## public

fn scan_and_write_index (root: &Path, out: &Path) -> Result<Vec<FileIntentEntry>> {
fn index_project (root: &Path) -> Result<Vec<FileIntentEntry>> {
fn index_project_with_opts (root: &Path, opts: &ScanOptions) -> Result<Vec<FileIntentEntry>> {
fn estimate_tokens (s: &str) -> usize {
fn read_index (path: &Path) -> Result<Vec<FileIntentEntry>> {

## internal

ScanOptions::fn default () -> Self {
fn build_ignore_matcher (root: &Path) -> Result<ignore::gitignore::Gitignore > {
fn normalize_rel (root: &Path, path: &Path) -> String {
fn is_noise_path (rel: &str) -> bool {
fn file_is_probably_binary (path: &Path, sniff: usize) -> Result<bool > {
fn read_utf8 (path: &Path) -> Result<String > {
fn sha1_hex (bytes: &[u8]) -> String {
fn slice_prefix<'a>(s:&'a str, max: usize) ->&'a str {
fn detect_lang (path: &Path) -> Result<String > {
fn lang_is_doc_or_config (lang: &str) -> bool {
fn build_entry (rel_path: &str, lang: &str, sha1: &str, size: usize, meta: &fs::Metadata, snip: &str, full_content: &str, _parent: Option<(& str, &str)>,) -> (FileIntentEntry, bool) {
HtmlBlock::fn id_for_path (& self, idx1: usize) -> String {
fn extract_html_blocks (content: &str) -> Vec<HtmlBlock<'_>> {
fn html_structure_preview (content: &str, limit_bytes: usize) -> String {
fn find_ci (hay: &[u8], needle: &[u8], from: usize) -> Option<usize > {
fn find_byte (hay: &[u8], byte: u8, from: usize) -> Option<usize > {
fn memfind (hay: &[u8], needle: &[u8]) -> bool {
fn memchr_slice (hay: &[u8], needle: &[u8], from: usize) -> Option<usize > {

# src/snippet.rs

## public

fn extract_relevant_snippet (content: &str, lang: &str) -> String {

## internal

fn score_line (l: &str, lang: &str) -> u8 {
fn score_rust (l: &str, ll: &str) -> u8 {
fn score_python (l: &str, ll: &str) -> u8 {
fn score_js_ts (l: &str, _ll: &str) -> u8 {
fn score_go (l: &str, _ll: &str) -> u8 {
fn score_config (l: &str, _ll: &str) -> u8 {
fn score_md (l: &str, _ll: &str) -> u8 {
fn score_generic (l: &str, _ll: &str) -> u8 {
fn leading_doc_block (s: &str, lang: &str) -> Option<Vec<String>> {
fn leading_rust_docs (s: &str) -> Option<Vec<String>> {
fn leading_python_docs (s: &str) -> Option<Vec<String>> {
fn leading_js_docs (s: &str) -> Option<Vec<String>> {
fn leading_md_head (s: &str) -> Option<Vec<String>> {
fn leading_generic_head (s: &str) -> Option<Vec<String>> {
fn normalize_doc_opt (v: Vec<String >) -> Option<Vec<String>> {
fn normalize_doc (lines: Vec<String >) -> Vec<String > {
fn push_lines (out: &mut Vec<String>, lines: Vec<String >) {
fn join (lines: &[String]) -> String {

## tests

fn rust_doc_capture () {
fn py_triple_quote () {
fn js_block_doc () {
fn fallback_head () {

# src/types_view.rs

## public

fn build_types_from_index (index_path: &Path, output_path: &Path) -> io::Result<() > {
fn norm_tokens<T: quote::ToTokens>(t: T) -> String {
fn diff_paths (path: &Path, base: &Path) -> Option<PathBuf > {

## internal

fn resolve_path (root: &Path, p: &str) -> PathBuf {
fn to_rel (root: &Path, p: &Path) -> PathBuf {
TypeCollector::fn visit_item (& mut self, i:&'ast Item) {
TypeCollector::fn push_struct (& mut self, s: &ItemStruct) {
TypeCollector::fn push_enum (& mut self, e: &ItemEnum) {
fn render_attrs (attrs: &[Attribute]) -> Vec<String > {
fn normalize_token_string (s: &str) -> String {
fn project_root_from_index (index_path: &Path) -> PathBuf {

# src/util.rs

## public

fn workdir_slug () -> String {
fn prefixed_filename (stem: &str, ext: &str) -> String {
fn safe_join (base: &Path, rel: &Path) -> PathBuf {
fn now_timestamp () -> String {
fn now_ts_compact () -> String {
fn to_unix_epoch (meta: &Metadata) -> String {
fn safe_write (path: &Path, contents: impl AsRef<[u8] >) -> io::Result<() > {
fn humanize_bytes (n: u64) -> String {
fn count_loc (text: &str) -> usize {
fn is_probably_binary (bytes: &[u8]) -> bool {
fn ext_to_lang (path: &Path) ->&'static str {
fn infer_tags (path: &str, lang: &str) -> Vec<String > {
fn normalize_lang (lang: &str) -> Cow<'static, str > {

## internal

fn project_name_from_path (p: &Path) -> String {
fn slugify (s: &str) -> String {
fn dedup_preserve_order (mut v: Vec<String >) -> Vec<String > {

## tests

fn slug_basic () {
fn prefixed_name () {
fn bytes_humanize () {
fn loc_counts () {
fn lang_map () {
fn tags_have_lang_and_struct () {
fn ts_compact_shape () {
fn binary_detector () {

