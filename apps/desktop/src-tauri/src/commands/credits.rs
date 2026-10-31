//! Credits: the expected cost of a send while its panel is open, the
//! ledger of what paid sends spent, the providers' balances and saving the
//! ledger as CSV.

use maquette_core::providers::{Balance, BatchRequest, Ledger, SendRequest};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt as _;

use super::CommandResult;
use super::providers::Studio;

/// What a send should cost, from the price lists; null for By hand.
#[tauri::command(async)]
#[specta::specta]
pub fn provider_estimate(
    studio: State<'_, Studio>,
    request: SendRequest,
) -> CommandResult<Option<f64>> {
    Ok(studio.0.estimate(&request)?)
}

/// What a batch should cost in all; null for By hand.
#[tauri::command(async)]
#[specta::specta]
pub fn provider_estimate_batch(
    studio: State<'_, Studio>,
    request: BatchRequest,
) -> CommandResult<Option<f64>> {
    Ok(studio.0.estimate_batch(&request)?)
}

/// Every paid send of the open workspace, newest first.
#[tauri::command]
#[specta::specta]
pub async fn credits_ledger(studio: State<'_, Studio>) -> CommandResult<Ledger> {
    let jobs = studio.0.clone();
    Ok(tauri::async_runtime::spawn_blocking(move || jobs.ledger())
        .await
        .map_err(super::join_failed)??)
}

/// Asks Tripo and Meshy for their balances with the saved keys.
#[tauri::command]
#[specta::specta]
pub async fn credits_balances(studio: State<'_, Studio>) -> CommandResult<Vec<Balance>> {
    let jobs = studio.0.clone();
    tauri::async_runtime::spawn_blocking(move || jobs.balances())
        .await
        .map_err(super::join_failed)
}

/// Asks where to save `csv` and writes it there. Null when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn credits_save_csv(
    app: AppHandle,
    csv: String,
    name: String,
) -> CommandResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let picked = app
            .dialog()
            .file()
            .set_title("Save the credits ledger")
            .set_file_name(&name)
            .add_filter("CSV", &["csv"])
            .blocking_save_file();
        let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
            return Ok(None);
        };
        std::fs::write(&path, csv.as_bytes())
            .map_err(|e| maquette_core::CoreError::io(&path, e).payload())?;
        Ok(Some(path.display().to_string()))
    })
    .await
    .map_err(super::join_failed)?
}
