//! A game project's settings, in `project.json` (`game`): the model format
//! the engine reads, the model folder in the game repo, the Game export
//! settings, a budget per model folder and where each asset's model goes.
//! Scanning, importing and shipping them is core's `game` module.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

mod lookup;
mod new;
mod templates;

pub use lookup::{asset_budget, budget_for, file_of, folder_of, settings_of};
pub use new::{default_budgets, new_settings, stays_inside};
pub use new::{model_files, model_folders};
pub use templates::{
    GameTemplate, GameTemplateGroup, GameTemplateId, game_template, game_templates, merge_budgets,
};

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
    /// Leaves normal and metal-roughness maps out of the exported models,
    /// for a game whose shader reads neither. Off (keep them) by default.
    #[serde(default)]
    pub drop_maps: bool,
}

impl Default for GameExport {
    fn default() -> Self {
        Self {
            textures: GameTextures::Png,
            texture_cap: DEFAULT_TEXTURE_CAP,
            drop_maps: false,
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
    /// Asset id to the folder its model goes to, relative to the model
    /// folder (`kit`, `enemies/bosses`), in place of the folder its kind or
    /// its imported file gives it. Moving an asset leaves its file name.
    #[serde(default)]
    pub folders: BTreeMap<String, String>,
}

/// What New project asks for a game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewGame {
    pub format: GameFormat,
    pub models_dir: String,
    /// Lays the template's folders and budgets over the ones found in the
    /// repo. The format above still decides the format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<GameTemplateId>,
}

/// The fewest triangles a folder budget allows, the floor of Decimate.
pub const MIN_BUDGET_TRIANGLES: u32 = 100;
/// The smallest texture size a folder budget allows, the floor of Shrink
/// textures.
pub const MIN_BUDGET_TEXTURE: u32 = 64;

pub const DEFAULT_MODELS_DIR: &str = "assets/models";
