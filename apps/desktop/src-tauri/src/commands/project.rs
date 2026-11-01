use std::path::{Path, PathBuf};

use maquette_core::app_state::RepoSetBy;
use maquette_core::concepts::Concepts;
use maquette_core::fs::real_folder;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::providers::ProviderJobs;
use maquette_core::{
    CoreError, ImportScan, NewProject, ProjectActivity, ProjectManifest, ProjectSummary,
    RepoInspection, project,
};
use tauri::State;

use super::app::{WorkspaceSnapshot, snapshot};
use super::lab::Lab;
use super::pipeline::Pipelines;
use super::providers::Studio;
use super::site::gate::mark_repo;
use super::{CommandResult, blocking, invalid};
use crate::session::Session;

/// What runs in `id` right now: its pipeline run, concept images and sends.
pub(crate) fn activity(
    runner: &PipelineRunner,
    lab: &Concepts,
    studio: &ProviderJobs,
    id: &str,
) -> ProjectActivity {
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    ProjectActivity {
        run: runner.active(id).0,
        concepts: count(
            lab.jobs(Some(id))
                .iter()
                .filter(|j| !j.state.is_finished())
                .count(),
        ),
        sends: count(
            studio
                .list(Some(id))
                .iter()
                .filter(|j| j.state.is_active())
                .count(),
        ),
    }
}

/// Every project in the open workspace with what runs in it. Shared with
/// the MCP server.
pub(crate) fn summaries(
    session: &Session,
    runner: &PipelineRunner,
    lab: &Concepts,
    studio: &ProviderJobs,
) -> CommandResult<Vec<ProjectSummary>> {
    let (root, config) = session.workspace()?;
    Ok(project::summaries(&root, &config.layout)?
        .into_iter()
        .map(|mut s| {
            s.activity = activity(runner, lab, studio, &s.id);
            s
        })
        .collect())
}

/// The projects for the switcher.
#[tauri::command(async)]
#[specta::specta]
pub fn project_list(
    session: State<'_, Session>,
    pipelines: State<'_, Pipelines>,
    lab: State<'_, Lab>,
    studio: State<'_, Studio>,
) -> CommandResult<Vec<ProjectSummary>> {
    summaries(&session, &pipelines.0, &lab.0, &studio.0)
}

/// Makes `id` the current project and remembers it in `maquette.json`.
/// Runs, concept jobs and sends of the project left behind keep going.
pub(crate) fn switch(session: &Session, id: &str) -> CommandResult<WorkspaceSnapshot> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, id)?;
    let (root, config) = session.update_config(|c| c.current_project = Some(manifest.id))?;
    snapshot(session, &root, &config)
}

#[tauri::command(async)]
#[specta::specta]
pub fn project_switch(session: State<'_, Session>, id: String) -> CommandResult<WorkspaceSnapshot> {
    switch(&session, &id)
}

/// Creates a project in the open workspace, with its pipeline. Shared with
/// the MCP server, which leaves the app's open project alone.
pub(crate) fn create_in_workspace(
    session: &Session,
    project: NewProject,
    by: RepoSetBy,
) -> CommandResult<ProjectManifest> {
    let (root, config) = session.workspace()?;
    let new = NewProject {
        site_repo: project
            .site_repo
            .map(|p| session.expand(&p).display().to_string()),
        source_folder: project
            .source_folder
            .map(|p| session.expand(&p).display().to_string()),
        ..project
    };
    let has_repo = new
        .site_repo
        .as_deref()
        .is_some_and(|r| !r.trim().is_empty());
    let by = if has_repo { by } else { RepoSetBy::Person };
    // The mark goes down before the repo does, so a failure in between asks.
    // The id is claimed but has no manifest until after this, so no save can
    // reach it; the turn keeps the book's write with the others.
    let _turn = session.repo_turn();
    let manifest = project::create_with(&root, &config.layout, new, |id| {
        mark_repo(session, &root, id, by).map_err(|e| CoreError::Invalid(e.message))
    })?;
    session.save_repos(&root)?;
    let dir = project::dir(&root, &config.layout, &manifest.id)?;
    maquette_core::pipeline::ensure(&dir, &manifest)?;
    Ok(manifest)
}

/// Creates a project in the open workspace and makes it the current one.
#[tauri::command(async)]
#[specta::specta]
pub fn project_create(
    session: State<'_, Session>,
    project: NewProject,
) -> CommandResult<WorkspaceSnapshot> {
    let manifest = create_in_workspace(&session, project, RepoSetBy::Person)?;
    let (root, config) = session.update_config(|c| c.current_project = Some(manifest.id))?;
    snapshot(&session, &root, &config)
}

#[tauri::command(async)]
#[specta::specta]
pub fn project_update(
    session: State<'_, Session>,
    pipelines: State<'_, Pipelines>,
    id: String,
    name: String,
    site_repo: Option<String>,
) -> CommandResult<ProjectManifest> {
    set_repo(
        &session,
        &pipelines.0,
        &id,
        name,
        site_repo,
        RepoSetBy::Person,
    )
}

/// Checks a repo an agent names: `~` expanded, an absolute path to a git repo
/// or worktree, and nowhere Ship could write over Maquette's own files or a
/// home folder. Every door an agent has to a repo goes through this, and it
/// answers the real folder, so a link moved later cannot change where Ship
/// writes and the Ship card names the folder itself.
pub(crate) fn checked_repo(session: &Session, path: &str) -> CommandResult<PathBuf> {
    let expanded = session.expand(path);
    let at = maquette_core::game::check_repo(&expanded.display().to_string())?;
    let found = real_folder(&at);
    refuse_ship_folder(session, &found)?;
    Ok(found)
}

/// Refuses a resolved folder Ship must not write to. A saved repo comes back
/// through here at run time, since a link it went through can have moved.
pub(crate) fn refuse_ship_folder(session: &Session, found: &Path) -> CommandResult<()> {
    let (root, config) = session.workspace()?;
    let workspace = real_folder(&root);
    let projects = real_folder(&root.join(&config.layout.projects));
    let home = real_folder(&session.expand("~"));
    match repo_refusal(found, &workspace, &projects, &home) {
        Some(why) => Err(invalid(&format!(
            "{} {why} Pick the site's own repo.",
            found.display()
        ))),
        None => Ok(()),
    }
}

/// Why a repo cannot take Ship's files, all paths already resolved.
fn repo_refusal(
    found: &Path,
    workspace: &Path,
    projects: &Path,
    home: &Path,
) -> Option<&'static str> {
    if found.starts_with(workspace) {
        Some("is inside the Maquette workspace.")
    } else if workspace.starts_with(found) {
        Some("holds the Maquette workspace.")
    } else if found.starts_with(projects) {
        Some("is inside a project's folder.")
    } else if projects.starts_with(found) {
        // Only a linked projects folder gets here: its name is one plain
        // segment under the workspace, which the checks above cover.
        Some("holds the projects folder.")
    } else if found == home {
        Some("is the home folder.")
    } else if home.starts_with(found) {
        Some("holds the home folder.")
    } else {
        None
    }
}

/// A repo change while a run is going would ship somewhere other than the
/// plan a person approves.
fn refuse_during_run(runner: &PipelineRunner, id: &str) -> CommandResult<()> {
    let (active, queued) = runner.active(id);
    if active.is_some() || !queued.is_empty() {
        return Err(invalid(&format!(
            "A run is going in {id}. Wait for it or cancel it, then change the repo."
        )));
    }
    Ok(())
}

/// Names a project and sets this computer's repo for it. Refused while the
/// project has a run going when the repo would change. Shared with the MCP
/// server.
pub(crate) fn set_repo(
    session: &Session,
    runner: &PipelineRunner,
    id: &str,
    name: String,
    site_repo: Option<String>,
    by: RepoSetBy,
) -> CommandResult<ProjectManifest> {
    let (root, config) = session.workspace()?;
    let site_repo = site_repo.map(|p| session.expand(&p).display().to_string());
    let _turn = session.repo_turn();
    let before = project::load(&root, &config.layout, id)?;
    let moved = before.site_repo != site_repo;
    if moved {
        refuse_during_run(runner, id)?;
    }
    // Before the repo, so a failure in between asks; the turn rules out a
    // person's clear landing on an agent's repo. An agent
    // clearing a repo keeps its mark: only a person's move drops it.
    if by == RepoSetBy::Agent && site_repo.is_some() {
        mark_repo(session, &root, id, RepoSetBy::Agent)?;
    }
    let manifest = project::update(&root, &config.layout, id, name, site_repo)?;
    session.save_repos(&root)?;
    // Settings and onboarding save the same repo as they click through,
    // which says nothing about it, so only a real move clears the mark.
    if by == RepoSetBy::Person && moved {
        mark_repo(session, &root, id, RepoSetBy::Person)?;
    }
    Ok(manifest)
}

#[tauri::command]
#[specta::specta]
pub async fn project_inspect_repo(
    session: State<'_, Session>,
    path: String,
) -> CommandResult<RepoInspection> {
    let path = session.expand(&path);
    blocking(move || project::inspect_site_repo(&path)).await
}

#[tauri::command]
#[specta::specta]
pub async fn project_scan_import(
    session: State<'_, Session>,
    path: String,
) -> CommandResult<ImportScan> {
    let path = session.expand(&path);
    Ok(blocking(move || project::scan_import(&path)).await??)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::workspace;
    use maquette_core::{AppState, ProjectTemplate, WorkspaceConfig};

    fn new(name: &str, template: ProjectTemplate) -> NewProject {
        NewProject {
            name: name.into(),
            template,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        }
    }

    #[test]
    fn switching_persists_and_the_snapshot_follows() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        for (name, template) in [
            ("Moon Island", ProjectTemplate::FloatingDiorama),
            ("Rooftop cinema", ProjectTemplate::Room),
        ] {
            project::create(&root, &config.layout, new(name, template)).expect("project");
        }

        let snap = switch(&session, "rooftop-cinema").expect("switch");
        assert_eq!(snap.project.expect("open").id, "rooftop-cinema");
        assert_eq!(snap.projects.len(), 2);
        assert_eq!(
            session
                .workspace()
                .expect("ws")
                .1
                .current_project
                .as_deref(),
            Some("rooftop-cinema")
        );
        assert_eq!(
            workspace::open(&root)
                .expect("reopen")
                .current_project
                .as_deref(),
            Some("rooftop-cinema")
        );
        assert!(switch(&session, "nope").is_err());
        assert_eq!(
            session
                .workspace()
                .expect("ws")
                .1
                .current_project
                .as_deref(),
            Some("rooftop-cinema")
        );

        let lab = Concepts::new(None, 2, |_| {});
        let runner = PipelineRunner::new(std::sync::Arc::new(NoSink));
        let studio = ProviderJobs::new(
            std::sync::Arc::new(maquette_core::providers::secrets::MemoryStore::default()),
            maquette_core::providers::Options::default(),
            maquette_core::inbox::send_ports(),
            |_| {},
        );
        let rows = summaries(&session, &runner, &lab, &studio).expect("list");
        assert_eq!(
            rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["moon-island", "rooftop-cinema"]
        );
        assert!(
            rows.iter()
                .all(|r| r.activity == ProjectActivity::default())
        );
    }

    struct NoSink;
    impl maquette_core::pipeline::runner::RunSink for NoSink {
        fn event(&self, _: maquette_core::pipeline::runner::PipelineEvent) {}
    }
}

#[cfg(test)]
mod repo_tests {
    use std::path::Path;

    use super::repo_refusal;

    #[test]
    fn ship_cannot_be_pointed_at_home_or_maquettes_own_folders() {
        let check = |path: &str| {
            repo_refusal(
                Path::new(path),
                Path::new("/w/maquette"),
                Path::new("/elsewhere/projects"),
                Path::new("/home/me"),
            )
        };
        assert!(check("/home/me").is_some());
        assert!(check("/home").is_some(), "a folder holding home");
        assert!(check("/").is_some());
        assert!(check("/elsewhere").is_some(), "a folder holding projects");
        assert!(check("/w/maquette").is_some());
        assert!(check("/w/maquette/projects/web").is_some());
        assert!(check("/w").is_some());
        assert!(check("/elsewhere/projects/web").is_some());
        assert!(check("/home/me/code/site").is_none());
    }
}
