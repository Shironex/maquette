//! Game projects: models for a person's own engine instead of a web scene.
//!
//! A project's [`ProjectTarget`] says where it ships. A game project keeps
//! its game settings in `project.json` ([`GameSettings`]): the model format
//! the engine reads, the folder of the game repo the models live in, the
//! Game export settings, a budget per model folder and where each asset's
//! model goes in that folder. The game repo itself is the project's
//! `siteRepo`, the repo Ship writes to whatever the target.
//!
//! A game starts from its own models ([`scan`], [`import`]): each one the
//! person picks becomes an asset whose version 1 is the repo file. The
//! pipeline then fixes and exports models in the engine's format
//! ([`index`]), and Ship writes them back into the model folder, replacing
//! only files it wrote or imported ([`ship`]).

use std::collections::BTreeMap;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::assets::{Asset, AssetKind};
use crate::error::{CoreError, CoreResult};
use crate::project::ProjectManifest;

pub mod budget;
pub mod detect;
pub mod import;
pub mod index;
mod obj;
pub mod scan;
pub mod ship;

pub use budget::{BudgetCheck, check_asset, default_budgets, folder_of};
pub use detect::{GameRepoInspection, inspect_game_repo};
pub use import::{ImportResult, import_models};
pub use scan::{GameScan, ScannedModel, scan_models};
pub use ship::{ShipChange, ShipFile, ShipPlan, plan as ship_plan};

/// Where a project ships.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectTarget {
    /// A web scene: Ship copies models, posters and scene data into a site.
    #[default]
    Website,
    /// Models for a person's own engine: Ship writes model files into a
    /// folder of the game repo.
    Game,
}

/// The model files a game engine reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum GameFormat {
    /// Binary glTF, textures inside.
    #[default]
    Glb,
    /// Wavefront `.obj` with its `.mtl` and texture files.
    Obj,
}

impl GameFormat {
    pub fn ext(self) -> &'static str {
        match self {
            GameFormat::Glb => "glb",
            GameFormat::Obj => "obj",
        }
    }

    /// The format of a model file by its extension.
    pub fn of(file: &str) -> Option<Self> {
        let ext = Path::new(file).extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "glb" | "gltf" => Some(GameFormat::Glb),
            "obj" => Some(GameFormat::Obj),
            _ => None,
        }
    }
}

/// The image files Game export writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum GameTextures {
    #[default]
    Png,
    Jpg,
    /// PNG stays PNG and JPG stays JPG.
    Keep,
}

impl GameTextures {
    pub fn word(self) -> &'static str {
        match self {
            GameTextures::Png => "png",
            GameTextures::Jpg => "jpg",
            GameTextures::Keep => "keep",
        }
    }
}

/// The texture edges Game export can cap at.
pub const TEXTURE_CAPS: [u32; 5] = [256, 512, 1024, 2048, 4096];
pub const DEFAULT_TEXTURE_CAP: u32 = 1024;

/// What Game export writes: plain files, never KTX2 or meshopt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameExport {
    pub textures: GameTextures,
    /// The longest texture edge in pixels, one of [`TEXTURE_CAPS`].
    pub texture_cap: u32,
}

impl Default for GameExport {
    fn default() -> Self {
        Self {
            textures: GameTextures::Png,
            texture_cap: DEFAULT_TEXTURE_CAP,
        }
    }
}

/// The budget of every model in one folder of the model folder. Each model
/// checks on its own; a game has no scene total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderBudget {
    /// Relative to the model folder, `props`; empty for its top level.
    pub folder: String,
    pub triangles: u32,
    /// The longest texture edge in pixels.
    pub texture: u32,
}

/// Where an asset's model lives in the game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameModel {
    /// Relative to the model folder, with `/`: `props/brazier.glb`.
    pub file: String,
    /// The version that is the repo file as it was imported, so Game
    /// export copies it untouched until something changes it.
    pub imported_version: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameSettings {
    pub format: GameFormat,
    /// The folder of the game repo the models live in, `assets/models`.
    pub models_dir: String,
    pub export: GameExport,
    /// Optional, off for a new game: engines light at runtime.
    pub bake: bool,
    pub budgets: Vec<FolderBudget>,
    /// Asset id to its file in the model folder.
    pub models: BTreeMap<String, GameModel>,
}

/// What New project asks for a game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewGame {
    pub format: GameFormat,
    pub models_dir: String,
}

pub const DEFAULT_MODELS_DIR: &str = "assets/models";

/// True for a relative path with `/` that cannot leave the folder it is
/// joined to.
pub fn stays_inside(path: &str) -> bool {
    let p = Path::new(path);
    !path.trim().is_empty()
        && !path.starts_with(['/', '\\'])
        && !path.contains('\\')
        && !p.is_absolute()
        && p.components().all(|c| matches!(c, Component::Normal(_)))
}

/// The settings of a new game project in `repo`, with a budget for every
/// model folder found there.
pub fn new_settings(new: &NewGame, repo: Option<&Path>) -> CoreResult<GameSettings> {
    let models_dir = new.models_dir.trim().trim_end_matches('/').to_string();
    if !stays_inside(&models_dir) {
        return Err(CoreError::Invalid(format!(
            "The model folder {models_dir:?} must be a folder inside the game repo, without .."
        )));
    }
    let folders = repo
        .map(|r| detect::model_folders(&r.join(&models_dir)))
        .unwrap_or_default();
    Ok(GameSettings {
        format: new.format,
        models_dir,
        export: GameExport::default(),
        bake: false,
        budgets: default_budgets(&folders),
        models: BTreeMap::new(),
    })
}

/// Checks the settings a person or an agent saves.
pub fn validate(settings: &GameSettings) -> CoreResult<()> {
    let invalid = |m: String| Err(CoreError::Invalid(m));
    if !stays_inside(&settings.models_dir) {
        return invalid(format!(
            "The model folder {:?} must be a folder inside the game repo.",
            settings.models_dir
        ));
    }
    if !TEXTURE_CAPS.contains(&settings.export.texture_cap) {
        return invalid(format!(
            "The texture size cap must be one of 256, 512, 1024, 2048 or 4096 px, not {}.",
            settings.export.texture_cap
        ));
    }
    for budget in &settings.budgets {
        if !budget.folder.is_empty() && !stays_inside(&budget.folder) {
            return invalid(format!("{:?} is not a folder name.", budget.folder));
        }
        if budget.triangles == 0 || budget.texture == 0 {
            return invalid(format!(
                "The budget of {}/ needs triangles and a texture size above 0.",
                budget.folder
            ));
        }
    }
    for (asset, model) in &settings.models {
        if !stays_inside(&model.file) || GameFormat::of(&model.file).is_none() {
            return invalid(format!(
                "{asset}: {:?} must be a .glb or .obj file inside the model folder.",
                model.file
            ));
        }
    }
    Ok(())
}

/// The folder a new asset's model goes to by its kind: `characters` for a
/// character, `props` for the rest, when the game has such a folder.
fn default_folder(settings: &GameSettings, kind: AssetKind) -> String {
    let wanted = match kind {
        AssetKind::Character => "characters",
        AssetKind::Prop | AssetKind::Scene => "props",
    };
    if settings.budgets.iter().any(|b| b.folder == wanted) {
        wanted.to_string()
    } else {
        String::new()
    }
}

/// Where `asset`'s model goes in the model folder: its recorded file, or
/// `<folder>/<snake name>.<format>` for an asset made in Maquette.
pub fn file_of(settings: &GameSettings, asset: &Asset) -> String {
    if let Some(model) = settings.models.get(&asset.id) {
        return model.file.clone();
    }
    let stem = asset.id.replace('-', "_");
    let folder = default_folder(settings, asset.kind);
    let name = format!("{stem}.{}", settings.format.ext());
    if folder.is_empty() {
        name
    } else {
        format!("{folder}/{name}")
    }
}

/// The game settings of a game project, or a sentence for anything else.
pub fn settings_of(manifest: &ProjectManifest) -> CoreResult<&GameSettings> {
    match (manifest.target, manifest.game.as_ref()) {
        (ProjectTarget::Game, Some(settings)) => Ok(settings),
        _ => Err(CoreError::Invalid(format!(
            "{} is not a game project.",
            manifest.name
        ))),
    }
}

#[cfg(test)]
mod blender_tests;
#[cfg(test)]
mod tests;
