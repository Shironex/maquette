use maquette_core::{ImportScan, NewProject, ProjectManifest, RepoInspection, project, workspace};
use tauri::State;

use super::CommandResult;
use super::app::{WorkspaceSnapshot, snapshot};
use crate::session::Session;

/// Creates a project in the open workspace and makes it the current one.
#[tauri::command]
#[specta::specta]
pub fn project_create(
    session: State<'_, Session>,
    project: NewProject,
) -> CommandResult<WorkspaceSnapshot> {
    let (root, mut config) = session.workspace()?;
    let new = NewProject {
        site_repo: project
            .site_repo
            .map(|p| session.expand(&p).display().to_string()),
        source_folder: project
            .source_folder
            .map(|p| session.expand(&p).display().to_string()),
        ..project
    };
    let manifest = project::create(&root, &config.layout, new)?;
    let dir = project::dir(&root, &config.layout, &manifest.id)?;
    maquette_core::pipeline::ensure(&dir, &manifest)?;
    config.current_project = Some(manifest.id);
    workspace::save(&root, &config)?;
    session.set_config(config.clone());
    snapshot(&session, &root, &config)
}

#[tauri::command]
#[specta::specta]
pub fn project_update(
    session: State<'_, Session>,
    id: String,
    name: String,
    site_repo: Option<String>,
) -> CommandResult<ProjectManifest> {
    let (root, config) = session.workspace()?;
    let site_repo = site_repo.map(|p| session.expand(&p).display().to_string());
    Ok(project::update(
        &root,
        &config.layout,
        &id,
        name,
        site_repo,
    )?)
}

#[tauri::command]
#[specta::specta]
pub fn project_inspect_repo(session: State<'_, Session>, path: String) -> RepoInspection {
    project::inspect_site_repo(&session.expand(&path))
}

#[tauri::command]
#[specta::specta]
pub fn project_scan_import(session: State<'_, Session>, path: String) -> CommandResult<ImportScan> {
    Ok(project::scan_import(&session.expand(&path))?)
}
