use std::path::Path;

use maquette_core::providers::{self, ProviderCatalog};
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
    /// The Tripo and Meshy models the selects offer.
    pub catalog: ProviderCatalog,
    pub workspace: Option<WorkspaceSnapshot>,
    /// Set when the remembered workspace could not be opened.
    pub last_workspace_error: Option<String>,
}

pub(crate) fn snapshot(
    session: &Session,
    root: &Path,
    config: &WorkspaceConfig,
) -> CommandResult<WorkspaceSnapshot> {
    let mut list = projects::list(root, &config.layout)?;
    let open = config
        .current_project
        .as_deref()
        .filter(|id| list.iter().any(|p| p.id == *id))
        .or_else(|| list.first().map(|p| p.id.as_str()))
        .map(str::to_string);
    if let Some(id) = open {
        // Cards an older Send picks to Refs left without a picture get one.
        match maquette_core::concepts::heal_ref_thumbnails(root, &config.layout, &id) {
            Ok(Some(healed)) => {
                if let Some(slot) = list.iter_mut().find(|p| p.id == id) {
                    *slot = healed;
                }
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(project = %id, error = %err, "could not heal ref thumbnails")
            }
        }
    }
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
        catalog: providers::catalog::catalog(),
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
    fn snapshot_heals_a_refs_card_without_a_picture() {
        use maquette_core::concepts::{self, Aspect, NewConcepts};
        use maquette_core::{AssetKind, NewProject, ProjectTemplate};
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("create");
        let manifest = projects::create(
            &root,
            &config.layout,
            NewProject {
                name: "Moon Island".into(),
                template: ProjectTemplate::FloatingDiorama,
                target: Default::default(),
                game: None,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        let project_dir = projects::dir(&root, &config.layout, &manifest.id).expect("dir");
        let request = NewConcepts {
            project: manifest.id.clone(),
            direction: "Maquette landing studio".into(),
            prompt: "A studio desk".into(),
            refs: Vec::new(),
            aspect: Aspect::Wide,
            count: 1,
        };
        let (direction, files) = concepts::prepare(&project_dir, &request).expect("prepare");
        let (index, file) = files[0].clone();
        std::fs::write(project_dir.join(&file), b"png").expect("image");
        concepts::add_image(&project_dir, &direction.id, index, &file, None).expect("add");
        concepts::mark(&project_dir, &direction.id, &file, None, Some(true)).expect("pick");
        concepts::send_picks(
            &root,
            &config.layout,
            &manifest.id,
            &direction.id,
            AssetKind::Scene,
        )
        .expect("send");
        projects::modify(&root, &config.layout, &manifest.id, |m| {
            m.assets[0].thumbnail = None;
            Ok(())
        })
        .expect("stuck");

        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        let snap = snapshot(&session, &root, &config).expect("snapshot");
        let want = Some("refs/maquette-landing-studio_01.png");
        assert_eq!(
            snap.project.expect("open").assets[0].thumbnail.as_deref(),
            want
        );
        assert_eq!(snap.projects[0].assets[0].thumbnail.as_deref(), want);
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
