//! The Plan view: a project's planning canvases, one file each in its
//! `plans/` folder. Listing, making, reading, saving, renaming and removing
//! them, picking a picture for an image node, opening a link node's page and
//! reading the assets and Lab directions a canvas links, live.

use std::path::{Path, PathBuf};

use maquette_core::assets::remove::project_trash;
use maquette_core::plan_links::{self, LinkFacts, PlanLinks};
use maquette_core::plans::log::PlanLogLine;
use maquette_core::plans::ops::PlanOp;
use maquette_core::plans::{self, PlanDoc, PlanPreset, PlanSummary, images::PlanImage};
use maquette_core::{CoreError, inbox, project};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt as _;

use super::lab::Lab;
use super::providers::Studio;
use super::{CommandResult, blocking};
use crate::session::Session;

pub(super) fn project_dir(session: &Session, project_id: &str) -> CommandResult<PathBuf> {
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

/// Makes a canvas from `preset`, with its starter notes for the project's
/// target (a game or a website), as the person's first change.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_create(
    session: State<'_, Session>,
    project: String,
    name: String,
    preset: PlanPreset,
) -> CommandResult<PlanDoc> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, &project)?;
    let dir = project::dir(&root, &config.layout, &project)?;
    Ok(plans::create_seeded(&dir, &name, preset, manifest.target, None)?.doc)
}

/// Writes a person's edits: the operations between the canvas the view
/// last had from the file and what it shows now, laid over the file as it
/// is, so an agent's write made meanwhile stays. Returns what the file now
/// holds, which the view takes as its new base. `created` is the opened
/// canvas's, so a save never lands on another canvas that took the id.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_apply(
    session: State<'_, Session>,
    project: String,
    id: String,
    created: String,
    ops: Vec<PlanOp>,
) -> CommandResult<PlanDoc> {
    let dir = project_dir(&session, &project)?;
    Ok(plans::apply_person(&dir, &id, &created, ops)?.doc)
}

/// The newest lines of the canvas's change log, newest first.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_log(
    session: State<'_, Session>,
    project: String,
    id: String,
    limit: u32,
) -> CommandResult<Vec<PlanLogLine>> {
    let limit = usize::try_from(limit.clamp(1, 500)).unwrap_or(200);
    Ok(plans::log::lines(
        &project_dir(&session, &project)?,
        &id,
        limit,
    )?)
}

/// What an undo did: the canvas as the file now holds it, and whether it
/// changed anything (everything it would undo may have changed since).
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanUndone {
    pub doc: PlanDoc,
    pub changed: bool,
    /// Nodes kept because someone edited or linked them since.
    pub kept: u32,
}

/// Undoes log lines (an agent's run, or one change) as a person; a field
/// changed since stays.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_undo(
    session: State<'_, Session>,
    project: String,
    id: String,
    changes: Vec<String>,
) -> CommandResult<PlanUndone> {
    let dir = project_dir(&session, &project)?;
    let done = plans::undo(&dir, &id, &changes, None, None)?;
    Ok(PlanUndone {
        changed: done.entry.as_ref().is_some_and(|e| !e.ops.is_empty()),
        kept: done.skipped,
        doc: done.doc,
    })
}

/// Keeps (accepts) agent changes as a person: frees the room the log kept
/// to undo them. The canvas does not change. `changes` are the ones the
/// person clicked and `confirmed` the earlier ones they confirmed; refused
/// unless `confirmed` is exactly what [`plan_keep_needs`] lists now.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_keep(
    session: State<'_, Session>,
    project: String,
    id: String,
    changes: Vec<String>,
    confirmed: Vec<String>,
) -> CommandResult<()> {
    let dir = project_dir(&session, &project)?;
    plans::keep_confirmed(&dir, &id, &changes, &confirmed)?;
    Ok(())
}

/// The earlier agent changes a Keep of `changes` must take along, for the
/// person to confirm first; empty when the Keep can go as it is.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_keep_needs(
    session: State<'_, Session>,
    project: String,
    id: String,
    changes: Vec<String>,
) -> CommandResult<Vec<plans::KeepNeed>> {
    Ok(plans::keep_needs(
        &project_dir(&session, &project)?,
        &id,
        &changes,
    )?)
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
    let url = link_url(&doc.plan, &node)?;
    crate::opener::open_url(&url).map_err(|_| {
        super::invalid("Only https pages open from a canvas. Copy the address to open it.")
    })
}

/// What the asset and Lab direction nodes of a canvas show right now: each
/// asset's name, picture, newest version and state (no model, a model to
/// check in the Inbox, its file missing, ready, or gone from the Board),
/// and each direction's images, picks and running jobs. Nothing of it is
/// stored in the canvas. The Inbox is read only when assets are asked for.
#[tauri::command]
#[specta::specta]
pub async fn plan_links(
    session: State<'_, Session>,
    lab: State<'_, Lab>,
    studio: State<'_, Studio>,
    project: String,
    assets: Vec<String>,
    directions: Vec<String>,
) -> CommandResult<PlanLinks> {
    let (root, config) = session.workspace()?;
    let jobs = lab.0.jobs(Some(&project));
    let studio = studio.0.clone();
    blocking(move || {
        let manifest = project::load(&root, &config.layout, &project)?;
        let dir = project::dir(&root, &config.layout, &project)?;
        // Reading the Inbox checks every model in it, so only when an asset asks.
        let mut items = Vec::new();
        if !assets.is_empty() {
            items = inbox::list(&root, &config)?;
            studio.link_inbox(&mut items);
            inbox::place(&root, &config, &mut items);
        }
        let facts = LinkFacts {
            project_dir: &dir,
            manifest: &manifest,
            inbox: &items,
            jobs: &jobs,
        };
        Ok::<_, CoreError>(plan_links::read(&facts, &assets, &directions))
    })
    .await?
    .map_err(Into::into)
}

/// The address of the link node `node` as the saved file has it: only a
/// link node's, and only an https one.
fn link_url(plan: &plans::PlanFile, node: &str) -> CommandResult<String> {
    let url = plan
        .nodes
        .iter()
        .find(|n| n.id == node && n.kind == plans::NodeKind::Link)
        .and_then(|n| n.url.clone())
        .ok_or_else(|| CoreError::NotFound(format!("the link {node}")).payload())?;
    if !url.starts_with("https://") {
        return Err(super::invalid(
            "Only https pages open from a canvas. Copy the address to open it.",
        ));
    }
    Ok(url)
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

    fn node(id: &str, kind: plans::NodeKind, url: Option<&str>) -> plans::PlanNode {
        plans::PlanNode {
            id: id.into(),
            kind,
            x: 0,
            y: 0,
            width: None,
            title: String::new(),
            text: String::new(),
            image: (kind == plans::NodeKind::Image).then(|| "concepts/a.png".to_string()),
            url: url.map(str::to_string),
            asset: None,
            direction: None,
            by: None,
            person_wrote: false,
            person_placed: false,
            stop: None,
        }
    }

    #[test]
    fn only_a_link_nodes_https_address_opens() {
        let mut plan = plans::PlanFile::new("Links", PlanPreset::Brainstorm);
        plan.nodes = vec![
            node(
                "n-1",
                plans::NodeKind::Link,
                Some("https://example.com/a,b/select"),
            ),
            node("n-2", plans::NodeKind::Link, Some("http://example.com")),
            node("n-3", plans::NodeKind::Note, None),
            node("n-4", plans::NodeKind::Image, None),
        ];
        plans::validate(&plan).expect("valid");
        assert_eq!(
            link_url(&plan, "n-1").expect("link"),
            "https://example.com/a,b/select"
        );
        assert!(link_url(&plan, "n-2").is_err(), "http stays copy only");
        assert!(link_url(&plan, "n-3").is_err(), "a note has no address");
        assert!(link_url(&plan, "n-4").is_err(), "a picture has no address");
        assert!(link_url(&plan, "n-9").is_err(), "a missing node");
        // A note carrying an address is refused when the file is read.
        plan.nodes[2].url = Some("https://example.com".into());
        assert!(plans::validate(&plan).is_err());
    }
}
