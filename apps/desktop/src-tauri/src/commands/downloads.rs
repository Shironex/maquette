//! The By hand Downloads watcher: the shell side of
//! `maquette_core::providers::arrivals`. Settings turn it on or off and
//! point it at a folder (kept per machine in the app state); every file it
//! takes reaches the web view as a `DownloadsArrival` event for a quiet
//! notice, and "Which asset is this?" answers through `downloads_assign`.

use std::path::PathBuf;

use maquette_core::providers::{Arrival, Arrivals, ProviderJobs, WatchStatus, WatchTiming};
use maquette_core::{CoreError, app_state::DownloadsWatch};
use tauri::{AppHandle, Manager as _, State};
use tauri_specta::Event as _;

use super::CommandResult;
use crate::events::DownloadsArrival;
use crate::session::Session;

/// The service the app shares between commands and the MCP tools.
pub struct Watch(pub Arrivals);

impl Watch {
    /// `data_dir` is the app's per computer folder, where the list of
    /// downloads already taken is kept.
    pub fn new(handle: AppHandle, studio: ProviderJobs, data_dir: PathBuf) -> Self {
        Self(Arrivals::new(
            studio,
            WatchTiming::APP,
            data_dir,
            move |arrival| {
                if let Err(err) = DownloadsArrival(arrival.clone()).emit(&handle) {
                    tracing::warn!(error = %err, "could not emit a downloads event");
                }
            },
        ))
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
pub fn downloads_watch_status(
    session: State<'_, Session>,
    watch: State<'_, Watch>,
) -> CommandResult<WatchStatus> {
    Ok(status(&session, &watch))
}

/// The watcher's status, saying whether Settings picked its folder.
fn status(session: &Session, watch: &Watch) -> WatchStatus {
    let mut status = watch.0.status();
    status.custom = session.app_state().downloads_watch.folder.is_some();
    status
}

/// Turns the watch on or off and sets its folder (`None`: Downloads). A
/// folder must exist; it is only ever read.
#[tauri::command]
#[specta::specta]
pub fn downloads_watch_set(
    app: AppHandle,
    session: State<'_, Session>,
    watch: State<'_, Watch>,
    enabled: bool,
    folder: Option<String>,
) -> CommandResult<WatchStatus> {
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
    Ok(status(&session, &watch))
}

/// "Which asset is this?": copies a download into the inbox for `job`.
#[tauri::command]
#[specta::specta]
pub async fn downloads_assign(
    watch: State<'_, Watch>,
    arrival: String,
    job: String,
) -> CommandResult<Arrival> {
    let arrivals = watch.0.clone();
    Ok(
        tauri::async_runtime::spawn_blocking(move || arrivals.assign(&arrival, &job))
            .await
            .map_err(super::join_failed)??,
    )
}

/// Leaves a download alone, and never asks about it again.
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
