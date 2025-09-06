// indexer/src/file_intent_entry.rs
//! File-level intent record: what is this file, what does it export, and how should GPT treat it?
//!
//! Backward-compat:
//! - `#[serde(default)]` keeps old JSONL readable (missing new fields).
//! - `role` accepts legacy string values (case-insensitive); unknown -> Role::Other.
//!
//! Zero extra deps beyond `serde`.

use serde::{
    de::{
        self,         
        Visitor,
        Error as DeError,        
        Deserializer,
    }, 
};
use std::fmt;

/// Coarse role for retrieval/ranking. Keep small & stable.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Bin, Lib, Test, Doc, Config, Script, Ui, Core, Other,
}

impl Role {
    pub fn from_str_ic<S: AsRef<str>>(s: S) -> Self {
        match s.as_ref().to_ascii_lowercase().as_str() {
            "bin"    => Role::Bin,
            "lib"    => Role::Lib,
            "test"   => Role::Test,
            "doc"    => Role::Doc,
            "config" => Role::Config,
            "script" => Role::Script,
            "ui"     => Role::Ui,
            "core"   => Role::Core,
            _        => Role::Other,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Bin => "bin", Role::Lib => "lib", Role::Test => "test",
            Role::Doc => "doc", Role::Config => "config", Role::Script => "script",
            Role::Ui => "ui", Role::Core => "core", Role::Other => "other",
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
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(default)]
pub struct FileIntentEntry {
    #[serde(deserialize_with = "de_string_from_any")] pub path: String,
    #[serde(deserialize_with = "de_string_from_any")] pub lang: String,
    #[serde(deserialize_with = "de_string_from_any")] pub sha1: String,
    pub size: usize,
    #[serde(deserialize_with = "de_string_from_any")] pub last_modified: String,
    #[serde(deserialize_with = "de_string_from_any")] pub snippet: String,
    #[serde(deserialize_with = "de_vec_string_from_any")] pub tags: Vec<String>,
    #[serde(default, deserialize_with = "de_opt_string_from_any")] pub summary: Option<String>,
    pub token_estimate: usize,

    // enrichment
    #[serde(deserialize_with = "de_string_from_any")] pub role: String,
    #[serde(deserialize_with = "de_string_from_any")] pub module: String,
    #[serde(deserialize_with = "de_vec_string_from_any")] pub imports: Vec<String>,
    #[serde(deserialize_with = "de_vec_string_from_any")] pub exports: Vec<String>,
    pub lines_total: usize,
    pub lines_nonblank: usize,
    #[serde(deserialize_with = "de_string_from_any")] pub rel_dir: String,
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

impl FileIntentEntry {
    pub fn role_enum(&self) -> Role {
        Role::from_str_ic(&self.role)
    }
    pub fn set_role_enum(&mut self, r: Role) {
        self.role = r.as_str().to_string();
    }
}

// --------- lenient deserializer(s) ---------

// Accept string OR number -> String
pub fn de_string_from_any<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = String;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a stringable value")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: de::Error {
            Ok(v.to_owned())
        }
        fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: de::Error {
            Ok(v)
        }
        fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E> where E: de::Error {
            Ok(v.to_owned())
        }
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: de::Error {
            Ok(v.to_string())
        }
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> where E: de::Error {
            Ok(v.to_string())
        }
        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> where E: de::Error {
            // Avoid scientific notation weirdness
            Ok(if v.fract() == 0.0 { (v as i64).to_string() } else { v.to_string() })
        }
        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> where E: de::Error {
            Ok(v.to_string())
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> where E: de::Error {
            Ok(String::new())
        }
        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> where E: de::Error {
            match std::str::from_utf8(v) {
                Ok(s) => Ok(s.to_owned()),
                Err(_) => Ok(hex::encode(v)), // fallback
            }
        }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            // Join simple seq into a single string
            let mut out = String::new();
            let mut first = true;
            while let Some(serde_json::Value::String(s)) = seq.next_element()? {
                if !first { out.push_str(","); }
                first = false;
                out.push_str(&s);
            }
            if out.is_empty() {
                Err(DeError::invalid_type(de::Unexpected::Seq, &"string or scalar"))
            } else {
                Ok(out)
            }
        }
    }
    d.deserialize_any(V)
}

pub fn de_opt_string_from_any<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    // NOTE: This avoids E0282 by making the error type concrete
    struct OptV;
    impl<'de> Visitor<'de> for OptV {
        type Value = Option<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("an optional stringable value")
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> where E: de::Error { Ok(None) }
        fn visit_none<E>(self) -> Result<Self::Value, E> where E: de::Error { Ok(None) }
        fn visit_some<D2>(self, d2: D2) -> Result<Self::Value, D2::Error>
        where D2: Deserializer<'de>
        {
            de_string_from_any(d2).map(Some)
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: de::Error {
            Ok(Some(v.to_owned()))
        }
    }
    d.deserialize_option(OptV)
}

pub fn de_vec_string_from_any<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a string or array of strings/scalars")
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: de::Error {
            Ok(vec![v.to_owned()])
        }
        fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: de::Error {
            Ok(vec![v])
        }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut out = Vec::new();
            while let Some(val) = seq.next_element::<serde_json::Value>()? {
                match val {
                    serde_json::Value::String(s) => out.push(s),
                    serde_json::Value::Number(n) => out.push(n.to_string()),
                    serde_json::Value::Bool(b)   => out.push(b.to_string()),
                    serde_json::Value::Null      => {}, // skip
                    other => out.push(other.to_string()),
                }
            }
            Ok(out)
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> where E: de::Error {
            Ok(Vec::new())
        }
    }
    d.deserialize_any(V)
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
        if self.role_enum() != Role::Other {
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
        self.role = guess.to_string();
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
        assert_eq!(f.role_enum(), Role::Other);    // <- was: f.role == Role::Other (wrong)
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
        assert_eq!(f.role_enum(), Role::Bin);      // <- compare enum via helper

        let mut g = FileIntentEntry {
            path: "docs/guide.md".into(),
            tags: vec!["docs".into()],
            ..Default::default()
        };
        g.backfill_role();
        assert_eq!(g.role_enum(), Role::Doc);
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
