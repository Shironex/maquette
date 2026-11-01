//! Model thumbnails: the shell side of `maquette_core::thumbs`. Inbox rows
//! ask for one when they are listed, Accept and Import models ask for the
//! new versions', the Board's Render previews for every model without one,
//! and finished renders reach the web view as `ThumbnailReady` events.

use std::path::{Path, PathBuf};

use maquette_core::inbox::InboxItem;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::thumbs::{self, Finished, Requested, ThumbState, ThumbTarget, Thumbnails};
use maquette_core::{ProjectManifest, WorkspaceConfig, project};
use tauri::{AppHandle, Manager as _, State};
use tauri_specta::Event as _;

use serde::Serialize;

use super::{CommandResult, toolchain};
use crate::events::ThumbnailReady;
use crate::session::Session;

/// The service the app shares between commands.
pub struct Thumbs(pub Thumbnails);

impl Thumbs {
    /// A service whose finished renders update the Board and tell the web view.
    pub fn new(handle: AppHandle) -> Self {
        Self(Thumbnails::new(move |done| finished(&handle, done)))
    }
}

fn emit(handle: &AppHandle, event: ThumbnailReady) {
    if let Err(err) = event.emit(handle) {
        tracing::warn!(error = %err, "could not emit a thumbnail event");
    }
}

fn finished(handle: &AppHandle, done: Finished) {
    let state = if done.png.is_some() {
        ThumbState::Ready
    } else {
        ThumbState::Fallback
    };
    if let Some(error) = &done.error {
        tracing::info!(file = %done.file.display(), %error, "no Blender thumbnail, the preview stands in");
    }
    let thumbnail = done.png.as_ref().map(|p| p.display().to_string());
    let file = done.file.display().to_string();
    let bytes = done.fingerprint.map(|f| f.bytes);
    let modified_ms = done.fingerprint.map(|f| f.modified_ms);
    let workspace = handle.state::<Session>().workspace().ok();
    for target in &done.targets {
        let (Some(png), Some((root, config))) = (&done.png, &workspace) else {
            continue;
        };
        match thumbs::apply(root, config, target, png) {
            Ok(Some(project)) => emit(
                handle,
                ThumbnailReady {
                    file: file.clone(),
                    bytes,
                    modified_ms,
                    thumbnail: thumbnail.clone(),
                    state,
                    project: Some(project),
                },
            ),
            Ok(None) => {}
            Err(err) => tracing::warn!(error = %err, "could not give the asset its thumbnail"),
        }
    }
    emit(
        handle,
        ThumbnailReady {
            file,
            bytes,
            modified_ms,
            thumbnail,
            state,
            project: None,
        },
    );
}

/// Gives each readable inbox model its thumbnail, or queues a render.
pub(crate) fn fill(
    service: &Thumbnails,
    root: &Path,
    config: &WorkspaceConfig,
    tools: &Toolbox,
    items: &mut [InboxItem],
) {
    let dir = thumbs::thumbs_dir(root, config);
    for item in items.iter_mut().filter(|i| i.report.is_some()) {
        match service.request(&dir, Path::new(&item.file), None, tools) {
            Ok(found) => {
                item.thumbnail_state = found.state();
                item.thumbnail = found.path().map(|p| p.display().to_string());
            }
            Err(err) => tracing::warn!(error = %err, item = %item.id, "no thumbnail"),
        }
    }
}

/// What asking for one asset version's thumbnail led to.
pub(crate) enum Asked {
    /// A cached render was applied (the changed project comes back) or
    /// found already applied.
    Ready(Option<ProjectManifest>),
    /// A render is queued; `ThumbnailReady` follows.
    Rendering,
    /// No render can come, with the reason.
    Failed(String),
}

/// Asks for the thumbnail of an asset version and says what came of it.
pub(crate) fn ask_version(
    service: &Thumbnails,
    root: &Path,
    config: &WorkspaceConfig,
    tools: &Toolbox,
    file: &Path,
    target: ThumbTarget,
) -> Asked {
    let dir = thumbs::thumbs_dir(root, config);
    match service.request(&dir, file, Some(target.clone()), tools) {
        Ok(Requested::Ready(png)) => Asked::Ready(
            thumbs::apply(root, config, &target, &png)
                .inspect_err(|err| tracing::warn!(error = %err, "could not apply the thumbnail"))
                .ok()
                .flatten(),
        ),
        Ok(Requested::Rendering) => Asked::Rendering,
        Ok(Requested::Fallback(reason)) => Asked::Failed(reason),
        Err(err) => {
            tracing::warn!(error = %err, "no thumbnail for the new version");
            Asked::Failed(err.to_string())
        }
    }
}

/// Asks for the thumbnail of an asset version. A cached one is applied right
/// away and the changed project comes back; a render applies it later.
pub(crate) fn for_version(
    service: &Thumbnails,
    root: &Path,
    config: &WorkspaceConfig,
    tools: &Toolbox,
    file: &Path,
    target: ThumbTarget,
) -> Option<maquette_core::ProjectManifest> {
    match ask_version(service, root, config, tools, file, target) {
        Asked::Ready(project) => project,
        Asked::Rendering | Asked::Failed(_) => None,
    }
}

/// What Render previews asked for.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewsAsked {
    /// Models that had no preview and were asked for one.
    pub asked: u32,
    /// The project, with the previews a cached render gave at once.
    pub project: ProjectManifest,
    /// Models whose render is queued: each one ends in a `ThumbnailReady`.
    pub rendering: u32,
    /// The model file of each of those, as its `ThumbnailReady` will name it.
    pub rendering_files: Vec<String>,
    /// Models no render could be queued for (Blender missing, a model it
    /// cannot read), so nothing more will come for them.
    pub failed: u32,
    /// Why the first of them failed.
    pub reason: Option<String>,
}

/// Asks for the thumbnail of every model in the project whose card shows
/// none, or of the assets in `only`. A cached render is applied at once;
/// the others arrive as `ThumbnailReady` while Blender works through them.
pub(crate) fn render_missing(
    service: &Thumbnails,
    root: &Path,
    config: &WorkspaceConfig,
    tools: &Toolbox,
    project_id: &str,
    only: Option<&[String]>,
) -> CommandResult<PreviewsAsked> {
    let manifest = project::load(root, &config.layout, project_id)?;
    let dir = project::dir(root, &config.layout, project_id)?;
    let missing = thumbs::missing::missing(&dir, &manifest, only);
    let mut latest = manifest;
    let (mut rendering_files, mut failed, mut reason) = (Vec::new(), 0_u32, None);
    for item in &missing {
        match ask_version(
            service,
            root,
            config,
            tools,
            &item.file,
            item.target.clone(),
        ) {
            Asked::Ready(changed) => latest = changed.unwrap_or(latest),
            Asked::Rendering => rendering_files.push(item.file.display().to_string()),
            Asked::Failed(why) => {
                failed += 1;
                reason.get_or_insert(why);
            }
        }
    }
    Ok(PreviewsAsked {
        asked: u32::try_from(missing.len()).unwrap_or(u32::MAX),
        project: latest,
        rendering: u32::try_from(rendering_files.len()).unwrap_or(u32::MAX),
        rendering_files,
        failed,
        reason,
    })
}

/// Renders a preview for every model on the Board that has none, in the
/// background: the cards update as each render finishes.
#[tauri::command]
#[specta::specta]
pub async fn thumbnail_render_missing(
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    project: String,
) -> CommandResult<PreviewsAsked> {
    let (root, config) = session.workspace()?;
    let tools = toolchain::toolbox(&session).await?;
    render_missing(&service.0, &root, &config, &tools, &project, None)
}

/// Keeps a PNG the web view rendered from its preview (when Blender cannot)
/// as the thumbnail of `file`, a model inside the open workspace.
pub(crate) fn store(session: &Session, file: &str, png: &[u8]) -> CommandResult<String> {
    let (root, config) = session.workspace()?;
    let path = PathBuf::from(file);
    let inside = path
        .canonicalize()
        .ok()
        .zip(root.canonicalize().ok())
        .is_some_and(|(p, r)| p.starts_with(r));
    if !inside {
        return Err(super::invalid(
            "Only models inside the workspace get thumbnails.",
        ));
    }
    let saved = thumbs::store(&thumbs::thumbs_dir(&root, &config), &path, png)?;
    Ok(saved.display().to_string())
}

#[tauri::command(async)]
#[specta::specta]
pub fn thumbnail_store(
    session: State<'_, Session>,
    file: String,
    png: Vec<u8>,
) -> CommandResult<String> {
    store(&session, &file, &png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::{AppState, workspace};

    #[test]
    fn store_keeps_pngs_for_workspace_models_only() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let model = maquette_core::inbox::inbox_dir(&root, &config).join("crate.glb");
        std::fs::write(&model, b"glb").expect("model");
        let png = b"\x89PNG\r\n\x1a\nrest";
        let saved = store(&session, &model.display().to_string(), png).expect("store");
        assert!(Path::new(&saved).starts_with(thumbs::thumbs_dir(&root, &config)));

        let outside = dir.path().join("elsewhere.glb");
        std::fs::write(&outside, b"glb").expect("outside");
        assert_eq!(
            store(&session, &outside.display().to_string(), png)
                .expect_err("outside")
                .code,
            "invalid"
        );
    }
}
