//! Restore: a removed asset back on the Board from the trash, with its
//! files where they were and what the project's files said about it.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::trash::{FILES_DIR, entry_dir, read_record};
use super::{Busy, Place, files, links, refuse_while_running};
use crate::assets::{self, Asset};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest};
use crate::workspace::FolderLayout;

/// What a restore did, with the project as it is now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetRestored {
    pub project: ProjectManifest,
    pub asset: Asset,
    /// The files and folders moved back.
    pub files: u32,
    /// The sends the remove stopped. They stay stopped; send again to
    /// wait for a model.
    pub stopped: Vec<String>,
    /// What could not be put back; the asset is back all the same.
    pub warnings: Vec<String>,
}

/// Puts the removed asset `trash_id` back into the project `project_id`.
///
/// # Errors
///
/// Refuses, changing nothing, while a pipeline run is going in the
/// project, when the trash has no such asset, when an asset with its id or
/// name is on the Board again, or when one of its files is back in the
/// project (it would be overwritten).
pub fn restore(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    trash_id: &str,
    busy: &Busy<'_>,
) -> CoreResult<AssetRestored> {
    let place = Place::new(root, layout, project_id)?;
    let manifest = project::load(root, layout, project_id)?;
    refuse_while_running(&manifest, busy)?;
    let dir = entry_dir(root, layout, project_id, trash_id)?;
    let record = read_record(&dir)?;
    let asset = record.asset.clone();
    if assets::find(&manifest, &asset.id).is_ok()
        || assets::find_by_name(&manifest, &asset.name).is_some()
    {
        return Err(CoreError::Invalid(format!(
            "An asset called {} is on the Board again. Rename or remove it, then restore.",
            asset.name
        )));
    }
    if let Some(bad) = record.files.iter().find(|f| !files::is_plain(f)) {
        return Err(CoreError::Invalid(format!(
            "The trash record names {bad}, which is not a path inside the project, so nothing was restored."
        )));
    }
    let files_dir = dir.join(FILES_DIR);
    let paths: Vec<String> = record
        .files
        .iter()
        .filter(|f| files_dir.join(f).exists())
        .cloned()
        .collect();
    if let Some(taken) = paths.iter().find(|f| place.dir.join(f).exists()) {
        return Err(CoreError::Invalid(format!(
            "{taken} is in the project again, and restoring would write over it. Move it away, then restore."
        )));
    }
    files::move_all(&files_dir, &place.dir, &paths)?;
    let restored = project::modify(root, layout, project_id, |m| {
        if m.assets.iter().any(|a| a.id == asset.id) {
            return Err(CoreError::Invalid(format!(
                "An asset called {} is on the Board again.",
                asset.name
            )));
        }
        m.assets.push(asset.clone());
        if let (Some(game), Some(model)) = (m.game.as_mut(), record.links.game.as_ref())
            && !game.models.values().any(|g| g.file == model.file)
        {
            game.models.insert(asset.id.clone(), model.clone());
        }
        Ok(())
    });
    if let Err(err) = restored {
        if let Err(undo) = files::move_all(&place.dir, &files_dir, &paths) {
            tracing::warn!(error = %undo, "could not move restored files back to the trash");
        }
        return Err(err);
    }
    let mut warnings = links::put_back(&place.dir, &asset.id, &record.links);
    if let Err(err) = fs::remove_dir_all(&dir) {
        warnings.push(format!(
            "The asset is back, but its trash folder could not be removed. {err}"
        ));
    }
    if let Some(parent) = dir.parent() {
        // Only goes when it is empty.
        let _ = fs::remove_dir(parent);
    }
    let missing = record.files.len() - paths.len();
    if missing > 0 {
        warnings.push(format!(
            "{missing} of its files were no longer in the trash, so they did not come back."
        ));
    }
    Ok(AssetRestored {
        project: project::load(root, layout, project_id)?,
        asset,
        files: u32::try_from(paths.len()).unwrap_or(u32::MAX),
        stopped: record.stopped,
        warnings,
    })
}
