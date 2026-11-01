//! Changing a game project's settings, and the fix-ups its budgets ask for.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    FolderBudget, GameExport, GameFormat, GameSettings, ProjectTarget, budget, file_of,
    settings_of, ship, validate,
};
use crate::error::{CoreError, CoreResult};
use crate::prepare::{self, Fixup};
use crate::project::{self, ProjectManifest};
use crate::workspace::FolderLayout;

/// What a person or an agent changes in a game project's settings. A field
/// left out keeps what is saved.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameSettingsChange {
    /// The format of models made in Maquette. Imported models keep the
    /// format of their file.
    #[serde(default)]
    pub format: Option<GameFormat>,
    #[serde(default)]
    pub models_dir: Option<String>,
    #[serde(default)]
    pub export: Option<GameExport>,
    /// Whether the pipeline runs Bake light.
    #[serde(default)]
    pub bake: Option<bool>,
    /// The whole budget table, one row per folder.
    #[serde(default)]
    pub budgets: Option<Vec<FolderBudget>>,
    /// Asset id to the folder its model goes to in the model folder
    /// (`kit`, relative, no `..`); null or an empty folder puts it back where its kind or its
    /// imported file puts it. Assets left out keep theirs.
    #[serde(default)]
    pub asset_folders: Option<BTreeMap<String, Option<String>>>,
}

/// Sets or clears the folder of each asset `folders` names, refusing an
/// asset the project does not have.
fn apply_folders(
    next: &mut GameSettings,
    manifest: &ProjectManifest,
    folders: &BTreeMap<String, Option<String>>,
) -> CoreResult<()> {
    for (asset, folder) in folders {
        if !manifest.assets.iter().any(|a| a.id == *asset) {
            return Err(CoreError::Invalid(format!(
                "{} has no asset {asset}.",
                manifest.name
            )));
        }
        let folder = folder.as_deref().unwrap_or("").trim().trim_end_matches('/');
        if folder.is_empty() {
            next.folders.remove(asset);
        } else {
            next.folders.insert(asset.clone(), folder.to_string());
        }
    }
    Ok(())
}

/// The files the moved assets leave behind in the model folder: each asset
/// `folders` names whose file is somewhere else now, old path first.
fn moves(
    before: &GameSettings,
    after: &GameSettings,
    manifest: &ProjectManifest,
    folders: &BTreeMap<String, Option<String>>,
) -> Vec<(String, String)> {
    manifest
        .assets
        .iter()
        .filter(|a| folders.contains_key(&a.id))
        .map(|a| (file_of(before, a), file_of(after, a)))
        .filter(|(old, new)| old != new)
        .collect()
}

/// Saves `change` into the game settings of project `id` and fits its
/// pipeline to them (Bake light in or out, the export arguments). Refused,
/// saving nothing, for a website or when the result does not validate.
pub fn save_settings(
    workspace_root: &Path,
    layout: &FolderLayout,
    id: &str,
    change: GameSettingsChange,
) -> CoreResult<ProjectManifest> {
    let mut moved = Vec::new();
    let mut models_dir = String::new();
    let manifest = project::modify(workspace_root, layout, id, |m| {
        let next = changed(m, &change)?;
        if let (Some(folders), Some(game)) = (&change.asset_folders, m.game.as_deref()) {
            moved = moves(game, &next, m, folders);
        }
        models_dir.clone_from(&next.models_dir);
        m.game = Some(Box::new(next));
        Ok(())
    })?;
    let dir = project::dir(workspace_root, layout, id)?;
    ship::record_moves(&dir, &models_dir, &moved)?;
    crate::pipeline::ensure(&dir, &manifest)?;
    Ok(manifest)
}

/// The settings `change` leads to, checked, with nothing saved.
fn changed(m: &ProjectManifest, change: &GameSettingsChange) -> CoreResult<GameSettings> {
    let Some(game) = m
        .game
        .as_deref()
        .filter(|_| m.target == ProjectTarget::Game)
    else {
        return Err(CoreError::Invalid(format!(
            "{} is not a game project.",
            m.name
        )));
    };
    let mut next = game.clone();
    if let Some(format) = change.format {
        next.format = format;
    }
    if let Some(dir) = &change.models_dir {
        next.models_dir = dir.trim().trim_matches('/').to_string();
    }
    if let Some(export) = &change.export {
        next.export = export.clone();
    }
    if let Some(bake) = change.bake {
        next.bake = bake;
    }
    if let Some(budgets) = &change.budgets {
        next.budgets = budgets.clone();
    }
    if let Some(folders) = &change.asset_folders {
        apply_folders(&mut next, m, folders)?;
    }
    validate(&next)?;
    Ok(next)
}

/// Refuses a change [`save_settings`] would refuse, writing nothing, for a
/// call that moves the repo first.
pub fn check_settings(m: &ProjectManifest, change: &GameSettingsChange) -> CoreResult<()> {
    changed(m, change).map(|_| ())
}

/// The fix-ups that bring a model within `check`: Decimate to the folder's
/// triangles, Shrink textures to its texture size. A rigged model gets no
/// Decimate: it would tear its skin weights and shape keys, so its
/// triangles are reduced by hand.
pub fn fixups_for(check: &budget::BudgetCheck) -> Vec<Fixup> {
    let mut out = Vec::new();
    if check.triangles > check.max_triangles && !check.rigged {
        out.push(Fixup::Decimate {
            triangles: check.max_triangles,
        });
    }
    if check.texture > check.max_texture {
        out.push(Fixup::ShrinkTextures {
            max_size: check.max_texture,
        });
    }
    out
}

/// Adds to every over budget asset the fix-ups that bring it within its
/// folder's budget, replacing a fix-up of the same kind. Returns the ids
/// of the assets that changed; a rigged model only over its triangles is
/// not one of them. Build models applies them on the next run.
pub fn add_budget_fixups(
    project_dir: &Path,
    manifest: &ProjectManifest,
) -> CoreResult<Vec<String>> {
    let settings = settings_of(manifest)?;
    let wanted: Vec<(String, Vec<Fixup>)> = manifest
        .assets
        .iter()
        .filter_map(|asset| {
            let fixes = fixups_for(&budget::check_asset(settings, asset)?);
            (!fixes.is_empty()).then(|| (asset.id.clone(), fixes))
        })
        .collect();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    prepare::update(project_dir, |current| {
        let mut next = current.clone();
        for (asset, fixes) in &wanted {
            let entry = next.models.entry(asset.clone()).or_default();
            for fix in fixes {
                entry.fixups.retain(|f| f.kind() != fix.kind());
                entry.fixups.push(*fix);
            }
        }
        Ok(next)
    })?;
    Ok(wanted.into_iter().map(|(asset, _)| asset).collect())
}
