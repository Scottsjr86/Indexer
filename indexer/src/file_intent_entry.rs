// indexer/src/file_intent_entry.rs

use serde::{
    Deserialize, 
    Serialize
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)] // ← lets us read older JSONL that lacks the new fields

pub struct FileIntentEntry {
    // --- existing ---
    pub path: String,
    pub lang: String,
    pub sha1: String,
    pub size: usize,
    pub last_modified: String,
    pub snippet: String,
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub token_estimate: usize,

    // --- new signals for LLMs ---
    /// "bin" | "lib" | "test" | "doc" | "config" | "script" | "ui" | "core"
    pub role: String,
    /// Best‑effort module path (lang‑aware), e.g. `scan`, `foo::bar`, or `pkg.module`
    pub module: String,
    /// Cheap import edges (regex‑free skim)
    pub imports: Vec<String>,
    /// Cheap public surface (fn/struct/trait/def/class)
    pub exports: Vec<String>,
    /// Line counts for quick size/churn heuristics
    pub lines_total: usize,
    pub lines_nonblank: usize,
    /// Top-level directory (e.g. "src")
    pub rel_dir: String,
    /// True if file lives in noisy infra dirs
    pub noise: bool,
}

impl Default for FileIntentEntry {
    fn default() -> Self {
        Self {
            path: String::new(),
            lang: String::new(),
            sha1: String::new(),
            size: 0,
            last_modified: String::new(),
            snippet: String::new(),
            tags: Vec::new(),
            summary: None,
            token_estimate: 0,

            role: String::new(),
            module: String::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            lines_total: 0,
            lines_nonblank: 0,
            rel_dir: String::new(),
            noise: false,
        }
    }
}
