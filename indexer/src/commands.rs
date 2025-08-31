// indexer/src/commands.rs

use anyhow::{
    Result, 
    Context, 
    anyhow
};
use std::{
    env, 
    fs, 
    path::{
        Path, 
        PathBuf
    }
};
use crate::{
    scan, 
    diff, 
    util, 
    tree_view, 
    map_view, 
    chunker
};


pub fn run_cli() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "init"    => index_root(false)?,
        "reindex" => index_root(true)?,
        "sub"     => index_subdir()?,
        "tree"    => generate_tree()?,
        "map"     => generate_map()?,
        "chunk"   => chunk_index(args.get(2).map(|s| s.as_str()))?,
        "help" | _ => print_help(),
    }
    Ok(())
}

/// Resolve all standard output paths under .gpt_index for the current working dir.
fn resolve_paths() -> Result<ResolvedPaths> {
    let cwd = std::env::current_dir().context("failed to get current_dir")?;
    let dir_name = util::get_dir_name().context("failed to infer project dir name")?;

    let index_dir  = cwd.join(".gpt_index");
    let maps_dir   = index_dir.join("maps");
    let trees_dir  = index_dir.join("trees");
    let chunks_dir = index_dir.join("chunks");
    let indexes_dir= index_dir.join("indexes");
    let history_full = index_dir.join("history/full");
    let history_diffs= index_dir.join("history/diffs");

    // Ensure structure exists (idempotent)
    for d in [&index_dir, &maps_dir, &trees_dir, &chunks_dir, &indexes_dir, &history_full, &history_diffs] {
        fs::create_dir_all(d).with_context(|| format!("creating {}", d.display()))?;
    }

    let index_file = indexes_dir.join(format!("{}.jsonl", dir_name));
    Ok(ResolvedPaths {
        cwd, dir_name, index_dir, maps_dir, trees_dir, chunks_dir, indexes_dir, history_full, history_diffs, index_file
    })
}

struct ResolvedPaths {
    cwd: PathBuf,
    dir_name: String,
    #[allow(dead_code)]
    index_dir: PathBuf,
    maps_dir: PathBuf,
    trees_dir: PathBuf,
    chunks_dir: PathBuf,
    #[allow(dead_code)]
    indexes_dir: PathBuf,
    history_full: PathBuf,
    history_diffs: PathBuf,
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

        // Diff old/new
        let old_entries = scan::read_index(&backup_path).unwrap_or_default();
        let new_entries = scan::scan_and_write_index(&p.cwd, &p.index_file)
            .context("reindex scan/write failed")?;
        let diff_val = diff::diff_indexes(&old_entries, &new_entries);
        let diff_path = p.history_diffs.join(format!("{}_{}.json", p.dir_name, ts));
        let mut f = fs::File::create(&diff_path)
            .with_context(|| format!("creating {}", diff_path.display()))?;
        serde_json::to_writer_pretty(&mut f, &diff_val)
            .context("writing diff json")?;
        println!("Index updated. Diff written to {}.", diff_path.display());
    } else {
        scan::scan_and_write_index(&p.cwd, &p.index_file)
            .context("initial scan/write failed")?;
        println!("Initial index complete: {}", p.index_file.display());
    }

    // FULL AUTO: Tree, Map, Chunk (as promised in README)
    let out_tree = p.trees_dir.join("PROJECT_TREE.md");
    let out_map  = p.maps_dir.join("PROJECT_MAP.md");
    let out_prefix = p.chunks_dir.join("paste_");

    tree_view::build_tree_from_index(&p.index_file, &out_tree)
        .with_context(|| format!("writing {}", out_tree.display()))?;
    println!("Tree view written to {}", out_tree.display());

    map_view::build_map_from_index(&p.index_file, &out_map)
        .with_context(|| format!("writing {}", out_map.display()))?;
    println!("Map view written to {}", out_map.display());

    let out_prefix_str = out_prefix.to_str().ok_or_else(|| anyhow!("non-utf8 path: {}", out_prefix.display()))?;
    chunker::chunk_index_for_gpt(&p.index_file, out_prefix_str, 15_000)
        .context("chunking index")?;
    println!("Paste chunks written to {}", p.chunks_dir.display());
    Ok(())
}

fn index_subdir() -> Result<()> {
    let cwd = std::env::current_dir().context("get current_dir")?;
    let sub_name = util::get_dir_name().context("infer subdir name")?;
    let index_dir = cwd.join(".sub_index");
    let indexes_dir = index_dir.join("indexes");
    fs::create_dir_all(indexes_dir.join("history"))
        .with_context(|| format!("creating {}", indexes_dir.display()))?;

    let index_file = indexes_dir.join(format!("{}.jsonl", sub_name));
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

fn generate_tree() -> Result<()> {
    let p = resolve_paths()?;
    ensure_index_exists(&p.index_file)?;
    let out_path = p.trees_dir.join("PROJECT_TREE.md");
    tree_view::build_tree_from_index(&p.index_file, &out_path)
        .with_context(|| format!("writing {}", out_path.display()))?;
    println!("Tree view written to {}", out_path.display());
    Ok(())
}

fn generate_map() -> Result<()> {
    let p = resolve_paths()?;
    ensure_index_exists(&p.index_file)?;
    let out_path = p.maps_dir.join("PROJECT_MAP.md");
    map_view::build_map_from_index(&p.index_file, &out_path)
        .with_context(|| format!("writing {}", out_path.display()))?;
    println!("Map view written to {}", out_path.display());
    Ok(())
}

/// Support: `indexer chunk` or `indexer chunk --cap=12000`
fn chunk_index(arg: Option<&str>) -> Result<()> {
    let p = resolve_paths()?;
    ensure_index_exists(&p.index_file)?;
    let out_prefix = p.chunks_dir.join("paste_");
    let out_prefix_str = out_prefix.to_str().ok_or_else(|| anyhow!("non-utf8 path: {}", out_prefix.display()))?;

    let cap = parse_cap(arg).unwrap_or(15_000);
    chunker::chunk_index_for_gpt(&p.index_file, out_prefix_str, cap)
        .with_context(|| format!("chunking {} with cap {}", p.index_file.display(), cap))?;
    println!("Paste chunks written to {}", p.chunks_dir.display());
    Ok(())
}

fn parse_cap(arg: Option<&str>) -> Option<usize> {
    let a = arg?;
    let s = a.trim();
    if let Some(rest) = s.strip_prefix("--cap=") {
        return rest.parse::<usize>().ok();
    }
    None
}

fn ensure_index_exists(p: &Path) -> Result<()> {
    if p.exists() { return Ok(()); }
    Err(anyhow!(
        "Index not found at {}. Run `indexer init` or `indexer reindex` first.",
        p.display()
    ))
}

fn print_help() {
    println!(
r#"
Forge Indexer Godmode CLI

USAGE:
    indexer init          # Index the current dir, create .gpt_index/indexes/PROJECT.jsonl
    indexer reindex       # Re-index, archive last, diff, write fresh index
    indexer sub           # Index just this subdir (creates .sub_index/indexes/SUBDIR.jsonl)
    indexer tree          # Build trees/PROJECT_TREE.md (dir tree, inline summaries)
    indexer map           # Build maps/PROJECT_MAP.md (flat manifest, with summaries)
    indexer chunk [--cap=N]
                          # Split index into chunks/paste-ready markdown files (by token cap)
    indexer help          # Show this message

All output written to .gpt_index/ and subfolders (or .sub_index/ for subdirs).
"#    );
}
