# Forge Indexer

A fast, zero-config **project indexer** that scans your repo and emits:

* a skimmable **Project Map**,
* a **Types** digest (structs/enums by file),
* a **Functions** digest (public/internal/tests by file),
* **paste-ready chunks** for LLMs—split by an approximate token cap.

It uses a compact **JSONL index** as the single source of truth, so all views stay in sync.

> Repo layout & one-liners are documented in the generated Project Map.&#x20;

---

## Quickstart

```bash
# in your repo root
cargo install --path .
indexer init
# => .gpt_index/
#    ├─ indexes/<slug>.jsonl
#    ├─ maps/<slug>_PROJECT_MAP.md
#    ├─ types/<slug>_PROJECT_TYPES.md
#    ├─ functions/<slug>_PROJECT_FUNCTIONS.md
#    └─ chunks/<slug>_paste_1.md, _2.md, ...
```

### Requirements

* Rust 1.75+ (edition 2021)
* Cross-platform (Linux/macOS/Windows)

---

## Why JSONL?

Each line is one `FileIntentEntry`—stable, grep-friendly, and easy to diff. All views (map / types / functions / chunks) read this **same** JSONL so they never drift.

---

## Commands

The CLI supports **global help** and **per-command help** in both styles:

* Root help:

  * `indexer --help`, `indexer -h`, or `indexer help`
* Command help:

  * `indexer chunk --help`, `indexer chunk -h`, or `indexer help chunk`

### `indexer init`

Scan the current directory, write `.gpt_index/indexes/<slug>.jsonl`, then auto-emit **Map → Types → Functions → Chunks**.

**Usage**

```
indexer init
```

**Outputs**

* `maps/<slug>_PROJECT_MAP.md`
* `types/<slug>_PROJECT_TYPES.md`
* `functions/<slug>_PROJECT_FUNCTIONS.md`
* `chunks/<slug>_paste_*.md`

---

### `indexer reindex`

Re-scan, archive the previous index, write a structured diff, then rebuild all views.

**Usage**

```
indexer reindex
```

**Side-effects**

* Archive: `.gpt_index/history/full/<slug>_<ts>.jsonl`
* Diff: `.gpt_index/history/diffs/<slug>_<ts>.json`

---

### `indexer sub`

Index only the **current subdirectory** into `.sub_index/indexes/<slug>.jsonl`.

**Usage**

```
indexer sub
```

---

### `indexer map`

Rebuild the project map markdown from the existing index.

**Usage**

```
indexer map
```

---

### `indexer types`

Rebuild **Project Types** (structs/enums grouped by file, field/variant names verbatim with selected attributes).

**Usage**

```
indexer types
```

---

### `indexer functions`

Rebuild **Project Functions** (functions & methods by file, grouped into public / internal / tests). Methods are prefixed with `Type::`.

**Usage**

```
indexer functions
```

---

### `indexer chunk`

Split the indexed content into pasteable markdown chunks, aiming for an approximate token budget per chunk.

**Usage**

```
indexer chunk [--cap=<N>]
```

**Flags**

* `--cap=<N>`: Approximate token cap per chunk (default: `15000`)

**Examples**

```
indexer chunk
indexer chunk --cap=12000
```

---

## Output Anatomy

```
.gpt_index/
  indexes/
    <slug>.jsonl          # authoritative JSONL index
  maps/
    <slug>_PROJECT_MAP.md # grouped catalog + tree appendix
  types/
    <slug>_PROJECT_TYPES.md
  functions/
    <slug>_PROJECT_FUNCTIONS.md
  chunks/
    <slug>_paste_1.md, _2.md, ...
  history/
    full/<slug>_<ts>.jsonl
    diffs/<slug>_<ts>.json
```

---

## Custom View Blocks (optional)

You can embed **custom index blocks** in source files to stitch bespoke docs from the current file:

```rust
//--functions public
$ # Project Functions
$ *Functions and methods by module. Signatures are shown verbatim (one line).*
//--end
```

Directives:

* `//--<category> [filters...]` starts a block. Supported categories:

  * `types` (aliases: `type`, `structs`, `enums`)
  * `functions` (aliases: `fn`, `fns`, `function`)
* Lines starting with `$` are emitted **verbatim** (header, intro text, etc.)
* `//--end` closes the block

Then run your custom renderer (if wired) to emit per-file sections that follow your prelude.

---

## Building

```bash
# dev build
cargo build

# release build
cargo build --release
```

---

## Troubleshooting

**“Types/Functions files are empty”**

* Ensure you’ve run `indexer init` or `indexer reindex` first.
* The views resolve real files **relative to your project root**, not the `.gpt_index/indexes` folder. If you moved or hand-edited the index file, keep paths repo-relative (e.g., `src/foo.rs`) so the resolvers can find them.
* Confirm that the files exist and are `.rs` (or have `lang == "rust"` in the index).

**“Chunks look too big/small”**

* Adjust `--cap` to your target LLM’s context window: `indexer chunk --cap=12000` (defaults to `15000`).
* Token counting is an estimate; it intentionally errs on the safe side.

---

## Internals (high-level)

* **scan**: walks the repo (git-aware ignores), detects language, grabs a signal-rich snippet, computes a summary & tags, writes JSONL.
* **map\_view**: renders a grouped catalog (by top-level dir) and a compact directory tree appendix.
* **types\_view**: parses Rust files in the index, listing public/private **structs/enums** with field attrs.
* **functions\_view**: parses Rust files, grouping **functions & methods** into public/internal/tests, with one-line verbatim signatures.
* **chunker**: converts the index into project **chunks** with simple token estimates and language fences.

(See `src/*.rs` for full details.)

---

## Contributing

* Issues / PRs welcome!
* Keep outputs **deterministic** (sorted) and **safe to diff**.
* Prefer **zero global state** and idempotent file ops.

---

## License

MIT (or your preferred license—fill this in).

---

## Credits

Built by Scott for a smoother “index → summarize → paste” loop, with stable JSONL at the core.
