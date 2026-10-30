//! Removing entries from the library and putting them back. A removed
//! entry's folder moves to `library/.removed/<id>/`, which the library
//! list skips, so restoring it is a move back. Projects are never touched:
//! their copies stay, and their origins name an entry no longer listed
//! until it comes back.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::merge_undo::{self, MERGE_RECORD};
use super::tags::unique;
use super::{ENTRY_FILE, dir, entry_dir, load, turn};
use crate::error::{CoreError, CoreResult};
use crate::project::ProjectManifest;
use crate::workspace::FolderLayout;

/// The folder under `library/` removed entries wait in.
pub const REMOVED_DIR: &str = ".removed";

/// What a restore did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRestored {
    pub entries: Vec<String>,
    /// Projects whose asset origins a restore changed (an undone merge).
    pub projects: Vec<ProjectManifest>,
}

/// Where removed entry `id` waits.
pub(super) fn removed_dir(root: &Path, id: &str) -> CoreResult<PathBuf> {
    entry_dir(root, id)?;
    Ok(dir(root).join(REMOVED_DIR).join(id))
}

/// Moves `from` to `to`, replacing what an older remove left at `to`.
pub(super) fn move_folder(from: &Path, to: &Path) -> CoreResult<()> {
    if to.exists() {
        fs::remove_dir_all(to).map_err(|e| CoreError::io(to, e))?;
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    fs::rename(from, to).map_err(|e| CoreError::io(from, e))
}

/// Moves entries out of the library. Answers the ids moved.
///
/// # Errors
///
/// Fails for an unknown entry before moving any, or when a folder cannot
/// be moved.
pub fn remove(root: &Path, entries: &[String]) -> CoreResult<Vec<String>> {
    if entries.is_empty() {
        return Err(CoreError::Invalid("Pick at least one model.".to_string()));
    }
    let _turn = turn();
    let ids = unique(entries);
    for id in &ids {
        load(root, id)?;
    }
    for id in &ids {
        move_folder(&entry_dir(root, id)?, &removed_dir(root, id)?)?;
    }
    Ok(ids)
}

/// Puts removed entries back. One that a merge removed is put back by
/// undoing the merge: the versions it added to the kept entry go, and the
/// project assets it pointed at the kept entry point back.
///
/// # Errors
///
/// Fails, before moving any, for an id that was not removed, one whose
/// folder is in use again, or a merge that can no longer be undone.
pub fn restore(
    root: &Path,
    layout: &FolderLayout,
    entries: &[String],
) -> CoreResult<LibraryRestored> {
    if entries.is_empty() {
        return Err(CoreError::Invalid("Pick at least one model.".to_string()));
    }
    let _turn = turn();
    let ids = unique(entries);
    for id in &ids {
        let waiting = removed_dir(root, id)?;
        if !waiting.join(ENTRY_FILE).is_file() {
            return Err(CoreError::NotFound(format!(
                "the removed library entry {id}"
            )));
        }
        if entry_dir(root, id)?.exists() {
            return Err(CoreError::Invalid(format!(
                "The library has a {id} folder again. Move it away, then restore."
            )));
        }
        if waiting.join(MERGE_RECORD).is_file() {
            merge_undo::check(root, layout, id)?;
        }
    }
    let mut projects: Vec<ProjectManifest> = Vec::new();
    for id in &ids {
        let waiting = removed_dir(root, id)?;
        if waiting.join(MERGE_RECORD).is_file() {
            for changed in merge_undo::undo(root, layout, id)? {
                projects.retain(|p| p.id != changed.id);
                projects.push(changed);
            }
        } else {
            move_folder(&waiting, &entry_dir(root, id)?)?;
        }
    }
    Ok(LibraryRestored {
        entries: ids,
        projects,
    })
}
