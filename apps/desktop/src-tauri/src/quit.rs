//! Closing the window, or quitting, while the web view holds work that is
//! not applied yet. The web view says whether something is pending; when it
//! is, a close request is held back and the web view asks the user, then
//! calls `quit_now` to let the close through. With nothing pending a close
//! never waits on the web view.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{Menu, MenuEvent, MenuItem, MenuItemKind};
use tauri::{AppHandle, Manager as _, Runtime, Window, WindowEvent};
use tauri_specta::Event as _;

use crate::events::QuitRequested;

/// The menu id of the Quit item that replaces the system one.
const QUIT_ID: &str = "quit";
/// The window a quit closes.
const MAIN_WINDOW: &str = "main";

/// What a close request should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDecision {
    /// Let the window close.
    Close,
    /// Hold the close back and ask the web view.
    Ask,
}

/// Whether the web view has unapplied work, and whether the user already
/// answered. Managed by the app, so commands and the window handler share it.
#[derive(Debug, Default)]
pub struct QuitGuard {
    unapplied: AtomicBool,
    approved: AtomicBool,
}

impl QuitGuard {
    pub fn set_unapplied(&self, unapplied: bool) {
        self.unapplied.store(unapplied, Ordering::SeqCst);
    }

    /// Whether the web view said it holds work that is not applied yet.
    pub fn has_unapplied(&self) -> bool {
        self.unapplied.load(Ordering::SeqCst)
    }

    /// The user answered: the next close request goes through.
    pub fn approve(&self) {
        self.approved.store(true, Ordering::SeqCst);
    }

    /// Takes the approval, so one answer lets one close through.
    pub fn on_close_requested(&self) -> CloseDecision {
        if self.approved.swap(false, Ordering::SeqCst) || !self.unapplied.load(Ordering::SeqCst) {
            CloseDecision::Close
        } else {
            CloseDecision::Ask
        }
    }
}

/// The window event hook: holds back a close that needs an answer.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    let Some(guard) = window.try_state::<QuitGuard>() else {
        return;
    };
    if guard.on_close_requested() == CloseDecision::Ask {
        api.prevent_close();
        if let Err(err) = QuitRequested.emit(window) {
            // Nobody can answer: let the close through rather than trap the user.
            tracing::warn!(error = %err, "could not ask the web view about quitting");
            guard.approve();
            let _ = window.close();
        }
    }
}

/// Cmd+Q and the Quit menu item end the process without a close request, so
/// the system item is swapped for one that closes the main window instead.
pub fn route_quit_through_close<R: Runtime>(handle: &AppHandle<R>) -> tauri::Result<()> {
    let menu = Menu::default(handle)?;
    for kind in menu.items()? {
        let MenuItemKind::Submenu(submenu) = kind else {
            continue;
        };
        for (at, item) in submenu.items()?.into_iter().enumerate() {
            let MenuItemKind::Predefined(system) = &item else {
                continue;
            };
            let text = system.text()?;
            if !text.replace('&', "").trim().starts_with("Quit") {
                continue;
            }
            let accelerator = cfg!(target_os = "macos").then_some("CmdOrCtrl+Q");
            let quit = MenuItem::with_id(handle, QUIT_ID, text, true, accelerator)?;
            submenu.remove(system)?;
            submenu.insert(&quit, at)?;
        }
    }
    handle.set_menu(menu)?;
    Ok(())
}

/// Handles the replacement Quit item: the same path as the close button.
pub fn on_menu_event<R: Runtime>(handle: &AppHandle<R>, event: MenuEvent) {
    if event.id().as_ref() != QUIT_ID {
        return;
    }
    match handle.get_webview_window(MAIN_WINDOW) {
        Some(window) => {
            let _ = window.close();
        }
        None => handle.exit(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_close_with_nothing_pending_goes_straight_through() {
        let guard = QuitGuard::default();
        assert_eq!(guard.on_close_requested(), CloseDecision::Close);
    }

    #[test]
    fn a_close_with_unapplied_work_asks_until_it_is_approved_once() {
        let guard = QuitGuard::default();
        guard.set_unapplied(true);
        assert_eq!(guard.on_close_requested(), CloseDecision::Ask);
        assert_eq!(guard.on_close_requested(), CloseDecision::Ask);
        guard.approve();
        assert_eq!(guard.on_close_requested(), CloseDecision::Close);
        // The approval was used up by that close.
        assert_eq!(guard.on_close_requested(), CloseDecision::Ask);
    }

    #[test]
    fn applying_the_work_stops_the_asking() {
        let guard = QuitGuard::default();
        guard.set_unapplied(true);
        guard.set_unapplied(false);
        assert_eq!(guard.on_close_requested(), CloseDecision::Close);
    }
}
