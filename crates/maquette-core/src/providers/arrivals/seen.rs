//! Files the watcher already took or was told to leave, by content hash, so
//! the same download is never imported twice: not when the browser touches
//! it again, not when it is saved a second time under another name, and not
//! after a restart. Kept per workspace in `downloads-seen.json`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;

pub(super) const SEEN_FILE: &str = "downloads-seen.json";
const VERSION: u32 = 1;
/// The newest hashes kept.
const KEEP: usize = 500;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Entry {
    pub(super) hash: String,
    /// The job it went to; `None` when it was dismissed.
    pub(super) job: Option<String>,
    pub(super) at_ms: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SeenFile {
    version: u32,
    entries: Vec<Entry>,
}

fn path(root: &Path) -> PathBuf {
    root.join(SEEN_FILE)
}

/// The SHA-256 of a file, as the pipeline cache hashes it.
pub(super) fn hash_file(file: &Path) -> CoreResult<String> {
    crate::pipeline::cache::hash_file(file).map_err(|e| CoreError::io(file, e))
}

/// Every hash seen in the workspace at `root`. A missing or unreadable
/// file is an empty list.
pub(super) fn load(root: &Path) -> Vec<Entry> {
    let Ok(text) = fs::read_to_string(path(root)) else {
        return Vec::new();
    };
    serde_json::from_str::<SeenFile>(&text)
        .ok()
        .filter(|f| f.version <= VERSION)
        .map(|f| f.entries)
        .unwrap_or_default()
}

pub(super) fn contains(root: &Path, hash: &str) -> bool {
    load(root).iter().any(|e| e.hash == hash)
}

/// Adds a hash, keeping the newest [`KEEP`].
pub(super) fn add(root: &Path, entry: Entry) -> CoreResult<()> {
    let mut entries = load(root);
    entries.retain(|e| e.hash != entry.hash);
    entries.push(entry);
    if entries.len() > KEEP {
        let drop = entries.len() - KEEP;
        entries.drain(..drop);
    }
    write_json(
        &path(root),
        &SeenFile {
            version: VERSION,
            entries,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(hash: &str) -> Entry {
        Entry {
            hash: hash.into(),
            job: Some("lamp-1".into()),
            at_ms: 1.0,
        }
    }

    #[test]
    fn the_same_bytes_hash_the_same_under_any_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.glb");
        let b = dir.path().join("b copy.glb");
        fs::write(&a, b"glTF same bytes").expect("a");
        fs::write(&b, b"glTF same bytes").expect("b");
        let c = dir.path().join("c.glb");
        fs::write(&c, b"glTF other bytes").expect("c");
        let ha = hash_file(&a).expect("hash");
        assert_eq!(ha.len(), 64);
        assert_eq!(ha, hash_file(&b).expect("hash"));
        assert_ne!(ha, hash_file(&c).expect("hash"));
    }

    #[test]
    fn seen_hashes_survive_a_reload_and_stay_bounded() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(!contains(dir.path(), "abc"));
        add(dir.path(), entry("abc")).expect("add");
        add(dir.path(), entry("abc")).expect("again");
        assert!(contains(dir.path(), "abc"));
        assert_eq!(load(dir.path()).len(), 1, "no duplicates");
        for n in 0..KEEP + 5 {
            add(dir.path(), entry(&format!("h{n}"))).expect("add");
        }
        let kept = load(dir.path());
        assert_eq!(kept.len(), KEEP);
        assert!(!contains(dir.path(), "abc"), "the oldest went first");
        assert!(contains(dir.path(), &format!("h{}", KEEP + 4)));
        fs::write(dir.path().join(SEEN_FILE), "not json").expect("corrupt");
        assert!(load(dir.path()).is_empty());
    }
}
