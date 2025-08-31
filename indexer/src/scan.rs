// indexer/src/scan.rs

use anyhow::{
    Context, 
    Result
};
use ignore::{
    gitignore::GitignoreBuilder, 
    WalkBuilder
};
use sha1::{
    Digest, 
    Sha1
};
use std::{
    fs,
    io::{
        BufRead, 
        BufReader, 
        Read, 
        Write
    },
    path::{
        Path
    },
};
use crate::{
    file_intent_entry::FileIntentEntry, 
    helpers::{
        infer_module_id, 
        infer_role, 
        skim_symbols
    }, 
    intent, 
    snippet, 
    util
};


const MAX_FILE_BYTES: u64 = 512_000;   // ~0.5 MB hard cap per file
const BINARY_SNIFF_BYTES: usize = 4096;
const SNIPPET_BYTES: usize = 32 * 1024; // read up to 32k for snippet extraction

pub fn scan_and_write_index(root: &Path, out: &Path) -> Result<Vec<FileIntentEntry>> {
    let mut entries = index_project(root)?;
    // Deterministic output: sort by path
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let mut f = fs::File::create(out)
        .with_context(|| format!("creating index file {}", out.display()))?;
    for entry in &entries {
        writeln!(f, "{}", serde_json::to_string(entry)?)?;
    }
    Ok(entries)
}

pub fn index_project(root: &Path) -> Result<Vec<FileIntentEntry>> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut entries = Vec::new();

    // Build ignore matcher
    let mut gitignore = GitignoreBuilder::new(&root);
    if root.join(".gitignore").exists() {
        gitignore.add(".gitignore");
    }
    if root.join(".gptignore").exists() {
        gitignore.add(".gptignore");
    }
    let matcher = gitignore.build()?;

    // Walk with standard filters (.git, target, node_modules, etc.)
    let walker = WalkBuilder::new(&root).standard_filters(true).build();

    for dent in walker.filter_map(|e| e.ok()) {
        let path = dent.path();
        if !path.is_file() {
            continue;
        }

        // Relative, normalized
        let rel_path = normalize_rel(&root, path);

        // Respect ignore rules
        if matcher.matched(&rel_path, false).is_ignore() {
            continue;
        }

        // Metadata / size gate
        let meta = dent.metadata()?;
        let size = meta.len();
        if size == 0 || size > MAX_FILE_BYTES {
            continue;
        }

        // Quick binary sniff from head
        if is_probably_binary(path)? {
            continue;
        }

        // Language detection (ext + shebang)
        let lang = detect_lang(path).unwrap_or_else(|| {
            // fallback: try from extension only
            ext_to_lang(path.extension().and_then(|e| e.to_str()).unwrap_or(""))
        });
        if lang.is_empty() {
            continue; // skip unknowns
        }

        // Read snippet window & full content for SHA1 (we’ll buffer once; size is capped)
        let mut file = fs::File::open(path)
            .with_context(|| format!("open {}", path.display()))?;

        let mut content = String::with_capacity(size as usize);
        file.read_to_string(&mut content)
            .with_context(|| format!("read {}", path.display()))?;

        // UTF-8 only
        if content.is_empty() {
            continue;
        }

        // Compute full-file SHA1 (correct)
        let sha = {
            let mut hasher = Sha1::new();
            hasher.update(content.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        // Build snippet from (up to) first 32k to keep extraction fast
        let snippet_src = if content.len() > SNIPPET_BYTES {
            &content[..SNIPPET_BYTES]
        } else {
            &content
        };
        let snip = snippet::extract_relevant_snippet(snippet_src, &lang);

        // Tags / summary / tokens
        let tags = util::infer_tags(&rel_path, &lang);
        let summary = Some(intent::guess_summary(&rel_path, &snip, &lang));
        let token_estimate = estimate_tokens(&snip);
        let last_modified = util::to_unix_epoch(&meta);
        // --- LLM signals ---
        let role = infer_role(&rel_path, &lang, &snip);               // small helper using your heuristics
        let module = infer_module_id(&rel_path, &lang);               // cheap path→module mapping
        let (imports, exports) = skim_symbols(&snip, &lang);          // regex-free skim, per-lang
        let (lines_total, lines_nonblank) = {
            let t = content.lines().count();
            let nb = content.lines().filter(|l| !l.trim().is_empty()).count();
            (t, nb)
        };
        let rel_dir = rel_path.split('/').next().unwrap_or(".").to_string();
        let noise = matches!(rel_dir.as_str(), "target" | "node_modules" | ".git" | ".github" | ".idea" | ".vscode");

        entries.push(FileIntentEntry {
            path: rel_path,
            lang,
            sha1: sha,
            size: size as usize,
            last_modified,
            snippet: snip,
            tags,
            summary,
            token_estimate,

            role,
            module,
            imports,
            exports,
            lines_total,
            lines_nonblank,
            rel_dir,
            noise,
        });
    }

    Ok(entries)
}

// Very basic token estimator (customize as needed)
pub fn estimate_tokens(s: &str) -> usize {
    // 1 token ≈ 0.75 words (rough GPT rule of thumb)
    ((s.split_whitespace().count() as f64) / 0.75).ceil() as usize
}

// Reading for diffing, history, etc.
pub fn read_index(path: &Path) -> Result<Vec<FileIntentEntry>> {
    let f = fs::File::open(path)
        .with_context(|| format!("open index {}", path.display()))?;
    let rdr = BufReader::new(f);
    let mut entries = Vec::new();
    for (i, line) in rdr.lines().enumerate() {
        let line = line.with_context(|| format!("read jsonl line {}", i + 1))?;
        let entry: FileIntentEntry = serde_json::from_str(&line)
            .with_context(|| format!("parse jsonl line {}", i + 1))?;
        entries.push(entry);
    }
    Ok(entries)
}

/* ----------------------------- helpers ----------------------------- */

fn normalize_rel(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.to_string_lossy().replace('\\', "/")
}

fn is_probably_binary(path: &Path) -> Result<bool> {
    let mut f = fs::File::open(path)?;
    let mut buf = [0u8; BINARY_SNIFF_BYTES];
    let n = f.read(&mut buf)?;
    Ok(buf[..n].iter().any(|&b| b == 0))
}

// Light language detector: extension first, then shebang for scripts.
fn detect_lang(path: &Path) -> Option<String> {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let lang = ext_to_lang(ext);
        if !lang.is_empty() {
            return Some(lang);
        }
    }
    // Shebang on first line
    let mut f = fs::File::open(path).ok()?;
    let mut first = String::new();
    BufReader::new(&mut f).read_line(&mut first).ok()?;
    let l = first.trim_start();
    if l.starts_with("#!") {
        if l.contains("python") { return Some("python".into()); }
        if l.contains("bash") || l.contains("sh") { return Some("bash".into()); }
        if l.contains("node") { return Some("javascript".into()); }
    }
    None
}

fn ext_to_lang(ext: &str) -> String {
    match ext.to_ascii_lowercase().as_str() {
        // core
        "rs" => "rust",
        "py" => "python",
        "ts" => "typescript",
        "js" => "javascript",
        "go" => "go",
        "java" => "java",
        "kt" => "kotlin",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
        "c" | "h" => "c",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        // config / docs (keep if you want them in map/tree; else return "")
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "md" => "markdown",
        "sh" | "bash" | "zsh" => "bash",
        _ => "",
    }.to_string()
}
