
---

# ARCHITECT.md

## Purpose (What this thing is)

`indexer` is a Rust-powered **project reconnaissance tool**: it scans a repository, fingerprints files, extracts high-signal snippets, infers intent, and emits multiple **LLM-ready** and human-readable views (tree, map, pasted chunks) plus **diffs** between snapshots. It’s optimized for **speed, robustness, and clean outputs** that slot directly into review, onboarding, and LLM workflows.

---

## System Overview (How it flows)

```
Filesystem ──▶ scan.rs ──▶ indexer.jsonl (JSONL of FileIntentEntry)
                                 │
                                 ├─▶ tree_view.rs  ──▶ tree.md (hierarchical FS view)
                                 ├─▶ map_view.rs   ──▶ map.md (semantic grouped map)
                                 └─▶ chunker.rs    ──▶ chunks/paste_1.md … (LLM-sized chunks)
Snapshots (JSONL) A & B ──▶ diff.rs ──▶ diff.json (adds/removes/modifies/renames)
CLI (commands.rs) orchestrates all subcommands & .gpt_index paths
```

* **Single binary entrypoint** (`main.rs`) dispatches to the CLI brain (`commands.rs`).
* The **scan engine** constructs a typed index (vector of `FileIntentEntry`) and writes **JSONL** to disk for streaming/large repos.
* **Views** are pure functions over the index: filesystem tree, semantic map, and GPT chunk files.
* **Diff** detects adds/removes/modifies and **renames via `sha1` content match** (path-agnostic).

---

## CLI Surface (commands.rs)

`commands.rs` provides subcommands that resolve standard output locations under `.gpt_index/` and call into modules:

* `init` / reindex current repo → writes `indexer.jsonl`
* `tree` → builds `tree.md` from index
* `map` → builds `map.md` from index
* `chunk [--cap=<tokens>]` → writes `chunks/paste_*.md` (LLM paste-ready)
* `diff <old.jsonl> <new.jsonl>` → JSON diff (adds/removes/renames)
* Utilities include path resolution, `ensure_index_exists`, `parse_cap`, and `print_help`.

> Implementation bits you’ll see: `index_root`, `index_subdir`, `generate_tree`, `generate_map`, `chunk_index`, and helpers for token cap parsing and path normalization.

---

## Core Modules (anatomy)

* **`scan.rs`** — **Engine room**: walk the repo (via `ignore`), skip binaries, detect language, hash (`sha1`), extract relevant snippet, estimate tokens, infer tags, and serialize `FileIntentEntry` as JSONL.
* **`file_intent_entry.rs`** — The **intel packet** schema for each file: role (`bin|lib|test|doc|config|script|ui|core`), best-effort module path, skimmed imports/exports/public symbols, line counts, top-level dir, “noisy infra” flags, serde defaults for backward compatibility.
* **`snippet.rs`** — **High-signal snippet** selection with language-aware scoring (prefers defs, public surface, TODO/FIXME, doc blocks) and doc normalization.
* **`helpers.rs`** — Cheap parsers: `infer_role`, module id derivation (Rust/Python/JS-TS), symbol skimmers, identifier extraction, and de-dup utilities.
* **`intent.rs`** — One-line **summary guesser**: pulls from leading docs/headers or falls back to heuristics; used across map/tree for skim-friendly lines.
* **`tree_view.rs`** — Emits **hierarchical Markdown** tree with summaries, indentation helpers, and tidy IO utilities.
* **`map_view.rs`** — Emits **semantic project map**: groups by top-level dir, clamps summaries to one line, caps entries per dir with `+N more…`, skips obvious noise.
* **`chunker.rs`** — Splits the index into **paste-friendly Markdown chunks** respecting token budgets; normalizes fence languages, trims oversized snippets while keeping fences valid; writes `chunks/paste_1.md` etc..
* **`diff.rs`** — **Structured diff**: computes adds/removes/modifies and **renames by `sha1` match across different paths**; emits compact per-file JSON plus a summary.
* **`util.rs`** — **Infra helpers**: safe project name from CWD, RFC3339 & compact timestamps, fs metadata to epoch seconds, heuristic tagger (adds `dir:...`, `ext:...`, language normalization), and dedup preserving order.
* **`commands.rs` / `main.rs`** — CLI orchestration; `main` simply calls `run_cli()`.

---

## Data & Outputs

### Index format (`.gpt_index/indexer.jsonl`)

* **One JSON object per line** (`FileIntentEntry`), enabling streaming processing for large repos and backward-compatible reads via `#[serde(default)]`.

### Views (Markdown)

* **`tree.md`** — Indented directory tree with `[lang]` and summary tails.
* **`map.md`** — Grouped by top-level dir with concise, single-line summaries and per-dir caps.

### GPT Chunk Files

* **`chunks/paste_1.md`, `paste_2.md`, …** — Paste-ready docs sized by a token cap (configurable via CLI), with chunk headers and per-file sections; code fences constrained to renderer-friendly languages.

### Diffs

* **`diff.json`** — JSON summary object plus compact items per file (path + minimal fields), with rename detection via hash equality.

---

## Key Design Principles

* **Stream-first** JSONL for resilience on big codebases; avoid memory blowups.
* **Heuristic, not AST-heavy**: cheap language-aware scoring and symbol skims to stay fast while surfacing the good parts.
* **LLM-oriented** outputs: short, clean summaries; tight snippets; chunk sizing; fence-safe Markdown.
* **Backwards compatibility** on index reading via serde defaults.

---

## Dependencies & Runtime

* Rust 2021 edition; crate version **0.1.2**
* Deps: `anyhow`, `chrono(serde)`, `ignore`, `serde(derive)`, `serde_json`, `sha1`

---

## Performance & Correctness Notes

* **Rename detection** is content-based (`sha1`) so it’s robust to path churn.
* **Binary detection**/skips avoid garbage snippets and weird encodings during scan.
* **Token estimation** exists both as fast estimator (scan) and conservative fallbacks (chunker) to keep chunk caps sane.
* **Noise handling**: utilities tag and optionally suppress infra/test/config from top views to keep the signal high.

---

## Extension Points

* **Add languages**: plug into `helpers.rs` (module id, symbol skim) and `snippet.rs` (scoring).
* **Custom views**: consume `indexer.jsonl` and render alternative reports (graphviz, HTML) sans rescanning.
* **Richer intent**: augment `intent.rs` with doc extraction patterns per language or repo conventions.

---

## Repo Topography (at a glance)

* `src/` → all Rust modules listed above; **binary crate** with `main.rs`
* `.gpt_index/` (created at runtime) → `indexer.jsonl`, `tree.md`, `map.md`, `chunks/` outputs

---

## Invariants & Guarantees

* Each **index line** corresponds to exactly one file snapshot at scan time.
* **Chunk files** never break Markdown fences or exceed the requested token cap by large margins (best-effort with conservative estimators).
* **Diff** won’t report a rename as add+remove when content fingerprints match.

---

## Known Limitations

* **Heuristics ≠ AST**: No full parsing; rare edge cases may under/over-score snippets.
* **Language detection** is extension-driven; exotic setups may need overrides.

---

## Quick Start (mental model)

1. Run `indexer init` (or equivalent reindex) → generates `.gpt_index/indexer.jsonl`.
2. Run `indexer tree` / `indexer map` → emits Markdown reports.
3. Run `indexer chunk --cap=12000` → LLM-paste files under `.gpt_index/chunks/`.
4. Run `indexer diff old.jsonl new.jsonl` → machine-readable change summary.
   All of the above are orchestrated by `commands.rs` and plain file IO; no services required.

---

