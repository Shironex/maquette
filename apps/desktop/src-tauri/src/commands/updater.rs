//! The update commands the About section calls. The steps also go out as
//! `UpdaterUpdate` events, so the header pill follows a check the app made
//! on its own. See [`crate::updater`].

use tauri::{AppHandle, State};

use super::CommandResult;
use super::pipeline::Pipelines;
use crate::quit::QuitGuard;
use crate::updater::{UpdateInfo, UpdaterEvent, UpdaterStatus, Updates};

/// Whether this build updates itself, and what the last check found.
#[tauri::command]
#[specta::specta]
pub fn updater_status(updates: State<'_, Updates>) -> UpdaterStatus {
    updates.status()
}

/// Checks the feed now. Answers with the step it ended on: an update,
/// current, pending (no release out yet) or a failure.
#[tauri::command]
#[specta::specta]
pub async fn updater_check(
    app: AppHandle,
    updates: State<'_, Updates>,
) -> CommandResult<UpdaterEvent> {
    updates.check(&app).await
}

/// Downloads the update the last check found; progress goes out as events.
#[tauri::command]
#[specta::specta]
pub async fn updater_download(
    app: AppHandle,
    updates: State<'_, Updates>,
) -> CommandResult<UpdateInfo> {
    updates.download(&app).await
}

/// What stops a restart right now, if anything.
pub(crate) fn restart_blocker(work_running: bool, unapplied: bool) -> Option<&'static str> {
    if work_running {
        Some("A pipeline run or a build is still going. Let it finish or cancel it, then restart.")
    } else if unapplied {
        Some("Some changes are not applied yet. Apply or discard them, then restart.")
    } else {
        None
    }
}

/// Installs the downloaded update and restarts into it, unless work is
/// running that a restart would cut off.
#[tauri::command]
#[specta::specta]
pub fn updater_install(
    app: AppHandle,
    updates: State<'_, Updates>,
    pipelines: State<'_, Pipelines>,
    guard: State<'_, QuitGuard>,
) -> CommandResult<()> {
    if let Some(why) = restart_blocker(pipelines.0.any_active(), guard.has_unapplied()) {
        return Err(super::invalid(why));
    }
    updates.install(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_restart_waits_for_running_work_and_unapplied_changes() {
        assert!(restart_blocker(true, false).is_some_and(|w| w.contains("pipeline run")));
        assert!(restart_blocker(false, true).is_some_and(|w| w.contains("not applied")));
        assert!(restart_blocker(true, true).is_some_and(|w| w.contains("pipeline run")));
        assert_eq!(restart_blocker(false, false), None);
    }
}
