//! The workspace trash: one folder per removed asset under
//! `<workspace>/trash/<project>/`, with its files under `files/` and
//! `removed.json` saying where they came from.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::links::{self, AssetLinks};
use super::{Place, RemoveOutcome, RemoveState, files};
use crate::assets::{Asset, AssetKind};
use crate::built;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::{self, ProjectManifest, now_rfc3339};
use crate::providers::ProviderJob;
use crate::workspace::{FolderLayout, is_single_segment};

/// The workspace folder removed assets go to.
pub const TRASH_DIR: &str = "trash";
/// The record in each trash folder.
pub(super) const RECORD_FILE: &str = "removed.json";
/// The folder in each trash folder that holds the files, at their project paths.
pub(super) const FILES_DIR: &str = "files";
const RECORD_VERSION: u32 = 1;

/// What the trash keeps of one removed asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrashRecord {
    pub version: u32,
    pub project: String,
    pub removed_at: String,
    /// The asset entry as `project.json` had it.
    pub asset: Asset,
    /// The moved files and folders, relative to the project folder; each is
    /// at the same path under `files/`.
    pub files: Vec<String>,
    #[serde(default)]
    pub links: AssetLinks,
    /// The sends stopped by the remove, by job id.
    #[serde(default)]
    pub stopped: Vec<String>,
}

/// One removed asset in the trash, newest first in [`list_trash`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrashEntry {
    /// The trash id `restore` takes.
    pub id: String,
    pub project: String,
    /// The asset's id and name when it was removed.
    pub asset: String,
    pub name: String,
    pub kind: AssetKind,
    pub versions: u32,
    pub removed_at: String,
    pub files: u32,
    /// Relative to the workspace, with `/` separators.
    pub path: String,
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// `<workspace>/trash/<project>`.
pub(super) fn project_trash(
    root: &Path,
    layout: &FolderLayout,
    project: &str,
) -> CoreResult<PathBuf> {
    if [&layout.projects, &layout.inbox, &layout.cache]
        .iter()
        .any(|f| f.as_str() == TRASH_DIR)
    {
        return Err(CoreError::Invalid(format!(
            "The workspace uses a folder called {TRASH_DIR} for something else, so there is no room for removed assets."
        )));
    }
    if !is_single_segment(project) {
        return Err(CoreError::NotFound(format!("the project {project}")));
    }
    Ok(root.join(TRASH_DIR).join(project))
}

/// The trash folder of `id`, which must be a plain folder name.
pub(super) fn entry_dir(
    root: &Path,
    layout: &FolderLayout,
    project: &str,
    id: &str,
) -> CoreResult<PathBuf> {
    if !is_single_segment(id) {
        return Err(CoreError::NotFound(format!("the removed asset {id}")));
    }
    Ok(project_trash(root, layout, project)?.join(id))
}

/// `clay-bowl-20261027-101530123`, or with `-2` when that folder is taken.
fn free_id(trash: &Path, asset: &str) -> String {
    let digits: String = now_rfc3339().chars().filter(char::is_ascii_digit).collect();
    let (date, time) = digits.split_at(digits.len().min(8));
    let base = format!("{asset}-{date}-{time}");
    if !trash.join(&base).exists() {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !trash.join(id).exists())
        .unwrap_or(base)
}

pub(super) fn read_record(dir: &Path) -> CoreResult<TrashRecord> {
    let file = dir.join(RECORD_FILE);
    let text = fs::read_to_string(&file).map_err(|err| match err.kind() {
        io::ErrorKind::NotFound => CoreError::NotFound(format!(
            "the removed asset {}",
            dir.file_name().unwrap_or_default().to_string_lossy()
        )),
        _ => CoreError::io(&file, err),
    })?;
    let record: TrashRecord = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if record.version > RECORD_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: record.version,
            supported: RECORD_VERSION,
        });
    }
    Ok(record)
}

/// Moves the asset's files to a new trash folder, writes its record, takes
/// it out of `project.json` and then out of the project's other files.
/// When the files or `project.json` cannot be changed, everything goes
/// back and the error says why.
pub(super) fn move_to_trash(
    place: &Place<'_>,
    manifest: &ProjectManifest,
    asset: &Asset,
    jobs: &[ProviderJob],
    stopped: &[String],
) -> CoreResult<RemoveOutcome> {
    let trash = project_trash(place.root, place.layout, place.project)?;
    let id = free_id(&trash, &asset.id);
    let dir = trash.join(&id);
    let built = built::load(&place.dir).unwrap_or_default();
    let paths = files::of_asset(&place.dir, manifest, asset, jobs, &built);
    let (links, mut warnings) = links::take(&place.dir, manifest, &asset.id);

    fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    let undo_dir = || {
        if let Err(err) = fs::remove_dir_all(&dir) {
            tracing::warn!(error = %err, "could not remove an unused trash folder");
        }
    };
    let files_dir = dir.join(FILES_DIR);
    if let Err(err) = files::move_all(&place.dir, &files_dir, &paths) {
        undo_dir();
        return Err(err);
    }
    let undo_files = || {
        if let Err(err) = files::move_all(&files_dir, &place.dir, &paths) {
            tracing::warn!(error = %err, "could not move the files back from the trash");
        }
    };
    let record = TrashRecord {
        version: RECORD_VERSION,
        project: place.project.to_string(),
        removed_at: now_rfc3339(),
        asset: asset.clone(),
        files: paths.clone(),
        links,
        stopped: stopped.to_vec(),
    };
    let written = write_json(&dir.join(RECORD_FILE), &record).and_then(|()| {
        project::modify(place.root, place.layout, place.project, |m| {
            m.assets.retain(|a| a.id != asset.id);
            if let Some(game) = m.game.as_mut() {
                game.models.remove(&asset.id);
            }
            Ok(())
        })
    });
    if let Err(err) = written {
        undo_files();
        undo_dir();
        return Err(err);
    }
    warnings.extend(links::clear(&place.dir, &asset.id, &record.links));

    Ok(RemoveOutcome {
        trash: Some(id.clone()),
        trash_path: Some(format!("{TRASH_DIR}/{}/{id}", place.project)),
        files: count(paths.len()),
        warnings,
        ..RemoveOutcome::new(&asset.id, Some(&asset.name), RemoveState::Removed)
    })
}

/// The assets removed from `project`, newest first. A folder without a
/// readable record is left out.
///
/// # Errors
///
/// Fails when there is no such project, or its trash folder exists but
/// cannot be read.
pub fn list_trash(
    root: &Path,
    layout: &FolderLayout,
    project: &str,
) -> CoreResult<Vec<TrashEntry>> {
    project::load(root, layout, project)?;
    let trash = project_trash(root, layout, project)?;
    let entries = match fs::read_dir(&trash) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&trash, err)),
    };
    let mut found: Vec<TrashEntry> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            let record = read_record(&e.path()).ok()?;
            Some(TrashEntry {
                path: format!("{TRASH_DIR}/{project}/{id}"),
                id,
                project: project.to_string(),
                asset: record.asset.id.clone(),
                name: record.asset.name.clone(),
                kind: record.asset.kind,
                versions: count(record.asset.versions.len()),
                removed_at: record.removed_at,
                files: count(record.files.len()),
            })
        })
        .collect();
    found.sort_by(|a, b| b.removed_at.cmp(&a.removed_at).then(b.id.cmp(&a.id)));
    Ok(found)
}
