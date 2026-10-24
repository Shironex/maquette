use maquette_core::{RootInspection, WorkspaceConfig, workspace};
use serde::Serialize;
use tauri::State;

use super::CommandResult;
use super::app::{WorkspaceSnapshot, snapshot};
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
    session: State<'_, Session>,
    root: String,
    config: WorkspaceConfig,
) -> CommandResult<WorkspaceSnapshot> {
    let root = session.expand(&root);
    let config = workspace::create(&root, &config)?;
    session.set_workspace(root.clone(), config.clone())?;
    snapshot(&session, &root, &config)
}

#[tauri::command]
#[specta::specta]
pub fn workspace_open(
    session: State<'_, Session>,
    root: String,
) -> CommandResult<WorkspaceSnapshot> {
    let root = session.expand(&root);
    let config = workspace::open(&root)?;
    session.set_workspace(root.clone(), config.clone())?;
    snapshot(&session, &root, &config)
}

#[tauri::command]
#[specta::specta]
pub fn workspace_save_config(
    session: State<'_, Session>,
    config: WorkspaceConfig,
) -> CommandResult<WorkspaceConfig> {
    let (root, _) = session.workspace()?;
    workspace::save(&root, &config)?;
    session.set_config(config.clone());
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
