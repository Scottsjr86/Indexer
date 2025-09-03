Upgrade Order (one-by-one)

# util.rs ✅ Done
Core helpers: workdir slug, filename prefixing, slugify, hashing, size/LOC helpers, glob matchers, safe write.
Why first: every emitter + CLI depends on it.

# scan.rs ✅ Done
Repo walker with ignores, size caps, binary detection, language detection, returns normalized FileDescriptor list.
Why now: all downstream steps need a clean, deterministic file list.

## chunker.rs ✅ Done
Deterministic chunking by language/heuristics (line + semantic fences), byte/line spans, stable IDs.
Why now: this feeds PASTE + INDEX and any summaries.

## intent.rs + file_intent_entry.rs (together) ✅ Done
File→intent classification (code, config, data, doc), priority scoring, emits FileIntentEntry.
Why now: decides what gets summarized vs. referenced vs. skipped.

## diff.rs ✅ Done
Hashing + delta decisions: reuse prior chunk IDs, detect renames/moves, minimal rewrites.
Why now: stabilizes IDs before we write MAP/PASTE/INDEX.

## helpers.rs ✅ Done
Pure utilities not tied to I/O (format tables, language map, humanize bytes, etc.).
Why now: shared by TREE/MAP/PASTE emitters.

## tree_view.rs ✅ Done
Structural emitter only: sizes, LOC, collapse heuristic, language totals; writes <slug>_PROJECT_TREE.md.
Why now: depends on scan/helpers, not on chunking.

## map_view.rs ✅ Done
Semantic map emitter: entrypoints, flows, modules, configs, build/run, conventions, gotchas; writes <slug>_PROJECT_MAP.md.
Why now: uses intents + scan; independent of chunk text.

## snippet.rs (or paste.rs, whichever you’re using) ✅ Done
Model-optimized prompt pack: SYSTEM_INSTRUCTIONS, GLOBAL_CONTEXT, CHUNKS, QUERY_PLAYBOOK; writes <slug>_PASTE.md.
Why now: consumes chunks + map; needs the earlier pieces rock solid.

## commands.rs ✅ Done
CLI subcommands wired to the new emitters (init, tree, map, paste, optionally index).
Why now: finally glue everything into user-facing commands.

## main.rs ✅ Done
Clap (or Argp) CLI parser, logging initialization, error handling policy, exit codes.
Why now: last mile, minimal churn.

## Cargo.toml ✅ Done
Features, bins, profile settings, dependency tightening, build scripts if needed.
Why last: wire up features discovered during refactors, pin versions, enable LTO, panic=abort for release, etc.