//! Splitting a model into parts on a provider, the texture bake back after
//! it, and assembling one prop from part models: the shell side of
//! `maquette_core::providers::split` and `maquette_core::parts`. A split
//! from the app needs no extra approval: confirming the split sheet is it.

use std::path::{Path, PathBuf};

use maquette_core::parts::assemble::{self, AssembleRequest, Assembled, Recipe};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::split::{
    MESHY_SPLIT_CREDITS, TRIPO_COMPLETE_CREDITS, TRIPO_SEGMENT_CREDITS,
};
use maquette_core::providers::{
    KeyStatus, ProviderJob, ProviderJobs, RetextureRequest, SplitRequest, SplitSource,
};
use maquette_core::thumbs::{ThumbTarget, Thumbnails};
use maquette_core::{WorkspaceConfig, project};
use serde::Serialize;
use tauri::{AppHandle, Manager as _, State};

use super::providers::{Studio, key_statuses};
use super::thumbs::{self, Thumbs};
use super::toolchain;
use super::{CommandResult, blocking};
use crate::session::Session;

/// The published credits of each way, for the sheet's estimates.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SplitPrices {
    pub tripo_segment: f64,
    pub tripo_complete: f64,
    pub meshy_split: f64,
}

/// Everything the split sheet needs for one model.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SplitSheet {
    pub source: SplitSource,
    pub keys: Vec<KeyStatus>,
    pub prices: SplitPrices,
}

pub(crate) fn sheet(
    studio: &ProviderJobs,
    project: &str,
    asset: &str,
    version: Option<u32>,
    item: Option<&str>,
) -> CommandResult<SplitSheet> {
    Ok(SplitSheet {
        source: studio.split_source(project, asset, version, item)?,
        keys: key_statuses(studio)?,
        prices: SplitPrices {
            tripo_segment: TRIPO_SEGMENT_CREDITS,
            tripo_complete: TRIPO_COMPLETE_CREDITS,
            meshy_split: MESHY_SPLIT_CREDITS,
        },
    })
}

/// Runs every scheduled bake back on its own thread, then asks for the
/// thumbnails of the versions they made. For after an accept and a
/// workspace open; `tools` is only looked up when a bake waits.
pub(crate) fn spawn_bakes(
    studio: ProviderJobs,
    thumbs: Thumbnails,
    tools: impl FnOnce() -> Toolbox + Send + 'static,
    root: PathBuf,
    config: WorkspaceConfig,
) {
    if studio.scheduled_bakes().is_empty() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("bake-back".into())
        .spawn(move || {
            let tools = tools();
            for job in studio.run_scheduled_bakes(&tools) {
                thumb_for(&thumbs, &root, &config, &tools, &job);
            }
        });
    if let Err(err) = spawned {
        tracing::warn!(error = %err, "could not start the texture bake back");
    }
}

/// The bakes left scheduled when the app closed, once a workspace opens.
pub(crate) fn bakes_after_open(handle: &AppHandle, root: &Path, config: &WorkspaceConfig) {
    let (Some(studio), Some(thumbs)) = (handle.try_state::<Studio>(), handle.try_state::<Thumbs>())
    else {
        return;
    };
    let app = handle.clone();
    spawn_bakes(
        studio.0.clone(),
        thumbs.0.clone(),
        move || toolchain::toolbox_blocking(&app.state::<Session>()),
        root.to_path_buf(),
        config.clone(),
    );
}

/// Asks for the thumbnail of the version a finished bake back made.
pub(crate) fn thumb_for(
    thumbs: &Thumbnails,
    root: &Path,
    config: &WorkspaceConfig,
    tools: &Toolbox,
    job: &ProviderJob,
) {
    let Some(version) = job
        .split
        .as_ref()
        .and_then(|s| s.bake.as_ref())
        .and_then(|b| b.version)
    else {
        return;
    };
    version_thumb(
        thumbs,
        root,
        config,
        tools,
        &job.project,
        &job.asset,
        version,
    );
}

/// Asks for one version's thumbnail.
pub(crate) fn version_thumb(
    thumbs: &Thumbnails,
    root: &Path,
    config: &WorkspaceConfig,
    tools: &Toolbox,
    project_id: &str,
    asset: &str,
    version: u32,
) {
    let Ok(manifest) = project::load(root, &config.layout, project_id) else {
        return;
    };
    let Ok(found) = maquette_core::assets::find(&manifest, asset) else {
        return;
    };
    let Some(file) = found.versions.iter().find(|v| v.version == version) else {
        return;
    };
    let Ok(dir) = project::dir(root, &config.layout, project_id) else {
        return;
    };
    let target = ThumbTarget {
        project: project_id.to_string(),
        asset: asset.to_string(),
        version,
    };
    thumbs::for_version(thumbs, root, config, tools, &dir.join(&file.file), target);
}

/// The model a split would cut and what each provider can do with it.
#[tauri::command]
#[specta::specta]
pub async fn split_sheet(
    studio: State<'_, Studio>,
    project: String,
    asset: String,
    version: Option<u32>,
    item: Option<String>,
) -> CommandResult<SplitSheet> {
    let jobs = studio.0.clone();
    blocking(move || sheet(&jobs, &project, &asset, version, item.as_deref())).await?
}

/// Starts a split from the app. Confirming it in the sheet is the approval.
#[tauri::command]
#[specta::specta]
pub async fn split_send(
    studio: State<'_, Studio>,
    request: SplitRequest,
) -> CommandResult<ProviderJob> {
    let jobs = studio.0.clone();
    Ok(blocking(move || jobs.send_split(&request, false, false)).await??)
}

/// Paints a version again on Meshy. Confirming the price in the app is the
/// approval; no agent tool reaches this.
#[tauri::command]
#[specta::specta]
pub async fn retexture_send(
    studio: State<'_, Studio>,
    request: RetextureRequest,
) -> CommandResult<ProviderJob> {
    let jobs = studio.0.clone();
    Ok(blocking(move || jobs.send_retexture(&request, false, false)).await??)
}

/// Bakes the texture back onto a split now: an offered one, or one that
/// failed. Runs Blender, so off the main thread.
#[tauri::command]
#[specta::specta]
pub async fn split_bake_back(
    session: State<'_, Session>,
    studio: State<'_, Studio>,
    service: State<'_, Thumbs>,
    id: String,
) -> CommandResult<ProviderJob> {
    let (root, config) = session.workspace()?;
    let tools = toolchain::toolbox(&session).await?;
    let jobs = studio.0.clone();
    let thumbs = service.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        jobs.schedule_bake(&id)?;
        let job = jobs.bake_back(&id, &tools)?;
        thumb_for(&thumbs, &root, &config, &tools, &job);
        Ok(job)
    })
    .await
    .map_err(super::join_failed)?
}

/// Assembles one prop from part models, headless, as the next version of
/// its target asset.
#[tauri::command]
#[specta::specta]
pub async fn assemble_prop(
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    request: AssembleRequest,
) -> CommandResult<Assembled> {
    let (root, config) = session.workspace()?;
    let tools = toolchain::toolbox(&session).await?;
    let thumbs = service.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let made = run_assembly(&root, &config, &tools, &request)?;
        version_thumb(
            &thumbs,
            &root,
            &config,
            &tools,
            &request.project,
            &made.asset,
            made.version.version,
        );
        Ok(made)
    })
    .await
    .map_err(super::join_failed)?
}

/// Assembles in a folder of the workspace cache of its own.
pub(crate) fn run_assembly(
    root: &Path,
    config: &WorkspaceConfig,
    tools: &Toolbox,
    request: &AssembleRequest,
) -> CommandResult<Assembled> {
    let mut bytes = [0u8; 6];
    getrandom::fill(&mut bytes).map_err(|e| {
        maquette_core::CoreError::Unavailable(format!("No random bytes for a work folder: {e}"))
    })?;
    let stamp = hex::encode(bytes);
    let work = root
        .join(&config.layout.cache)
        .join("assemblies")
        .join(format!("{}-{stamp}", request.project));
    let made = assemble::assemble(root, &config.layout, &work, request, tools);
    let _ = std::fs::remove_dir_all(&work);
    Ok(made?)
}

/// The recipe a prop was last assembled with, to assemble it again.
#[tauri::command(async)]
#[specta::specta]
pub fn assemble_recipe(
    session: State<'_, Session>,
    project: String,
    asset: String,
) -> CommandResult<Option<Recipe>> {
    let (root, config) = session.workspace()?;
    let dir = project::dir(&root, &config.layout, &project)?;
    Ok(assemble::recipe(&dir, &asset))
}
