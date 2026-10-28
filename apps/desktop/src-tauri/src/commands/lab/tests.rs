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
            target: Default::default(),
            game: None,
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
                target: Default::default(),
                game: None,
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
        use_style: true,
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
            target: Default::default(),
            game: None,
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
        use_style: true,
    };
    let err = make_objects(
        &session,
        &lab,
        &Toolbox::default(),
        &request,
        Backend::default(),
    )
    .expect_err("codex");
    assert!(err.message.contains("Codex CLI"), "{}", err.message);
    assert!(
        view(&session, &lab, &manifest.id)
            .expect("view")
            .directions
            .is_empty(),
        "nothing written without Codex"
    );
    request.objects.clear();
    let err = make_objects(
        &session,
        &lab,
        &Toolbox::default(),
        &request,
        Backend::default(),
    )
    .expect_err("none");
    assert!(
        err.message.contains("at least one object"),
        "{}",
        err.message
    );
    request.objects = vec!["lamp".into()];
    request.source = "../outside.png".into();
    assert!(
        make_objects(
            &session,
            &lab,
            &Toolbox::default(),
            &request,
            Backend::default()
        )
        .is_err()
    );
    request.project = "nope".into();
    assert!(
        make_objects(
            &session,
            &lab,
            &Toolbox::default(),
            &request,
            Backend::default()
        )
        .is_err()
    );
}

#[test]
fn the_style_board_takes_files_from_anywhere_for_the_app_but_only_the_workspace_for_agents() {
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
            name: "Frostveil".into(),
            template: ProjectTemplate::Room,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    // The PNG signature and header are all the board's image check reads.
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&[0, 0, 0, 4, 0, 0, 0, 4, 8, 2, 0, 0, 0]);
    let outside = dir.path().join("frost_glass.png");
    std::fs::write(&outside, &png).expect("png");
    let shared = root.join("shared.png");
    std::fs::write(&shared, &png).expect("png");
    let change = |path: &std::path::Path| StyleBoardChange {
        line: Some("frosted glass".into()),
        images: Some(vec![path.display().to_string()]),
        ..Default::default()
    };
    let refused = set_style(&session, &manifest.id, &change(&outside), false).expect_err("agent");
    assert!(
        refused.message.contains("outside the workspace"),
        "{refused:?}"
    );
    let agent = set_style(&session, &manifest.id, &change(&shared), false).expect("agent");
    assert_eq!(agent.files(), ["style/shared.png"]);
    let app = set_style(&session, &manifest.id, &change(&outside), true).expect("app");
    assert_eq!(app.files(), ["style/frost_glass.png"]);
    assert_eq!(app.revision, 2);
    let lab = Concepts::new(None, 2, |_| {});
    let view = view(&session, &lab, &manifest.id).expect("view");
    assert_eq!(view.style, app);
}
