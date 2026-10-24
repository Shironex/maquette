use maquette_core::{RootInspection, WorkspaceConfig, workspace};
use serde::Serialize;
use tauri::{AppHandle, State};

use super::CommandResult;
use super::app::{WorkspaceSnapshot, snapshot};
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
pub fn workspace_inspect(
    session: State<'_, Session>,
    root: String,
) -> CommandResult<RootInspection> {
    Ok(workspace::inspect_root(
        &session.expand(&root),
        session.home(),
    )?)
}

#[tauri::command]
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

#[tauri::command]
#[specta::specta]
pub fn workspace_open(
    app: AppHandle,
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    root: String,
) -> CommandResult<WorkspaceSnapshot> {
    let root = session.expand(&root);
    let config = workspace::open(&root)?;
    session.set_workspace(root.clone(), config.clone())?;
    super::agents::sync(&app, &session, &agents);
    snapshot(&session, &root, &config)
}

#[tauri::command]
#[specta::specta]
pub fn workspace_save_config(
    app: AppHandle,
    session: State<'_, Session>,
    agents: State<'_, Agents>,
    config: WorkspaceConfig,
) -> CommandResult<WorkspaceConfig> {
    let (root, open) = session.workspace()?;
    // The switcher owns the current project; a settings save never moves it.
    let config = WorkspaceConfig {
        current_project: open.current_project,
        ..config
    };
    workspace::save(&root, &config)?;
    session.set_config(config.clone());
    // Turning the server on or off, or a new port, takes effect at once.
    super::agents::sync(&app, &session, &agents);
    Ok(config)
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
