//! Applying a game template: what it would change in a project, and filling
//! the settings from it. The templates themselves are in the settings
//! module. Nothing here writes to the game repo.
//!
//! A format or a new budget row can change where an asset made in Maquette
//! goes. A model Ship already wrote into the game stays put: its file is
//! recorded in [`GameSettings::models`], so the next export lands on it and
//! no old file is left behind. A model that never shipped has nothing in
//! the game yet and simply follows the new settings.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    FolderBudget, GameFormat, GameModel, GameSettings, budget, file_of, settings_of, ship, validate,
};
use crate::error::CoreResult;
use crate::pipeline;
use crate::project::{self, ProjectManifest};
use crate::settings::game::{
    GameTemplate, GameTemplateId, game_template, merge_budgets, model_files,
};
use crate::workspace::FolderLayout;

/// One budget row a template adds or changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TemplateBudgetChange {
    pub folder: String,
    /// The row as it is saved, none for a new row.
    pub before: Option<FolderBudget>,
    pub after: FolderBudget,
}

/// A model that has not shipped and exports at a new path after the template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TemplateNewPath {
    pub asset: String,
    pub from: String,
    pub to: String,
}

/// What applying a template would change in one project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameTemplatePreview {
    pub template: GameTemplate,
    pub format_before: GameFormat,
    pub format_after: GameFormat,
    /// Only the rows that differ; every other row stays as it is.
    pub budgets: Vec<TemplateBudgetChange>,
    /// Files already in the game that the template would have put elsewhere:
    /// they keep their path and format.
    pub kept_files: Vec<String>,
    pub new_paths: Vec<TemplateNewPath>,
    pub over_before: u32,
    pub over_after: u32,
    /// What to look at before applying.
    pub notes: Vec<String>,
    /// False when the settings already match the template.
    pub changes: bool,
}

struct Outcome {
    next: GameSettings,
    kept_files: Vec<String>,
    new_paths: Vec<TemplateNewPath>,
}

/// The settings after `template`, from the settings saved in `manifest`.
fn outcome(
    settings: &GameSettings,
    manifest: &ProjectManifest,
    shipped: &BTreeSet<String>,
    template: &GameTemplate,
) -> Outcome {
    let mut next = settings.clone();
    next.format = template.format;
    next.budgets = merge_budgets(&settings.budgets, &template.budgets);
    let mut kept_files = Vec::new();
    let mut new_paths = Vec::new();
    for asset in manifest.assets.iter().filter(|a| !a.versions.is_empty()) {
        let (from, to) = (file_of(settings, asset), file_of(&next, asset));
        if from == to {
            continue;
        }
        if shipped.contains(&from) {
            next.models.insert(
                asset.id.clone(),
                GameModel {
                    file: from.clone(),
                    imported_version: None,
                },
            );
            kept_files.push(from);
        } else {
            new_paths.push(TemplateNewPath {
                asset: asset.id.clone(),
                from,
                to,
            });
        }
    }
    Outcome {
        next,
        kept_files,
        new_paths,
    }
}

/// The files the ship record lists in this game's model folder.
fn shipped_files(project_dir: &Path, settings: &GameSettings) -> BTreeSet<String> {
    ship::read_record(project_dir)
        .targets
        .remove(&settings.models_dir)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn over_count(settings: &GameSettings, manifest: &ProjectManifest) -> u32 {
    let over = manifest
        .assets
        .iter()
        .filter_map(|a| budget::check_asset(settings, a))
        .filter(budget::BudgetCheck::over)
        .count();
    u32::try_from(over).unwrap_or(u32::MAX)
}

/// The game's own files in the other format, when most of them are.
fn repo_format_note(
    manifest: &ProjectManifest,
    settings: &GameSettings,
    to: GameFormat,
) -> Option<String> {
    let repo = manifest.site_repo.as_deref()?;
    let files = model_files(&Path::new(repo).join(&settings.models_dir));
    let same = files
        .iter()
        .filter(|f| GameFormat::of(f) == Some(to))
        .count();
    (files.len() - same > same).then(|| {
        format!(
            "Most of the {} models in the model folder are not .{}. Check that the engine reads it.",
            files.len(),
            to.ext()
        )
    })
}

fn notes(
    settings: &GameSettings,
    manifest: &ProjectManifest,
    template: &GameTemplate,
    new_paths: &[TemplateNewPath],
) -> Vec<String> {
    let mut out = Vec::new();
    if template.format != settings.format {
        out.extend(repo_format_note(manifest, settings, template.format));
    }
    if template.format == GameFormat::Obj {
        let rigged = new_paths
            .iter()
            .filter_map(|p| manifest.assets.iter().find(|a| a.id == p.asset))
            .filter_map(|a| budget::check_asset(settings, a))
            .filter(|c| c.rigged)
            .count();
        if rigged > 0 {
            out.push(format!(
                "{rigged} rigged models would lose their rig and clips in .obj."
            ));
        }
    }
    let lays_out_kit = template.budgets.iter().any(|b| b.folder == "kit");
    if lays_out_kit && settings.budgets.iter().any(|b| b.folder == "props") {
        out.push(
            "This game has a budget for props/, so new models still go there. Move a piece to kit/ with its Game folder in the Budget tab."
                .to_string(),
        );
    }
    out
}

/// What applying `id` to the project would change. Changes nothing.
pub fn preview_template(
    project_dir: &Path,
    manifest: &ProjectManifest,
    id: GameTemplateId,
) -> CoreResult<GameTemplatePreview> {
    let settings = settings_of(manifest)?;
    let template = game_template(id);
    let shipped = shipped_files(project_dir, settings);
    let done = outcome(settings, manifest, &shipped, &template);
    let budgets: Vec<TemplateBudgetChange> = template
        .budgets
        .iter()
        .filter_map(|row| {
            let before = settings.budgets.iter().find(|b| b.folder == row.folder);
            (before != Some(row)).then(|| TemplateBudgetChange {
                folder: row.folder.clone(),
                before: before.cloned(),
                after: row.clone(),
            })
        })
        .collect();
    let changes = settings.format != template.format || !budgets.is_empty();
    Ok(GameTemplatePreview {
        format_before: settings.format,
        format_after: template.format,
        over_before: over_count(settings, manifest),
        over_after: over_count(&done.next, manifest),
        notes: notes(settings, manifest, &template, &done.new_paths),
        budgets,
        kept_files: done.kept_files,
        new_paths: done.new_paths,
        changes,
        template,
    })
}

/// Fills the game settings of project `id` from a template and fits its
/// pipeline to them. Moves and deletes no file, in the game or the project;
/// the settings stay editable like any others.
pub fn apply_template(
    workspace_root: &Path,
    layout: &FolderLayout,
    id: &str,
    template: GameTemplateId,
) -> CoreResult<ProjectManifest> {
    let dir = project::dir(workspace_root, layout, id)?;
    let template = game_template(template);
    let manifest = project::modify(workspace_root, layout, id, |m| {
        let settings = settings_of(m)?;
        let shipped = shipped_files(&dir, settings);
        let next = outcome(settings, m, &shipped, &template).next;
        validate(&next)?;
        m.game = Some(Box::new(next));
        Ok(())
    })?;
    pipeline::ensure(&dir, &manifest)?;
    Ok(manifest)
}
