//! Files the watcher already took or was told to leave, by content hash, so
//! the same download is never imported twice: not when the browser touches
//! it again, not when it is saved a second time under another name, and not
//! after a restart. Kept per computer in `downloads-seen.json` in the app's
//! data folder, since every computer has its own Downloads. Older versions
//! kept it in the workspace root; [`adopt`] moves that file over once.

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

fn path(dir: &Path) -> PathBuf {
    dir.join(SEEN_FILE)
}

/// The SHA-256 of a file, as the pipeline cache hashes it.
pub(super) fn hash_file(file: &Path) -> CoreResult<String> {
    maquette_base::hash::hash_file(file).map_err(|e| CoreError::io(file, e))
}

/// Every hash seen, from the file in `dir`. A missing or unreadable file
/// is an empty list.
pub(super) fn load(dir: &Path) -> Vec<Entry> {
    let Ok(text) = fs::read_to_string(path(dir)) else {
        return Vec::new();
    };
    serde_json::from_str::<SeenFile>(&text)
        .ok()
        .filter(|f| f.version <= VERSION)
        .map(|f| f.entries)
        .unwrap_or_default()
}

pub(super) fn contains(dir: &Path, hash: &str) -> bool {
    load(dir).iter().any(|e| e.hash == hash)
}

/// Adds a hash, keeping the newest [`KEEP`].
pub(super) fn add(dir: &Path, entry: Entry) -> CoreResult<()> {
    let mut entries = load(dir);
    entries.retain(|e| e.hash != entry.hash);
    entries.push(entry);
    save(dir, entries)
}

fn save(dir: &Path, mut entries: Vec<Entry>) -> CoreResult<()> {
    if entries.len() > KEEP {
        let drop = entries.len() - KEEP;
        entries.drain(..drop);
    }
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    write_json(
        &path(dir),
        &SeenFile {
            version: VERSION,
            entries,
        },
    )
}

/// Moves an older workspace's `downloads-seen.json` at `root` into the one
/// in `dir`: its hashes join this computer's list (oldest first, without
/// duplicates), and only once that is saved is the workspace copy removed.
/// Without the file it does nothing, so it can run on every look.
pub(super) fn adopt(root: &Path, dir: &Path) -> CoreResult<()> {
    let old = path(root);
    if !old.is_file() || root == dir {
        return Ok(());
    }
    let mut entries = load(root);
    entries.sort_by(|a, b| a.at_ms.total_cmp(&b.at_ms));
    let mut merged = Vec::new();
    for entry in entries.into_iter().chain(load(dir)) {
        merged.retain(|e: &Entry| e.hash != entry.hash);
        merged.push(entry);
    }
    save(dir, merged)?;
    match fs::remove_file(&old) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(CoreError::io(&old, err)),
    }
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

    #[test]
    fn an_older_workspace_list_moves_to_the_app_data_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("ws");
        let data = dir.path().join("app-data");
        fs::create_dir_all(&root).expect("ws");
        // The workspace file as an older Maquette wrote it.
        fs::write(
            root.join(SEEN_FILE),
            r#"{"version":1,"entries":[{"hash":"old1","job":"lamp-1","atMs":5.0},{"hash":"both","job":null,"atMs":6.0}]}"#,
        )
        .expect("old file");
        add(&data, entry("both")).expect("add");
        add(&data, entry("mine")).expect("add");

        adopt(&root, &data).expect("adopt");
        assert!(!root.join(SEEN_FILE).exists(), "the workspace copy went");
        let hashes: Vec<String> = load(&data).into_iter().map(|e| e.hash).collect();
        assert_eq!(hashes, ["old1", "both", "mine"]);
        assert!(contains(&data, "old1"));

        adopt(&root, &data).expect("again");
        assert_eq!(load(&data).len(), 3, "nothing changes the second time");
    }

    #[test]
    fn a_failed_move_keeps_the_workspace_copy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("ws");
        fs::create_dir_all(&root).expect("ws");
        fs::write(root.join(SEEN_FILE), r#"{"version":1,"entries":[]}"#).expect("old");
        // The app data folder cannot be made: a file stands where it goes.
        let blocked = dir.path().join("blocked");
        fs::write(&blocked, "file").expect("file");
        assert!(adopt(&root, &blocked.join("data")).is_err());
        assert!(
            root.join(SEEN_FILE).is_file(),
            "kept until the copy is saved"
        );
    }
}
