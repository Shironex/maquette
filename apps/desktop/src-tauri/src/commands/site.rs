//! Preview on site: starts the project's site with its own dev server and
//! opens it in the browser, so the person sees what Ship wrote. Nothing is
//! built or deployed; deploying stays the person's step.

use std::path::PathBuf;
use std::sync::Arc;

use maquette_core::site::{self, preview::SitePreview};
use maquette_core::{CoreError, ProbeEnv, project};
use tauri::State;

use super::CommandResult;
use crate::opener;
use crate::session::Session;

/// The one preview server the app runs; stopped when the app quits.
#[derive(Default)]
pub struct Preview(pub Arc<SitePreview>);

/// The project's site folder, checked.
pub(crate) fn site_dir(session: &Session, project_id: &str) -> CommandResult<PathBuf> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let repo = manifest.site_repo.map(PathBuf::from).ok_or_else(|| {
        CoreError::Invalid(
            "This project has no site repo yet. Pick one in Settings > Project.".to_string(),
        )
    })?;
    if !repo.is_dir() {
        return Err(CoreError::NotFound(format!("the site repo {}", repo.display())).into());
    }
    Ok(repo)
}

/// Starts (or reuses) the site's preview server and opens it in the
/// browser. Returns the address it opened.
#[tauri::command]
#[specta::specta]
pub async fn site_preview(
    session: State<'_, Session>,
    preview: State<'_, Preview>,
    project: String,
) -> CommandResult<String> {
    let repo = site_dir(&session, &project)?;
    let config = site::load_config(&repo)?;
    let plan = site::preview::plan(&repo, &config, &ProbeEnv::from_system())?;
    let server = Arc::clone(&preview.0);
    let url = tauri::async_runtime::spawn_blocking(move || server.start(&plan))
        .await
        .map_err(super::join_failed)??;
    opener::open_local_url(&url).map_err(|err| {
        CoreError::Unavailable(format!("Could not open {url} in the browser. {err}")).payload()
    })?;
    tracing::info!(%url, "opened the site preview");
    Ok(url)
}
