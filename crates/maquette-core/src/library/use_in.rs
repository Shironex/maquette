//! Use in project: copies a library version into a project as a new asset
//! that records where it came from. The project owns the copy.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{LibraryEntry, LibraryVersion, entry_dir, files, load};
use crate::assets::copy::{free_name, key};
use crate::assets::{Asset, AssetOrigin, AssetStatus, AssetVersion, Stage};
use crate::error::{CoreError, CoreResult};
use crate::game::{self, GameFormat, ProjectTarget};
use crate::project::{self, ProjectManifest, now_rfc3339};
use crate::workspace::FolderLayout;
use crate::{materials, prepare};

/// What Use in project did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryUsed {
    /// The new asset.
    pub asset: Asset,
    pub project: ProjectManifest,
    /// What the project's target does with it, in a sentence.
    pub note: Option<String>,
    /// True when its real size, fix-ups or material tweaks came along.
    pub settings: bool,
}

/// What the project's target makes of a model file, said plainly: a game
/// exports it at its next run, in its own format. A website builds its
/// scene from glTF only, so an `.obj` is refused.
///
/// # Errors
///
/// Refuses a model the project cannot use.
pub(super) fn target_note(
    manifest: &ProjectManifest,
    asset: &Asset,
    file: &str,
) -> CoreResult<Option<String>> {
    let format = GameFormat::of(file);
    let ext = Path::new(file)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match manifest.target {
        ProjectTarget::Website if format != Some(GameFormat::Glb) => {
            Err(CoreError::Invalid(format!(
                "{} builds its scene from .glb models, and this one is .{ext}. \
                 Use it in a game project, or add a .glb of it to the library.",
                manifest.name
            )))
        }
        ProjectTarget::Website => Ok(None),
        ProjectTarget::Game => {
            let settings = game::settings_of(manifest)?;
            let target = game::file_of(settings, asset);
            let wanted = settings.format.ext();
            Ok(Some(if format == Some(settings.format) {
                format!("The next run exports it to {target} in the game.")
            } else {
                format!(
                    "This game takes .{wanted} and the model is .{ext}, so the next run's \
                     Game export writes it as .{wanted} to {target}. Nothing is converted now."
                )
            }))
        }
    }
}

fn pick(entry: &LibraryEntry, version: Option<u32>) -> CoreResult<&LibraryVersion> {
    match version {
        Some(n) => entry.find(n).ok_or_else(|| {
            CoreError::NotFound(format!("version {n} of the library entry {}", entry.id))
        }),
        None => entry.latest().ok_or_else(|| {
            CoreError::Invalid(format!("The library entry {} has no model.", entry.name))
        }),
    }
}

/// Copies version `version` (the newest when `None`) of library entry
/// `entry_id` into project `project_id` as a new asset at Blender, with its
/// picture, refs, real size, fix-ups and material tweaks.
///
/// # Errors
///
/// Fails for an unknown entry, version or project, a model the project's
/// target cannot use, or a file that cannot be copied.
pub fn use_in_project(
    root: &Path,
    layout: &FolderLayout,
    entry_id: &str,
    version: Option<u32>,
    project_id: &str,
) -> CoreResult<LibraryUsed> {
    let entry = load(root, entry_id)?;
    let chosen = pick(&entry, version)?.clone();
    let manifest = project::load(root, layout, project_id)?;
    let project_dir = project::dir(root, layout, project_id)?;
    let from = entry_dir(root, &entry.id)?;
    let now = now_rfc3339();
    let draft = Asset {
        id: entry.id.clone(),
        name: entry.name.clone(),
        kind: entry.kind,
        stage: Stage::Blender,
        status: AssetStatus::Done,
        versions: vec![AssetVersion {
            version: 1,
            file: chosen.file.clone(),
            created_at: now.clone(),
            stats: chosen.stats.clone(),
        }],
        thumbnail: None,
        created_at: now.clone(),
        updated_at: now,
        origin: Some(AssetOrigin {
            entry: entry.id.clone(),
            version: chosen.version,
        }),
    };
    let (name, id) = free_name(&manifest, &project_dir, &draft);
    let mut asset = Asset { id, name, ..draft };
    let note = target_note(&manifest, &asset, &chosen.file)?;

    let asset_key = key(&asset.id);
    let file = files::copy_version(&from, &chosen.file, &project_dir, &asset_key, 1)?;
    let placed = (|| {
        files::copy_refs(&from, &entry.refs, &project_dir)?;
        let picture = chosen
            .thumbnail
            .as_deref()
            .map(|p| files::copy_picture(&from, p, &project_dir, &asset_key, 1))
            .transpose()?
            .flatten();
        asset.thumbnail = picture;
        asset.versions[0].file = file.clone();
        let placed = asset.clone();
        project::modify(root, layout, project_id, |m| {
            if m.assets.iter().any(|a| a.id == placed.id) {
                return Err(CoreError::Invalid(format!(
                    "{} already has an asset called {}.",
                    m.name, placed.name
                )));
            }
            m.assets.push(placed);
            Ok(())
        })
    })();
    let project = match placed {
        Ok(project) => project,
        Err(err) => {
            files::remove_version(&project_dir, &file);
            return Err(err);
        }
    };
    let settings = bring_settings(&project_dir, &asset.id, &chosen);
    Ok(LibraryUsed {
        asset,
        project,
        note,
        settings,
    })
}

/// Saves the version's real size, fix-ups and material tweaks for the new
/// asset. A failure leaves the asset in place and is logged: the person can
/// set them again in the Inspector.
fn bring_settings(project_dir: &Path, asset: &str, chosen: &LibraryVersion) -> bool {
    let mut brought = false;
    if let Some(prep) = chosen.prep.as_ref().filter(|p| !p.is_empty()) {
        match prepare::save_entry(project_dir, asset, prep) {
            Ok(_) => brought = true,
            Err(err) => tracing::warn!(asset, error = %err, "could not save the library fix-ups"),
        }
    }
    if let Some(look) = chosen.materials.as_ref().filter(|m| !m.is_empty()) {
        let saved = materials::update(project_dir, |current| {
            let mut next = current.clone();
            next.models.insert(asset.to_string(), look.clone());
            Ok(next)
        });
        match saved {
            Ok(_) => brought = true,
            Err(err) => tracing::warn!(asset, error = %err, "could not save the library tweaks"),
        }
    }
    brought
}
