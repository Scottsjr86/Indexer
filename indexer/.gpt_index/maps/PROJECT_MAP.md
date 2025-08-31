# Project File Map

> Files: 13  •  Groups: 2  •  Tag varieties: 7
> Tags: `dir:src`(12), `ext:rs`(12), `rust`(12), `crate:bin`(1), `dir:cargo.toml`(1), `ext:toml`(1), `toml`(1)

## `Cargo.toml/`  _(files: 1)_

> Tags: `dir:cargo.toml`(1), `ext:toml`(1), `toml`(1)

- `.` [toml] _[dir:cargo.toml, ext:toml, toml]_ — Cargo manifest / workspace config.

## `src/`  _(files: 12)_

> Tags: `dir:src`(12), `ext:rs`(12), `rust`(12), `crate:bin`(1)

- `chunker.rs` [rust] _[dir:src, ext:rs, rust]_ — Splits index into paste-friendly markdown chunks.
- `commands.rs` [rust] _[dir:src, ext:rs, rust]_ — Resolve all standard output paths under .gpt_index for the current working dir.
- `diff.rs` [rust] _[dir:src, ext:rs, rust]_ — Compute a structured diff between two index snapshots.
- `file_intent_entry.rs` [rust] _[dir:src, ext:rs, rust]_ — "bin" | "lib" | "test" | "doc" | "config" | "script" | "ui" | "core"
- `helpers.rs` [rust] _[dir:src, ext:rs, rust]_ — pub fn infer_role(path: &str, lang: &str, snippet: &str) -> String {
- `intent.rs` [rust] _[dir:src, ext:rs, rust]_ — pub fn guess_summary(path: &str, snippet: &str, lang: &str) -> String {
- `main.rs` [rust] _[crate:bin, dir:src, ext:rs, rust]_ — Entrypoint for this Rust binary.
- `map_view.rs` [rust] _[dir:src, ext:rs, rust]_ — Builds hierarchical project map (markdown).
- `scan.rs` [rust] _[dir:src, ext:rs, rust]_ — Project indexer: walk, hash, snippet, summarize.
- `snippet.rs` [rust] _[dir:src, ext:rs, rust]_ — ------------------------- scoring + helpers -------------------------
- `tree_view.rs` [rust] _[dir:src, ext:rs, rust]_ — Builds project directory tree (markdown).
- `util.rs` [rust] _[dir:src, ext:rs, rust]_ — Filesystem / IO utilities.

