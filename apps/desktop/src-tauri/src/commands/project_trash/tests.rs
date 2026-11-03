use std::sync::Arc;

use maquette_core::{AppState, NewProject, ProjectTemplate, WorkspaceConfig, workspace};

use super::*;

struct NoSink;
impl maquette_core::pipeline::runner::RunSink for NoSink {
    fn event(&self, _: maquette_core::pipeline::runner::PipelineEvent) {}
}

struct World {
    _dir: tempfile::TempDir,
    session: Session,
    runner: PipelineRunner,
    lab: Concepts,
    studio: ProviderJobs,
}

impl World {
    fn new(names: &[&str]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        for name in names {
            let new = NewProject {
                name: (*name).into(),
                template: ProjectTemplate::FloatingDiorama,
                target: Default::default(),
                game: None,
                site_repo: None,
                source_folder: None,
            };
            project::create(&root, &config.layout, new).expect("project");
        }
        Self {
            _dir: dir,
            session,
            runner: PipelineRunner::new(Arc::new(NoSink)),
            lab: Concepts::new(None, 2, |_| {}),
            studio: ProviderJobs::new(
                Arc::new(maquette_core::providers::secrets::MemoryStore::default()),
                maquette_core::providers::Options::default(),
                maquette_core::inbox::send_ports(),
                |_| {},
            ),
        }
    }

    fn remove(&self, id: &str) -> CommandResult<ProjectRemoved> {
        let running = Running {
            runner: &self.runner,
            lab: &self.lab,
            studio: &self.studio,
            building: Vec::new(),
        };
        remove(&self.session, &running, None, None, id)
    }

    fn open(&self) -> Option<String> {
        self.session.workspace().expect("ws").1.current_project
    }
}

#[test]
fn removing_the_open_project_opens_the_next_one_then_none() {
    let world = World::new(&["Moon Island", "Rooftop cinema", "Study room"]);
    super::super::project::switch(&world.session, "rooftop-cinema").expect("switch");

    let done = world.remove("rooftop-cinema").expect("remove");
    assert_eq!(done.removed.id, "rooftop-cinema");
    assert_eq!(world.open().as_deref(), Some("study-room"));
    assert_eq!(done.workspace.project.expect("open").id, "study-room");
    assert_eq!(done.workspace.projects.len(), 2);

    let done = world.remove("moon-island").expect("remove another");
    assert_eq!(
        world.open().as_deref(),
        Some("study-room"),
        "the open one stays"
    );
    assert_eq!(done.workspace.projects.len(), 1);

    let done = world.remove("study-room").expect("remove the last");
    assert_eq!(world.open(), None);
    assert!(done.workspace.project.is_none());
    assert!(done.workspace.projects.is_empty());

    let (root, _) = world.session.workspace().expect("ws");
    let removed = trash::list_removed(&root).expect("trash");
    assert_eq!(removed.len(), 3);
    let back = restore(&world.session, &removed[0].trash).expect("restore");
    assert_eq!(back.project.expect("open").id, removed[0].id);
}

#[test]
fn a_project_that_is_not_there_cannot_be_removed() {
    let world = World::new(&["Moon Island"]);
    let err = world.remove("nope").expect_err("missing");
    assert_eq!(err.code, "not-found");
}

#[test]
fn work_going_on_refuses_the_remove_with_why() {
    let quiet = Busy::default();
    assert_eq!(refusal(quiet, "Moon Island"), None);
    let says = |busy: Busy| refusal(busy, "Moon Island").expect("refused");
    assert!(says(Busy { run: true, ..quiet }).contains("pipeline run"));
    assert!(
        says(Busy {
            building: 1,
            ..quiet
        })
        .starts_with("1 built asset is building")
    );
    assert!(
        says(Busy {
            concepts: 3,
            ..quiet
        })
        .starts_with("3 concept images are")
    );
    let sends = says(Busy { sends: 1, ..quiet });
    assert!(sends.starts_with("1 send is not over"));
    assert!(sends.contains("records stay"));
}

#[test]
fn the_next_project_is_the_one_after_else_before() {
    let world = World::new(&["Alpha", "Beta", "Gamma"]);
    let (root, config) = world.session.workspace().expect("ws");
    let rows = project::summaries(&root, &config.layout).expect("rows");
    assert_eq!(next_open(&rows, "alpha").as_deref(), Some("beta"));
    assert_eq!(next_open(&rows, "gamma").as_deref(), Some("beta"));
    assert_eq!(next_open(&rows[..1], "alpha"), None);
    assert_eq!(next_open(&rows, "nope"), None);
}

#[test]
fn a_restored_repo_waits_for_a_person_again() {
    let world = World::new(&["Moon Island"]);
    let (root, _) = world.session.workspace().expect("ws");
    let key = repos::root_key(&root);
    repos::set(&root, "moon-island", Some("/code/site".into()));
    mark_repo(&world.session, &root, "moon-island", RepoSetBy::Person).expect("mark");
    world.session.save_repos(&root).expect("save");
    assert!(
        !world
            .session
            .app_state()
            .repo_set_by_agent(&key, "moon-island")
    );

    let done = world.remove("moon-island").expect("remove");
    repos::set(&root, "moon-island", None);
    world.session.save_repos(&root).expect("save");
    restore(&world.session, &done.removed.trash).expect("restore");

    assert_eq!(
        repos::get(&root, "moon-island").as_deref(),
        Some("/code/site")
    );
    assert!(
        world
            .session
            .app_state()
            .repo_set_by_agent(&key, "moon-island"),
        "nobody confirmed the repo it came back with"
    );
}
