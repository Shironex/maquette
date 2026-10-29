//! Living next to a file sync tool. A workspace may be shared between
//! computers with Syncthing, which keeps its own files in the folder
//! (`.stfolder`, `.stversions`, `.stignore`), writes temp files while a file
//! arrives and, when two computers change the same file, keeps the loser as
//! `<name>.sync-conflict-<date>-<time>-<device>.<ext>`. None of these are
//! Maquette's: every scan skips them, and a conflict copy of a data file is
//! reported so a person can choose, never merged.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{WorkspaceConfig, lock::LOCK_FILE};

/// Syncthing's own entries in a synced folder.
const SYNC_ENTRIES: [&str; 4] = [".stfolder", ".stversions", ".stignore", ".stglobalignore"];
const CONFLICT_MARK: &str = ".sync-conflict-";
/// How deep the conflict scan looks: `projects/<id>/handoff/<job>/send.json`
/// is four folders down.
const CONFLICT_DEPTH: usize = 5;
/// Files Maquette keeps its data in, by extension.
const DATA_EXTENSIONS: [&str; 2] = ["json", "jsonl"];

/// True for a name a sync tool owns: its folders, a file on its way in
/// (`.syncthing.<name>.tmp`, or `~syncthing~<name>.tmp` on Windows), a
/// conflict copy, and the workspace lock. Scans skip these.
pub fn is_sync_name(name: &str) -> bool {
    if SYNC_ENTRIES.contains(&name) || name == LOCK_FILE {
        return true;
    }
    let temp = (name.starts_with(".syncthing.") || name.starts_with("~syncthing~"))
        && name.to_ascii_lowercase().ends_with(".tmp");
    temp || name.contains(CONFLICT_MARK)
}

/// [`is_sync_name`] for the last segment of `path`.
pub fn is_sync_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(is_sync_name)
}

/// The file a conflict copy stands for: `project.sync-conflict-20261027-101500-ABC1234.json`
/// gives `project.json`. `None` when `name` is not a conflict copy.
pub fn conflict_original(name: &str) -> Option<String> {
    let at = name.find(CONFLICT_MARK)?;
    let rest = &name[at + CONFLICT_MARK.len()..];
    let tail = rest.find('.').map_or("", |dot| &rest[dot..]);
    Some(format!("{}{tail}", &name[..at]))
}

/// A conflict copy of one of Maquette's data files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflict {
    /// The copy, relative to the workspace, with `/` between folders.
    pub path: String,
    /// The file it is a copy of, relative to the workspace.
    pub original: String,
}

/// Every conflict copy of a JSON data file in the workspace (the config,
/// the ledger, projects, sends, library entries), sorted by path. The cache
/// and Syncthing's own folders are not looked in. A conflict copy of the
/// workspace lock is only an old heartbeat, so it is removed, not reported.
pub fn conflicts(root: &Path, config: &WorkspaceConfig) -> Vec<SyncConflict> {
    let mut found = Vec::new();
    walk(root, root, &config.layout.cache, 0, &mut found);
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

fn walk(root: &Path, dir: &Path, cache: &str, depth: usize, found: &mut Vec<SyncConflict>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            let skip = SYNC_ENTRIES.contains(&name.as_str()) || (depth == 0 && name == cache);
            if !skip && depth + 1 < CONFLICT_DEPTH {
                walk(root, &path, cache, depth + 1, found);
            }
            continue;
        }
        let Some(original) = conflict_original(&name) else {
            continue;
        };
        if depth == 0 && is_lock_conflict(&name) {
            // Best effort: if it stays, the next scan tries again.
            let _ = fs::remove_file(&path);
            continue;
        }
        if !is_data_file(&original) {
            continue;
        }
        let rel = |p: &Path| relative(root, p);
        found.push(SyncConflict {
            path: rel(&path),
            original: rel(&path.with_file_name(&original)),
        });
    }
}

/// True for a sync conflict copy of the workspace lock
/// (`.maquette-lock.sync-conflict-<date>-<time>-<device>.json`).
pub fn is_lock_conflict(name: &str) -> bool {
    conflict_original(name).is_some_and(|original| original == LOCK_FILE)
}

/// Removes every conflict copy of the workspace lock in the workspace root,
/// quietly: a copy is only an old heartbeat, and one that stays is skipped by
/// every scan and tried again next time.
pub fn remove_lock_conflicts(root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let is_copy = entry.file_name().to_str().is_some_and(is_lock_conflict);
        if is_copy && entry.file_type().is_ok_and(|kind| kind.is_file()) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn is_data_file(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| DATA_EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

fn relative(root: &Path, path: &Path) -> String {
    let rel: PathBuf = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_names_are_recognised() {
        for name in [
            ".stfolder",
            ".stversions",
            ".stignore",
            ".syncthing.lamp.glb.tmp",
            "~syncthing~lamp.glb.tmp",
            "lamp.sync-conflict-20261027-101500-ABC1234.glb",
            ".maquette-lock.json",
        ] {
            assert!(is_sync_name(name), "{name}");
        }
        for name in [
            "lamp.glb",
            "project.json",
            ".syncthing",
            "stfolder",
            "notes.tmp",
        ] {
            assert!(!is_sync_name(name), "{name}");
        }
        assert!(is_sync_path(Path::new("inbox/.syncthing.a.glb.tmp")));
    }

    #[test]
    fn scans_skip_sync_files() {
        use crate::inbox::watch::is_model;
        assert!(is_model(Path::new("inbox/lamp.glb")));
        assert!(!is_model(Path::new(
            "inbox/lamp.sync-conflict-20261027-101500-ABC1234.glb"
        )));
        assert!(!is_model(Path::new("inbox/~syncthing~lamp.glb.tmp")));

        // A folder Syncthing made, holding only its marker, is still empty.
        let dir = tempfile::tempdir().expect("tempdir");
        fs::create_dir(dir.path().join(".stfolder")).expect("marker");
        fs::write(dir.path().join(".stignore"), "cache/\n").expect("ignore");
        let found = super::super::inspect_root(dir.path(), None).expect("inspect");
        assert_eq!(found.state, super::super::RootState::Empty);

        // A project folder that is a conflict leftover is not a project.
        let layout = super::super::FolderLayout::default();
        let copy = dir.path().join("projects/.syncthing.studio.tmp");
        fs::create_dir_all(&copy).expect("dir");
        fs::write(copy.join("project.json"), "{}").expect("write");
        assert!(
            crate::project::list(dir.path(), &layout)
                .expect("list")
                .is_empty()
        );
    }

    #[test]
    fn a_conflict_copy_names_its_original() {
        assert_eq!(
            conflict_original("project.sync-conflict-20261027-101500-ABC1234.json").as_deref(),
            Some("project.json")
        );
        assert_eq!(
            conflict_original("credits.sync-conflict-20261027-101500-ABC1234.jsonl").as_deref(),
            Some("credits.jsonl")
        );
        assert_eq!(
            conflict_original("README.sync-conflict-20261027-101500-ABC1234").as_deref(),
            Some("README")
        );
        assert_eq!(conflict_original("project.json"), None);
    }

    #[test]
    fn conflicts_lists_data_copies_and_skips_the_cache_and_models() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let stamp = "sync-conflict-20261027-101500-ABC1234";
        let write = |rel: &str| {
            let path = root.join(rel);
            fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
            fs::write(path, b"{}").expect("write");
        };
        write(&format!("maquette.{stamp}.json"));
        write(&format!("projects/studio/project.{stamp}.json"));
        write(&format!("projects/studio/handoff/lamp-1/send.{stamp}.json"));
        write(&format!("library/lamp/entry.{stamp}.json"));
        write(&format!("projects/studio/models/lamp_v1.{stamp}.glb"));
        write(&format!("cache/inbox.{stamp}.json"));
        write(&format!(".maquette-lock.{stamp}.json"));
        write(&format!(".stversions/project.{stamp}.json"));
        write("projects/studio/project.json");
        let found = conflicts(root, &WorkspaceConfig::default());
        let paths: Vec<&str> = found.iter().map(|c| c.original.as_str()).collect();
        assert_eq!(
            paths,
            [
                "library/lamp/entry.json",
                "maquette.json",
                "projects/studio/handoff/lamp-1/send.json",
                "projects/studio/project.json",
            ]
        );
        assert!(found[3].path.ends_with(&format!("project.{stamp}.json")));
        assert!(!root.join(format!(".maquette-lock.{stamp}.json")).exists());
    }

    #[test]
    fn lock_conflict_copies_are_recognised_and_removed_alone() {
        let stamp = "sync-conflict-20261027-101500-ABC1234";
        assert!(is_lock_conflict(&format!(".maquette-lock.{stamp}.json")));
        assert!(!is_lock_conflict(LOCK_FILE));
        assert!(!is_lock_conflict(&format!("maquette.{stamp}.json")));

        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let copy = root.join(format!(".maquette-lock.{stamp}.json"));
        let other = root.join(".maquette-lock.sync-conflict-20261028-090000-XYZ9876.json");
        let data = root.join(format!("maquette.{stamp}.json"));
        for path in [&copy, &other, &data] {
            fs::write(path, b"{}").expect("write");
        }
        fs::write(root.join(LOCK_FILE), b"{}").expect("lock");
        remove_lock_conflicts(root);
        assert!(!copy.exists() && !other.exists(), "lock copies go");
        assert!(data.exists(), "a data copy stays for the person");
        assert!(root.join(LOCK_FILE).exists(), "the lock itself stays");
    }
}
