//! The Page view: the project's site (or an HTML page concept) inside the
//! app, on the dev server Maquette starts from the site repo. The view
//! starts the server when it opens and lets go of it when it closes; a
//! server Preview on site or an agent started stays until stopped.

use std::sync::Arc;

use maquette_core::page::{self, PageSettings, PageView};
use maquette_core::site::preview::{ServerOwner, ServerState, SitePreview};
use maquette_core::{CoreError, project};
use tauri::State;

use super::CommandResult;
use super::site::Preview;
use super::site::gate::StartGate;
use crate::mcp::Agents;
use crate::opener;
use crate::session::Session;

/// The project's page view against the app's dev server.
pub(crate) fn view(
    session: &Session,
    server: &SitePreview,
    project_id: &str,
) -> CommandResult<PageView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    let settings = page::load(&dir, &root)?;
    Ok(page::view(
        project_id,
        manifest.site_repo.as_deref(),
        settings,
        &server.status(),
    ))
}

/// Stops the dev server when it serves `project_id`.
pub(crate) fn stop(
    session: &Session,
    server: &SitePreview,
    project_id: &str,
) -> CommandResult<PageView> {
    if server.status().project.as_deref() == Some(project_id) {
        server.stop();
    }
    view(session, server, project_id)
}

#[tauri::command]
#[specta::specta]
pub async fn page_get(
    session: State<'_, Session>,
    preview: State<'_, Preview>,
    project: String,
) -> CommandResult<PageView> {
    view(&session, &preview.0, &project)
}

/// Saves the project's page settings: the dev command, the page concept,
/// which one the view shows and the path on the site.
#[tauri::command]
#[specta::specta]
pub async fn page_save(
    session: State<'_, Session>,
    preview: State<'_, Preview>,
    project: String,
    settings: PageSettings,
) -> CommandResult<PageView> {
    let (root, config) = session.workspace()?;
    let dir = project::dir(&root, &config.layout, &project)?;
    project::load(&root, &config.layout, &project)?;
    page::save(&dir, &root, &settings)?;
    view(&session, &preview.0, &project)
}

#[tauri::command]
#[specta::specta]
pub async fn page_server_start(
    session: State<'_, Session>,
    preview: State<'_, Preview>,
    agents: State<'_, Agents>,
    project: String,
) -> CommandResult<PageView> {
    // While a card waits the server stays idle and the view says why.
    if let StartGate::Clear(plan) = agents.preview_gate(&project)? {
        let server = Arc::clone(&preview.0);
        let id = project.clone();
        tauri::async_runtime::spawn_blocking(move || server.begin(&plan, &id, ServerOwner::Page))
            .await
            .map_err(super::join_failed)?;
    }
    view(&session, &preview.0, &project)
}

#[tauri::command]
#[specta::specta]
pub async fn page_server_stop(
    session: State<'_, Session>,
    preview: State<'_, Preview>,
    project: String,
) -> CommandResult<PageView> {
    let server = Arc::clone(&preview.0);
    let id = project.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if server.status().project.as_deref() == Some(id.as_str()) {
            server.stop();
        }
    })
    .await
    .map_err(super::join_failed)?;
    view(&session, &preview.0, &project)
}

/// The Page view closed: stops the server only when the view started it.
#[tauri::command]
#[specta::specta]
pub async fn page_server_release(
    preview: State<'_, Preview>,
    project: String,
) -> CommandResult<bool> {
    let server = Arc::clone(&preview.0);
    tauri::async_runtime::spawn_blocking(move || server.release(&project, ServerOwner::Page))
        .await
        .map_err(super::join_failed)
}

/// Opens the page the view shows in the browser: the dev server's address
/// with `path`. Only addresses on this machine open.
#[tauri::command]
#[specta::specta]
pub async fn page_open_in_browser(
    preview: State<'_, Preview>,
    project: String,
    path: String,
) -> CommandResult<String> {
    let status = preview.0.status();
    let url = status
        .url
        .filter(|_| {
            status.state == ServerState::Running && status.project.as_deref() == Some(&project)
        })
        .ok_or_else(|| {
            CoreError::Invalid("The site's dev server is not running yet.".to_string()).payload()
        })?;
    let full = format!("{}{}", url.trim_end_matches('/'), path_or_root(&path));
    opener::open_local_url(&full).map_err(|err| {
        CoreError::Unavailable(format!("Could not open {full} in the browser. {err}")).payload()
    })?;
    Ok(full)
}

fn path_or_root(path: &str) -> &str {
    if path.starts_with('/') { path } else { "/" }
}
