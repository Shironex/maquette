use std::collections::BTreeMap;
use std::path::PathBuf;

use maquette_core::pipeline::tools::Toolbox;
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

/// A new check, remembered. One that a Locate or Reset overtook read the
/// old overrides, so it runs again rather than answer or be kept.
fn check_now(session: &Session) -> Vec<ToolReport> {
    loop {
        let turn = session.tools_turn();
        let overrides = overrides(session, &session.app_state());
        let reports = toolchain::detect_all(&ProbeEnv::from_system(), &overrides, &SystemRunner);
        if session.keep_tools(turn, reports.clone()) {
            return reports;
        }
    }
}

/// [`check_now`] off the async runtime's threads.
pub(crate) async fn check_blocking(session: &Session) -> CommandResult<Vec<ToolReport>> {
    loop {
        let turn = session.tools_turn();
        let overrides = overrides(session, &session.app_state());
        let reports = run_blocking(move || {
            toolchain::detect_all(&ProbeEnv::from_system(), &overrides, &SystemRunner)
        })
        .await?;
        if session.keep_tools(turn, reports.clone()) {
            return Ok(reports);
        }
    }
}

/// [`toolbox`] for callers off the async runtime, such as the MCP server's
/// blocking tasks.
pub(crate) fn toolbox_blocking(session: &Session) -> Toolbox {
    let reports = session.tools().unwrap_or_else(|| check_now(session));
    Toolbox::from_reports(&reports, &ProbeEnv::from_system())
}

/// The tools a job runs with: the last check, or a fresh one.
pub(crate) async fn toolbox(session: &Session) -> CommandResult<Toolbox> {
    let reports = match session.tools() {
        Some(reports) => reports,
        None => check_blocking(session).await?,
    };
    Ok(Toolbox::from_reports(&reports, &ProbeEnv::from_system()))
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
    check_blocking(&session).await
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
    session.forget_tools();
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
    session.forget_tools();
    run_blocking(move || toolchain::detect(tool, &ProbeEnv::from_system(), None, &SystemRunner))
        .await
}
