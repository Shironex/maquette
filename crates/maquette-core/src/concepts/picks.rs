//! Send picks to Refs: copying picked concepts into `refs/`, the Board
//! asset they become and the ref thumbnails that asset cards show.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{CONCEPTS_DIR, Direction, REFS_DIR, list_directions, load_direction, modify_direction};
use crate::assets::{self, Asset, AssetKind, NewAsset, Stage};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest};
use crate::workspace::FolderLayout;

/// What Send picks to Refs did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SentPicks {
    pub direction: Direction,
    pub asset: Asset,
    /// The new files in `refs/`, relative to the project folder.
    pub copied: Vec<String>,
    /// The project with the asset added or moved, for the Board.
    pub project: ProjectManifest,
}

/// Copies the picked images of a direction into `refs/` and registers the
/// asset of the same name on the Board: a new one at Refs, or an existing
/// one moved up to Refs when it is still at Concept.
pub fn send_picks(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    direction_id: &str,
    kind: AssetKind,
) -> CoreResult<SentPicks> {
    let project_dir = project::dir(workspace_root, layout, project_id)?;
    let current = load_direction(&project_dir, direction_id)?
        .ok_or_else(|| CoreError::NotFound(format!("the direction {direction_id}")))?;
    if current.refs_job.is_some() {
        return Err(CoreError::Invalid(
            "These images were made for an asset's refs. Use them from that asset's refs on the Board.".to_string(),
        ));
    }
    if !current.images.iter().any(|i| i.picked) {
        return Err(CoreError::Invalid(
            "Pick at least one concept to send to Refs.".to_string(),
        ));
    }
    let refs = project_dir.join(REFS_DIR);
    fs::create_dir_all(&refs).map_err(|e| CoreError::io(&refs, e))?;
    let (direction, copied) = modify_direction(&project_dir, direction_id, |d| {
        let mut copied = Vec::new();
        for image in d.images.iter_mut().filter(|i| i.picked) {
            let from = project_dir.join(&image.file);
            let Some(name) = from.file_name() else {
                continue;
            };
            let rel = format!("{REFS_DIR}/{}", name.to_string_lossy());
            let to = project_dir.join(&rel);
            fs::copy(&from, &to).map_err(|e| CoreError::io(&from, e))?;
            image.ref_file = Some(rel.clone());
            copied.push(rel);
        }
        Ok(copied)
    })?;
    let mut asset = None;
    let manifest = project::modify(workspace_root, layout, project_id, |manifest| {
        asset = Some(place_at_refs(manifest, &direction.name, kind)?);
        // The card shows the first pick until a model render replaces it.
        if let (Some(placed), Some(first)) = (asset.as_mut(), copied.first())
            && assets::set_ref_thumbnail(manifest, &placed.id, first)?
        {
            *placed = assets::find(manifest, &placed.id)?.clone();
        }
        Ok(())
    })?;
    let asset = asset.ok_or_else(|| CoreError::Unavailable("The asset was not saved.".into()))?;
    Ok(SentPicks {
        direction,
        asset,
        copied,
        project: manifest,
    })
}

/// `armillary sphere` reads `Armillary sphere`: only the first letter
/// changes, the rest stays as typed (`HUD panel`).
pub(super) fn capitalised(name: &str) -> String {
    let name = name.trim();
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// The asset called `name` at Refs: an existing one as it is (moved up
/// when it is still at Concept), else a new one of `kind`, its name with a
/// capital first letter like the ones made by hand (the id is the same
/// either way). Every way a pick reaches the Board goes through here.
pub(super) fn place_at_refs(
    manifest: &mut ProjectManifest,
    name: &str,
    kind: AssetKind,
) -> CoreResult<Asset> {
    match assets::find_by_name(manifest, name).cloned() {
        Some(existing) if existing.stage < Stage::Refs => {
            // A card images were dropped for only guessed its kind.
            if existing
                .thumbnail
                .as_deref()
                .is_some_and(|t| t.starts_with(&format!("{CONCEPTS_DIR}/")))
            {
                assets::find_mut(manifest, &existing.id)?.kind = kind;
            }
            assets::move_to(manifest, &existing.id, Stage::Refs)
        }
        Some(existing) => Ok(existing),
        None => assets::add(
            manifest,
            NewAsset {
                name: capitalised(name),
                kind,
                stage: Stage::Refs,
            },
        ),
    }
}

/// The newest ref copy of the direction named like `asset_id` that is
/// still on disk, relative to the project folder.
fn latest_sent_ref(
    project_dir: &Path,
    directions: &[Direction],
    manifest: &ProjectManifest,
    asset_id: &str,
) -> Option<String> {
    directions
        .iter()
        .filter(|d| assets::find_by_name(manifest, &d.name).is_some_and(|a| a.id == asset_id))
        .flat_map(|d| d.images.iter())
        .filter_map(|i| i.ref_file.as_ref().map(|r| (i.index, r)))
        .filter(|(_, r)| project_dir.join(r).is_file())
        .max_by_key(|(index, _)| *index)
        .map(|(_, r)| r.clone())
}

/// Gives every asset without a picture and without a model the newest ref
/// its direction sent, so a card made by an older Send picks to Refs shows
/// its image. Returns the saved project when anything changed.
pub fn heal_ref_thumbnails(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
) -> CoreResult<Option<ProjectManifest>> {
    let manifest = project::load(workspace_root, layout, project_id)?;
    let waiting = |a: &Asset| a.thumbnail.is_none() && a.versions.is_empty();
    if !manifest.assets.iter().any(waiting) {
        return Ok(None);
    }
    let project_dir = project::dir(workspace_root, layout, project_id)?;
    let directions = list_directions(&project_dir);
    let found: Vec<(String, String)> = manifest
        .assets
        .iter()
        .filter(|a| waiting(a))
        .filter_map(|a| {
            latest_sent_ref(&project_dir, &directions, &manifest, &a.id).map(|r| (a.id.clone(), r))
        })
        .collect();
    if found.is_empty() {
        return Ok(None);
    }
    let saved = project::modify(workspace_root, layout, project_id, |m| {
        for (id, file) in &found {
            // Checked again under the write lock: a render may have landed.
            if assets::find(m, id).is_ok_and(waiting) {
                assets::set_ref_thumbnail(m, id, file)?;
            }
        }
        Ok(())
    })?;
    Ok(Some(saved))
}

#[cfg(test)]
mod tests {
    use super::capitalised;

    #[test]
    fn only_the_first_letter_is_capitalised() {
        assert_eq!(capitalised("armillary sphere"), "Armillary sphere");
        assert_eq!(capitalised(" brazier "), "Brazier");
        assert_eq!(capitalised("HUD panel"), "HUD panel");
        assert_eq!(capitalised("ørsted lamp"), "Ørsted lamp");
        assert_eq!(capitalised("3d printer"), "3d printer");
        assert_eq!(capitalised(""), "");
    }
}
