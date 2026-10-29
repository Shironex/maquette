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

use crate::assets::{Asset, AssetKind};
use crate::error::{CoreError, CoreResult};
use crate::project::ProjectManifest;

pub mod budget;
pub mod detect;
pub mod import;
pub mod index;
mod obj;
pub mod scan;
pub mod settings;
pub mod ship;

pub use crate::settings::game::{
    DEFAULT_MODELS_DIR, DEFAULT_TEXTURE_CAP, FolderBudget, GameExport, GameFormat, GameModel,
    GameSettings, GameTextures, MIN_BUDGET_TEXTURE, MIN_BUDGET_TRIANGLES, NewGame, ProjectTarget,
    TEXTURE_CAPS,
};
pub use budget::{BudgetCheck, asset_budget, check_asset, default_budgets, folder_of};
pub use detect::{GameRepoInspection, check_repo, inspect_game_repo};
pub use import::{ImportResult, import_models};
pub use scan::{GameScan, ScannedModel, scan_models};
pub use settings::{GameSettingsChange, add_budget_fixups, save_settings};
pub use ship::{ShipChange, ShipFile, ShipPlan, plan as ship_plan};

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
        folders: BTreeMap::new(),
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
        if budget.triangles < MIN_BUDGET_TRIANGLES || budget.texture < MIN_BUDGET_TEXTURE {
            return invalid(format!(
                "The budget of {}/ needs at least {MIN_BUDGET_TRIANGLES} triangles and a texture size of {MIN_BUDGET_TEXTURE} px or more.",
                budget.folder
            ));
        }
    }
    for (asset, folder) in &settings.folders {
        if !stays_inside(folder) {
            return invalid(format!(
                "{asset}: the folder {folder:?} must be a relative folder inside the model folder, without .."
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
/// `<folder>/<snake name>.<format>` for an asset made in Maquette. A folder
/// set for the asset ([`GameSettings::folders`]) moves either, keeping the
/// file name.
pub fn file_of(settings: &GameSettings, asset: &Asset) -> String {
    let (folder, name) = match settings.models.get(&asset.id) {
        Some(model) => match model.file.rsplit_once('/') {
            Some((folder, name)) => (folder.to_string(), name.to_string()),
            None => (String::new(), model.file.clone()),
        },
        None => (
            default_folder(settings, asset.kind),
            format!("{}.{}", asset.id.replace('-', "_"), settings.format.ext()),
        ),
    };
    let folder = settings.folders.get(&asset.id).cloned().unwrap_or(folder);
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
mod move_tests;
#[cfg(test)]
mod rig_export_tests;
#[cfg(test)]
mod tests;

/// Registers this module's IPC types (a game repo's checks, scan, import and ship plan) with the
/// shell's TypeScript export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<GameSettingsChange>()
        .register::<GameRepoInspection>()
        .register::<BudgetCheck>()
        .register::<ScannedModel>()
        .register::<GameScan>()
        .register::<ImportResult>()
        .register::<import::SkippedFile>()
        .register::<ShipChange>()
        .register::<ShipFile>()
        .register::<ship::ShipCheck>()
        .register::<ShipPlan>()
}
