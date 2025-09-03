// indexer/src/file_intent_entry.rs
//! File-level intent record: what is this file, what does it export, and how should GPT treat it?
//!
//! Backward-compat:
//! - `#[serde(default)]` keeps old JSONL readable (missing new fields).
//! - `role` accepts legacy string values (case-insensitive); unknown -> Role::Other.
//!
//! Zero extra deps beyond `serde`.

use serde::{Deserialize, Serialize};
use std::{fmt, hash::{Hash,}};

/// Coarse role for retrieval/ranking. Keep small & stable.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[serde(alias = "bin")]     Bin,
    #[serde(alias = "lib")]     Lib,
    #[serde(alias = "test")]    Test,
    #[serde(alias = "doc")]     Doc,
    #[serde(alias = "config")]  Config,
    #[serde(alias = "script")]  Script,
    #[serde(alias = "ui")]      Ui,
    #[serde(alias = "core")]    Core,
    #[serde(other)]             Other,
}

impl Role {
    pub fn from_str_ic<S: AsRef<str>>(s: S) -> Self {
        match s.as_ref().to_ascii_lowercase().as_str() {
            "bin"     => Role::Bin,
            "lib"     => Role::Lib,
            "test"    => Role::Test,
            "doc"     => Role::Doc,
            "config"  => Role::Config,
            "script"  => Role::Script,
            "ui"      => Role::Ui,
            "core"    => Role::Core,
            _         => Role::Other,
        }
    }
}

impl From<String> for Role {
    fn from(s: String) -> Self { Role::from_str_ic(&s) }
}
impl From<&str> for Role {
    fn from(s: &str) -> Self { Role::from_str_ic(s) }
}

impl Default for Role {
    fn default() -> Self { Role::Other }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Role::*;
        let s = match self {
            Bin => "bin", Lib => "lib", Test => "test", Doc => "doc",
            Config => "config", Script => "script", Ui => "ui", Core => "core", Other => "other",
        };
        f.write_str(s)
    }
}

/// Primary record emitted per file. This is your JSONL unit.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct FileIntentEntry {
    // --- existing core fields ---
    pub path: String,
    pub lang: String,
    pub sha1: String,
    pub size: usize,              // bytes
    pub last_modified: String,    // unix secs or rfc3339; we print, not parse
    pub snippet: String,          // source excerpt (may be trimmed)
    pub tags: Vec<String>,        // structural + heuristic tags
    pub summary: Option<String>,  // short, high-signal description
    pub token_estimate: usize,    // rough token count for snippet

    // --- enriched signals for LLMs ---
    pub role: Role,               // typed (enum) but deserializes from legacy strings
    /// Best-effort module path (lang-aware), e.g. `scan`, `foo::bar`, or `pkg.module`
    pub module: String,
    /// Cheap import edges (regex-free skim)
    pub imports: Vec<String>,
    /// Cheap public surface (fn/struct/trait/def/class)
    pub exports: Vec<String>,
    /// Line counts for quick size/churn heuristics
    pub lines_total: usize,
    pub lines_nonblank: usize,
    /// Top-level directory (e.g., "src")
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

            role: Role::Other,
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

/* ============================== Convenience API ============================== */

impl FileIntentEntry {
    /// Stable-ish chunk/grouping ID for retrieval/ranking (path + sha1 prefix).
    /// Use this as a neighbor key in your INDEX.jsonl or for dedup across runs.
    pub fn stable_id(&self) -> String {
        format!("F:{}:{}", self.path, self.sha1.chars().take(8).collect::<String>())
    }

    /// Ensure we always have a token estimate even if upstream forgot.
    pub fn ensure_token_estimate(&mut self) {
        if self.token_estimate == 0 {
            self.token_estimate = rough_token_estimate(&self.snippet);
        }
    }

    /// Add tag if missing (preserve order of first occurrence).
    pub fn tag(&mut self, t: impl Into<String>) {
        let t = t.into();
        if !self.tags.iter().any(|x| x == &t) {
            self.tags.push(t);
        }
    }

    /// Quick role inference from existing tags/path when role == Other.
    /// Idempotent: only sets when currently Other.
    pub fn backfill_role(&mut self) {
        if self.role != Role::Other {
            return;
        }
        let pl = self.path.to_ascii_lowercase();
        let guess = if pl.ends_with("src/main.rs") || pl.contains("/bin/") || has(&self.tags, "cli") {
            Role::Bin
        } else if pl.contains("/tests") || pl.ends_with("_test.rs") || has(&self.tags, "test") {
            Role::Test
        } else if pl.ends_with(".md") || has(&self.tags, "docs") {
            Role::Doc
        } else if pl.ends_with(".toml") || pl.ends_with(".yml") || pl.ends_with(".yaml") || pl.ends_with(".env") || has(&self.tags, "config") {
            Role::Config
        } else if pl.ends_with(".sh") || has(&self.tags, "script") {
            Role::Script
        } else if pl.contains("/ui") || has(&self.tags, "ui") {
            Role::Ui
        } else if has(&self.tags, "core") {
            Role::Core
        } else {
            Role::Other
        };
        self.role = guess;
    }

    /// Cheap line metrics (nonblank + total). No allocation beyond iterators.
    pub fn compute_line_metrics(&mut self) {
        let mut total = 0usize;
        let mut nonblank = 0usize;
        for l in self.snippet.lines() {
            total += 1;
            if !l.trim().is_empty() {
                nonblank += 1;
            }
        }
        self.lines_total = total;
        self.lines_nonblank = nonblank;
    }

    /// Returns `true` if this looks like infra/noise (cache, build, vendored).
    pub fn is_probably_noise(&self) -> bool {
        let pl = self.path.to_ascii_lowercase();
        self.noise ||
        pl.contains("/target/") ||
        pl.contains("/node_modules/") ||
        pl.contains("/dist/") ||
        pl.contains("/build/") ||
        pl.contains("/.cache/") ||
        pl.contains("/__pycache__/")
    }
}

/* ============================== Small helpers ============================== */

fn has(v: &[String], needle: &str) -> bool {
    v.iter().any(|t| t == needle)
}

/// ~1 token per 4 chars, min 12 (mirrors chunker fallback).
fn rough_token_estimate(s: &str) -> usize {
    (s.len() / 4).max(12)
}

/* ================================== Tests ================================== */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_sane() {
        let f = FileIntentEntry::default();
        assert_eq!(f.role, Role::Other);
        assert_eq!(f.token_estimate, 0);
        assert!(f.tags.is_empty());
    }

    #[test]
    fn stable_id_uses_path_and_sha_prefix() {
        let mut f = FileIntentEntry::default();
        f.path = "src/main.rs".into();
        f.sha1 = "deadbeefcafebabef00d".into();
        let id = f.stable_id();
        assert!(id.starts_with("F:src/main.rs:deadbeef"));
    }

    #[test]
    fn backfill_role_from_tags_and_path() {
        let mut f = FileIntentEntry {
            path: "src/main.rs".into(),
            ..Default::default()
        };
        f.backfill_role();
        assert_eq!(f.role, Role::Bin);

        let mut g = FileIntentEntry {
            path: "docs/guide.md".into(),
            tags: vec!["docs".into()],
            ..Default::default()
        };
        g.backfill_role();
        assert_eq!(g.role, Role::Doc);
    }

    #[test]
    fn line_metrics_count() {
        let mut f = FileIntentEntry {
            snippet: "a\n\n b \n".into(),
            ..Default::default()
        };
        f.compute_line_metrics();
        assert_eq!(f.lines_total, 3);
        assert_eq!(f.lines_nonblank, 2);
    }

    #[test]
    fn ensure_token_estimate_sets_min() {
        let mut f = FileIntentEntry { snippet: "abcd".into(), ..Default::default() };
        f.ensure_token_estimate();
        assert!(f.token_estimate >= 12);
    }
}
