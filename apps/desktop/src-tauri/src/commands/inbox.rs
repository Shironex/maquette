//! The model inbox: list, check, reprocess, accept into a project, open in
//! Blender.

use maquette_core::AssetKind;
use maquette_core::inbox::{self, AcceptRequest, AcceptResult, InboxItem};
use maquette_core::thumbs::ThumbTarget;
use serde::Serialize;
use tauri::State;

use super::providers::Studio;
use super::thumbs::{self, Thumbs};
use super::{CommandResult, blender, toolchain};
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
pub async fn inbox_list(
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    studio: State<'_, Studio>,
) -> CommandResult<InboxSnapshot> {
    let (root, config) = session.workspace()?;
    let tools = toolchain::toolbox(&session).await?;
    let service = service.0.clone();
    let studio = studio.0.clone();
    let items = blocking(move || {
        let mut items = inbox::list(&root, &config)?;
        thumbs::fill(&service, &root, &config, &tools, &mut items);
        studio.link_inbox(&mut items);
        Ok::<_, maquette_core::CoreError>(items)
    })
    .await??;
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
pub async fn inbox_reprocess(
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    studio: State<'_, Studio>,
    id: String,
) -> CommandResult<InboxItem> {
    let (root, config) = session.workspace()?;
    let tools = toolchain::toolbox(&session).await?;
    let service = service.0.clone();
    let studio = studio.0.clone();
    Ok(blocking(move || {
        let mut item = inbox::reprocess(&root, &config, &id)?;
        thumbs::fill(
            &service,
            &root,
            &config,
            &tools,
            std::slice::from_mut(&mut item),
        );
        studio.link_inbox(std::slice::from_mut(&mut item));
        Ok::<_, maquette_core::CoreError>(item)
    })
    .await??)
}

#[tauri::command]
#[specta::specta]
pub async fn inbox_set_kind(
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    studio: State<'_, Studio>,
    id: String,
    kind: AssetKind,
) -> CommandResult<InboxItem> {
    let (root, config) = session.workspace()?;
    let tools = toolchain::toolbox(&session).await?;
    let mut item = inbox::set_kind(&root, &config, &id, kind)?;
    thumbs::fill(
        &service.0,
        &root,
        &config,
        &tools,
        std::slice::from_mut(&mut item),
    );
    studio.0.link_inbox(std::slice::from_mut(&mut item));
    Ok(item)
}

#[tauri::command]
#[specta::specta]
pub async fn inbox_accept(
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    studio: State<'_, Studio>,
    request: AcceptRequest,
) -> CommandResult<AcceptResult> {
    let (root, config) = session.workspace()?;
    let tools = toolchain::toolbox(&session).await?;
    let mut result = inbox::accept(&root, &config, &request)?;
    // A model from a Send to 3D: its job is done now, and so is any By hand
    // send of the asset still waiting for a model.
    studio.0.accepted(
        &request.item,
        &request.project,
        &result.asset.id,
        result.version.version,
    );
    // A Meshy split lost its texture: it bakes back from the model it came from.
    let bake_tools = tools.clone();
    super::split::spawn_bakes(
        studio.0.clone(),
        service.0.clone(),
        move || bake_tools,
        root.clone(),
        config.clone(),
    );
    // The file kept its content, so a render made while it sat in the inbox
    // is reused at once; otherwise the Board gets it from ThumbnailReady.
    let target = ThumbTarget {
        project: request.project.clone(),
        asset: result.asset.id.clone(),
        version: result.version.version,
    };
    let file = std::path::PathBuf::from(&result.file);
    if let Some(project) = thumbs::for_version(&service.0, &root, &config, &tools, &file, target) {
        if let Some(asset) = project.assets.iter().find(|a| a.id == result.asset.id) {
            result.asset = asset.clone();
        }
        result.project = project;
    }
    Ok(result)
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
        assert_eq!(snap.items[0].checks.len(), 6);
    }
}

/// What [`inbox_add_model_file`] did.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelFileAdded {
    /// The inbox item it became, ready for Accept.
    pub inbox_file: String,
    /// The By hand send it went to, when the asset had one waiting.
    pub job: Option<String>,
    pub asset: String,
    pub asset_name: String,
}

/// Adds a model file someone fixed by hand (a `.glb`, `.gltf`, `.fbx` or
/// `.obj` outside the workspace's versions) to the inbox for `asset`, as
/// its next version once accepted. The file is copied, never moved; a file
/// that is no GLB is turned into one by headless Blender first.
#[tauri::command]
#[specta::specta]
pub async fn inbox_add_model_file(
    session: State<'_, Session>,
    studio: State<'_, Studio>,
    pipelines: State<'_, super::pipeline::Pipelines>,
    project: String,
    asset: String,
    path: String,
) -> CommandResult<ModelFileAdded> {
    let (root, config) = session.workspace()?;
    let tools = toolchain::toolbox(&session).await?;
    let studio = studio.0.clone();
    let runner = pipelines.0.clone();
    let added = blocking(move || {
        maquette_core::providers::own_file::add(
            &studio,
            &runner,
            &tools,
            &maquette_core::providers::own_file::OwnFile {
                root: &root,
                config: &config,
                project: &project,
                asset: &asset,
                file: std::path::Path::new(path.trim()),
            },
        )
    })
    .await??;
    Ok(ModelFileAdded {
        inbox_file: added.inbox_file,
        job: added.job,
        asset: added.asset,
        asset_name: added.asset_name,
    })
}
