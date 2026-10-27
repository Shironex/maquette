//! The managed updater state and the plugin calls. A check finds an update,
//! a download fetches and verifies it, and an install puts it in place and
//! restarts. The user decides between each step, so the found update and
//! the downloaded bytes are kept here between the commands.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use maquette_core::{CoreError, ErrorPayload};
use tauri::AppHandle;
use tauri_plugin_updater::{Update, UpdaterExt as _};
use tauri_specta::Event as _;

use super::{CHECK_EVERY, FIRST_CHECK, UpdateInfo, UpdaterEvent, UpdaterMode, UpdaterStatus};
use crate::events::UpdaterUpdate;

/// The updater as the app manages it.
pub struct Updates {
    mode: UpdaterMode,
    endpoint: Option<String>,
    found: Mutex<Option<Update>>,
    ready: Mutex<Option<(Update, Vec<u8>)>>,
    /// A check or a download is going.
    busy: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn describe(update: &Update) -> UpdateInfo {
    UpdateInfo {
        version: update.version.clone(),
        notes: update.body.clone().filter(|notes| !notes.trim().is_empty()),
        date: update.date.map(|date| date.date().to_string()),
    }
}

/// Sends a step to every window. A window that cannot hear it is not worth
/// failing the step for.
pub(crate) fn emit(app: &AppHandle, event: UpdaterEvent) {
    if let Err(err) = UpdaterUpdate(event).emit(app) {
        tracing::warn!(error = %err, "could not emit an updater event");
    }
}

fn off(mode: UpdaterMode) -> ErrorPayload {
    let why = match mode {
        UpdaterMode::Unreleased => "Updates start with the first public release.",
        UpdaterMode::DevBuild => "A development build does not update itself.",
        UpdaterMode::TurnedOff => "Updates are turned off for this run.",
        UpdaterMode::On => "Updates are on.",
    };
    CoreError::Invalid(why.to_string()).payload()
}

/// Clears `busy` when a step ends, however it ends.
struct BusyGuard<'a>(&'a AtomicBool);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl Updates {
    pub fn new(mode: UpdaterMode, endpoint: Option<String>) -> Self {
        Self {
            mode,
            endpoint,
            found: Mutex::new(None),
            ready: Mutex::new(None),
            busy: AtomicBool::new(false),
        }
    }

    pub fn mode(&self) -> UpdaterMode {
        self.mode
    }

    pub fn status(&self) -> UpdaterStatus {
        let ready = lock(&self.ready)
            .as_ref()
            .map(|(update, _)| describe(update));
        UpdaterStatus {
            mode: self.mode,
            endpoint: self.endpoint.clone(),
            available: ready
                .clone()
                .or_else(|| lock(&self.found).as_ref().map(describe)),
            ready: ready.is_some(),
        }
    }

    fn start(&self) -> Result<BusyGuard<'_>, ErrorPayload> {
        if !self.mode.is_on() {
            return Err(off(self.mode));
        }
        if self.busy.swap(true, Ordering::SeqCst) {
            return Err(CoreError::Invalid(
                "An update check or download is already going.".to_string(),
            )
            .payload());
        }
        Ok(BusyGuard(&self.busy))
    }

    /// Asks the feed for a newer version. A failure comes back as an event
    /// (and as the answer), never as a rejection, so a background check that
    /// fails offline stays quiet.
    pub async fn check(&self, app: &AppHandle) -> Result<UpdaterEvent, ErrorPayload> {
        if lock(&self.ready).is_some() {
            return Ok(self.ready_event());
        }
        let _busy = self.start()?;
        emit(app, UpdaterEvent::Checking);
        let handle = app.clone();
        let built = app
            .updater_builder()
            .on_before_exit(move || crate::shutdown(&handle))
            .build();
        let event = match built {
            Err(err) => UpdaterEvent::failed(err.to_string()),
            Ok(updater) => match updater.check().await {
                Ok(Some(update)) => {
                    let info = describe(&update);
                    *lock(&self.found) = Some(update);
                    UpdaterEvent::Available { update: info }
                }
                Ok(None) => {
                    *lock(&self.found) = None;
                    UpdaterEvent::Current
                }
                Err(err) => UpdaterEvent::failed(err.to_string()),
            },
        };
        if let UpdaterEvent::Failed { message } = &event {
            tracing::warn!(error = %message, "the update check failed");
        }
        emit(app, event.clone());
        Ok(event)
    }

    fn ready_event(&self) -> UpdaterEvent {
        match lock(&self.ready).as_ref() {
            Some((update, _)) => UpdaterEvent::Ready {
                update: describe(update),
            },
            None => UpdaterEvent::Current,
        }
    }

    /// Downloads the update the last check found and checks its signature.
    pub async fn download(&self, app: &AppHandle) -> Result<UpdateInfo, ErrorPayload> {
        let _busy = self.start()?;
        let Some(update) = lock(&self.found).clone() else {
            return Err(CoreError::Invalid(
                "There is no update to download. Check for updates first.".to_string(),
            )
            .payload());
        };
        let info = describe(&update);
        let progress = app.clone();
        let mut downloaded = 0_f64;
        let outcome = update
            .download(
                move |chunk, total| {
                    downloaded += chunk as f64;
                    emit(
                        &progress,
                        UpdaterEvent::Progress {
                            downloaded,
                            total: total.map(|t| t as f64),
                        },
                    );
                },
                || {},
            )
            .await;
        match outcome {
            Ok(bytes) => {
                *lock(&self.found) = None;
                *lock(&self.ready) = Some((update, bytes));
                emit(
                    app,
                    UpdaterEvent::Ready {
                        update: info.clone(),
                    },
                );
                Ok(info)
            }
            Err(err) => {
                let message = err.to_string();
                tracing::warn!(error = %message, "the update download failed");
                emit(app, UpdaterEvent::failed(message.clone()));
                Err(
                    CoreError::Unavailable(format!("The update did not download. {message}"))
                        .payload(),
                )
            }
        }
    }

    /// Installs the downloaded update and restarts into it. On Windows the
    /// installer ends this process itself (after the plugin's before exit
    /// hook ran the shutdown); on macOS the bundle is swapped in place and
    /// the restart is ours.
    pub fn install(&self, app: &AppHandle) -> Result<(), ErrorPayload> {
        if !self.mode.is_on() {
            return Err(off(self.mode));
        }
        let Some((update, bytes)) = lock(&self.ready).take() else {
            return Err(CoreError::Invalid(
                "There is no downloaded update to install.".to_string(),
            )
            .payload());
        };
        if let Err(err) = update.install(&bytes) {
            let message = err.to_string();
            tracing::warn!(error = %message, "the update did not install");
            emit(app, UpdaterEvent::failed(message.clone()));
            return Err(
                CoreError::Unavailable(format!("The update did not install. {message}")).payload(),
            );
        }
        crate::shutdown(app);
        app.restart()
    }
}

/// The automatic checks: one shortly after launch, then every few hours.
/// Only an updater that is on schedules any.
pub(crate) fn schedule(app: &AppHandle, updates: &Updates) {
    if !updates.mode().is_on() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut wait = FIRST_CHECK;
        loop {
            tokio::time::sleep(wait).await;
            wait = CHECK_EVERY;
            let Some(updates) = tauri::Manager::try_state::<Updates>(&app) else {
                return;
            };
            // A check already going, or a download, is enough for now.
            if let Err(err) = updates.check(&app).await {
                tracing::debug!(error = %err.message, "skipped an automatic update check");
            }
        }
    });
}
