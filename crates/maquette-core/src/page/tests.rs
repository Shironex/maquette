use super::*;

fn workspace() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("workspace");
    let project = dir.path().join("projects/moon-island");
    std::fs::create_dir_all(project.join("concepts")).expect("project");
    (dir, project)
}

#[test]
fn a_project_without_page_json_gets_the_defaults() {
    let (dir, project) = workspace();
    let settings = load(&project, dir.path()).expect("load");
    assert_eq!(settings, PageSettings::default());
    assert_eq!(settings.path, "/");
    assert_eq!(settings.source, PageSource::Site);
}

#[test]
fn settings_round_trip_tidied() {
    let (dir, project) = workspace();
    let concept = project.join("concepts/home_v2.html");
    std::fs::write(&concept, "<!doctype html><title>Home</title>").expect("concept");
    let saved = save(
        &project,
        dir.path(),
        &PageSettings {
            dev_command: Some("  pnpm dev --port {port}  ".into()),
            concept: Some(concept.to_string_lossy().into_owned()),
            source: PageSource::Concept,
            path: " /pricing ".into(),
            ..PageSettings::default()
        },
    )
    .expect("save");
    assert_eq!(saved.dev_command.as_deref(), Some("pnpm dev --port {port}"));
    assert_eq!(saved.path, "/pricing");
    assert_eq!(load(&project, dir.path()).expect("load"), saved);
    let text = std::fs::read_to_string(project.join(PAGE_FILE)).expect("read");
    assert!(
        text.contains(r#""concept": "projects/moon-island/concepts/home_v2.html""#),
        "kept relative to the workspace: {text}"
    );

    // An older file with the concept's absolute path still reads.
    let old = text.replace(
        "projects/moon-island/concepts/home_v2.html",
        &concept.to_string_lossy().replace('\\', "\\\\"),
    );
    std::fs::write(project.join(PAGE_FILE), old).expect("old");
    assert_eq!(load(&project, dir.path()).expect("old"), saved);

    // A blank command goes back to the site's own and a blank path to /.
    let cleared = save(
        &project,
        dir.path(),
        &PageSettings {
            dev_command: Some("   ".into()),
            path: String::new(),
            ..saved
        },
    )
    .expect("clear");
    assert_eq!(cleared.dev_command, None);
    assert_eq!(cleared.path, "/");
}

#[test]
fn bad_settings_are_refused_and_nothing_is_written() {
    let (dir, project) = workspace();
    let outside = tempfile::tempdir().expect("outside");
    let stray = outside.path().join("page.html");
    std::fs::write(&stray, "<html></html>").expect("stray");
    let text = project.join("concepts/notes.txt");
    std::fs::write(&text, "notes").expect("text");
    let cases = [
        PageSettings {
            dev_command: Some("./serve.sh".into()),
            ..PageSettings::default()
        },
        PageSettings {
            dev_command: Some("pnpm \"dev".into()),
            ..PageSettings::default()
        },
        PageSettings {
            concept: Some(stray.to_string_lossy().into_owned()),
            ..PageSettings::default()
        },
        PageSettings {
            concept: Some(text.to_string_lossy().into_owned()),
            ..PageSettings::default()
        },
        PageSettings {
            concept: Some(
                project
                    .join("concepts/missing.html")
                    .to_string_lossy()
                    .into_owned(),
            ),
            ..PageSettings::default()
        },
        PageSettings {
            source: PageSource::Concept,
            ..PageSettings::default()
        },
        PageSettings {
            path: "pricing".into(),
            ..PageSettings::default()
        },
        PageSettings {
            path: "//evil.example".into(),
            ..PageSettings::default()
        },
        PageSettings {
            path: "/a b".into(),
            ..PageSettings::default()
        },
    ];
    for case in cases {
        assert!(save(&project, dir.path(), &case).is_err(), "{case:?}");
    }
    assert!(!project.join(PAGE_FILE).exists());
}

#[test]
fn a_file_from_a_newer_maquette_is_refused() {
    let (dir, project) = workspace();
    std::fs::write(project.join(PAGE_FILE), r#"{ "version": 9 }"#).expect("write");
    assert_eq!(
        load(&project, dir.path()).expect_err("newer").code(),
        "unsupported-version"
    );
    std::fs::write(project.join(PAGE_FILE), r#"{ "version": 1, "path": "/x" }"#).expect("old");
    let partial = load(&project, dir.path()).expect("partial");
    assert_eq!(partial.path, "/x");
    assert_eq!(partial.source, PageSource::Site);
}
