//! Importing a game's own models: each file picked from the model folder
//! becomes an asset whose version 1 is a copy of the repo file, and joins
//! the ship record so Ship may replace it later. The repo is only read.
//!
//! A `.glb` is copied to `models/<id>_v1.glb`. An `.obj` goes into a folder
//! of its own, `models/<id>_v1/`, with its `.mtl` files where its
//! `mtllib` lines expect them and its textures beside it, so Blender and the
//! viewer find them while the copies of the `.obj` and `.mtl` stay byte for
//! byte the repo's.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::scan::stats_of_file;
use super::ship::add_to_record;
use super::{GameFormat, GameModel, budget, obj, settings_of, stays_inside};
use crate::assets::{self, AssetKind, AssetStatus, NewAsset, Stage};
use crate::error::{CoreError, CoreResult};
use crate::inbox::display_name;
use crate::project::{self, ProjectManifest};
use crate::workspace::FolderLayout;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub project: ProjectManifest,
    /// The new assets, by id.
    pub imported: Vec<String>,
    /// Files left as they were, with the reason.
    pub skipped: Vec<SkippedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SkippedFile {
    pub file: String,
    pub reason: String,
}

fn copy(from: &Path, to: &Path) -> CoreResult<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    fs::copy(from, to).map_err(|e| CoreError::io(to, e))?;
    Ok(())
}

/// Copies an `.obj` with what it reads into `folder` and returns the
/// paths, relative to the model folder, of its `.mtl` files inside it.
fn copy_obj(source: &Path, folder: &Path, models_dir: &Path) -> CoreResult<Vec<String>> {
    let model = obj::read(source)?;
    let name = source.file_name().unwrap_or_default();
    copy(source, &folder.join(name))?;
    let obj_dir = source.parent().unwrap_or(Path::new("."));
    let mut owned = Vec::new();
    for mtl in &model.materials {
        let rel = mtl.strip_prefix(obj_dir).map_or_else(
            |_| PathBuf::from(mtl.file_name().unwrap_or_default()),
            Path::to_path_buf,
        );
        copy(mtl, &folder.join(&rel))?;
        if let Ok(inside) = mtl.strip_prefix(models_dir) {
            owned.push(inside.to_string_lossy().replace('\\', "/"));
        }
    }
    for texture in &model.textures {
        copy(
            texture,
            &folder.join(texture.file_name().unwrap_or_default()),
        )?;
    }
    Ok(owned)
}

/// Imports `files` (relative to the game's model folder) into the project.
/// Files already imported, missing or not a model are skipped and listed.
///
/// # Errors
///
/// Fails for a project that is not a game or has no game repo, and when a
/// file cannot be copied or `project.json` cannot be written.
pub fn import_models(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    files: &[String],
) -> CoreResult<ImportResult> {
    let manifest = project::load(root, layout, project_id)?;
    let settings = settings_of(&manifest)?.clone();
    let repo = manifest
        .site_repo
        .clone()
        .ok_or_else(|| CoreError::Invalid("Pick the game repo in Settings first.".to_string()))?;
    let game_dir = Path::new(&repo).join(&settings.models_dir);
    let project_dir = project::dir(root, layout, project_id)?;
    let mut imported = Vec::new();
    let mut skipped = Vec::new();
    let mut owned = Vec::new();
    let manifest = project::modify(root, layout, project_id, |manifest| {
        let game = manifest.game.as_mut().ok_or_else(|| {
            CoreError::Invalid(format!("{} is not a game project.", manifest.name))
        })?;
        let mut adds = Vec::new();
        for file in files {
            let skip = |reason: &str| SkippedFile {
                file: file.clone(),
                reason: reason.to_string(),
            };
            let source = game_dir.join(file);
            let Some(format) = GameFormat::of(file).filter(|_| stays_inside(file)) else {
                skipped.push(skip("not a .glb or .obj file in the model folder"));
                continue;
            };
            if game.models.values().any(|m| m.file == *file) {
                skipped.push(skip("already imported"));
                continue;
            }
            if !source.is_file() {
                skipped.push(skip("not found in the model folder"));
                continue;
            }
            adds.push((file.clone(), source, format));
        }
        for (file, source, format) in adds {
            let stats = stats_of_file(&source)?;
            let kind = if stats.joints > 0 {
                AssetKind::Character
            } else {
                AssetKind::Prop
            };
            let asset = assets::add(
                manifest,
                NewAsset {
                    name: display_name(&file),
                    kind,
                    stage: Stage::Ship,
                },
            )?;
            let base = format!("{}_v1", asset.id.replace('-', "_"));
            let version_file = match format {
                GameFormat::Glb => {
                    let rel = format!("models/{base}.glb");
                    copy(&source, &project_dir.join(&rel))?;
                    rel
                }
                GameFormat::Obj => {
                    let folder = project_dir.join("models").join(&base);
                    owned.extend(copy_obj(&source, &folder, &game_dir)?);
                    let name = source.file_name().unwrap_or_default().to_string_lossy();
                    format!("models/{base}/{name}")
                }
            };
            let game = manifest
                .game
                .as_mut()
                .ok_or_else(|| CoreError::Invalid("not a game project".to_string()))?;
            game.models.insert(
                asset.id.clone(),
                GameModel {
                    file: file.clone(),
                    imported_version: Some(1),
                },
            );
            let over = budget::budget_for(game, &file)
                .is_some_and(|b| budget::check_stats(b, &stats).over());
            assets::add_version(manifest, &asset.id, version_file, Some(stats))?;
            if over {
                assets::move_to(manifest, &asset.id, Stage::Blender)?;
                assets::set_status(manifest, &asset.id, AssetStatus::Review)?;
            } else {
                assets::set_status(manifest, &asset.id, AssetStatus::Done)?;
            }
            owned.push(file);
            imported.push(asset.id);
        }
        Ok(())
    })?;
    if !owned.is_empty() {
        add_to_record(&project_dir, &settings.models_dir, &owned)?;
    }
    Ok(ImportResult {
        project: manifest,
        imported,
        skipped,
    })
}
