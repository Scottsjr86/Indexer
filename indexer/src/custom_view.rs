//! Project Custom view
//!
//! Scans source files for *inline, user-tagged* regions and emits a grouped
//! markdown report (similar to chunks) by category -> file.
//!
//! Tag syntax (anywhere in a source file, usually in comments):
//!   //--<category>          # capture a region (truncated by default)
//!   //--$<category>         # capture verbatim (no truncation)
//!   ... user content ...
//!   //--end                 # closes the region
//!
//! Examples:
//!   //--enum
//!   pub enum Xyz { A, B, C }
//!   //--end
//!
//!   //--$fn
//!   fn not_pub(x: i32) -> i32 { x+1 }  // verbatim
//!   //--end
//!
//! Output: `.gpt_index/custom/<slug>_PROJECT_CUSTOM.md`

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path,},
};

use crate::file_intent_entry::FileIntentEntry;

/* ---------- public entry ---------- */

pub fn build_custom_from_index(index_path: &Path, output_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let entries = load_entries(index_path)?;
    let mut buckets: BTreeMap<String, BTreeMap<String, Vec<Section>>> = BTreeMap::new();

    for e in entries.iter() {
        // Only operate on real files on disk (skip virtual/sub-splits).
        let path = Path::new(&e.path);
        let Ok(text) = fs::read_to_string(path) else { continue };

        // Scan tagged regions inside this file
        let sections = scan_custom_regions(&text, &e.lang);
        if sections.is_empty() { continue; }

        #[allow(unused_variables)]
        let per_file = buckets.entry(String::new()).or_default(); // placeholder not used
        // Place each section into buckets[category][file].push(section)
        for s in sections {
            buckets
                .entry(s.category.clone())
                .or_default()
                .entry(e.path.clone())
                .or_default()
                .push(s);
        }
    }

    // Render
    let mut out = File::create(output_path)?;
    writeln!(out, "# Project Custom View")?;
    writeln!(out)?;
    writeln!(
        out,
        "_User-tagged regions collected from source. Use `//--<cat>` ... `//--end`, or `//-$<cat>` for verbatim._"
    )?;
    writeln!(out)?;

    if buckets.is_empty() {
        writeln!(out, "> No custom-tagged regions were found.")?;
        return Ok(());
    }

    for (category, files) in buckets {
        writeln!(out, "# {}", category_heading(&category))?;
        writeln!(out)?;
        for (file, sections) in files {
            writeln!(out, "## {}", file)?;
            writeln!(out)?;
            for s in sections {
                // For the truncated mode we still show a little “meta line”
                if !s.verbatim {
                    writeln!(out, "<!-- truncated: ~{} chars -->", s.render.len())?;
                }
                writeln!(out, "```{}``", fence_lang(&s.lang))?;
                writeln!(out, "{}", s.render)?;
                writeln!(out, "```")?;
                writeln!(out)?;
            }
        }
        writeln!(out)?;
    }

    Ok(())
}

/* ---------- scanning ---------- */

#[derive(Clone)]
struct Section {
    category: String,
    verbatim: bool,
    lang: String,
    render: String, // final text to print (already truncated if not verbatim)
}

fn scan_custom_regions(text: &str, lang: &str) -> Vec<Section> {
    let mut out = Vec::new();
    let mut cur_cat: Option<String> = None;
    let mut verbatim = false;
    let mut buf = String::new();

    // accept markers with *any* leading whitespace; we require `//--` prefix
    // e.g. "   //--fn" or "\t//-$enum"
    for raw in text.lines() {
        let l = raw.trim_start();

        if let Some(rest) = l.strip_prefix("//--") {
            let tag = rest.trim();
            if tag.eq_ignore_ascii_case("end") {
                // close
                if let Some(cat) = cur_cat.take() {
                    let render = if verbatim {
                        buf.clone()
                    } else {
                        clamp_markdown_block(&buf, 4000 /* ~like chunk clamp */)
                    };
                    out.push(Section {
                        category: normalize_category(&cat),
                        verbatim,
                        lang: lang.to_string(),
                        render,
                    });
                    buf.clear();
                }
                verbatim = false;
                continue;
            }

            // opening tag
            let (is_verbatim, cat) = if let Some(rest) = tag.strip_prefix('$') {
                (true, rest.trim())
            } else {
                (false, tag)
            };
            // if we were inside a block without `//--end`, flush it first
            if cur_cat.is_some() && !buf.is_empty() {
                let cat0 = cur_cat.take().unwrap();
                let render = if verbatim {
                    buf.clone()
                } else {
                    clamp_markdown_block(&buf, 4000)
                };
                out.push(Section {
                    category: normalize_category(&cat0),
                    verbatim,
                    lang: lang.to_string(),
                    render,
                });
                buf.clear();
            }

            cur_cat = Some(cat.to_string());
            verbatim = is_verbatim;
            continue;
        }

        // inside region: accumulate
        if cur_cat.is_some() {
            buf.push_str(raw);
            buf.push('\n');
        }
    }

    // unclosed tail
    if let Some(cat) = cur_cat {
        let render = if verbatim {
            buf
        } else {
            clamp_markdown_block(&buf, 4000)
        };
        out.push(Section {
            category: normalize_category(&cat),
            verbatim,
            lang: lang.to_string(),
            render,
        });
    }

    out
}

/* ---------- utilities ---------- */

fn clamp_markdown_block(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars { return s.trim().to_string(); }
    let mut t = s[..max_chars].to_string();
    // Try to end at a line boundary for nicer output
    if let Some(p) = t.rfind('\n') {
        t.truncate(p);
    }
    t.push_str("\n…");
    t
}

fn normalize_category(c: &str) -> String {
    // keep simple, lower-case; map common synonyms to headings
    let c = c.trim().to_ascii_lowercase();
    match c.as_str() {
        "enum" | "enums"       => "enums".into(),
        "struct" | "structs"   => "structs".into(),
        "fn" | "func" | "funcs" | "function" | "functions" => "functions".into(),
        "types"                => "types".into(),
        "fields"               => "fields".into(),
        "defaults"             => "defaults".into(),
        other                  => other.to_string(),
    }
}

fn category_heading(c: &str) -> String {
    // Title case the known ones; otherwise just echo
    match c {
        "enums" => "enums".to_string(),
        "structs" => "structs".to_string(),
        "functions" => "functions".to_string(),
        _ => c.to_string(),
    }
}

fn fence_lang(lang: &str) -> &str {
    match lang.trim().to_ascii_lowercase().as_str() {
        "rust" | "rs" => "rust",
        "ts" | "tsx"  => "ts",
        "js" | "jsx"  => "javascript",
        "py" | "python" => "python",
        "go" | "golang" => "go",
        "bash" | "sh" => "bash",
        "c" => "c",
        "cpp" | "c++" => "cpp",
        "java" => "java",
        "md" | "markdown" => "md",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "json" => "json",
        _ => "",
    }
}

/* ---------- index loader (lenient via FileIntentEntry) ---------- */

fn load_entries(index_path: &Path) -> std::io::Result<Vec<FileIntentEntry>> {
    let f = File::open(index_path)?;
    let br = BufReader::new(f);
    let mut v = Vec::new();
    for (i, line) in br.lines().enumerate() {
        let Ok(line) = line else { continue };
        match serde_json::from_str::<FileIntentEntry>(&line) {
            Ok(e) => v.push(e),
            Err(e) => {
                eprintln!("[custom] warn: bad JSONL at line {}: {}", i + 1, e);
            }
        }
    }
    Ok(v)
}
