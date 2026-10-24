//! The model inbox: list, check, reprocess, accept into a project, open in
//! Blender.

use maquette_core::AssetKind;
use maquette_core::inbox::{self, AcceptRequest, AcceptResult, InboxItem};
use serde::Serialize;
use tauri::State;

use super::{CommandResult, blender};
use crate::session::Session;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InboxSnapshot {
    /// The folder, shown as `~/...`.
    pub dir: String,
    /// The full path, for the asset protocol.
    pub path: String,
    pub watching: bool,
    pub items: Vec<InboxItem>,
}

#[cfg(test)]
fn snapshot(session: &Session) -> CommandResult<InboxSnapshot> {
    let (root, config) = session.workspace()?;
    let mut snap = snapshot_shell(session)?;
    snap.items = inbox::list(&root, &config)?;
    Ok(snap)
}

async fn blocking<T: Send + 'static>(job: impl FnOnce() -> T + Send + 'static) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(super::join_failed)
}

/// Analysis can take a moment on big models, so it runs off the main thread.
#[tauri::command]
#[specta::specta]
pub async fn inbox_list(session: State<'_, Session>) -> CommandResult<InboxSnapshot> {
    let (root, config) = session.workspace()?;
    let items = blocking(move || inbox::list(&root, &config)).await??;
    let mut snap = snapshot_shell(&session)?;
    snap.items = items;
    Ok(snap)
}

/// The folder facts, without the (slow) analysis.
fn snapshot_shell(session: &Session) -> CommandResult<InboxSnapshot> {
    let (root, config) = session.workspace()?;
    let dir = inbox::inbox_dir(&root, &config);
    Ok(InboxSnapshot {
        dir: session.display(&dir),
        path: dir.display().to_string(),
        watching: session.inbox_watched(),
        items: Vec::new(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn inbox_reprocess(session: State<'_, Session>, id: String) -> CommandResult<InboxItem> {
    let (root, config) = session.workspace()?;
    Ok(blocking(move || inbox::reprocess(&root, &config, &id)).await??)
}

#[tauri::command]
#[specta::specta]
pub fn inbox_set_kind(
    session: State<'_, Session>,
    id: String,
    kind: AssetKind,
) -> CommandResult<InboxItem> {
    let (root, config) = session.workspace()?;
    Ok(inbox::set_kind(&root, &config, &id, kind)?)
}

#[tauri::command]
#[specta::specta]
pub fn inbox_accept(
    session: State<'_, Session>,
    request: AcceptRequest,
) -> CommandResult<AcceptResult> {
    let (root, config) = session.workspace()?;
    Ok(inbox::accept(&root, &config, &request)?)
}

/// Opens the model in Blender's window, imported into an empty scene.
#[tauri::command]
#[specta::specta]
pub async fn inbox_open_in_blender(session: State<'_, Session>, id: String) -> CommandResult<()> {
    let (root, config) = session.workspace()?;
    let found = inbox::item(&root, &config, &id)?;
    blender::open_gui(&session, std::path::Path::new(&found.file)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::{AppState, WorkspaceConfig, workspace};

    #[test]
    fn the_snapshot_lists_models_in_the_inbox() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let inbox_dir = inbox::inbox_dir(&root, &config);
        std::fs::write(inbox_dir.join("crate.glb"), inbox::fixture::clean_prop()).expect("write");
        let snap = snapshot(&session).expect("snapshot");
        assert_eq!(snap.items.len(), 1);
        assert!(!snap.watching, "no app in tests, so no watcher");
        assert_eq!(snap.items[0].checks.len(), 5);
    }
}
