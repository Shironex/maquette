use std::path::Path;

use maquette_core::{HostOs, ProjectManifest, WorkspaceConfig, project as projects, workspace};
use serde::Serialize;
use tauri::State;

use super::CommandResult;
use crate::session::Session;

/// The open workspace as the UI sees it.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSnapshot {
    pub root: String,
    pub display: String,
    pub config: WorkspaceConfig,
    pub project: Option<ProjectManifest>,
    pub projects: Vec<ProjectManifest>,
}

/// Everything the UI needs before its first paint.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub os: HostOs,
    pub version: String,
    /// Where onboarding suggests the workspace, for example `~/Maquette`.
    pub default_root: String,
    /// The home folder, so the UI can show paths as `~/...`.
    pub home: Option<String>,
    /// The config a new workspace starts from, so the UI never keeps its own copy.
    pub default_config: WorkspaceConfig,
    pub workspace: Option<WorkspaceSnapshot>,
    /// Set when the remembered workspace could not be opened.
    pub last_workspace_error: Option<String>,
}

pub(crate) fn snapshot(
    session: &Session,
    root: &Path,
    config: &WorkspaceConfig,
) -> CommandResult<WorkspaceSnapshot> {
    let list = projects::list(root, &config.layout)?;
    let project = config
        .current_project
        .as_deref()
        .and_then(|id| list.iter().find(|p| p.id == id))
        .or_else(|| list.first())
        .cloned();
    Ok(WorkspaceSnapshot {
        root: root.display().to_string(),
        display: session.display(root),
        config: config.clone(),
        project,
        projects: list,
    })
}

pub(crate) fn bootstrap_with(session: &Session, version: &str) -> Bootstrap {
    let default_root = session
        .home()
        .map(|home| session.display(&home.join("Maquette")))
        .unwrap_or_else(|| "Maquette".to_string());

    let mut last_workspace_error = None;
    let workspace = session.app_state().last_workspace.and_then(|pointer| {
        let root = session.expand(&pointer);
        let opened = workspace::open(&root)
            .map_err(|e| e.payload())
            .and_then(|config| {
                session.set_workspace(root.clone(), config.clone())?;
                snapshot(session, &root, &config)
            });
        match opened {
            Ok(snap) => Some(snap),
            Err(err) => {
                last_workspace_error = Some(err.message);
                None
            }
        }
    });

    Bootstrap {
        os: HostOs::current(),
        version: version.to_string(),
        default_root,
        home: session.home().map(|h| h.display().to_string()),
        default_config: WorkspaceConfig::default(),
        workspace,
        last_workspace_error,
    }
}

#[tauri::command]
#[specta::specta]
pub fn app_bootstrap(
    app: tauri::AppHandle,
    session: State<'_, Session>,
    agents: State<'_, crate::mcp::Agents>,
) -> Bootstrap {
    let boot = bootstrap_with(&session, &app.package_info().version.to_string());
    super::agents::sync(&app, &session, &agents);
    boot
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::AppState;

    #[test]
    fn bootstrap_reopens_the_last_workspace() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        workspace::create(&root, &WorkspaceConfig::default()).expect("create");
        let state = AppState {
            last_workspace: Some(root.display().to_string()),
            ..AppState::default()
        };
        let session = Session::with_state(dir.path().join("cfg"), None, state);

        let boot = bootstrap_with(&session, "0.1.0");
        assert!(boot.last_workspace_error.is_none());
        assert_eq!(
            boot.workspace.expect("opened").root,
            root.display().to_string()
        );
    }

    #[test]
    fn bootstrap_reports_a_workspace_that_moved() {
        let dir = tempfile::tempdir().expect("temp dir");
        let state = AppState {
            last_workspace: Some(dir.path().join("gone").display().to_string()),
            ..AppState::default()
        };
        let session = Session::with_state(dir.path().join("cfg"), None, state);

        let boot = bootstrap_with(&session, "0.1.0");
        assert!(boot.workspace.is_none());
        assert!(boot.last_workspace_error.is_some());
    }
}
