//! Where each project's site or game repo is on this computer.
//!
//! A workspace can be shared between computers, and a repo path such as
//! `/Users/me/Projects/site` means nothing on the other one. So the path is
//! kept per computer: the shell holds the map in its app state (keyed by
//! workspace root, then project id) and hands it to [`install`] when a
//! workspace opens. `project.json` keeps only the repo's folder name
//! (`siteRepoName`), so another computer knows to ask for it.
//!
//! Older files carry an absolute `siteRepo`. [`resolve`] still reads it when
//! the folder is there, and [`migrate`] copies it into this computer's map
//! and drops it from the file. A path that is not on this computer is left
//! in the file untouched: it is the other computer's to move.
//!
//! The map lives in memory for the whole process, behind [`resolve`], so
//! every reader of a manifest sees this computer's path. Entries are keyed
//! by the workspace root, so tests with their own temp workspaces never see
//! each other's.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use super::{ProjectManifest, load_raw, modify};
use crate::error::CoreResult;
use crate::workspace::FolderLayout;

/// Whether this computer knows where a project's repo is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SiteRepoState {
    /// No repo was ever picked.
    #[default]
    None,
    /// This computer has it, and the folder is there.
    Here,
    /// This computer has a path, but the folder is not there any more.
    Gone,
    /// Another computer picked one; this one has not been told where it is.
    Elsewhere,
}

impl SiteRepoState {
    /// True when a repo was picked but this computer cannot reach it.
    pub fn is_missing(self) -> bool {
        matches!(self, Self::Gone | Self::Elsewhere)
    }
}

/// Project id to repo path, for one workspace.
pub type RepoMap = BTreeMap<String, String>;

static BOOK: RwLock<BTreeMap<String, RepoMap>> = RwLock::new(BTreeMap::new());

/// The map key for a workspace root.
pub fn root_key(root: &Path) -> String {
    let text = root.display().to_string();
    let trimmed = text.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        text
    } else {
        trimmed.to_string()
    }
}

fn read_book() -> std::sync::RwLockReadGuard<'static, BTreeMap<String, RepoMap>> {
    BOOK.read().unwrap_or_else(|p| p.into_inner())
}

fn write_book() -> std::sync::RwLockWriteGuard<'static, BTreeMap<String, RepoMap>> {
    BOOK.write().unwrap_or_else(|p| p.into_inner())
}

/// Replaces this computer's repos for the workspace at `root`.
pub fn install(root: &Path, repos: RepoMap) {
    write_book().insert(root_key(root), repos);
}

/// This computer's repos for the workspace at `root`.
pub fn entries(root: &Path) -> RepoMap {
    read_book()
        .get(&root_key(root))
        .cloned()
        .unwrap_or_default()
}

/// This computer's path for one project, if it has one.
pub fn get(root: &Path, id: &str) -> Option<String> {
    read_book()
        .get(&root_key(root))
        .and_then(|m| m.get(id))
        .cloned()
}

/// Sets (or with `None` forgets) this computer's path for one project.
/// The shell saves [`entries`] to its app state afterwards.
pub fn set(root: &Path, id: &str, path: Option<String>) {
    let mut book = write_book();
    let map = book.entry(root_key(root)).or_default();
    match path {
        Some(path) => map.insert(id.to_string(), path),
        None => map.remove(id),
    };
}

/// The last folder name of a repo path, on any computer's separators.
pub fn folder_name(path: &str) -> Option<String> {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
}

/// The one place a project's repo path is decided: this computer's map,
/// else an older file's absolute path when that folder is here.
pub fn resolve(
    root: &Path,
    id: &str,
    legacy: Option<&str>,
    name: Option<&str>,
) -> (Option<String>, SiteRepoState) {
    if let Some(path) = get(root, id) {
        let state = if Path::new(&path).is_dir() {
            SiteRepoState::Here
        } else {
            SiteRepoState::Gone
        };
        return (Some(path), state);
    }
    match legacy {
        Some(path) if Path::new(path).is_dir() => (Some(path.to_string()), SiteRepoState::Here),
        Some(_) => (None, SiteRepoState::Elsewhere),
        None if name.is_some() => (None, SiteRepoState::Elsewhere),
        None => (None, SiteRepoState::None),
    }
}

/// What `project.json` keeps of an older file's `siteRepo`: nothing once
/// this computer's map holds that same path, else the path as it was.
pub(super) fn legacy_to_keep(root: &Path, id: &str, legacy: Option<String>) -> Option<String> {
    let mine = get(root, id);
    legacy.filter(|l| mine.as_deref() != Some(l.as_str()))
}

/// Moves every older `siteRepo` whose folder is on this computer into the
/// map and out of `project.json`. Projects this computer already has a path
/// for, and paths that are not here, are left alone, so running it again
/// changes nothing. Returns the projects it moved.
///
/// # Errors
///
/// Fails when a project file cannot be written. Projects moved before
/// that stay moved.
pub fn migrate(root: &Path, layout: &FolderLayout) -> CoreResult<Vec<String>> {
    let mut moved = Vec::new();
    for id in super::ids(root, layout) {
        let Ok(raw) = load_raw(root, layout, &id) else {
            continue;
        };
        let Some(legacy) = raw.site_repo.clone() else {
            continue;
        };
        if get(root, &id).is_some() || !Path::new(&legacy).is_dir() {
            continue;
        }
        set(root, &id, Some(legacy.clone()));
        let named = |m: &mut ProjectManifest| {
            if m.site_repo_name.is_none() {
                m.site_repo_name = folder_name(&legacy);
            }
            Ok(())
        };
        modify(root, layout, &id, named)?;
        moved.push(id);
    }
    Ok(moved)
}

#[cfg(test)]
mod tests;
