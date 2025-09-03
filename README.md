Here’s a **ready-to-drop `README.md`** that’s both “for dummies” and **highly optimized for GPT workflows**. It explains exactly how to run `indexer`, what to upload to ChatGPT, and how Map vs Tree vs Chunks differ. It also documents the JSONL schema, directory layout, and common tasks.

---

# Indexer — One-Command Repo Context for GPT

Indexer turns any codebase into a **portable, GPT-ready knowledge bundle**.
Run one command, upload a few files, and your AI has rich, structured context about your repo.

* **`indexer init`** → scans your repo and writes:

  * **Indexes**: `.gpt_index/indexes/<slug>.jsonl`
  * **Map** (catalog view): `.gpt_index/maps/<slug>_PROJECT_MAP.md`
  * **Tree** (structure view): `.gpt_index/trees/<slug>_PROJECT_TREE.md`
  * **Paste chunks** (LLM-ready): `.gpt_index/chunks/<slug>_paste_1.md`, `_2.md`, …

The **workdir slug** (`<slug>`) is derived from the current directory (safe for filenames), so running in multiple repos never collides.

---

## Why this exists

Large repos overwhelm LLM context windows. Indexer generates **compact, high-signal artifacts** you can upload that make your assistant instantly “code-aware”:

* **Map**: What’s in here? (top-level catalog + terse summaries + tags)
* **Tree**: Where is it? (hierarchy with per-file summaries)
* **Chunks**: Show me the code. (paste-ready, token-capped markdown with file fences)
* **Index JSONL**: Raw, structured metadata (for tooling / future automations)

Upload those four outputs and you’re good to go.

---

## Quick Start (For Humans)

1. **Install (Rust required)**

```bash
# Rust toolchain: stable recommended
cargo build --release
# Or run directly
cargo run --release -- init
```

2. **From your repo root**, run:

```bash
indexer init
# or:
cargo run --release -- init
```

You’ll get:

```
.gpt_index/
  indexes/
    <slug>.jsonl
  maps/
    <slug>_PROJECT_MAP.md
  trees/
    <slug>_PROJECT_TREE.md
  chunks/
    <slug>_paste_1.md
    <slug>_paste_2.md
    ...
  history/
    full/<slug>_<timestamp>.jsonl       # older snapshots when reindexing
    diffs/<slug>_<timestamp>.json       # structured diffs old→new
```

3. **Upload to ChatGPT (or your LLM app):**

   * `maps/<slug>_PROJECT_MAP.md`
   * `trees/<slug>_PROJECT_TREE.md`
   * `chunks/<slug>_paste_*.md` (as many as needed)
   * `indexes/<slug>.jsonl` (optional, for advanced queries or tools)

**Tip:** Start with Map + Tree + the first one or two chunk files. Add more chunks if/when the model asks for deeper code.

---

## Quick Start (For GPT)

Paste this when you begin a session (after uploading the files):

> **System prompt for GPT:**
> You have a repository bundle consisting of:
>
> * A **Map** (catalog by top-level directory with one-line summaries and tags),
> * A **Tree** (hierarchical file structure with inline summaries),
> * One or more **Paste chunks** (code snippets with fences and per-file headers), and
> * An **Index JSONL** file (structured metadata per file).
>   Use the Map for discovery, Tree for location and relationships, and Paste chunks to read source. Ask me for more chunks as needed.

---

## Views & What They’re For

### Project Map (catalog view)

**File:** `maps/<slug>_PROJECT_MAP.md`
**Purpose:** Scan the repo by **top-level directory**. Each entry is **one line**, with **`rel/path [lang] — summary`** and **tag rollups**.

* Fast onboarding (“what’s here?”).
* Groups noisy directories out of the way (e.g., `target/`, `.git/`, `node_modules/`, etc.).
* Deterministic & capped so humans/GPT can skim quickly.

### Project Tree (structure view)

**File:** `trees/<slug>_PROJECT_TREE.md`
**Purpose:** Show **hierarchy**. Within each directory, files are listed with brief summaries and tags. Use the Tree when you need to understand **where** things live and **how** modules relate.

### Paste Chunks (LLM code view)

**Files:** `chunks/<slug>_paste_1.md`, `_2.md`, …
**Purpose:** **Code you can paste** (or upload) directly.
Each chunk:

* Has a **header** with generated timestamp, file count, token estimate.
* Contains repeated sections per file: a small header (path, language, sha1, size, mtime), optional **Summary**, then a fenced code snippet.
* Honors a **token cap** (default \~15k tokens per chunk) to keep each chunk safely ingestible.

*Use more chunks as the conversation goes deeper.*

### JSONL Index (raw metadata)

**File:** `indexes/<slug>.jsonl`
**Purpose:** Machine-readable inventory. Each line is a `FileIntentEntry` for one file (see schema below). Great for building custom tools or doing structured analysis.

---

## Command Reference

```txt
indexer init
    Scan repo; write index, map, tree, and chunks.
    Creates slug-prefixed filenames to avoid collisions across repos.

indexer reindex
    Re-scan and overwrite the latest index. Archives the previous snapshot into
    history/full/<slug>_<timestamp>.jsonl and writes a structured diff to
    history/diffs/<slug>_<timestamp>.json.

indexer tree
    Rebuild only the Tree view at trees/<slug>_PROJECT_TREE.md.

indexer map
    Rebuild only the Map view at maps/<slug>_PROJECT_MAP.md.

indexer chunk [--cap=N]
    Re-split the JSONL index into paste-ready chunks with token cap N.
```

You can also scope to a subdirectory:

```bash
indexer sub
# Writes .sub_index/indexes/<slug>.jsonl for just the current subdir
```

---

## How It Works (Design Overview)

1. **Scan**

   * Walks the repository, skips obvious noise (binary, vendor, build outputs).
   * Computes per-file metadata: path, lang (from extension), size, mtime (unix), sha1, tags.
   * Extracts a **snippet** (fast, language-aware, doc-first) and a compact **summary** (intent heuristics).
   * Writes one **JSONL line per file**.

2. **Map** (`map_view`)

   * Groups files by **top-level directory** (with a `(root)` bucket).
   * Clamps summaries to one tight line; renders tag rollups for quick signal.
   * Caps per-group listings for readability.

3. **Tree** (`tree_view`)

   * Builds a hierarchical view with inline summaries and tags.

4. **Chunks** (`chunker`)

   * Sorts and packs files into **pasteable markdown** respecting a token cap.
   * Each file section includes metadata + fenced code (language normalized for renderers).
   * Snippets are safely truncated and fences remain valid.

5. **Diffs** (`diff`)

   * On `reindex`, compares old vs new by path/sha1.
   * Detects adds/removes/modifies and **renames** (via matching sha1 across different paths).
   * Emits a compact JSON summary + per-item details.

---

## JSONL Schema (`FileIntentEntry`)

Each line represents one file:

```jsonc
{
  "path": "src/chunker.rs",
  "lang": "rust",           // normalized from extension
  "sha1": "…",              // content hash
  "size": 12345,            // bytes
  "last_modified": "1693943921", // unix seconds (string)
  "snippet": "…",           // extracted doc/code lines (compact)
  "tags": ["rust","dir:src","ext:rs","chunk"],

  "summary": "Splits index into paste-friendly markdown chunks.",
  "token_estimate": 420,    // rough token count

  // Extra signals for advanced tooling (optional)
  "role": "bin|lib|test|doc|config|script|ui|core",
  "module": "scan" ,        // language-aware module id (e.g., rust "foo::bar")
  "imports": ["use foo::bar"], // skimmed edges (cheap)
  "exports": ["pub fn run_cli"],

  "lines_total": 321,
  "lines_nonblank": 250,
  "rel_dir": "src",         // top-level directory inside the repo
  "noise": false            // true for noisy infra dirs (filtered in views)
}
```

> **Compatibility:** The JSONL reader is tolerant of missing fields (`#[serde(default)]`). Older snapshots still load.

---

## File/Directory Conventions

* All outputs live in **`.gpt_index/`** (or `.sub_index/` for `indexer sub`).
* Filenames are **slug-prefixed** (derived from the working directory), e.g.:

  * `indexer_PROJECT_MAP.md`
  * `indexer_PROJECT_TREE.md`
  * `indexer_paste_1.md`

This keeps artifacts unambiguous across multiple repos.

---

## Prompts & Best Practices (LLM Use)

* Start the chat with:
  **“I’ve uploaded a Map, a Tree, and Paste chunks for this repo. Use Map for catalog, Tree for structure, and the chunks for code. Ask me if you need more chunks.”**
* When asking for code changes, **reference file paths** that appear in Map/Tree:
  *“Open `src/scan.rs` and adjust the filter to include dotfiles except `.git/`.”*
* If the model needs deeper context, upload another `…_paste_N.md`.
* Keep the `…jsonl` file for advanced / tooling flows (most conversations don’t need it).

---

## Controls & Tuning

* **Token cap:** `indexer chunk --cap=12000` (default \~15k).
  Lower for stricter models; raise for o1/o3-sized contexts.
* **Noise filtering:** Map/Tree skip or collapse noisy groups (`target/`, `node_modules/`, etc.). Adjust lists in code if needed.
* **Language fences:** Chunker normalizes fence languages (e.g., `rust`, `python`, `ts`, `javascript`, `bash`, `cpp`, `md`).

---

## Troubleshooting

* **“Index not found…”** → Run `indexer init` (or `reindex`) first.
* **Huge repos** → Lower chunk cap or upload the first few chunks only.
* **Weird summaries/tags** → They are **heuristics**. Improve by adding doc comments or README fragments at the top of critical files.
* **Renames not detected?** → We detect by **matching sha1**; if content changed significantly, it’s a modify + remove/add rather than a rename.

---

## Contributing / Extending

* Add language rules in:

  * **`util::ext_to_lang`** (extension → language label)
  * **`snippet`** (doc/line scoring per language)
  * **`intent`** (summary heuristics, config/test detection, entrypoints)
* Add/adjust noise directory lists in `map_view` / `tree_view`.
* Improve chunk packing (sorting strategy, per-file size hints) in `chunker`.

---

## Security & Privacy

* Indexer **never** uploads anything; it writes files locally.
* You decide what to share with your LLM provider.
* Exclude secrets before running or add ignore logic.

---

## License

MIT. See `LICENSE`.

---

## Example Workflow

```bash
# From repo root
indexer init

# Upload to your LLM:
.gpt_index/maps/<slug>_PROJECT_MAP.md
.gpt_index/trees/<slug>_PROJECT_TREE.md
.gpt_index/chunks/<slug>_paste_1.md
# (optionally more chunks)
# (optionally indexes/<slug>.jsonl)

# Then ask:
# “Given the Map and Tree, where is the CLI entrypoint implemented?
#  Show me the `run_cli()` function and summarize its subcommands.”
```

That’s it. You now have **Iron-Man-level repo context** in one command. 🛠️🚀
