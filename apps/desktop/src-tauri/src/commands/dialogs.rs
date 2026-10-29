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

/// Picks a model to bring back by hand: a GLB, or the zip a web studio
/// downloads with one inside.
#[tauri::command]
#[specta::specta]
pub async fn dialog_pick_model(app: AppHandle, title: String) -> CommandResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        to_string(
            app.dialog()
                .file()
                .set_title(title)
                .add_filter("GLB model or zip", &["glb", "zip"])
                .blocking_pick_file(),
        )
    })
    .await
    .map_err(super::join_failed)
}

/// Picks the Python script that builds a model.
#[tauri::command]
#[specta::specta]
pub async fn dialog_pick_script(app: AppHandle, title: String) -> CommandResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        to_string(
            app.dialog()
                .file()
                .set_title(title)
                .add_filter("Python script", &["py"])
                .blocking_pick_file(),
        )
    })
    .await
    .map_err(super::join_failed)
}

/// Picks any number of files, for example the helpers a script imports.
/// Empty when the user cancels.
#[tauri::command]
#[specta::specta]
pub async fn dialog_pick_files(app: AppHandle, title: String) -> CommandResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(title)
            .blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| to_string(Some(p)))
            .collect()
    })
    .await
    .map_err(super::join_failed)
}

/// Picks clip files to import: Mixamo FBX files or a pack zip of them.
/// Empty when the user cancels.
#[tauri::command]
#[specta::specta]
pub async fn dialog_pick_clips(app: AppHandle, title: String) -> CommandResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(title)
            .add_filter("FBX clips or a pack zip", &["fbx", "zip"])
            .blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| to_string(Some(p)))
            .collect()
    })
    .await
    .map_err(super::join_failed)
}

/// Picks a model file to add as an asset's next version: a GLB, glTF, FBX
/// or OBJ.
#[tauri::command]
#[specta::specta]
pub async fn dialog_pick_model_file(
    app: AppHandle,
    title: String,
) -> CommandResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        to_string(
            app.dialog()
                .file()
                .set_title(title)
                .add_filter("Model", &maquette_core::providers::own_file::MODEL_TYPES)
                .blocking_pick_file(),
        )
    })
    .await
    .map_err(super::join_failed)
}
