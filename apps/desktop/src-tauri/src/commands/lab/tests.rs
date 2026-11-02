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
    std::fs::write(&outside, png(b"sketch")).expect("png");
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

/// A file that passes the shared image check, its middle set by `body`.
fn png(body: &[u8]) -> Vec<u8> {
    [b"\x89PNG\r\n\x1a\n", body, b"IEND\xaeB`\x82"].concat()
}

struct RefsFixture {
    dir: tempfile::TempDir,
    session: Session,
    refs: PathBuf,
}

fn refs_fixture() -> RefsFixture {
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
    let refs = project::dir(&root, &config.layout, &manifest.id)
        .expect("dir")
        .join(REFS_DIR);
    RefsFixture { dir, session, refs }
}

impl RefsFixture {
    /// A file `name` in its own folder `from`, to hold two of one name.
    fn file(&self, from: &str, name: &str, bytes: &[u8]) -> String {
        let folder = self.dir.path().join(from);
        std::fs::create_dir_all(&folder).expect("folder");
        let path = folder.join(name);
        std::fs::write(&path, bytes).expect("write");
        path.display().to_string()
    }

    fn import(&self, paths: &[String]) -> CommandResult<Vec<String>> {
        import_refs(&self.session, "moon", paths)
    }

    fn on_disk(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.refs)
            .map(|d| {
                d.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }
}

#[test]
fn a_ref_of_the_same_name_and_other_bytes_keeps_both() {
    let f = refs_fixture();
    let first = f.file("a", "front.png", &png(b"first"));
    let second = f.file("b", "front.png", &png(b"second"));
    assert_eq!(f.import(&[first]).expect("first"), ["refs/front.png"]);
    assert_eq!(f.import(&[second]).expect("second"), ["refs/front-2.png"]);
    assert_eq!(
        std::fs::read(f.refs.join("front.png")).expect("kept"),
        png(b"first")
    );
    assert_eq!(
        std::fs::read(f.refs.join("front-2.png")).expect("new"),
        png(b"second")
    );
}

#[test]
fn a_ref_with_the_same_bytes_reuses_the_file_it_matches() {
    let f = refs_fixture();
    let first = f.file("a", "front.png", &png(b"first"));
    let second = f.file("b", "front.png", &png(b"second"));
    f.import(&[first.clone(), second.clone()]).expect("both");
    assert_eq!(f.import(&[second]).expect("again"), ["refs/front-2.png"]);
    assert_eq!(f.import(&[first]).expect("again"), ["refs/front.png"]);
    assert_eq!(f.on_disk(), ["front-2.png", "front.png"]);
}

#[test]
fn one_drop_with_one_image_twice_names_it_once() {
    let f = refs_fixture();
    let one = f.file("a", "front.png", &png(b"same"));
    let two = f.file("b", "front.png", &png(b"same"));
    assert_eq!(f.import(&[one, two]).expect("both"), ["refs/front.png"]);
}

#[test]
fn a_file_already_in_refs_stays_where_it_is() {
    let f = refs_fixture();
    let first = f.file("a", "front.png", &png(b"first"));
    f.import(&[first]).expect("first");
    let inside = f.refs.join("front.png").display().to_string();
    assert_eq!(f.import(&[inside]).expect("inside"), ["refs/front.png"]);
    assert_eq!(f.on_disk(), ["front.png"]);
}

#[test]
fn a_ref_with_no_extension_takes_the_one_its_header_says() {
    let f = refs_fixture();
    let bare = f.file("a", "front", &png(b"first"));
    assert_eq!(f.import(&[bare]).expect("bare"), ["refs/front.png"]);
}

#[test]
fn a_bad_ref_stops_the_whole_import_and_names_the_file() {
    let f = refs_fixture();
    let good = f.file("a", "good.png", &png(b"good"));
    let bad = f.file("a", "fake.png", b"not a png");
    let error = format!("{:?}", f.import(&[good, bad]).expect_err("fake"));
    assert!(error.contains("fake.png"), "{error}");
    assert!(f.on_disk().is_empty(), "{:?}", f.on_disk());
}

#[test]
fn a_real_image_with_bytes_after_its_end_is_a_ref() {
    let f = refs_fixture();
    let mut bytes = png(b"first");
    bytes.push(0);
    let trailing = f.file("a", "front.png", &bytes);
    assert_eq!(f.import(&[trailing]).expect("png"), ["refs/front.png"]);
}

#[test]
fn names_that_differ_only_in_case_are_one_name_on_every_disk() {
    let f = refs_fixture();
    let upper = f.file("a", "Front.png", &png(b"first"));
    let lower = f.file("b", "front.png", &png(b"first"));
    let other = f.file("c", "FRONT.png", &png(b"second"));
    assert_eq!(
        f.import(&[upper, lower.clone()]).expect("one"),
        ["refs/Front.png"]
    );
    assert_eq!(f.import(&[lower]).expect("again"), ["refs/Front.png"]);
    assert_eq!(f.import(&[other]).expect("other"), ["refs/FRONT-2.png"]);
    assert_eq!(f.on_disk(), ["FRONT-2.png", "Front.png"]);
}

#[test]
fn a_ref_must_be_a_file_within_the_limits() {
    let f = refs_fixture();
    let folder = f.dir.path().join("folder.png");
    std::fs::create_dir_all(&folder).expect("folder");
    let not_file = format!("{:?}", f.import(&[folder.display().to_string()]));
    assert!(not_file.contains("folder.png"), "{not_file}");

    let big = f.file("a", "big.png", &png(b"big"));
    std::fs::OpenOptions::new()
        .write(true)
        .open(&big)
        .expect("open")
        .set_len(51 * 1024 * 1024)
        .expect("grow");
    let too_big = format!("{:?}", f.import(&[big]));
    assert!(
        too_big.contains("big.png") && too_big.contains("50 MB"),
        "{too_big}"
    );

    let many: Vec<String> = (0..51)
        .map(|n| f.file("many", &format!("{n}.png"), &png(b"x")))
        .collect();
    let too_many = format!("{:?}", f.import(&many));
    assert!(too_many.contains("50 images"), "{too_many}");

    let heavy: Vec<String> = (0..11)
        .map(|n| {
            let path = f.file("heavy", &format!("{n}.png"), &png(b"x"));
            std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .expect("open")
                .set_len(49 * 1024 * 1024)
                .expect("grow");
            path
        })
        .collect();
    let too_heavy = format!("{:?}", f.import(&heavy));
    assert!(too_heavy.contains("512 MB"), "{too_heavy}");
    assert!(f.on_disk().is_empty(), "{:?}", f.on_disk());
}

#[test]
fn a_direction_in_another_script_belongs_to_its_own_card() {
    let f = refs_fixture();
    let (root, config, _) = project_dir(&f.session, "moon").expect("dir");
    let mut manifest = project::load(&root, &config.layout, "moon").expect("load");
    for name in ["雪", "月"] {
        maquette_core::assets::add(
            &mut manifest,
            maquette_core::NewAsset {
                name: name.into(),
                kind: AssetKind::Prop,
                stage: maquette_core::Stage::Concept,
            },
        )
        .expect("card");
    }
    let id = |name: &str| {
        manifest
            .assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.id.clone())
    };
    assert_eq!(direction_asset(&manifest, "月"), id("月"));
    assert_eq!(direction_asset(&manifest, "雪"), id("雪"));
    assert_eq!(direction_asset(&manifest, "星"), None);
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
        maquette_core::inbox::send_ports(),
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
