//! Removing a whole project to the workspace trash and putting it back.
//!
//! A removed project waits in `<workspace>/trash/_projects/<id>-<time>/`:
//! its folder under `project/`, the asset and canvas trash it had under
//! `trash/` and `removed.json` saying what it was. Nothing is deleted, and
//! the site or game repo a project ships to is never touched: only the
//! project folder inside the workspace moves. The provider jobs record
//! (`providers.json`) keeps every send of the project, so a restore finds
//! them again.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{ProjectTemplate, load, projects_dir, repos};
use crate::assets::remove::{TRASH_DIR, project_trash};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::now_rfc3339;
use crate::settings::game::ProjectTarget;
use crate::workspace::{FolderLayout, is_single_segment};

/// The folder in the workspace trash that holds removed projects. Project
/// ids are slugs, which never hold `_`, so no project's own trash folder
/// can take this name.
pub const PROJECTS_TRASH: &str = "_projects";
const RECORD_FILE: &str = "removed.json";
const FOLDER: &str = "project";
const OWN_TRASH: &str = "trash";
const RECORD_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    version: u32,
    id: String,
    name: String,
    target: ProjectTarget,
    template: ProjectTemplate,
    removed_at: String,
    assets: u32,
    /// This computer's repo path, put back on restore when the map has none.
    #[serde(default)]
    site_repo: Option<String>,
    /// Whether the project's own trash (removed assets and canvases) came along.
    #[serde(default)]
    own_trash: bool,
}

/// One removed project in the trash, newest first in [`list_removed`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RemovedProject {
    /// The trash id [`restore_project`] takes.
    pub trash: String,
    /// The project's id and name when it was removed.
    pub id: String,
    pub name: String,
    pub target: ProjectTarget,
    pub removed_at: String,
    pub assets: u32,
    /// Relative to the workspace, with `/` separators.
    pub path: String,
}

/// Taken while a project folder is claimed: by a create picking its slug
/// and by a restore checking its id is free and moving the folder back.
static CLAIMS: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(super) fn claim_turn() -> std::sync::MutexGuard<'static, ()> {
    CLAIMS.lock().unwrap_or_else(|p| p.into_inner())
}

fn trash_root(root: &Path) -> PathBuf {
    root.join(TRASH_DIR).join(PROJECTS_TRASH)
}

fn entry(trash: &str, record: Record) -> RemovedProject {
    RemovedProject {
        path: format!("{TRASH_DIR}/{PROJECTS_TRASH}/{trash}"),
        trash: trash.to_string(),
        id: record.id,
        name: record.name,
        target: record.target,
        removed_at: record.removed_at,
        assets: record.assets,
    }
}

/// `moon-island-20261103-101530123`, or with `-2` when that folder is taken.
fn free_id(dir: &Path, id: &str) -> String {
    let digits: String = now_rfc3339().chars().filter(char::is_ascii_digit).collect();
    let (date, time) = digits.split_at(digits.len().min(8));
    let base = format!("{id}-{date}-{time}");
    if !dir.join(&base).exists() {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|t| !dir.join(t).exists())
        .unwrap_or(base)
}

/// A folder move that failed. `partial` says the folder was copied in full
/// to another volume but the source could not be fully removed: both stay,
/// and nothing that holds either may be deleted.
struct MoveFailed {
    error: CoreError,
    partial: bool,
}

impl From<MoveFailed> for CoreError {
    fn from(failed: MoveFailed) -> Self {
        failed.error
    }
}

/// Moves a folder with [`maquette_base::fs::move_tree`]: a rename tried
/// again on Windows while a file in it is held open, a copy only to another
/// volume, and never a deleted copy.
fn move_folder(from: &Path, to: &Path) -> Result<(), MoveFailed> {
    maquette_base::fs::move_tree(from, to).map_err(|err| {
        match maquette_base::fs::partial_move(&err) {
            Some(partial) => MoveFailed {
                error: CoreError::Invalid(format!("Nothing is lost: {partial}.")),
                partial: true,
            },
            None => MoveFailed {
                error: CoreError::io(from, err),
                partial: false,
            },
        }
    })
}

fn read_record(dir: &Path) -> CoreResult<Record> {
    let file = dir.join(RECORD_FILE);
    let text = fs::read_to_string(&file).map_err(|err| match err.kind() {
        io::ErrorKind::NotFound => CoreError::NotFound(format!(
            "the removed project {}",
            dir.file_name().unwrap_or_default().to_string_lossy()
        )),
        _ => CoreError::io(&file, err),
    })?;
    let record: Record = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if record.version > RECORD_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: record.version,
            supported: RECORD_VERSION,
        });
    }
    Ok(record)
}

/// Moves project `id` to the workspace trash with its own asset and canvas
/// trash. The caller checks first that nothing runs in it. When a move or
/// the record fails, what moved goes back and the error says why.
///
/// # Errors
///
/// Fails when there is no such project, or its folder cannot move (a file
/// held open by another app on Windows, for example).
pub fn remove_project(root: &Path, layout: &FolderLayout, id: &str) -> CoreResult<RemovedProject> {
    let manifest = load(root, layout, id)?;
    let from = projects_dir(root, layout).join(id);
    let own = project_trash(root, layout, id)?;
    let trash = trash_root(root);
    fs::create_dir_all(&trash).map_err(|e| CoreError::io(&trash, e))?;
    let name = free_id(&trash, id);
    let dir = trash.join(&name);
    fs::create_dir(&dir).map_err(|e| CoreError::io(&dir, e))?;
    let drop_dir = || {
        if let Err(err) = fs::remove_dir_all(&dir) {
            tracing::warn!(error = %err, "could not remove an unused trash folder");
        }
    };
    let record = Record {
        version: RECORD_VERSION,
        id: id.to_string(),
        name: manifest.name.clone(),
        target: manifest.target,
        template: manifest.template,
        removed_at: now_rfc3339(),
        assets: u32::try_from(manifest.assets.len()).unwrap_or(u32::MAX),
        site_repo: repos::get(root, id),
        own_trash: own.is_dir(),
    };
    if let Err(err) = write_json(&dir.join(RECORD_FILE), &record) {
        drop_dir();
        return Err(err);
    }
    if let Err(failed) = move_folder(&from, &dir.join(FOLDER)) {
        // A partial move left the whole project in the trash folder: keep it.
        if !failed.partial {
            drop_dir();
        }
        return Err(failed.error);
    }
    if record.own_trash
        && let Err(failed) = move_folder(&own, &dir.join(OWN_TRASH))
    {
        if failed.partial {
            // Its trash is whole in the removed project's folder; the
            // project stays removed and the error names what is left.
            return Err(failed.error);
        }
        let err = failed.error;
        if let Err(back) = move_folder(&dir.join(FOLDER), &from) {
            return Err(CoreError::Invalid(format!(
                "The project's trash could not move ({err}), and the project, already in the workspace trash, could not come back ({}). Use Restore in Removed projects.",
                back.error
            )));
        }
        drop_dir();
        return Err(err);
    }
    Ok(entry(&name, record))
}

/// The removed projects in the workspace trash, newest first. A folder
/// without a readable record is left out.
///
/// # Errors
///
/// Fails when the trash folder exists but cannot be read.
pub fn list_removed(root: &Path) -> CoreResult<Vec<RemovedProject>> {
    let trash = trash_root(root);
    let entries = match fs::read_dir(&trash) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&trash, err)),
    };
    let mut found: Vec<RemovedProject> = entries
        .flatten()
        .filter(|e| e.path().join(FOLDER).is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            read_record(&e.path()).ok().map(|r| entry(&name, r))
        })
        .collect();
    found.sort_by(|a, b| b.removed_at.cmp(&a.removed_at).then(b.trash.cmp(&a.trash)));
    Ok(found)
}

/// Whether project `id` waits in the workspace trash.
pub fn is_removed(root: &Path, id: &str) -> bool {
    list_removed(root).is_ok_and(|all| all.iter().any(|p| p.id == id))
}

/// What [`restore_project`] put back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    pub id: String,
    /// True when the repo path the project had was written back. Nobody
    /// confirmed it just now, so the caller marks it as set by an agent:
    /// its dev command waits for a person again.
    pub repo_put_back: bool,
}

/// Puts the removed project `trash` back where it was, with its own trash.
/// Refused when a project with its id is there again: nothing is written
/// over. The check and the move hold the claim turn a create takes, so a
/// new project cannot take the folder in between.
///
/// # Errors
///
/// Fails when there is no such removed project, its id is taken, or a
/// folder cannot move back.
pub fn restore_project(root: &Path, layout: &FolderLayout, trash: &str) -> CoreResult<Restored> {
    if !is_single_segment(trash) {
        return Err(CoreError::NotFound(format!("the removed project {trash}")));
    }
    let dir = trash_root(root).join(trash);
    let record = read_record(&dir)?;
    let to = super::dir(root, layout, &record.id)?;
    let own = project_trash(root, layout, &record.id)?;
    let claim = claim_turn();
    if to.exists() {
        return Err(CoreError::Invalid(format!(
            "A project with the id {} is in the workspace again, so {} stays in the trash. Remove that project first to restore this one.",
            record.id, record.name
        )));
    }
    let own_from = dir.join(OWN_TRASH);
    let own_back = record.own_trash && own_from.is_dir();
    if own_back && own.exists() {
        return Err(CoreError::Invalid(format!(
            "The workspace trash already has a folder for {}, so its removed assets and canvases would mix. Move trash/{} aside, then restore.",
            record.id, record.id
        )));
    }
    let parent = projects_dir(root, layout);
    fs::create_dir_all(&parent).map_err(|e| CoreError::io(&parent, e))?;
    move_folder(&dir.join(FOLDER), &to)?;
    drop(claim);
    // The folder is kept when the project's own trash could not come back,
    // so nothing in it is lost; it no longer lists as a removed project.
    let kept = own_back
        && move_folder(&own_from, &own)
            .inspect_err(|err| {
                tracing::warn!(error = %err.error, "the project's own trash stayed in the removed project's folder");
            })
            .is_err();
    let repo_put_back = match record.site_repo {
        Some(path) if repos::get(root, &record.id).is_none() => {
            repos::set(root, &record.id, Some(path));
            true
        }
        _ => false,
    };
    if !kept && let Err(err) = fs::remove_dir_all(&dir) {
        tracing::warn!(error = %err, "could not clear a restored project's trash folder");
    }
    Ok(Restored {
        id: record.id,
        repo_put_back,
    })
}

/// The error for a project id that is not in the workspace: it names the
/// trash when the project waits there.
pub(super) fn missing(root: &Path, id: &str) -> CoreError {
    if is_removed(root, id) {
        CoreError::NotFound(format!(
            "the project {id}: it was removed to the workspace trash, and only a person can restore it, from Removed projects in the Maquette app"
        ))
    } else {
        CoreError::NotFound(format!("the project {id}"))
    }
}

#[cfg(test)]
mod tests;
