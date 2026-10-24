//! Native pickers. They open from Rust so the web layer needs no dialog
//! permission, and they return `None` when the user cancels.

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt as _, FilePath};

use super::CommandResult;
use crate::session::Session;

fn to_string(path: Option<FilePath>) -> Option<String> {
    path.and_then(|p| p.into_path().ok())
        .map(|p| p.display().to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn dialog_pick_folder(
    app: AppHandle,
    title: String,
    start: Option<String>,
) -> CommandResult<Option<String>> {
    let start = start.map(|s| {
        use tauri::Manager as _;
        app.state::<Session>().expand(&s)
    });
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = app.dialog().file().set_title(title);
        if let Some(dir) = start.filter(|d| d.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        to_string(dialog.blocking_pick_folder())
    })
    .await
    .map_err(super::join_failed)
}

#[tauri::command]
#[specta::specta]
pub async fn dialog_pick_file(app: AppHandle, title: String) -> CommandResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        to_string(app.dialog().file().set_title(title).blocking_pick_file())
    })
    .await
    .map_err(super::join_failed)
}
