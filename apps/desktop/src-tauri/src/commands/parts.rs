//! An asset's parts for the Inspector's Parts tab and Compare: the parts
//! its newest model ships with, by name, renaming, joining and hiding them
//! (kept in `prepare.json`), and two versions' numbers side by side. A
//! model the split fix-up cuts is read from the fix-up preview's copy,
//! which runs headless when it is missing.

use std::path::PathBuf;

use maquette_core::parts::{self, ModelParts, PartRow};
use maquette_core::prepare::preview;
use maquette_core::prepare::{ModelPrep, PartsPlan};
use maquette_core::{AssetKind, ProjectManifest, ProjectTarget, assets, game, project};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use super::CommandResult;
use super::prepare::{PreviewRequest, asset_view, preview_job, save};
use crate::session::Session;

/// One asset's parts as its newest model ships them.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetParts {
    pub project: String,
    pub asset: String,
    pub name: String,
    pub kind: AssetKind,
    /// The newest version, when the asset has a model.
    pub version: Option<u32>,
    /// The file the parts were read from, an absolute path: the newest
    /// model, or the fix-up preview's copy once a split ran.
    pub file: Option<String>,
    /// Its fix-ups cut or join the parts.
    pub split: bool,
    /// They do, and the preview that lists the parts has not run yet.
    pub needs_preview: bool,
    pub rows: Vec<PartRow>,
    pub plan: PartsPlan,
    /// A game project: no Timeline, so Compare explodes the parts itself.
    pub game: bool,
}

/// One side of a compare: a version as it came, or after its fix-ups.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PartsSide {
    pub version: u32,
    #[serde(default)]
    pub fixed: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PartsCompare {
    pub left: ModelParts,
    pub right: ModelParts,
    /// The files each side shows, absolute paths.
    pub left_file: String,
    pub right_file: String,
}

struct Found {
    manifest: ProjectManifest,
    dir: PathBuf,
    asset: maquette_core::Asset,
    entry: ModelPrep,
    previews: PathBuf,
    script: String,
}

fn find(session: &Session, project_id: &str, asset_id: &str) -> CommandResult<Found> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?.clone();
    let entry = asset_view(session, project_id, &asset.id)?.entry;
    Ok(Found {
        previews: preview::previews_dir(&root.join(&config.layout.cache)),
        script: preview::script_for(Some(&dir)),
        manifest,
        dir,
        asset,
        entry,
    })
}

fn view(found: &Found, project_id: &str) -> CommandResult<AssetParts> {
    let latest = found.asset.latest();
    let input = latest.map(|v| found.dir.join(&v.file));
    let split = parts::changes_parts(&found.entry);
    let listing = parts::listing_plan(&found.entry);
    let file = match (&input, split) {
        (Some(input), true) => preview::cached(&found.previews, input, &listing, &found.script)
            .map(|report| PathBuf::from(report.output)),
        (Some(input), false) => Some(input.clone()),
        (None, _) => None,
    };
    let plan = found.entry.parts.clone().unwrap_or_default();
    let rows = match &file {
        Some(file) => parts::rows(&parts::read(file)?, Some(&plan)),
        None => Vec::new(),
    };
    Ok(AssetParts {
        project: project_id.to_string(),
        asset: found.asset.id.clone(),
        name: found.asset.name.clone(),
        kind: found.asset.kind,
        version: latest.map(|v| v.version),
        needs_preview: split && input.is_some() && file.is_none(),
        file: file.map(|f| f.display().to_string()),
        split,
        rows,
        plan,
        game: found.manifest.target == ProjectTarget::Game,
    })
}

pub(crate) fn get(
    session: &Session,
    project_id: &str,
    asset_id: &str,
) -> CommandResult<AssetParts> {
    view(&find(session, project_id, asset_id)?, project_id)
}

/// The asset's parts as its newest model ships them. Reads files only: a
/// split that has not been previewed says so in `needsPreview`.
#[tauri::command]
#[specta::specta]
pub fn parts_get(
    session: State<'_, Session>,
    project: String,
    asset: String,
) -> CommandResult<AssetParts> {
    get(&session, &project, &asset)
}

/// The preview that lists a split asset's parts by their model names: its
/// fix-ups alone, headless on a copy. Blocks.
pub(crate) fn run_listing(
    session: &Session,
    tools: &maquette_core::pipeline::tools::Toolbox,
    project_id: &str,
    asset_id: &str,
) -> CommandResult<AssetParts> {
    let found = find(session, project_id, asset_id)?;
    if parts::changes_parts(&found.entry) && found.asset.latest().is_some() {
        let request = PreviewRequest {
            project: Some(project_id.to_string()),
            asset: Some(found.asset.id.clone()),
            inbox: None,
            plan: parts::listing_plan(&found.entry),
        };
        preview_job(session, &request)?.run(tools)?;
    }
    view(&found, project_id)
}

/// Runs the split headless on a copy of the newest model so its parts can
/// be listed, then returns them. A split that already ran answers at once.
#[tauri::command]
#[specta::specta]
pub async fn parts_preview(
    app: AppHandle,
    session: State<'_, Session>,
    project: String,
    asset: String,
) -> CommandResult<AssetParts> {
    let tools = super::toolchain::toolbox(&session).await?;
    tauri::async_runtime::spawn_blocking(move || {
        run_listing(app.state::<Session>().inner(), &tools, &project, &asset)
    })
    .await
    .map_err(super::join_failed)?
}

pub(crate) fn save_plan(
    session: &Session,
    project_id: &str,
    asset_id: &str,
    plan: &PartsPlan,
) -> CommandResult<AssetParts> {
    let mut entry = asset_view(session, project_id, asset_id)?.entry;
    entry.parts = (!plan.is_empty()).then(|| plan.clone());
    save(session, project_id, asset_id, &entry)?;
    get(session, project_id, asset_id)
}

/// Saves the asset's part names, joins and hidden parts in `prepare.json`.
/// The next build ships the names and joins; hiding changes no export.
#[tauri::command]
#[specta::specta]
pub fn parts_save(
    session: State<'_, Session>,
    project: String,
    asset: String,
    parts: PartsPlan,
) -> CommandResult<AssetParts> {
    save_plan(&session, &project, &asset, &parts)
}

/// Two sides' numbers. A fixed side runs the asset's fix-ups headless on a
/// copy when they have not been previewed yet. Blocks.
pub(crate) fn compare(
    session: &Session,
    tools: &maquette_core::pipeline::tools::Toolbox,
    project_id: &str,
    asset_id: &str,
    sides: [&PartsSide; 2],
) -> CommandResult<PartsCompare> {
    let found = find(session, project_id, asset_id)?;
    let (_, config) = session.workspace()?;
    let folder = game::asset_budget(&found.manifest, &found.asset.id);
    let mut out = Vec::new();
    for side in sides {
        let Some(version) = found
            .asset
            .versions
            .iter()
            .find(|v| v.version == side.version)
        else {
            return Err(super::invalid(&format!(
                "{} has no v{}.",
                found.asset.name, side.version
            )));
        };
        let input = found.dir.join(&version.file);
        let (file, plan) = if side.fixed && !found.entry.fixups.is_empty() {
            let request = PreviewRequest {
                project: Some(project_id.to_string()),
                asset: Some(found.asset.id.clone()),
                inbox: None,
                plan: found.entry.clone(),
            };
            let job = preview_job(session, &request)?.on(input);
            (PathBuf::from(job.run(tools)?.output), None)
        } else {
            (input, found.entry.parts.as_ref())
        };
        let numbers = parts::numbers(&file, found.asset.kind, &config, folder.as_ref(), plan)?;
        out.push((numbers, file.display().to_string()));
    }
    let (right, right_file) = out.pop().unwrap_or_default();
    let (left, left_file) = out.pop().unwrap_or_default();
    Ok(PartsCompare {
        left,
        right,
        left_file,
        right_file,
    })
}

/// Two versions of an asset (or one before and after its fix-ups) side by
/// side: the parts Motion can move, draw calls, triangles, textures,
/// transfer size and Inbox checks.
#[tauri::command]
#[specta::specta]
pub async fn parts_compare(
    app: AppHandle,
    session: State<'_, Session>,
    project: String,
    asset: String,
    left: PartsSide,
    right: PartsSide,
) -> CommandResult<PartsCompare> {
    let tools = super::toolchain::toolbox(&session).await?;
    tauri::async_runtime::spawn_blocking(move || {
        let session = app.state::<Session>();
        compare(session.inner(), &tools, &project, &asset, [&left, &right])
    })
    .await
    .map_err(super::join_failed)?
}

/// Makes one version the asset's current one, for the next export. Every
/// version stays in the asset.
#[tauri::command]
#[specta::specta]
pub fn parts_keep_version(
    session: State<'_, Session>,
    project: String,
    asset: String,
    version: u32,
) -> CommandResult<ProjectManifest> {
    let (root, config) = session.workspace()?;
    Ok(project::modify(
        &root,
        &config.layout,
        &project,
        |manifest| {
            parts::keep_version(manifest, &asset, version)?;
            Ok(manifest.clone())
        },
    )?)
}

#[cfg(test)]
mod tests;
