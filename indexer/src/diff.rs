// indexer/src/diff.rs

use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use crate::{
    file_intent_entry::FileIntentEntry
};


/// Compute a structured diff between two index snapshots.
/// - Detects adds/removes/modifies
/// - Detects renames by matching `sha1` across different paths
/// - Emits compact items (path + minimal fields) and a summary
pub fn diff_indexes(old: &[FileIntentEntry], new: &[FileIntentEntry]) -> Value {
    // Index by path
    let old_by_path: HashMap<&str, &FileIntentEntry> =
        old.iter().map(|e| (e.path.as_str(), e)).collect();
    let new_by_path: HashMap<&str, &FileIntentEntry> =
        new.iter().map(|e| (e.path.as_str(), e)).collect();

    // Index by sha1 (for rename detection)
    let mut old_by_sha: HashMap<&str, Vec<&FileIntentEntry>> = HashMap::new();
    let mut new_by_sha: HashMap<&str, Vec<&FileIntentEntry>> = HashMap::new();
    for e in old {
        old_by_sha.entry(e.sha1.as_str()).or_default().push(e);
    }
    for e in new {
        new_by_sha.entry(e.sha1.as_str()).or_default().push(e);
    }

    // BTreeMap for stable, path‑sorted output
    let mut added: BTreeMap<String, Value> = BTreeMap::new();
    let mut removed: BTreeMap<String, Value> = BTreeMap::new();
    let mut modified: BTreeMap<String, Value> = BTreeMap::new();
    let mut renamed: Vec<Value> = Vec::new();
    let mut unchanged_count: usize = 0;

    // Detect adds / modifies / renames / unchanged
    for (path, new_e) in &new_by_path {
        match old_by_path.get(path) {
            None => {
                // Not previously at this path — is it a rename?
                if let Some(old_hits) = old_by_sha.get(new_e.sha1.as_str()) {
                    // Choose the lexicographically first old path for determinism if multiple
                    let mut candidates: Vec<&str> = old_hits.iter().map(|e| e.path.as_str()).collect();
                    candidates.sort_unstable();
                    let from = candidates[0];
                    renamed.push(json!({
                        "from": from,
                        "to": *path,
                        "sha1": new_e.sha1,
                        "size": new_e.size
                    }));
                } else {
                    // True add
                    added.insert((*path).to_string(), json_min(new_e));
                }
            }
            Some(old_e) => {
                if old_e.sha1 != new_e.sha1 {
                    // Modified in place — compute focused field deltas
                    modified.insert((*path).to_string(), json!({
                        "path": path,
                        "before": {
                            "sha1": old_e.sha1,
                            "size": old_e.size,
                            "token_estimate": old_e.token_estimate,
                        },
                        "after": {
                            "sha1": new_e.sha1,
                            "size": new_e.size,
                            "token_estimate": new_e.token_estimate,
                        },
                        "deltas": {
                            "size": (new_e.size as i64) - (old_e.size as i64),
                            "token_estimate": (new_e.token_estimate as i64) - (old_e.token_estimate as i64),
                        }
                    }));
                } else {
                    unchanged_count += 1;
                }
            }
        }
    }

    // Detect removals that aren’t accounted for by rename
    for (path, old_e) in &old_by_path {
        if !new_by_path.contains_key(path) {
            // If sha1 still exists in new, we consider it a rename and skip removal
            let is_renamed = new_by_sha
                .get(old_e.sha1.as_str())
                .map(|v| !v.is_empty())
                .unwrap_or(false);
            if !is_renamed {
                removed.insert((*path).to_string(), json_min(old_e));
            }
        }
    }

    // Convert ordered maps to arrays
    let added = added.into_values().collect::<Vec<_>>();
    let removed = removed.into_values().collect::<Vec<_>>();
    let modified = modified.into_values().collect::<Vec<_>>();

    // Summary up top for quick UX and history files
    let summary = json!({
        "total_old": old.len(),
        "total_new": new.len(),
        "added": added.len(),
        "removed": removed.len(),
        "modified": modified.len(),
        "renamed": renamed.len(),
        "unchanged": unchanged_count
    });

    json!({
        "version": 1,
        "summary": summary,
        "added": added,
        "removed": removed,
        "modified": modified,
        "renamed": renamed
    })
}

/// Minimal JSON for a file to keep diff payloads lean.
fn json_min(e: &FileIntentEntry) -> Value {
    json!({
        "path": e.path,
        "sha1": e.sha1,
        "size": e.size,
        "lang": e.lang
    })
}
