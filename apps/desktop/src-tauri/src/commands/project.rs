use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::providers::ProviderJobs;
use maquette_core::{
    ImportScan, NewProject, ProjectActivity, ProjectManifest, ProjectSummary, RepoInspection,
    project, workspace,
};
use tauri::State;

use super::app::{WorkspaceSnapshot, snapshot};
use super::lab::Lab;
use super::pipeline::Pipelines;
use super::providers::Studio;
use super::{CommandResult, blocking};
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
    let (config, _) = project::switch(&root, &config, id)?;
    session.set_config(config.clone());
    snapshot(session, &root, &config)
}

#[tauri::command(async)]
#[specta::specta]
pub fn project_switch(session: State<'_, Session>, id: String) -> CommandResult<WorkspaceSnapshot> {
    switch(&session, &id)
}

/// Creates a project in the open workspace and makes it the current one.
#[tauri::command(async)]
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
    session.save_repos(&root)?;
    let dir = project::dir(&root, &config.layout, &manifest.id)?;
    maquette_core::pipeline::ensure(&dir, &manifest)?;
    config.current_project = Some(manifest.id);
    workspace::save(&root, &config)?;
    session.set_config(config.clone());
    snapshot(&session, &root, &config)
}

#[tauri::command(async)]
#[specta::specta]
pub fn project_update(
    session: State<'_, Session>,
    id: String,
    name: String,
    site_repo: Option<String>,
) -> CommandResult<ProjectManifest> {
    let (root, config) = session.workspace()?;
    let site_repo = site_repo.map(|p| session.expand(&p).display().to_string());
    let manifest = project::update(&root, &config.layout, &id, name, site_repo)?;
    session.save_repos(&root)?;
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
