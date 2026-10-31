//! Remove from project and Restore: assets out of a project into the
//! workspace trash and back. The Board's commands and the MCP tools share
//! these helpers, so both check the same runs, builds and sends.

use maquette_core::assets::remove::{self, AssetRestored, AssetsRemoved, Busy, TrashEntry};
use tauri::AppHandle;

use super::CommandResult;
use super::built::building;
use crate::mcp::host::{AppHost, Host};
use crate::session::Session;

/// Tells the windows about every file a remove or restore touches.
fn changed(host: &dyn Host, project: &maquette_core::ProjectManifest) {
    host.project_changed(project);
    host.layout_changed(&project.id);
    host.prepare_changed(&project.id);
    host.materials_changed(&project.id);
    host.built_changed(&project.id);
}

/// Removes `ids` from `project_id`, stopping their waiting sends. Refused
/// while the project's pipeline runs.
pub(crate) fn remove(
    host: &dyn Host,
    project_id: &str,
    ids: &[String],
) -> CommandResult<AssetsRemoved> {
    let (root, config) = host.session().workspace()?;
    let run = host.runner().active(project_id).0;
    let building = building(project_id);
    let busy = Busy {
        run: run.as_deref(),
        building: &building,
    };
    let done = remove::remove_assets(&root, &config.layout, project_id, ids, &busy, host.studio())?;
    changed(host, &done.project);
    Ok(done)
}

/// Puts the removed asset `trash` back into `project_id`.
pub(crate) fn restore(
    host: &dyn Host,
    project_id: &str,
    trash: &str,
) -> CommandResult<AssetRestored> {
    let (root, config) = host.session().workspace()?;
    let run = host.runner().active(project_id).0;
    let busy = Busy {
        run: run.as_deref(),
        building: &[],
    };
    let done = remove::restore(&root, &config.layout, project_id, trash, &busy)?;
    changed(host, &done.project);
    Ok(done)
}

pub(crate) fn list(session: &Session, project_id: &str) -> CommandResult<Vec<TrashEntry>> {
    let (root, config) = session.workspace()?;
    Ok(remove::list_trash(&root, &config.layout, project_id)?)
}

/// Removes assets from the project into the workspace trash.
#[tauri::command]
#[specta::specta]
pub async fn asset_remove(
    app: AppHandle,
    project: String,
    ids: Vec<String>,
) -> CommandResult<AssetsRemoved> {
    // Moving the files can take a moment, so this runs off the UI thread.
    tauri::async_runtime::spawn_blocking(move || remove(&AppHost(app), &project, &ids))
        .await
        .map_err(super::join_failed)?
}

/// The assets removed from the project, newest first.
#[tauri::command(async)]
#[specta::specta]
pub fn asset_trash_list(
    session: tauri::State<'_, Session>,
    project: String,
) -> CommandResult<Vec<TrashEntry>> {
    list(&session, &project)
}

/// Puts a removed asset back on the Board.
#[tauri::command]
#[specta::specta]
pub async fn asset_restore(
    app: AppHandle,
    project: String,
    trash: String,
) -> CommandResult<AssetRestored> {
    tauri::async_runtime::spawn_blocking(move || restore(&AppHost(app), &project, &trash))
        .await
        .map_err(super::join_failed)?
}
