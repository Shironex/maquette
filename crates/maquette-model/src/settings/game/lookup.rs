//! Where an asset's model goes in a game's model folder, and the folder
//! budget it checks against. The Inbox checks a model sent for an asset
//! against that budget, so the provider sends read it too.

use super::{FolderBudget, GameSettings, ProjectTarget};
use crate::assets::{Asset, AssetKind};
use crate::error::{CoreError, CoreResult};
use crate::project::ProjectManifest;

/// The folder a new asset's model goes to by its kind: `characters` for a
/// character, `environment` (or `environments`, or `kit`) for a scene, `props`
/// for a prop or `kit` in a game with no `props`, when the game has such a
/// folder. A scene falls back to `props` in a game with none of those.
fn default_folder(settings: &GameSettings, kind: AssetKind) -> String {
    let wanted: &[&str] = match kind {
        AssetKind::Character => &["characters"],
        AssetKind::Prop => &["props", "kit"],
        AssetKind::Scene => &["environment", "environments", "kit", "props"],
    };
    wanted
        .iter()
        .find(|w| settings.budgets.iter().any(|b| b.folder == **w))
        .map_or_else(String::new, ToString::to_string)
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

/// `props` for `props/brazier.glb`, empty at the top level.
pub fn folder_of(file: &str) -> String {
    file.rsplit_once('/')
        .map(|(folder, _)| folder.to_string())
        .unwrap_or_default()
}

/// The budget a file in the model folder checks against: its folder's, or
/// the nearest parent folder's.
pub fn budget_for<'a>(settings: &'a GameSettings, file: &str) -> Option<&'a FolderBudget> {
    let mut folder = folder_of(file);
    loop {
        if let Some(found) = settings.budgets.iter().find(|b| b.folder == folder) {
            return Some(found);
        }
        if folder.is_empty() {
            return None;
        }
        folder = folder_of(&folder);
    }
}

/// The budget an asset's models check against in a game project: that of
/// the folder its model maps to. `None` for a website project and for an
/// asset the project does not have. The Inbox checks a model sent for the
/// asset against it, so the Inbox and the top bar agree.
pub fn asset_budget(manifest: &ProjectManifest, asset_id: &str) -> Option<FolderBudget> {
    let settings = settings_of(manifest).ok()?;
    let asset = manifest.assets.iter().find(|a| a.id == asset_id)?;
    budget_for(settings, &file_of(settings, asset)).cloned()
}
