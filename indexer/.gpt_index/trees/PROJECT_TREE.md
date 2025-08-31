# Project File Tree

- `Cargo.toml` _[toml, dir:cargo.toml, ext:toml]_ — Cargo manifest / workspace config.
- **src/**
  - `chunker.rs` _[rust, dir:src, ext:rs]_ — Splits index into paste-friendly markdown chunks.
  - `commands.rs` _[rust, dir:src, ext:rs]_ — Resolve all standard output paths under .gpt_index for the current working dir.
  - `diff.rs` _[rust, dir:src, ext:rs]_ — Compute a structured diff between two index snapshots.
  - `file_intent_entry.rs` _[rust, dir:src, ext:rs]_ — "bin" | "lib" | "test" | "doc" | "config" | "script" | "ui" | "core"
  - `helpers.rs` _[rust, dir:src, ext:rs]_ — pub fn infer_role(path: &str, lang: &str, snippet: &str) -> String {
  - `intent.rs` _[rust, dir:src, ext:rs]_ — pub fn guess_summary(path: &str, snippet: &str, lang: &str) -> String {
  - `main.rs` _[rust, dir:src, ext:rs, crate:bin]_ — Entrypoint for this Rust binary.
  - `map_view.rs` _[rust, dir:src, ext:rs]_ — Builds hierarchical project map (markdown).
  - `scan.rs` _[rust, dir:src, ext:rs]_ — Project indexer: walk, hash, snippet, summarize.
  - `snippet.rs` _[rust, dir:src, ext:rs]_ — ------------------------- scoring + helpers -------------------------
  - `tree_view.rs` _[rust, dir:src, ext:rs]_ — Builds project directory tree (markdown).
  - `util.rs` _[rust, dir:src, ext:rs]_ — Filesystem / IO utilities.
