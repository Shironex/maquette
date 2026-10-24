//! The web view's side of quitting: it reports unapplied work, and answers
//! the question a held-back close raised. See [`crate::quit`].

use tauri::{State, Window};

use crate::quit::QuitGuard;

/// Tells the shell whether the web view has work that is not applied yet.
#[tauri::command]
#[specta::specta]
pub fn quit_set_unapplied(guard: State<'_, QuitGuard>, unapplied: bool) {
    guard.set_unapplied(unapplied);
}

/// Closes the window now, whatever is unapplied: the user answered.
#[tauri::command]
#[specta::specta]
pub fn quit_now(guard: State<'_, QuitGuard>, window: Window) {
    guard.approve();
    if let Err(err) = window.close() {
        tracing::warn!(error = %err, "could not close the window");
    }
}
