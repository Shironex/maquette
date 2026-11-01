//! The site's dev server: Preview on site starts it and opens it in the
//! browser, the Page view shows it in the app (see `page.rs`). Both use the
//! one server the app runs, so a server one started is reused by the
//! other. Nothing is built or deployed; deploying stays the person's step.

use std::path::PathBuf;
use std::sync::Arc;

use maquette_core::site::preview::{self, PreviewAsk, PreviewPlan, ServerOwner, SitePreview};
use maquette_core::{CoreError, ProbeEnv, page, project};
use tauri::{AppHandle, State};
use tauri_specta::Event as _;

use super::{CommandResult, invalid};
use crate::events::PageServerChanged;
use crate::mcp::Agents;
use crate::opener;
use crate::session::Session;

pub mod gate;

use gate::StartGate;

/// The one dev server the app runs; stopped when the app quits.
pub struct Preview(pub Arc<SitePreview>);

impl Preview {
    /// Tells every window when the server changes state or prints.
    pub fn new(app: AppHandle) -> Self {
        Self(Arc::new(SitePreview::new(move |status| {
            if let Err(err) = PageServerChanged(status.clone()).emit(&app) {
                tracing::warn!(error = %err, "could not emit a page server event");
            }
        })))
    }
}

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

/// What runs the project's site: its page settings' command, else the
/// site's own, with a free port when it wants one, and the question that
/// names it. Private so every start goes through [`gate::check`].
fn dev_plan(session: &Session, project_id: &str) -> CommandResult<(PreviewPlan, PreviewAsk)> {
    let repo = site_dir(session, project_id)?;
    let (root, config) = session.workspace()?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    let settings = page::load(&dir, &root)?;
    let repo_text = repo.to_string_lossy();
    let command = page::dev_command(Some(&repo_text), &settings)?;
    let plan = preview::plan(&repo, &command, &ProbeEnv::from_system())?;
    Ok((plan, PreviewAsk::of(&repo, &command)))
}

/// Starts (or reuses) the site's dev server and opens it in the browser.
/// Returns the address it opened.
#[tauri::command]
#[specta::specta]
pub async fn site_preview(
    preview: State<'_, Preview>,
    agents: State<'_, Agents>,
    project: String,
) -> CommandResult<String> {
    let StartGate::Clear(plan) = agents.preview_gate(&project)? else {
        return Err(invalid(
            "An agent set this site repo, so its dev command waits for your approval on the card. Approve it to start the server, then use Preview on site again.",
        ));
    };
    let server = Arc::clone(&preview.0);
    let url = tauri::async_runtime::spawn_blocking(move || {
        server.start(&plan, &project, ServerOwner::Pipeline)
    })
    .await
    .map_err(super::join_failed)??;
    opener::open_local_url(&url).map_err(|err| {
        CoreError::Unavailable(format!("Could not open {url} in the browser. {err}")).payload()
    })?;
    tracing::info!(%url, "opened the site preview");
    Ok(url)
}
