//! Replacing a model with a built asset in place, for the Board and for
//! agents: the plan (reads only), the replace, its undo, and the replaces no
//! Ship has written yet, which the Ship approval shows. Both sides go
//! through these helpers, so they check the same runs, builds and sends.

use maquette_core::CoreResult;
use maquette_core::assets::remove::Busy;
use maquette_core::built::replace::{
    self, ReplaceDone, ReplaceEnv, ReplaceReport, ShipSwap, UndoDone, UndoReport,
};
use maquette_core::pipeline::cache::project_cache_dir;
use maquette_core::pipeline::history;
use maquette_core::project;
use maquette_core::thumbs::ThumbTarget;
use tauri::{AppHandle, State};

use super::building;
use crate::commands::{CommandResult, join_failed, thumbs, trash};
use crate::mcp::host::{AppHost, Host};
use crate::session::Session;

/// Runs `job` with where the project is and what is going on in it.
fn with_env<T>(
    host: &dyn Host,
    project_id: &str,
    job: impl FnOnce(&ReplaceEnv<'_>) -> CoreResult<T>,
) -> CommandResult<T> {
    let (root, config) = host.session().workspace()?;
    let run = host.runner().active(project_id).0;
    let building = building(project_id);
    let env = ReplaceEnv {
        root: &root,
        config: &config,
        project: project_id,
        busy: Busy {
            run: run.as_deref(),
            building: &building,
        },
        sends: host.studio(),
    };
    Ok(job(&env)?)
}

/// What replacing `asset`'s model with the built asset `built` would change.
pub(crate) fn plan(
    host: &dyn Host,
    project_id: &str,
    asset: &str,
    built: &str,
) -> CommandResult<ReplaceReport> {
    with_env(host, project_id, |env| replace::plan(env, asset, built))
}

/// Replaces `asset`'s model with the built asset `built`, tells the windows
/// and asks for the new version's thumbnail.
pub(crate) fn run(
    host: &dyn Host,
    project_id: &str,
    asset: &str,
    built: &str,
) -> CommandResult<ReplaceDone> {
    let mut done = with_env(host, project_id, |env| replace::replace(env, asset, built))?;
    trash::changed(host, &done.project);
    let (root, config) = host.session().workspace()?;
    let file = project::dir(&root, &config.layout, project_id)?.join(&done.report.new.file);
    let target = ThumbTarget {
        project: project_id.to_string(),
        asset: done.report.asset.clone(),
        version: done.report.new.version,
    };
    let tools = host.toolbox();
    if let Some(project) = thumbs::for_version(host.thumbs(), &root, &config, &tools, &file, target)
    {
        host.project_changed(&project);
        done.project = project;
    }
    Ok(done)
}

/// What bringing back the model `asset` had before would change.
pub(crate) fn undo_plan(
    host: &dyn Host,
    project_id: &str,
    asset: &str,
) -> CommandResult<UndoReport> {
    with_env(host, project_id, |env| replace::undo_plan(env, asset))
}

/// Brings back the model `asset` had before a built asset took its place.
pub(crate) fn undo(host: &dyn Host, project_id: &str, asset: &str) -> CommandResult<UndoDone> {
    let done = with_env(host, project_id, |env| replace::undo(env, asset))?;
    trash::changed(host, &done.project);
    Ok(done)
}

/// The replaces of `project_id` that no Ship has written yet.
pub(crate) fn swaps(session: &Session, project_id: &str) -> CommandResult<Vec<ShipSwap>> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    let cache = project_cache_dir(&root.join(&config.layout.cache), project_id);
    Ok(replace::swaps(&dir, &manifest, &history::load_runs(&cache)))
}

/// What Use in place of a model would change. Reads only.
#[tauri::command]
#[specta::specta]
pub async fn built_replace_plan(
    app: AppHandle,
    project: String,
    asset: String,
    built: String,
) -> CommandResult<ReplaceReport> {
    tauri::async_runtime::spawn_blocking(move || plan(&AppHost(app), &project, &asset, &built))
        .await
        .map_err(join_failed)?
}

/// Puts the built asset's model in the place of `asset`'s. The asset keeps
/// its id, its place in the layout and its name; the built asset's own card
/// goes to the asset trash.
#[tauri::command]
#[specta::specta]
pub async fn built_replace(
    app: AppHandle,
    project: String,
    asset: String,
    built: String,
) -> CommandResult<ReplaceDone> {
    tauri::async_runtime::spawn_blocking(move || run(&AppHost(app), &project, &asset, &built))
        .await
        .map_err(join_failed)?
}

/// What Bring back the old model would change. Reads only.
#[tauri::command]
#[specta::specta]
pub async fn built_replace_undo_plan(
    app: AppHandle,
    project: String,
    asset: String,
) -> CommandResult<UndoReport> {
    tauri::async_runtime::spawn_blocking(move || undo_plan(&AppHost(app), &project, &asset))
        .await
        .map_err(join_failed)?
}

/// Undoes a replace: the model the asset had is current again and the built
/// asset's card comes back from the asset trash.
#[tauri::command]
#[specta::specta]
pub async fn built_replace_undo(
    app: AppHandle,
    project: String,
    asset: String,
) -> CommandResult<UndoDone> {
    tauri::async_runtime::spawn_blocking(move || undo(&AppHost(app), &project, &asset))
        .await
        .map_err(join_failed)?
}

/// The replaces no Ship has written yet, for the Ship approval and the Ship
/// views.
#[tauri::command(async)]
#[specta::specta]
pub fn built_swaps(session: State<'_, Session>, project: String) -> CommandResult<Vec<ShipSwap>> {
    swaps(&session, &project)
}
