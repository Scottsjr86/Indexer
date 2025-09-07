# indexer Project Map (combined)

_Legend: grouped by top-level dir • each line = `rel/path [lang] — summary` • tags shown when present_

> Files: 16  •  Groups: 2  •  Tag varieties: 13
> Tags: `dir:src`(15), `ext:rs`(15), `rust`(15), `chunk`(1), `cli`(1), `crate:bin`(1), `crate:lib`(1), `dir:cargo.toml`(1), `ext:toml`(1), `map`(1), `paste`(1), `scan`(1), `toml`(1)

## `Cargo.toml/`  _(files: 1)_

> Tags: `dir:cargo.toml`(1), `ext:toml`(1), `toml`(1)

- `.` [toml] _[toml, dir:cargo.toml, ext:toml]_ — Cargo manifest / workspace configuration.

## `src/`  _(files: 15)_

> Tags: `dir:src`(15), `ext:rs`(15), `rust`(15), `chunk`(1), `cli`(1), `crate:bin`(1), `crate:lib`(1), `map`(1), `paste`(1), `scan`(1)

- `chunker.rs` [rust] _[rust, dir:src, ext:rs, chunk]_ — Splits indexed files into GPT-ready paste chunks.
- `commands.rs` [rust] _[rust, dir:src, ext:rs]_ — CLI subcommands wiring and user-facing flows.
- `custom_view.rs` [rust] _[rust, dir:src, ext:rs]_ — Filesystem / IO utilities.
- `diff.rs` [rust] _[rust, dir:src, ext:rs]_ — Compute a structured diff between two index snapshots.
- `file_intent_entry.rs` [rust] _[rust, dir:src, ext:rs]_ — File-level intent record: what is this file, what does it export, and how should GPT treat it?
- `functions_view.rs` [rust] _[rust, dir:src, ext:rs]_ — Filesystem / IO utilities.
- `helpers.rs` [rust] _[rust, dir:src, ext:rs]_ — Formatting and shared helper utilities.
- `intent.rs` [rust] _[rust, dir:src, ext:rs]_ — Intent classifier: offline file purpose inference.
- `lib.rs` [rust] _[rust, dir:src, ext:rs, crate:lib]_ — Root library file for this Rust crate.
- `main.rs` [rust] _[rust, dir:src, ext:rs, cli, crate:bin]_ — Entrypoint for this Rust binary.
- `map_view.rs` [rust] _[rust, dir:src, ext:rs, map]_ — Builds semantic project map (markdown).
- `scan.rs` [rust] _[rust, dir:src, ext:rs, scan]_ — Repo scanner: walk, hash, detect, snippet, summarize.
- `snippet.rs` [rust] _[rust, dir:src, ext:rs, paste]_ — PASTE emitter: model-optimized prompt pack.
- `types_view.rs` [rust] _[rust, dir:src, ext:rs]_ — Filesystem / IO utilities.
- `util.rs` [rust] _[rust, dir:src, ext:rs]_ — Utility helpers for the crate.

---

## Appendix: Directory Tree

> Compact, skimmable tree with size/lang and one-line summary

- **src/** — 0 dirs, 15 files
    - `chunker.rs` — rust • 13 KB • Splits indexed files into GPT-ready paste chunks.
    - `commands.rs` — rust • 15 KB • CLI subcommands wiring and user-facing flows.
    - `custom_view.rs` — rust • 16 KB • Filesystem / IO utilities.
    - `diff.rs` — rust • 8.3 KB • Compute a structured diff between two index snapshots.
    - `file_intent_entry.rs` — rust • 14 KB • File-level intent record: what is this file, what does it export, and how should GPT treat it?
    - `functions_view.rs` — rust • 9.9 KB • Filesystem / IO utilities.
    - `helpers.rs` — rust • 13 KB • Formatting and shared helper utilities.
    - `intent.rs` — rust • 11 KB • Intent classifier: offline file purpose inference.
    - `lib.rs` — rust • 313 B • Root library file for this Rust crate.
    - `main.rs` — rust • 97 B • Entrypoint for this Rust binary.
    - `map_view.rs` — rust • 9.2 KB • Builds semantic project map (markdown).
    - `scan.rs` — rust • 17 KB • Repo scanner: walk, hash, detect, snippet, summarize.
    - `snippet.rs` — rust • 14 KB • PASTE emitter: model-optimized prompt pack.
    - `types_view.rs` — rust • 9.3 KB • Filesystem / IO utilities.
    - `util.rs` — rust • 13 KB • Utility helpers for the crate.
  - `Cargo.toml` — toml • 360 B • Cargo manifest / workspace configuration.
