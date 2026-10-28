//! Real size and fix-ups: what happens to an asset's model before the scene
//! is built, kept in the project's `prepare.json`. The Board, the Inbox and
//! the Inspector read and save one asset's entry; previews run the fix-ups
//! headless on one model to show what they do.

use std::path::PathBuf;
use std::sync::LazyLock;

use maquette_core::inbox::analyse;
use maquette_core::inbox::checks::texture_px;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::prepare::preview::{self, FixupReport, Previews, previews_dir};
use maquette_core::prepare::{
    self, DEFAULT_LOOSE_SHARE, ModelPrep, ScenePrep, Suggestion, extent, scale_for, suggest,
};
use maquette_core::{AssetKind, CoreResult, WorkspaceConfig, assets, inbox, project};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::CommandResult;
use crate::session::Session;

/// Previews run one at a time for the whole app, app and agents alike.
static PREVIEWS: LazyLock<Previews> = LazyLock::new(Previews::default);

/// The numbers a new fix-up starts from: the workspace budgets.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FixupDefaults {
    /// The per model triangle budget, for Decimate.
    pub triangles: u32,
    /// The default texture size, for Shrink textures.
    pub max_texture: u32,
    /// For Remove loose parts.
    pub loose_share: f32,
}

pub(crate) fn defaults(config: &WorkspaceConfig) -> FixupDefaults {
    FixupDefaults {
        triangles: config.budgets.tris_per_model,
        max_texture: texture_px(config.compression.texture_size),
        loose_share: DEFAULT_LOOSE_SHARE,
    }
}

/// One asset's real size and fix-ups, with what its newest model measures.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetPrep {
    pub project: String,
    pub asset: String,
    pub name: String,
    pub kind: AssetKind,
    pub entry: ModelPrep,
    /// The newest version, when the asset has a model.
    pub version: Option<u32>,
    /// Width, height and depth of that model as it came, metres.
    pub measured: Option<[f32; 3]>,
    /// The same after the fix-ups, once a preview of them ran.
    pub fixed: Option<[f32; 3]>,
    /// The real size scale on the fixed (else measured) model.
    pub scale: Option<f32>,
    /// A size from the name and kind, never applied on its own.
    pub suggestion: Option<Suggestion>,
    /// The last preview of the entry on the newest model, when one ran.
    pub report: Option<FixupReport>,
    pub defaults: FixupDefaults,
}

/// What a preview runs on: an inbox model, or an asset's newest version.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRequest {
    /// The project whose fix-up script runs (an inbox model's target), and
    /// with `asset`, whose asset.
    pub project: Option<String>,
    pub asset: Option<String>,
    /// An inbox model by id, instead of an asset.
    pub inbox: Option<String>,
    pub plan: ModelPrep,
}

fn project_dir(session: &Session, project_id: &str) -> CommandResult<PathBuf> {
    let (root, config) = session.workspace()?;
    project::load(&root, &config.layout, project_id)?;
    Ok(project::dir(&root, &config.layout, project_id)?)
}

fn previews_root(session: &Session) -> CommandResult<PathBuf> {
    let (root, config) = session.workspace()?;
    Ok(previews_dir(&root.join(&config.layout.cache)))
}

pub(crate) fn get(session: &Session, project_id: &str) -> CommandResult<ScenePrep> {
    Ok(prepare::load(&project_dir(session, project_id)?)?)
}

#[tauri::command]
#[specta::specta]
pub fn prepare_get(session: State<'_, Session>, project: String) -> CommandResult<ScenePrep> {
    get(&session, &project)
}

/// The size of a model's bounds in metres.
fn size_of(bounds: Option<([f32; 3], [f32; 3])>) -> Option<[f32; 3]> {
    bounds.map(extent)
}

pub(crate) fn asset_view(
    session: &Session,
    project_id: &str,
    asset_id: &str,
) -> CommandResult<AssetPrep> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?;
    let entry = prepare::load(&dir)?
        .models
        .get(&asset.id)
        .cloned()
        .unwrap_or_default();
    let latest = asset.latest();
    let file = latest.map(|v| dir.join(&v.file));
    let previews = previews_root(session)?;
    let script = preview::script_for(Some(&dir));
    let measured = file.as_deref().and_then(|f| size_of(analyse::bounds(f)));
    let fixed = file
        .as_deref()
        .filter(|_| !entry.fixups.is_empty())
        .and_then(|f| size_of(preview::fixed_bounds(&previews, f, &entry, &script)));
    let scale = entry
        .size
        .and_then(|size| scale_for(&size, fixed.or(measured)?));
    let report = file
        .as_deref()
        .filter(|_| !entry.is_empty())
        .and_then(|f| preview::cached(&previews, f, &entry, &script));
    Ok(AssetPrep {
        project: project_id.to_string(),
        asset: asset.id.clone(),
        name: asset.name.clone(),
        kind: asset.kind,
        version: latest.map(|v| v.version),
        measured,
        fixed,
        scale,
        suggestion: suggest(&asset.name, asset.kind),
        report,
        defaults: defaults(&config),
        entry,
    })
}

#[tauri::command]
#[specta::specta]
pub fn prepare_asset(
    session: State<'_, Session>,
    project: String,
    asset: String,
) -> CommandResult<AssetPrep> {
    asset_view(&session, &project, &asset)
}

/// Replaces the asset's entry in `prepare.json` as it is on disk.
pub(crate) fn save(
    session: &Session,
    project_id: &str,
    asset_id: &str,
    entry: &ModelPrep,
) -> CommandResult<AssetPrep> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?;
    prepare::save_entry(&project_dir(session, project_id)?, &asset.id, entry)?;
    asset_view(session, project_id, asset_id)
}

/// Saves one asset's real size and fix-ups (an empty entry clears them) in
/// `prepare.json` as it is on disk, so every other asset keeps its own. The
/// next run of Build scene applies them.
#[tauri::command]
#[specta::specta]
pub fn prepare_save(
    session: State<'_, Session>,
    project: String,
    asset: String,
    entry: ModelPrep,
) -> CommandResult<AssetPrep> {
    save(&session, &project, &asset, &entry)
}

/// Applies `change` to the project's `prepare.json` as it is on disk.
pub(crate) fn update(
    session: &Session,
    project_id: &str,
    change: impl FnOnce(&ScenePrep) -> CoreResult<ScenePrep>,
) -> CommandResult<ScenePrep> {
    Ok(prepare::update(&project_dir(session, project_id)?, change)?)
}

/// A suggested real size for a name and kind, from a small table of
/// everyday objects.
#[tauri::command]
#[specta::specta]
pub fn prepare_suggest(name: String, kind: AssetKind) -> Option<Suggestion> {
    suggest(&name, kind)
}

/// The model a preview reads and the project folder whose script runs.
pub(crate) fn preview_input(
    session: &Session,
    request: &PreviewRequest,
) -> CommandResult<(PathBuf, Option<PathBuf>)> {
    let (root, config) = session.workspace()?;
    let dir = request
        .project
        .as_deref()
        .map(|p| project_dir(session, p))
        .transpose()?;
    if let Some(id) = &request.inbox {
        let found = inbox::item(&root, &config, id)?;
        return Ok((PathBuf::from(found.file), dir));
    }
    let (Some(project_id), Some(asset_id)) = (&request.project, &request.asset) else {
        return Err(super::invalid(
            "Name an inbox model, or a project and one of its assets.",
        ));
    };
    let manifest = project::load(&root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?;
    let Some(latest) = asset.latest() else {
        return Err(super::invalid(&format!(
            "{} has no model yet, so there is nothing to fix.",
            asset.name
        )));
    };
    let base = dir.clone().unwrap_or_default();
    Ok((base.join(&latest.file), dir))
}

/// A checked preview, ready to run off the main thread.
pub(crate) struct PreviewJob {
    root: PathBuf,
    input: PathBuf,
    plan: ModelPrep,
    script: String,
}

impl PreviewJob {
    /// The same plan on another model file, such as an older version.
    pub(crate) fn on(self, input: PathBuf) -> Self {
        Self { input, ..self }
    }

    /// Runs it headless and returns the numbers before and after. Blocks.
    pub(crate) fn run(&self, tools: &Toolbox) -> CommandResult<FixupReport> {
        Ok(PREVIEWS.run(&self.root, &self.input, &self.plan, &self.script, tools)?)
    }
}

/// Checks `request` and finds its model and script.
pub(crate) fn preview_job(
    session: &Session,
    request: &PreviewRequest,
) -> CommandResult<PreviewJob> {
    let mut check = ScenePrep::default();
    check
        .models
        .insert("preview".to_string(), request.plan.clone());
    prepare::validate(&check)?;
    let (input, dir) = preview_input(session, request)?;
    Ok(PreviewJob {
        root: previews_root(session)?,
        input,
        plan: request.plan.clone(),
        script: preview::script_for(dir.as_deref()),
    })
}

/// Runs the fix-ups and real size of `plan` on one model, always headless
/// (`blender -b`), and returns the numbers before and after each fix-up.
/// The model is only read. A plan that already ran answers at once.
#[tauri::command]
#[specta::specta]
pub async fn prepare_preview(
    session: State<'_, Session>,
    request: PreviewRequest,
) -> CommandResult<FixupReport> {
    let job = preview_job(&session, &request)?;
    let tools = super::toolchain::toolbox(&session).await?;
    tauri::async_runtime::spawn_blocking(move || job.run(&tools))
        .await
        .map_err(super::join_failed)?
}

#[cfg(test)]
mod tests;
