//! The shell side of `maquette_core::workspace::lock`: who this computer
//! is, the check before a workspace opens, the heartbeat while it is open
//! and the release on a clean exit.

use std::path::Path;
use std::time::Duration;

use maquette_core::pipeline::history::now_ms;
use maquette_core::workspace::lock::{self, HEARTBEAT_MS, Holder, LockCheck, WorkspaceLock};
use maquette_core::{CoreError, CoreResult, ErrorPayload, HostOs};
use tauri::{AppHandle, Manager as _};

use crate::session::Session;

/// The error code `workspace_open` answers with while another computer has
/// the workspace open.
pub const IN_USE: &str = "workspace-in-use";

/// This computer's random id, made once and kept in the app state.
fn machine_id(session: &Session) -> CoreResult<String> {
    if let Some(id) = session.app_state().machine_id {
        return Ok(id);
    }
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|e| CoreError::Unavailable(format!("Could not make a machine id. {e}")))?;
    let id = hex::encode(bytes);
    let saved = id.clone();
    session.update_app_state(move |state| state.machine_id = Some(saved))?;
    Ok(id)
}

/// This computer and this run of the app.
pub(crate) fn holder(session: &Session) -> Holder {
    let machine = machine_id(session).unwrap_or_else(|err| {
        tracing::warn!(error = %err, "no machine id, the lock matches by name");
        String::new()
    });
    let os = match HostOs::current() {
        HostOs::Mac => "macos",
        HostOs::Windows => "windows",
        HostOs::Linux => "linux",
    };
    Holder {
        host: session.host_name(),
        machine,
        os: os.to_string(),
        app_version: session.app_version(),
        pid: std::process::id(),
    }
}

/// Another computer's fresh lock on `root`, if there is one.
pub(crate) fn held_elsewhere(session: &Session, root: &Path) -> Option<WorkspaceLock> {
    match lock::check(root, &holder(session), now_ms()) {
        LockCheck::Held(lock) => Some(lock),
        LockCheck::Free | LockCheck::Ours(_) | LockCheck::Stale(_) => None,
    }
}

/// What the warning says.
pub(crate) fn in_use_message(lock: &WorkspaceLock) -> String {
    format!(
        "This workspace is open on {}. Close Maquette there first, or Syncthing may create conflict copies.",
        lock.host
    )
}

/// The error `workspace_open` answers with, so the UI can ask first.
pub(crate) fn in_use(lock: &WorkspaceLock) -> ErrorPayload {
    ErrorPayload {
        code: IN_USE.to_string(),
        message: in_use_message(lock),
    }
}

/// Writes this computer's lock into `root`. A stale or same machine lock is
/// taken over without a word.
pub(crate) fn claim(session: &Session, root: &Path) {
    if let Err(err) = lock::claim(root, &holder(session), now_ms()) {
        tracing::warn!(error = %err, "could not write the workspace lock");
    }
}

/// Removes this computer's lock from `root`.
pub(crate) fn release(session: &Session, root: &Path) {
    if let Err(err) = lock::release(root, &holder(session)) {
        tracing::warn!(error = %err, "could not remove the workspace lock");
    }
}

/// Refreshes the open workspace's heartbeat about once a minute, for as
/// long as the app runs.
pub fn start_heartbeat(handle: AppHandle) {
    let spawned = std::thread::Builder::new()
        .name("workspace-lock".into())
        .spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(HEARTBEAT_MS));
                let Some(session) = handle.try_state::<Session>() else {
                    continue;
                };
                let Ok((root, _)) = session.workspace() else {
                    continue;
                };
                match lock::beat(&root, &holder(&session), now_ms()) {
                    Ok(true) => {}
                    Ok(false) => tracing::warn!(
                        holder = ?lock::read(&root).map(|l| l.host),
                        "another computer took the workspace over"
                    ),
                    Err(err) => {
                        tracing::warn!(error = %err, "could not refresh the workspace lock")
                    }
                }
            }
        });
    if let Err(err) = spawned {
        tracing::warn!(error = %err, "the workspace lock has no heartbeat");
    }
}

/// Lets go of the open workspace's lock, for app exit.
pub fn release_open(handle: &AppHandle) {
    if let Some(session) = handle.try_state::<Session>()
        && let Ok((root, _)) = session.workspace()
    {
        release(&session, &root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::AppState;

    #[test]
    fn the_machine_id_is_made_once_and_kept() {
        let dir = tempfile::tempdir().expect("temp dir");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        let first = holder(&session);
        assert_eq!(first.machine.len(), 32);
        assert_eq!(holder(&session).machine, first.machine);
        let saved = maquette_core::app_state::load(&dir.path().join("cfg"));
        assert_eq!(saved.machine_id, Some(first.machine));
    }

    #[test]
    fn another_computers_fresh_lock_is_reported_and_ours_is_not() {
        let dir = tempfile::tempdir().expect("temp dir");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        let root = dir.path().join("ws");
        std::fs::create_dir_all(&root).expect("ws");
        assert!(held_elsewhere(&session, &root).is_none());
        claim(&session, &root);
        assert!(held_elsewhere(&session, &root).is_none(), "our own lock");

        let other = Holder {
            host: "Studio PC".into(),
            machine: "another".into(),
            os: "windows".into(),
            app_version: "0.1.0".into(),
            pid: 4,
        };
        lock::claim(&root, &other, now_ms()).expect("other claims");
        let held = held_elsewhere(&session, &root).expect("held");
        assert_eq!(
            in_use(&held).message,
            "This workspace is open on Studio PC. Close Maquette there first, or Syncthing may create conflict copies."
        );
        release(&session, &root);
        assert!(lock::read(&root).is_some(), "another computer's lock stays");

        lock::claim(&root, &other, now_ms() - 10.0 * 60.0 * 1000.0).expect("old");
        assert!(held_elsewhere(&session, &root).is_none(), "stale");
    }
}
