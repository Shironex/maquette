use std::collections::BTreeMap;
use std::path::PathBuf;

use maquette_core::{AppState, ProbeEnv, SystemRunner, ToolId, ToolReport, ToolSource, toolchain};
use tauri::State;

use super::{CommandResult, invalid};
use crate::session::Session;

pub(crate) fn overrides(session: &Session, state: &AppState) -> BTreeMap<ToolId, PathBuf> {
    state
        .tool_overrides
        .iter()
        .map(|(id, path)| (*id, session.expand(path)))
        .collect()
}

async fn run_blocking<T: Send + 'static>(
    job: impl FnOnce() -> T + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(super::join_failed)
}

#[tauri::command]
#[specta::specta]
pub async fn toolchain_detect(session: State<'_, Session>) -> CommandResult<Vec<ToolReport>> {
    let overrides = overrides(&session, &session.app_state());
    let reports = run_blocking(move || {
        toolchain::detect_all(&ProbeEnv::from_system(), &overrides, &SystemRunner)
    })
    .await?;
    session.set_tools(Some(reports.clone()));
    Ok(reports)
}

/// Points a tool at a file the user picked. The override is only saved when
/// that file answers `--version`.
#[tauri::command]
#[specta::specta]
pub async fn toolchain_locate(
    session: State<'_, Session>,
    tool: ToolId,
    path: String,
) -> CommandResult<ToolReport> {
    let picked = session.expand(&path);
    let probe_path = picked.clone();
    let report = run_blocking(move || {
        toolchain::detect(
            tool,
            &ProbeEnv::from_system(),
            Some(&probe_path),
            &SystemRunner,
        )
    })
    .await?;
    if report.source != Some(ToolSource::Override) {
        return Err(invalid(
            "That file did not answer --version. Pick the program itself.",
        ));
    }
    let stored = session.display(&picked);
    session.update_app_state(|state| {
        state.tool_overrides.insert(tool, stored);
    })?;
    session.set_tools(None);
    Ok(report)
}

#[tauri::command]
#[specta::specta]
pub async fn toolchain_reset(
    session: State<'_, Session>,
    tool: ToolId,
) -> CommandResult<ToolReport> {
    session.update_app_state(|state| {
        state.tool_overrides.remove(&tool);
    })?;
    session.set_tools(None);
    run_blocking(move || toolchain::detect(tool, &ProbeEnv::from_system(), None, &SystemRunner))
        .await
}
