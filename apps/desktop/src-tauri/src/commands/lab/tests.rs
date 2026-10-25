use super::*;
use maquette_core::{AppState, NewProject, ProjectTemplate, workspace};

#[test]
fn dropped_images_land_in_refs_and_show_in_the_view() {
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
            name: "Moon".into(),
            template: ProjectTemplate::Room,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let outside = dir.path().join("sketch.png");
    std::fs::write(&outside, b"png").expect("png");
    let got =
        import_refs(&session, &manifest.id, &[outside.display().to_string()]).expect("import");
    assert_eq!(got, vec!["refs/sketch.png".to_string()]);
    let text = dir.path().join("notes.txt");
    std::fs::write(&text, b"x").expect("txt");
    assert!(import_refs(&session, &manifest.id, &[text.display().to_string()]).is_err());
    let lab = Concepts::new(None, 2, |_| {});
    let view = view(&session, &lab, &manifest.id).expect("view");
    assert_eq!(view.refs.len(), 1);
    assert_eq!(view.concurrency, 4);
    assert!(view.directions.is_empty() && view.jobs.is_empty());
}

#[test]
fn a_direction_moves_to_another_project() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
    let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
    session
        .set_workspace(root.clone(), config.clone())
        .expect("open");
    for name in ["Moon Island", "Maquette landing"] {
        project::create(
            &root,
            &config.layout,
            NewProject {
                name: name.into(),
                template: ProjectTemplate::FloatingDiorama,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
    }
    let moon = project::dir(&root, &config.layout, "moon-island").expect("dir");
    let request = NewConcepts {
        project: "moon-island".into(),
        direction: "Maquette landing studio".into(),
        prompt: "A studio desk".into(),
        refs: Vec::new(),
        aspect: concepts::Aspect::Wide,
        count: 1,
    };
    let (direction, files) = concepts::prepare(&moon, &request).expect("prepare");
    std::fs::write(moon.join(&files[0].1), b"png").expect("png");
    concepts::add_image(&moon, &direction.id, 1, &files[0].1, None).expect("add");

    let lab = Concepts::new(None, 2, |_| {});
    let studio = ProviderJobs::new(
        std::sync::Arc::new(maquette_core::providers::MemoryStore::default()),
        maquette_core::providers::Options::default(),
        |_| {},
    );
    let moved = move_direction(
        &session,
        &lab,
        &studio,
        "moon-island",
        "maquette-landing-studio",
        "maquette-landing",
    )
    .expect("move");
    assert_eq!(moved.to.id, "maquette-landing");
    assert!(
        view(&session, &lab, "moon-island")
            .expect("moon")
            .directions
            .is_empty()
    );
    assert_eq!(
        view(&session, &lab, "maquette-landing")
            .expect("landing")
            .directions
            .len(),
        1
    );
    assert!(
        move_direction(
            &session,
            &lab,
            &studio,
            "moon-island",
            "nope",
            "maquette-landing"
        )
        .is_err()
    );
}

#[test]
fn object_refs_are_checked_and_need_codex() {
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
            name: "Landing".into(),
            template: ProjectTemplate::Room,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    std::fs::write(project_dir.join("refs/workbench.png"), b"png").expect("png");
    let lab = Concepts::new(None, 2, |_| {});
    let mut request = NewObjectRefs {
        project: manifest.id.clone(),
        source: "refs/workbench.png".into(),
        source_name: "Workbench".into(),
        objects: vec!["desk lamp".into()],
        notes: Default::default(),
        aspect: concepts::Aspect::Square,
        kind: AssetKind::Prop,
    };
    let err = make_objects(&session, &lab, &Toolbox::default(), &request).expect_err("codex");
    assert!(err.message.contains("Codex CLI"), "{}", err.message);
    assert!(
        view(&session, &lab, &manifest.id)
            .expect("view")
            .directions
            .is_empty(),
        "nothing written without Codex"
    );
    request.objects.clear();
    let err = make_objects(&session, &lab, &Toolbox::default(), &request).expect_err("none");
    assert!(
        err.message.contains("at least one object"),
        "{}",
        err.message
    );
    request.objects = vec!["lamp".into()];
    request.source = "../outside.png".into();
    assert!(make_objects(&session, &lab, &Toolbox::default(), &request).is_err());
    request.project = "nope".into();
    assert!(make_objects(&session, &lab, &Toolbox::default(), &request).is_err());
}
