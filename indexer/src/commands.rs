// src/commands.rs
use anyhow::{anyhow, Context, Result};
use std::{env, fs, path::{Path, PathBuf}};

use crate::{chunker, diff, functions_view, map_view, scan, types_view, util};

pub fn run_cli() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "--version" | "-V" | "version" => {
            print_version();
            Ok(())
        }
        "init"      => index_root(false),
        "reindex"   => index_root(true),
        "sub"       => index_subdir(),
        "map"       => generate_map(),
        "types"     => generate_types(),
        "functions" => generate_functions(),
        "chunk"     => chunk_index(args.get(2).map(|s| s.as_str())),
        _           => { print_help(); Ok(()) }
    }
}

fn print_version() {
    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

/// Resolve all standard output paths under .gpt_index for the current working dir.
fn resolve_paths() -> Result<ResolvedPaths> {
    let cwd = std::env::current_dir().context("failed to get current_dir")?;
    let dir_name = util::workdir_slug();
    let index_dir     = cwd.join(".gpt_index");
    let maps_dir      = index_dir.join("maps");
    let types_dir     = index_dir.join("types");
    let functions_dir = index_dir.join("functions");
    let chunks_dir    = index_dir.join("chunks");
    let indexes_dir   = index_dir.join("indexes");
    let history_full  = index_dir.join("history/full");
    let history_diff  = index_dir.join("history/diffs");

    // Ensure structure exists (idempotent)
    for d in [&index_dir, &functions_dir, &maps_dir, &types_dir, &chunks_dir, &indexes_dir, &history_full, &history_diff] {
        fs::create_dir_all(d).with_context(|| format!("creating {}", d.display()))?;
    }

    let index_file = indexes_dir.join(format!("{dir_name}.jsonl"));
    Ok(ResolvedPaths {
        cwd,
        dir_name,
        index_dir,
        maps_dir,
        types_dir,
        functions_dir,
        chunks_dir,
        indexes_dir,
        history_full,
        history_diff,
        index_file,
    })
}

struct ResolvedPaths {
    cwd: PathBuf,
    dir_name: String,
    #[allow(dead_code)]
    index_dir: PathBuf,
    maps_dir: PathBuf,
    types_dir: PathBuf,
    functions_dir: PathBuf,
    chunks_dir: PathBuf,
    #[allow(dead_code)]
    indexes_dir: PathBuf,
    history_full: PathBuf,
    history_diff: PathBuf,
    index_file: PathBuf,
}

fn index_root(is_reindex: bool) -> Result<()> {
    let p = resolve_paths()?;

    // Archive old index if reindexing
    if is_reindex && p.index_file.exists() {
        let ts = util::now_ts_compact();
        let backup_path = p.history_full.join(format!("{}_{}.jsonl", p.dir_name, ts));
        fs::rename(&p.index_file, &backup_path)
            .context("Failed to archive old index to history")?;

        let old_entries = scan::read_index(&backup_path).unwrap_or_default();
        let new_entries = scan::scan_and_write_index(&p.cwd, &p.index_file)
            .context("reindex scan/write failed")?;

        let diff_val  = diff::diff_indexes(&old_entries, &new_entries);
        let diff_path = p.history_diff.join(format!("{}_{}.json", p.dir_name, ts));
        let mut f = fs::File::create(&diff_path)
            .with_context(|| format!("creating {}", diff_path.display()))?;
        serde_json::to_writer_pretty(&mut f, &diff_val).context("writing diff json")?;
        println!("Index updated. Diff written to {}.", diff_path.display());
    } else {
        scan::scan_and_write_index(&p.cwd, &p.index_file)
            .context("initial scan/write failed")?;
        println!("Initial index complete: {}", p.index_file.display());
    }

    // === AUTO EMIT: MAP → TYPES → FUNCTIONS → CHUNKS ===

    // MAP (combined map + tree-lite)
    let out_map = p.maps_dir.join(util::prefixed_filename("PROJECT_MAP", "md"));
    map_view::build_map_from_index(&p.index_file, &out_map)
        .with_context(|| format!("writing {}", out_map.display()))?;
    println!("Map view written to {}", out_map.display());

    // TYPES (public structs/enums)
    let out_types = p.types_dir.join(util::prefixed_filename("PROJECT_TYPES", "md"));
    types_view::build_types_from_index(&p.index_file, &out_types)
        .with_context(|| format!("writing {}", out_types.display()))?;
    println!("Types view written to {}", out_types.display());

    // Functions
    let functions_name = util::prefixed_filename("PROJECT_FUNCTIONS", "md");
    let out_functions = p.functions_dir.join(functions_name);
    functions_view::build_functions_from_index(&p.index_file, &out_functions)
        .with_context(|| format!("writing {}", out_functions.display()))?;
    println!("Functions view written to {}", out_functions.display());

    // CHUNKS (paste_)
    let out_prefix = p.chunks_dir.join(format!("{}_paste_", p.dir_name));
    let out_prefix_str = out_prefix
        .to_str()
        .ok_or_else(|| anyhow!("non-utf8 path: {}", out_prefix.display()))?;
    chunker::chunk_index_for_gpt(&p.index_file, out_prefix_str, 15_000)
        .context("chunking index")?;
    println!("Paste chunks written to {}", p.chunks_dir.display());

    Ok(())
}

fn index_subdir() -> Result<()> {
    let cwd = std::env::current_dir().context("get current_dir")?;
    let sub_name = util::workdir_slug();
    let index_dir   = cwd.join(".sub_index");
    let indexes_dir = index_dir.join("indexes");
    fs::create_dir_all(indexes_dir.join("history"))
        .with_context(|| format!("creating {}", indexes_dir.display()))?;

    let index_file = indexes_dir.join(format!("{sub_name}.jsonl"));
    if index_file.exists() {
        let ts = util::now_ts_compact();
        let backup_path = indexes_dir.join(format!("history/{}_{}.jsonl", sub_name, ts));
        fs::rename(&index_file, &backup_path)
            .with_context(|| format!("archiving to {}", backup_path.display()))?;
    }

    scan::scan_and_write_index(&cwd, &index_file)
        .with_context(|| format!("writing {}", index_file.display()))?;
    println!("Subdir index complete: {}", index_file.display());
    Ok(())
}

fn generate_map() -> Result<()> {
    let p = resolve_paths()?;
    ensure_index_exists(&p.index_file)?;
    let out = p.maps_dir.join(util::prefixed_filename("PROJECT_MAP", "md"));
    map_view::build_map_from_index(&p.index_file, &out)
        .with_context(|| format!("writing {}", out.display()))?;
    println!("Map view written to {}", out.display());
    Ok(())
}

fn generate_types() -> Result<()> {
    let p = resolve_paths()?;
    ensure_index_exists(&p.index_file)?;
    let out = p.types_dir.join(util::prefixed_filename("PROJECT_TYPES", "md"));
    types_view::build_types_from_index(&p.index_file, &out)
        .with_context(|| format!("writing {}", out.display()))?;
    println!("Types view written to {}", out.display());
    Ok(())
}

fn generate_functions() -> Result<()> {
    let p = resolve_paths()?;
    ensure_index_exists(&p.index_file)?;
    let out = p.functions_dir.join(util::prefixed_filename("PROJECT_FUNCTIONS", "md"));
    functions_view::build_functions_from_index(&p.index_file, &out)
        .with_context(|| format!("writing {}", out.display()))?;
    println!("Functions view written to {}", out.display());
    Ok(())
}

/// Support: `indexer chunk` or `indexer chunk --cap=12000`
fn chunk_index(arg: Option<&str>) -> Result<()> {
    let p = resolve_paths()?;
    ensure_index_exists(&p.index_file)?;

    let out_prefix = p.chunks_dir.join(format!("{}_paste_", p.dir_name));
    let out_prefix_str = out_prefix
        .to_str()
        .ok_or_else(|| anyhow!("non-utf8 path: {}", out_prefix.display()))?;

    let cap = parse_cap(arg).unwrap_or(15_000);
    chunker::chunk_index_for_gpt(&p.index_file, out_prefix_str, cap)
        .with_context(|| format!("chunking {} with cap {}", p.index_file.display(), cap))?;
    println!("Paste chunks written to {}", p.chunks_dir.display());
    Ok(())
}

fn parse_cap(arg: Option<&str>) -> Option<usize> {
    let a = arg?;
    a.trim().strip_prefix("--cap=").and_then(|rest| rest.parse::<usize>().ok())
}

fn ensure_index_exists(p: &Path) -> Result<()> {
    if p.exists() { return Ok(()); }
    Err(anyhow!(
        "Index not found at {}. Run `indexer init` or `indexer reindex` first.",
        p.display()
    ))
}

fn print_help() {
    println!(r#"
Forge Indexer Godmode CLI

USAGE:
    indexer init
        # Index the current dir, write .gpt_index/indexes/<slug>.jsonl
        # Then emit:
        #   .gpt_index/maps/<slug>_PROJECT_MAP.md
        #   .gpt_index/types/<slug>_PROJECT_TYPES.md
        #   .gpt_index/functions/<slug>_PROJECT_FUNCTIONS.md
        #   .gpt_index/chunks/<slug>_paste_1.md (and _2, ...)

    indexer reindex
        # Re-index, archive last snapshot to .gpt_index/history/full/<slug>_<ts>.jsonl
        # Write diff to .gpt_index/history/diffs/<slug>_<ts>.json
        # Rebuild MAP, TYPES, FUNCTIONS, CHUNKS

    indexer sub
        # Index just this subdir: .sub_index/indexes/<slug>.jsonl

    indexer map
        # Rebuild maps/<slug>_PROJECT_MAP.md

    indexer types
        # Rebuild types/<slug>_PROJECT_TYPES.md

    indexer functions
        # Rebuild functions/<slug>_PROJECT_FUNCTIONS.md

    indexer chunk [--cap=N]
        # Split index into chunks/<slug>_paste_*.md files (by token cap)

    indexer help
        # Show this message
"#);
}
