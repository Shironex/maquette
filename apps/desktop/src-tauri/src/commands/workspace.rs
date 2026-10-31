use maquette_core::workspace::sync::SyncConflict;
use maquette_core::{CoreError, RootInspection, WorkspaceConfig, workspace};
use serde::Serialize;
use tauri::{AppHandle, Manager as _, State};

use super::app::{WorkspaceSnapshot, snapshot};
use super::{CommandResult, blocking};
use crate::mcp::Agents;
use crate::session::Session;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CacheInfo {
    /// Bytes as a JS safe number.
    pub bytes: f64,
}

#[expect(
    clippy::cast_precision_loss,
    reason = "cache sizes stay far below 2^53"
)]
fn cache_info(bytes: u64) -> CacheInfo {
    CacheInfo {
        bytes: bytes as f64,
    }
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_inspect(app: AppHandle, root: String) -> CommandResult<RootInspection> {
    blocking(move || -> CommandResult<RootInspection> {
        let session = app.state::<Session>();
        Ok(workspace::inspect_root(
            &session.expand(&root),
            session.home(),
        )?)
    })
    .await?
}

#[tauri::command(async)]
#[specta::specta]
pub fn workspace_create(
    app: AppHandle,
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    root: String,
    config: WorkspaceConfig,
) -> CommandResult<WorkspaceSnapshot> {
    let root = session.expand(&root);
    let config = workspace::create(&root, &config)?;
    session.set_workspace(root.clone(), config.clone())?;
    super::agents::sync(&app, &session, &agents);
    snapshot(&session, &root, &config)
}

/// Opens a workspace. While another computer has it open (a fresh lock)
/// it answers `workspace-in-use` with the warning to show, unless `force`
/// (the person chose Open anyway).
#[tauri::command(async)]
#[specta::specta]
pub fn workspace_open(
    app: AppHandle,
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    root: String,
    force: bool,
) -> CommandResult<WorkspaceSnapshot> {
    let root = session.expand(&root);
    let config = workspace::open(&root)?;
    if !force && let Some(lock) = crate::workspace_lock::held_elsewhere(&session, &root) {
        return Err(crate::workspace_lock::in_use(&lock));
    }
    session.set_workspace(root.clone(), config.clone())?;
    super::agents::sync(&app, &session, &agents);
    snapshot(&session, &root, &config)
}

#[tauri::command(async)]
#[specta::specta]
pub fn workspace_save_config(
    app: AppHandle,
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    config: WorkspaceConfig,
) -> CommandResult<WorkspaceConfig> {
    // The switcher owns the current project; a settings save never moves it.
    let (_, config) = session.update_config(|open| {
        *open = WorkspaceConfig {
            current_project: open.current_project.take(),
            ..config
        };
    })?;
    // Turning the server on or off, or a new port, takes effect at once.
    super::agents::sync(&app, &session, &agents);
    Ok(config)
}

/// Conflict copies the sync tool kept of the open workspace's data files.
#[tauri::command]
#[specta::specta]
pub async fn workspace_sync_conflicts(
    session: State<'_, Session>,
) -> CommandResult<Vec<SyncConflict>> {
    let (root, config) = session.workspace()?;
    tauri::async_runtime::spawn_blocking(move || workspace::sync::conflicts(&root, &config))
        .await
        .map_err(super::join_failed)
}

/// Shows a file of the open workspace (a conflict copy, say) selected in
/// Finder or Explorer. `path` is relative to the workspace, as
/// `workspace_sync_conflicts` gives it, and must stay inside it.
#[tauri::command(async)]
#[specta::specta]
pub fn workspace_reveal(session: State<'_, Session>, path: String) -> CommandResult<()> {
    let (root, _) = session.workspace()?;
    let inside = !maquette_core::fs::looks_absolute(&path)
        && path.split(['/', '\\']).all(|part| part != "..");
    if !inside {
        return Err(super::invalid("Show only files inside the workspace."));
    }
    let file = maquette_core::fs::from_workspace(&root, &path);
    if !file.exists() {
        return Err(CoreError::NotFound(format!("the file {path}")).into());
    }
    crate::opener::reveal_file(&file).map_err(|err| CoreError::io(&file, err).payload())
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_cache_size(session: State<'_, Session>) -> CommandResult<CacheInfo> {
    let (root, config) = session.workspace()?;
    let bytes = tauri::async_runtime::spawn_blocking(move || workspace::cache_size(&root, &config))
        .await
        .map_err(super::join_failed)?;
    Ok(cache_info(bytes))
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_clear_cache(session: State<'_, Session>) -> CommandResult<CacheInfo> {
    let (root, config) = session.workspace()?;
    tauri::async_runtime::spawn_blocking(move || workspace::clear_cache(&root, &config))
        .await
        .map_err(super::join_failed)??;
    Ok(cache_info(0))
}
