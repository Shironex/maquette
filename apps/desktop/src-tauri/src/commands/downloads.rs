//! The By hand Downloads watcher: the shell side of
//! `maquette_core::providers::arrivals`. Settings turn it on or off and
//! point it at a folder (kept per machine in the app state); every file it
//! takes reaches the web view as a `DownloadsArrival` event for a quiet
//! notice. A download waiting for a look or asking which asset it is gets
//! its thumbnail through `downloads_preview` (headless Blender, from a GLB
//! copy), and is attached to an asset through `downloads_attach` or left
//! alone through `downloads_dismiss`.

use std::path::PathBuf;
use std::sync::Arc;

use maquette_core::pipeline::runner::{NullSink, PipelineRunner};
use maquette_core::providers::{Arrival, Arrivals, ProviderJobs, WatchStatus, WatchTiming};
use maquette_core::thumbs::{self as core_thumbs, PictureRenderer, ThumbState};
use maquette_core::{CoreError, app_state::DownloadsWatch};
use serde::Serialize;
use tauri::{AppHandle, Manager as _, State};
use tauri_specta::Event as _;

use super::thumbs::Thumbs;
use super::{CommandResult, toolchain};
use crate::events::DownloadsArrival;
use crate::session::Session;

/// The service the app shares between commands and the MCP tools.
pub struct Watch(pub Arrivals);

impl Watch {
    /// `data_dir` is the app's per computer folder, where the list of
    /// downloads already taken is kept.
    pub fn new(handle: AppHandle, studio: ProviderJobs, data_dir: PathBuf) -> Self {
        let tools_handle = handle.clone();
        let arrivals = Arrivals::new(studio, WatchTiming::APP, data_dir, move |arrival| {
            if let Err(err) = DownloadsArrival(arrival.clone()).emit(&handle) {
                tracing::warn!(error = %err, "could not emit a downloads event");
            }
        });
        // Downloads it cannot place by name are rendered from the front, one
        // at a time on a runner of their own, and matched by picture.
        arrivals.set_renderer(PictureRenderer::new(
            PipelineRunner::new(Arc::new(NullSink)),
            move || toolchain::toolbox_blocking(&tools_handle.state::<Session>()),
        ));
        Self(arrivals)
    }
}

/// The folder a setting points at: its own, or the system's Downloads.
pub(crate) fn folder_for(
    session: &Session,
    settings: &DownloadsWatch,
    downloads: Option<PathBuf>,
) -> Option<PathBuf> {
    settings
        .folder
        .as_deref()
        .map(|f| session.expand(f))
        .or(downloads)
}

/// Starts (or stops) the watch as the saved settings say.
pub(crate) fn apply(app: &AppHandle) {
    let (Some(session), Some(watch)) = (app.try_state::<Session>(), app.try_state::<Watch>())
    else {
        return;
    };
    let settings = session.app_state().downloads_watch;
    let folder = folder_for(&session, &settings, app.path().download_dir().ok());
    watch.0.configure(settings.enabled, folder);
}

#[tauri::command]
#[specta::specta]
pub async fn downloads_watch_status(app: AppHandle) -> CommandResult<WatchStatus> {
    super::blocking(move || status(&app.state::<Session>(), &app.state::<Watch>())).await
}

/// The watcher's status, saying whether Settings picked its folder.
fn status(session: &Session, watch: &Watch) -> WatchStatus {
    watch.0.retry_held();
    let mut status = watch.0.status();
    status.custom = session.app_state().downloads_watch.folder.is_some();
    status
}

/// Turns the watch on or off and sets its folder (`None`: Downloads). A
/// folder must exist; it is only ever read.
#[tauri::command]
#[specta::specta]
pub async fn downloads_watch_set(
    app: AppHandle,
    enabled: bool,
    folder: Option<String>,
) -> CommandResult<WatchStatus> {
    super::blocking(move || -> CommandResult<WatchStatus> {
        let session = app.state::<Session>();
        let folder = folder
            .map(|f| f.trim().to_string())
            .filter(|f| !f.is_empty());
        if let Some(dir) = folder.as_deref().map(|f| session.expand(f))
            && !dir.is_dir()
        {
            return Err(CoreError::Invalid(format!("{} is not a folder.", dir.display())).into());
        }
        session.update_app_state(|state| {
            state.downloads_watch = DownloadsWatch { enabled, folder };
        })?;
        apply(&app);
        Ok(status(&session, &app.state::<Watch>()))
    })
    .await?
}

/// Attaches a download (one waiting for a look, or asking which asset it
/// is) to an asset: into the inbox for it, as a named export goes, where
/// accepting it closes the asset's By hand sends still waiting.
#[tauri::command]
#[specta::specta]
pub async fn downloads_attach(
    watch: State<'_, Watch>,
    arrival: String,
    project: String,
    asset: String,
) -> CommandResult<Arrival> {
    let arrivals = watch.0.clone();
    Ok(
        tauri::async_runtime::spawn_blocking(move || arrivals.attach(&arrival, &project, &asset))
            .await
            .map_err(super::join_failed)??,
    )
}

/// A download's thumbnail, rendered headless from a GLB copy of it.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadPreview {
    /// The copy the render is made from: `ThumbnailReady` names it.
    pub file: String,
    pub thumbnail: Option<String>,
    pub state: ThumbState,
}

/// The thumbnail of a download waiting for a look or for its asset: the
/// cached render, or one queued that arrives as `ThumbnailReady`.
#[tauri::command]
#[specta::specta]
pub async fn downloads_preview(
    session: State<'_, Session>,
    service: State<'_, Thumbs>,
    watch: State<'_, Watch>,
    arrival: String,
) -> CommandResult<DownloadPreview> {
    let (root, config) = session.workspace()?;
    let tools = toolchain::toolbox(&session).await?;
    let arrivals = watch.0.clone();
    let file = tauri::async_runtime::spawn_blocking(move || arrivals.preview_file(&arrival))
        .await
        .map_err(super::join_failed)??;
    let found = service.0.request(
        &core_thumbs::thumbs_dir(&root, &config),
        &file,
        None,
        &tools,
    )?;
    Ok(DownloadPreview {
        file: file.display().to_string(),
        thumbnail: found.path().map(|p| p.display().to_string()),
        state: found.state(),
    })
}

/// Leaves a download alone ("Not this one", "Not a Maquette model"), and
/// never asks about it again.
#[tauri::command]
#[specta::specta]
pub fn downloads_dismiss(watch: State<'_, Watch>, arrival: String) -> CommandResult<Arrival> {
    Ok(watch.0.dismiss(&arrival)?)
}

#[cfg(test)]
mod tests {
    use maquette_core::AppState;

    use super::*;

    #[test]
    fn a_set_folder_wins_over_downloads_and_expands_home() {
        let dir = tempfile::tempdir().expect("tempdir");
        let session = Session::with_state(
            dir.path().join("cfg"),
            Some(PathBuf::from("/Users/me")),
            AppState::default(),
        );
        let downloads = Some(PathBuf::from("/Users/me/Downloads"));
        let default = DownloadsWatch::default();
        assert_eq!(
            folder_for(&session, &default, downloads.clone()),
            downloads.clone()
        );
        let own = DownloadsWatch {
            enabled: true,
            folder: Some("~/Exports".into()),
        };
        assert_eq!(
            folder_for(&session, &own, downloads),
            Some(PathBuf::from("/Users/me/Exports"))
        );
        assert_eq!(folder_for(&session, &default, None), None);
    }
}
