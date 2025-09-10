use anyhow::{anyhow, Context, Result};
use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    chunker,
    custom_view::build_custom_from_index,
    diff,
    functions_view,
    map_view,
    scan,
    index_v3,
    types_view,
    util,
};

pub fn run_cli() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    // Global help/version
    if is_help_flag(cmd) {
        // `indexer --help` | `indexer -h` | `indexer help`
        let sub = args.get(2).map(|s| s.as_str());
        return print_help_dispatch(sub);
    }
    if matches!(cmd, "--version" | "-V" | "version") {
        print_version();
        return Ok(());
    }

    // Per-subcommand: accept `indexer <cmd> --help|-h`
    let sub_help = args.get(2).map(|s| s.as_str()).filter(|a| is_help_flag(a));
    if sub_help.is_some() {
        return print_help_dispatch(Some(cmd));
    }

    match cmd {
        "init" => index_root(false),
        "reindex" => index_root(true),
        "sub" => index_subdir(),
        "map" => generate_map(),
        "types" => generate_types(),
        "functions" => generate_functions(),
        "chunk" => chunk_index(args.get(2).map(|s| s.as_str())),
        "v3" | "emit-v3" => emit_v3(),
        "help" => {
            let sub = args.get(2).map(|s| s.as_str());
            print_help_dispatch(sub)
        }
        _ => {
            // Unknown: print main help
            print_help_main();
            Ok(())
        }
    }
}

fn is_help_flag(s: &str) -> bool {
    matches!(s, "--help" | "-h" | "help")
}

fn print_version() {
    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

/*───────────────────────────────────────────────────────────────────────────*
 * Path resolution & layout
 *───────────────────────────────────────────────────────────────────────────*/

/// Resolve all standard output paths under .gpt_index for the current working dir.
fn resolve_paths() -> Result<ResolvedPaths> {
    let cwd = std::env::current_dir().context("failed to get current_dir")?;
    let dir_name = util::workdir_slug();
    let index_dir = cwd.join(".gpt_index");
    let maps_dir = index_dir.join("maps");
    let types_dir = index_dir.join("types");
    let functions_dir = index_dir.join("functions");
    let chunks_dir = index_dir.join("chunks");
    let indexes_dir = index_dir.join("indexes");
    let history_full = index_dir.join("history/full");
    let history_diff = index_dir.join("history/diffs");

    // Ensure structure exists (idempotent)
    for d in [
        &index_dir,
        &functions_dir,
        &maps_dir,
        &types_dir,
        &chunks_dir,
        &indexes_dir,
        &history_full,
        &history_diff,
    ] {
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

/*───────────────────────────────────────────────────────────────────────────*
 * Commands
 *───────────────────────────────────────────────────────────────────────────*/

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

        let diff_val = diff::diff_indexes(&old_entries, &new_entries);
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

    // FUNCTIONS
    let out_functions = p
        .functions_dir
        .join(util::prefixed_filename("PROJECT_FUNCTIONS", "md"));
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

    // V3 (LLM-CODE-INDEX pack)
    let out_v3 = p.index_dir.join("index_v3.json");
    index_v3::build_index_v3(&p.index_file, &p.cwd, &out_v3)
        .context("emitting LLM-CODE-INDEX/v3")?;
    println!("LLM-CODE-INDEX/v3 written to {}", out_v3.display());


    Ok(())
}

fn index_subdir() -> Result<()> {
    let cwd = std::env::current_dir().context("get current_dir")?;
    let sub_name = util::workdir_slug();
    let index_dir = cwd.join(".sub_index");
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
    let out = p
        .functions_dir
        .join(util::prefixed_filename("PROJECT_FUNCTIONS", "md"));
    functions_view::build_functions_from_index(&p.index_file, &out)
        .with_context(|| format!("writing {}", out.display()))?;
    println!("Functions view written to {}", out.display());
    Ok(())
}

#[allow(dead_code)]
fn generate_custom() -> Result<()> {
    let paths = resolve_paths()?;
    build_custom_from_index(&paths.index_file, &paths.maps_dir.join("CUSTOM.md"))
        .map_err(anyhow::Error::from)
}

/// Support: `indexer chunk` or `indexer chunk --cap=12000`
/// Also supports: `indexer help chunk` | `indexer chunk --help`
fn chunk_index(arg: Option<&str>) -> Result<()> {
    // Accept `--help` passed as the sole arg: `indexer chunk --help`
    if arg.is_some() && is_help_flag(arg.unwrap()) {
        print_help_chunk();
        return Ok(());
    }

    let p = resolve_paths()?;
    ensure_index_exists(&p.index_file)?;

    let out_prefix = p.chunks_dir.join(format!("{}_paste_", p.dir_name));
    let out_prefix_str = out_prefix
        .to_str()
        .ok_or_else(|| anyhow!("non-utf8 path: {}", out_prefix.display()))?;

    let cap = parse_cap(arg).unwrap_or(15_000);
    chunker::chunk_index_for_gpt(&p.index_file, out_prefix_str, cap).with_context(|| {
        format!(
            "chunking {} with cap {}",
            p.index_file.display(),
            cap
        )
    })?;
    println!("Paste chunks written to {}", p.chunks_dir.display());
    Ok(())
}

fn parse_cap(arg: Option<&str>) -> Option<usize> {
    let a = arg?;
    a.trim()
        .strip_prefix("--cap=")
        .and_then(|rest| rest.parse::<usize>().ok())
}

fn ensure_index_exists(p: &Path) -> Result<()> {
    if p.exists() {
        return Ok(());
    }
    Err(anyhow!(
        "Index not found at {}. Run `indexer init` or `indexer reindex` first.",
        p.display()
    ))
}

fn emit_v3() -> Result<()> {
    let p = resolve_paths()?;
    ensure_index_exists(&p.index_file)?;
    let out = p.index_dir.join("index_v3.json");
    index_v3::build_index_v3(&p.index_file, &p.cwd, &out)
        .context("emitting LLM-CODE-INDEX/v3")?;
    println!("LLM-CODE-INDEX/v3 written to {}", out.display());
    Ok(())
}

/*───────────────────────────────────────────────────────────────────────────*
 * Help system
 *───────────────────────────────────────────────────────────────────────────*/

fn print_help_dispatch(sub: Option<&str>) -> Result<()> {
    match sub {
        None => {
            print_help_main();
            Ok(())
        }
        Some("init") => {
            print_help_init();
            Ok(())
        }
        Some("reindex") => {
            print_help_reindex();
            Ok(())
        }
        Some("sub") => {
            print_help_sub();
            Ok(())
        }
        Some("map") => {
            print_help_map();
            Ok(())
        }
        Some("types") => {
            print_help_types();
            Ok(())
        }
        Some("functions") => {
            print_help_functions();
            Ok(())
        }
        Some("chunk") => {
            print_help_chunk();
            Ok(())
        }
        Some("v3") | Some("emit-v3") => {
            print_help_v3();
            Ok(())
        }
        Some(cmd) if is_help_flag(cmd) => {
            print_help_main();
            Ok(())
        }
        Some(_unknown) => {
            print_help_main();
            Ok(())
        }
    }
}

fn print_help_main() {
    println!(
        r#"Forge Indexer — Godmode CLI

USAGE:
    indexer <COMMAND> [ARGS] [FLAGS]

GLOBAL COMMANDS:
    init         Scan & build everything (MAP, TYPES, FUNCTIONS, CHUNKS)
    reindex      Re-scan; archive previous index, write diff, then rebuild views
    sub          Index only the current subdirectory (.sub_index)
    map          Rebuild project map markdown
    types        Rebuild types markdown (structs, enums) grouped by file
    functions    Rebuild functions markdown (public/internal/tests) grouped by file
    chunk        Split index into pasteable chunks with token caps
    v3           Emit LLM-CODE-INDEX/v3 JSON (.gpt_index/index_v3.json)

GLOBAL FLAGS:
    -h, --help       Show this help or help for a subcommand
    -V, --version    Show version

EXAMPLES:
    indexer --help
    indexer help chunk
    indexer chunk --help
    indexer init
    indexer chunk --cap=12000
    indexer v3
"#
    );
}

fn print_help_init() {
    println!(
        r#"indexer init

DESCRIPTION:
    Scan the current directory into an index (.gpt_index/indexes/<slug>.jsonl),
    then automatically emit:
      - maps/<slug>_PROJECT_MAP.md
      - types/<slug>_PROJECT_TYPES.md
      - functions/<slug>_PROJECT_FUNCTIONS.md
      - chunks/<slug>_paste_*.md

USAGE:
    indexer init

NOTES:
    - Uses JSONL index; views are rebuilt every run.
"#
    );
}

fn print_help_reindex() {
    println!(
        r#"indexer reindex

DESCRIPTION:
    Archive the previous index snapshot (history/full/<slug>_<ts>.jsonl),
    write a structured diff (history/diffs/<slug>_<ts>.json), and rebuild views.

USAGE:
    indexer reindex
"#
    );
}

fn print_help_sub() {
    println!(
        r#"indexer sub

DESCRIPTION:
    Index only the current working directory into .sub_index/indexes/<slug>.jsonl

USAGE:
    indexer sub
"#
    );
}

fn print_help_map() {
    println!(
        r#"indexer map

DESCRIPTION:
    Rebuild the project map markdown from the existing index.

USAGE:
    indexer map
REQUIRES:
    A prior `indexer init` or `indexer reindex`
"#
    );
}

fn print_help_types() {
    println!(
        r#"indexer types

DESCRIPTION:
    Rebuild "Project Types" markdown: structs/enums grouped by source file,
    with field attributes verbatim (selected).

USAGE:
    indexer types
REQUIRES:
    A prior `indexer init` or `indexer reindex`
"#
    );
}

fn print_help_functions() {
    println!(
        r#"indexer functions

DESCRIPTION:
    Rebuild "Project Functions" markdown: functions & methods grouped by file,
    split into public / internal / tests. Methods are prefixed with Type::.

USAGE:
    indexer functions
REQUIRES:
    A prior `indexer init` or `indexer reindex`
"#
    );
}

fn print_help_chunk() {
    println!(
        r#"indexer chunk

DESCRIPTION:
    Split the indexed content into pasteable chunks (<slug>_paste_*.md), trying
    to respect an approximate token budget per chunk.

USAGE:
    indexer chunk [--cap=<N>]

FLAGS:
    --cap=<N>     Approximate token cap per chunk (default: 15000)

EXAMPLES:
    indexer chunk
    indexer chunk --cap=12000

REQUIRES:
    A prior `indexer init` or `indexer reindex`
"#
    );
}

fn print_help_v3() {
    println!(
        r#"indexer v3

DESCRIPTION:
    Emit a self-sufficient LLM-CODE-INDEX/v3 pack with:
      - file sha256 + Merkle chunking
      - per-anchor verbatim slices (base64) + slice sha256
      - normalized schemas & signatures

USAGE:
    indexer v3

REQUIRES:
    A prior `indexer init` or `indexer reindex` (for .jsonl existence)
"#
    );
}
