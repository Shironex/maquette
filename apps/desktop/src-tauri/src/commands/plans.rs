//! The Plan view: a project's planning canvases, one file each in its
//! `plans/` folder. Listing, making, reading, saving, renaming and removing
//! them, picking a picture for an image node and opening a link node's page.

use std::path::{Path, PathBuf};

use maquette_core::assets::remove::project_trash;
use maquette_core::plans::{self, PlanDoc, PlanFile, PlanPreset, PlanSummary, images::PlanImage};
use maquette_core::{CoreError, project};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt as _;

use super::CommandResult;
use crate::session::Session;

fn project_dir(session: &Session, project_id: &str) -> CommandResult<PathBuf> {
    let (root, config) = session.workspace()?;
    project::load(&root, &config.layout, project_id)?;
    Ok(project::dir(&root, &config.layout, project_id)?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn plan_list(session: State<'_, Session>, project: String) -> CommandResult<Vec<PlanSummary>> {
    Ok(plans::list(&project_dir(&session, &project)?)?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn plan_get(
    session: State<'_, Session>,
    project: String,
    id: String,
) -> CommandResult<PlanDoc> {
    Ok(plans::get(&project_dir(&session, &project)?, &id)?)
}

/// Makes an empty canvas. Presets only set the starting content, which a
/// later version fills in; every canvas starts blank for now.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_create(
    session: State<'_, Session>,
    project: String,
    name: String,
) -> CommandResult<PlanDoc> {
    let dir = project_dir(&session, &project)?;
    Ok(plans::create(&dir, &name, PlanPreset::Brainstorm)?)
}

/// Writes the whole canvas, normalised; returns what the file now holds.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_save(
    session: State<'_, Session>,
    project: String,
    id: String,
    plan: PlanFile,
) -> CommandResult<PlanDoc> {
    Ok(plans::save(&project_dir(&session, &project)?, &id, &plan)?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn plan_rename(
    session: State<'_, Session>,
    project: String,
    id: String,
    name: String,
) -> CommandResult<PlanSummary> {
    Ok(plans::rename(
        &project_dir(&session, &project)?,
        &id,
        &name,
    )?)
}

/// Moves the canvas file into the workspace trash; returns where it went.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_remove(
    session: State<'_, Session>,
    project: String,
    id: String,
) -> CommandResult<String> {
    let (root, config) = session.workspace()?;
    let dir = project_dir(&session, &project)?;
    let trash = project_trash(&root, &config.layout, &project)?;
    Ok(plans::remove(&dir, &trash, &id)?)
}

/// Picks a picture for an image node, starting in the project folder. One
/// from elsewhere is copied into `plans/images`. `None` when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn plan_pick_image(app: AppHandle, project: String) -> CommandResult<Option<PlanImage>> {
    use tauri::Manager as _;
    let dir = project_dir(&app.state::<Session>(), &project)?;
    tauri::async_runtime::spawn_blocking(move || {
        let picked = app
            .dialog()
            .file()
            .set_title("Add a picture to the canvas")
            .set_directory(&dir)
            .add_filter("Pictures", &plans::images::IMAGE_EXTENSIONS)
            .blocking_pick_file()
            .and_then(|p| p.into_path().ok());
        picked.map(|path| add_image(&dir, &path)).transpose()
    })
    .await
    .map_err(super::join_failed)?
}

fn add_image(project_dir: &Path, path: &Path) -> CommandResult<PlanImage> {
    Ok(plans::images::add(project_dir, path)?)
}

/// Opens a link node's page in the browser. The address is read from the
/// saved canvas, never taken from the web view, and only `https` opens.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_open_link(
    session: State<'_, Session>,
    project: String,
    id: String,
    node: String,
) -> CommandResult<()> {
    let doc = plans::get(&project_dir(&session, &project)?, &id)?;
    let url = doc
        .plan
        .nodes
        .iter()
        .find(|n| n.id == node)
        .and_then(|n| n.url.clone())
        .ok_or_else(|| CoreError::NotFound(format!("the link {node}")).payload())?;
    crate::opener::open_url(&url).map_err(|_| {
        super::invalid("Only https pages open from a canvas. Copy the address to open it.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::{AppState, NewProject, ProjectTemplate, WorkspaceConfig, workspace};

    #[test]
    fn a_canvas_is_made_saved_listed_and_removed_into_the_trash() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let manifest = project::create(
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
        let pdir = project_dir(&session, &manifest.id).expect("dir");
        let made = plans::create(&pdir, "Hero ideas", PlanPreset::Brainstorm).expect("create");
        assert_eq!(plans::list(&pdir).expect("list").len(), 1);
        let trash = project_trash(&root, &config.layout, &manifest.id).expect("trash");
        let moved = plans::remove(&pdir, &trash, &made.id).expect("remove");
        assert!(Path::new(&moved).starts_with(root.join("trash")));
        assert!(plans::list(&pdir).expect("list").is_empty());
        assert!(project_dir(&session, "nowhere").is_err());
    }
}
