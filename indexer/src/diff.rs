// indexer/src/diff.rs
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, BTreeSet};

use crate::file_intent_entry::{FileIntentEntry};

/// Compute a structured diff between two index snapshots.
/// - Adds / Removes / Modifies (by sha1 change or signal deltas)
/// - Renames (one-to-one sha1 match where old path disappeared and new path appeared)
/// - Stable, path-sorted output for deterministic diffs
pub fn diff_indexes(old: &[FileIntentEntry], new: &[FileIntentEntry]) -> Value {
    // Index by path
    let old_by_path: HashMap<&str, &FileIntentEntry> =
        old.iter().map(|e| (e.path.as_str(), e)).collect();
    let new_by_path: HashMap<&str, &FileIntentEntry> =
        new.iter().map(|e| (e.path.as_str(), e)).collect();

    // Index by sha1 (for rename/copy detection)
    let mut old_by_sha: HashMap<&str, Vec<&FileIntentEntry>> = HashMap::new();
    let mut new_by_sha: HashMap<&str, Vec<&FileIntentEntry>> = HashMap::new();
    for e in old { old_by_sha.entry(e.sha1.as_str()).or_default().push(e); }
    for e in new { new_by_sha.entry(e.sha1.as_str()).or_default().push(e); }

    // Build candidate renames:
    // For each sha present in both, if there is exactly one old path that disappeared
    // and exactly one new path that appeared for that sha, call it a rename.
    // Otherwise treat as copy/add/remove.
    let mut renamed_pairs: Vec<(&str, &str, &FileIntentEntry)> = Vec::new();
    for (&sha, new_list) in &new_by_sha {
        if let Some(old_list) = old_by_sha.get(sha) {
            // old paths that no longer exist at the same path
            let mut old_missing: Vec<&FileIntentEntry> = old_list
                .iter()
                .copied()
                .filter(|oe| !new_by_path.contains_key(oe.path.as_str()))
                .collect();
            // new paths that weren't present before
            let mut new_added: Vec<&FileIntentEntry> = new_list
                .iter()
                .copied()
                .filter(|ne| !old_by_path.contains_key(ne.path.as_str()))
                .collect();

            if old_missing.len() == 1 && new_added.len() == 1 {
                let oe = old_missing.pop().unwrap();
                let ne = new_added.pop().unwrap();
                renamed_pairs.push((oe.path.as_str(), ne.path.as_str(), ne));
            }
        }
    }
    // Deduplicate and sort rename pairs deterministically
    renamed_pairs.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

    // Track paths involved in renames to avoid double-counting as add/remove
    let mut renamed_from: BTreeSet<&str> = BTreeSet::new();
    let mut renamed_to: BTreeSet<&str> = BTreeSet::new();
    for (from, to, _) in &renamed_pairs {
        renamed_from.insert(*from);
        renamed_to.insert(*to);
    }

    // Stable output containers
    let mut added: BTreeMap<String, Value> = BTreeMap::new();
    let mut removed: BTreeMap<String, Value> = BTreeMap::new();
    let mut modified: BTreeMap<String, Value> = BTreeMap::new();
    let mut renamed: Vec<Value> = Vec::new();
    let mut unchanged_count: usize = 0;

    // Renamed list (already stable-sorted)
    for (from, to, ne) in &renamed_pairs {
        renamed.push(json!({
            "from": *from,
            "to": *to,
            "sha1": ne.sha1,
            "size": ne.size
        }));
    }

    // Detect adds / modifies / unchanged (ignoring rename targets for "added")
    for (path, new_e) in &new_by_path {
        if renamed_to.contains(path) {
            continue; // already accounted for in renamed[]
        }
        match old_by_path.get(path) {
            None => {
                // True add (not a rename)
                added.insert((*path).to_string(), json_min(new_e));
            }
            Some(old_e) => {
                if old_e.sha1 != new_e.sha1 {
                    modified.insert((*path).to_string(), json_delta(path, old_e, new_e));
                } else {
                    // Same sha1; still consider signal-only deltas (role/lang/module/tags/loc)
                    if signals_changed(old_e, new_e) {
                        modified.insert((*path).to_string(), json_delta(path, old_e, new_e));
                    } else {
                        unchanged_count += 1;
                    }
                }
            }
        }
    }

    // Detect removals (ignoring rename sources)
    for (path, old_e) in &old_by_path {
        if renamed_from.contains(path) {
            continue; // accounted for as rename
        }
        if !new_by_path.contains_key(path) {
            removed.insert((*path).to_string(), json_min(old_e));
        }
    }

    // Convert ordered maps to arrays
    let added = added.into_values().collect::<Vec<_>>();
    let removed = removed.into_values().collect::<Vec<_>>();
    let modified = modified.into_values().collect::<Vec<_>>();

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
        "version": 2,
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
        "lang": e.lang,
        "role": e.role.to_string(),
    })
}

/// Focused delta payload for a modified file (or signal-only change).
fn json_delta(path: &str, before: &FileIntentEntry, after: &FileIntentEntry) -> Value {
    json!({
        "path": path,
        "before": {
            "sha1": before.sha1,
            "size": before.size,
            "token_estimate": before.token_estimate,
            "lang": before.lang,
            "role": before.role.to_string(),
            "module": before.module,
            "lines_total": before.lines_total,
            "lines_nonblank": before.lines_nonblank,
            "tags": &before.tags,
        },
        "after": {
            "sha1": after.sha1,
            "size": after.size,
            "token_estimate": after.token_estimate,
            "lang": after.lang,
            "role": after.role.to_string(),
            "module": after.module,
            "lines_total": after.lines_total,
            "lines_nonblank": after.lines_nonblank,
            "tags": &after.tags,
        },
        "deltas": {
            "size": (after.size as i64) - (before.size as i64),
            "token_estimate": (after.token_estimate as i64) - (before.token_estimate as i64),
            "lang_changed": (before.lang != after.lang),
            "role_changed": (before.role != after.role),
            "module_changed": (before.module != after.module),
            "lines_total": (after.lines_total as i64) - (before.lines_total as i64),
            "lines_nonblank": (after.lines_nonblank as i64) - (before.lines_nonblank as i64),
            "tags_added": tags_added(&before.tags, &after.tags),
            "tags_removed": tags_removed(&before.tags, &after.tags),
        }
    })
}

/// Return true if non-content “signals” changed (lang/role/module/lines/tags).
fn signals_changed(a: &FileIntentEntry, b: &FileIntentEntry) -> bool {
    a.lang != b.lang
        || a.role != b.role
        || a.module != b.module
        || a.lines_total != b.lines_total
        || a.lines_nonblank != b.lines_nonblank
        || !tags_added(&a.tags, &b.tags).as_array().unwrap().is_empty()
        || !tags_removed(&a.tags, &b.tags).as_array().unwrap().is_empty()
}

fn tags_added(before: &[String], after: &[String]) -> Value {
    let b: BTreeSet<&str> = before.iter().map(|s| s.as_str()).collect();
    let a: BTreeSet<&str> = after.iter().map(|s| s.as_str()).collect();
    let add = a.difference(&b).cloned().collect::<Vec<&str>>();
    json!(add)
}

fn tags_removed(before: &[String], after: &[String]) -> Value {
    let b: BTreeSet<&str> = before.iter().map(|s| s.as_str()).collect();
    let a: BTreeSet<&str> = after.iter().map(|s| s.as_str()).collect();
    let rem = b.difference(&a).cloned().collect::<Vec<&str>>();
    json!(rem)
}
