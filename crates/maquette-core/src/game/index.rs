//! The game index: what the game pipeline's steps read about each model.
//! Maquette writes it to `web/game-index.json` before every run, and it is
//! an input of the steps that read it, so a change to a model's place, the
//! format or the texture settings reruns them.
//!
//! ```json
//! {
//!   "version": 1,
//!   "format": "glb",
//!   "textures": "png",
//!   "textureCap": 1024,
//!   "models": [
//!     { "asset": "brazier", "name": "Brazier", "source": "models/brazier_v2.glb",
//!       "target": "props/brazier.glb", "asIs": false, "copy": [], "textureCap": 1024 }
//!   ]
//! }
//! ```
//!
//! A model's `textureCap` is the texture size of its folder's budget, so a
//! folder that allows 2048 px keeps 2048 px textures; the top level
//! `textureCap` (the Game export setting) is for folders with no budget.
//!
//! `asIs` marks a model whose newest version is still the repo file it was
//! imported from, with no fix-ups or real size: Build models leaves it out
//! and Game export copies the files in `copy` (relative to the project) to
//! the target's folder untouched, so Ship finds it unchanged.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{GameFormat, GameSettings, GameTextures, budget, file_of, settings_of};
use crate::error::CoreResult;
use crate::json::write_json;
use crate::prepare::{self, ScenePrep};
use crate::project::ProjectManifest;

pub const INDEX_FILE: &str = "web/game-index.json";
pub const INDEX_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexModel {
    pub asset: String,
    pub name: String,
    /// The newest version's file, relative to the project.
    pub source: String,
    /// Where it goes in the model folder.
    pub target: String,
    pub as_is: bool,
    /// For an `asIs` model, the files to copy as they are.
    pub copy: Vec<String>,
    /// The longest texture edge Export writes for it: its folder's budget,
    /// or the Game export cap for a folder with no budget.
    pub texture_cap: u32,
    /// Clips the person left out of the game file; Export drops them, so
    /// such a model is never copied as it is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clips_off: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameIndex {
    pub version: u32,
    pub format: GameFormat,
    pub textures: GameTextures,
    pub texture_cap: u32,
    /// The export leaves normal and metal-roughness maps out.
    #[serde(default)]
    pub drop_maps: bool,
    pub models: Vec<IndexModel>,
}

/// The files an untouched import copies: the model, and for an `.obj` the
/// `.mtl` files next to it in its version folder.
fn as_is_files(project_dir: &Path, source: &str) -> Vec<String> {
    if GameFormat::of(source) != Some(GameFormat::Obj) {
        return vec![source.to_string()];
    }
    let Some((folder, _)) = source.rsplit_once('/') else {
        return vec![source.to_string()];
    };
    let mut files = vec![source.to_string()];
    files.extend(
        super::ship::files_under(&project_dir.join(folder))
            .into_iter()
            .filter(|f| f.to_lowercase().ends_with(".mtl"))
            .map(|f| format!("{folder}/{f}")),
    );
    files
}

/// The longest texture edge Game export writes for a model at `target`:
/// the texture size of its folder's budget, or the Game export cap when
/// the folder has none.
pub fn texture_cap(settings: &GameSettings, target: &str) -> u32 {
    budget::budget_for(settings, target).map_or(settings.export.texture_cap, |b| b.texture)
}

/// The index for the project as it is now.
pub fn build(
    project_dir: &Path,
    manifest: &ProjectManifest,
    settings: &GameSettings,
    prep: &ScenePrep,
) -> GameIndex {
    let layout = crate::layout::load(project_dir).unwrap_or_default();
    let models = manifest
        .assets
        .iter()
        .filter_map(|asset| {
            let latest = asset.latest()?;
            let clips_off: Vec<String> = crate::layout::states::clips_off(&layout, &asset.id)
                .into_iter()
                .map(str::to_string)
                .collect();
            let imported = settings
                .models
                .get(&asset.id)
                .and_then(|m| m.imported_version);
            let prepared = prep
                .models
                .get(&asset.id)
                .is_some_and(|p| p.size.is_some() || !p.fixups.is_empty());
            let as_is = imported == Some(latest.version) && !prepared && clips_off.is_empty();
            let target = file_of(settings, asset);
            Some(IndexModel {
                asset: asset.id.clone(),
                name: asset.name.clone(),
                source: latest.file.clone(),
                texture_cap: texture_cap(settings, &target),
                target,
                as_is,
                copy: if as_is {
                    as_is_files(project_dir, &latest.file)
                } else {
                    Vec::new()
                },
                clips_off,
            })
        })
        .collect();
    GameIndex {
        version: INDEX_VERSION,
        format: settings.format,
        textures: settings.export.textures,
        texture_cap: settings.export.texture_cap,
        drop_maps: settings.export.drop_maps,
        models,
    }
}

/// Writes the index of a game project, leaving the file alone when it
/// says the same, and returns its path.
pub fn write(project_dir: &Path, manifest: &ProjectManifest) -> CoreResult<std::path::PathBuf> {
    let settings = settings_of(manifest)?;
    let prep = prepare::load(project_dir).unwrap_or_default();
    let index = build(project_dir, manifest, settings, &prep);
    let path = project_dir.join(INDEX_FILE);
    let current = fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<GameIndex>(&t).ok());
    if current.as_ref() != Some(&index) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| crate::CoreError::io(parent, e))?;
        }
        write_json(&path, &index)?;
    }
    Ok(path)
}
