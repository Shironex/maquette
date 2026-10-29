use std::fs;
use std::path::Path;

use super::*;
use crate::project::{NewProject, PROJECT_FILE, ProjectTemplate, create, load, update};

const OTHER_COMPUTER: &str = "/Volumes/elsewhere/Projects/studio-site";

/// A `project.json` as Maquette wrote it before repos moved per computer.
fn old_project(root: &Path, id: &str, site_repo: &str) -> std::path::PathBuf {
    let dir = root.join("projects").join(id);
    fs::create_dir_all(&dir).expect("dir");
    let file = dir.join(PROJECT_FILE);
    let json = serde_json::json!({
        "version": 1,
        "id": id,
        "name": "Studio",
        "template": "room",
        "siteRepo": site_repo,
        "sourceFolder": null,
        "createdAt": "2026-01-01T00:00:00.000Z",
        "assets": []
    });
    fs::write(&file, serde_json::to_string_pretty(&json).expect("json")).expect("write");
    file
}

fn on_disk(file: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(file).expect("read")).expect("json")
}

#[test]
fn an_old_path_on_this_computer_moves_into_the_map_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("ws");
    let repo = dir.path().join("studio-site");
    fs::create_dir_all(&repo).expect("repo");
    let repo_text = repo.display().to_string();
    let file = old_project(&root, "studio", &repo_text);
    let layout = FolderLayout::default();

    // Before the move the old path is read as it is.
    let before = load(&root, &layout, "studio").expect("load");
    assert_eq!(before.site_repo.as_deref(), Some(repo_text.as_str()));
    assert_eq!(before.site_repo_state, SiteRepoState::Here);

    assert_eq!(migrate(&root, &layout).expect("migrate"), ["studio"]);
    assert_eq!(get(&root, "studio").as_deref(), Some(repo_text.as_str()));
    let saved = on_disk(&file);
    assert_eq!(saved["siteRepo"], serde_json::Value::Null);
    assert_eq!(saved["siteRepoName"], "studio-site");
    assert!(
        saved.get("siteRepoState").is_none(),
        "the state is never saved"
    );

    let after = load(&root, &layout, "studio").expect("load");
    assert_eq!(after.site_repo.as_deref(), Some(repo_text.as_str()));
    assert_eq!(after.site_repo_state, SiteRepoState::Here);

    let again = fs::read_to_string(&file).expect("read");
    assert!(migrate(&root, &layout).expect("again").is_empty());
    assert_eq!(
        fs::read_to_string(&file).expect("read"),
        again,
        "idempotent"
    );
}

#[test]
fn another_computers_path_is_kept_and_asked_for() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("ws");
    let file = old_project(&root, "studio", OTHER_COMPUTER);
    let layout = FolderLayout::default();
    let original = fs::read_to_string(&file).expect("read");

    let manifest = load(&root, &layout, "studio").expect("load");
    assert_eq!(manifest.site_repo, None);
    assert_eq!(manifest.site_repo_state, SiteRepoState::Elsewhere);
    assert_eq!(manifest.site_repo_name.as_deref(), Some("studio-site"));

    assert!(migrate(&root, &layout).expect("migrate").is_empty());
    assert_eq!(fs::read_to_string(&file).expect("read"), original);

    // Any other write keeps the other computer's path for it to move.
    crate::project::modify(&root, &layout, "studio", |m| {
        m.name = "Studio 2".into();
        Ok(())
    })
    .expect("modify");
    assert_eq!(on_disk(&file)["siteRepo"], OTHER_COMPUTER);

    // A rename from Settings, with no repo here, still asks afterwards.
    let renamed = update(&root, &layout, "studio", "Studio 2".into(), None).expect("rename");
    assert_eq!(renamed.site_repo_state, SiteRepoState::Elsewhere);
    assert_eq!(renamed.site_repo_name.as_deref(), Some("studio-site"));

    // Telling this computer where it is answers the question.
    let here = dir.path().join("my-checkout");
    fs::create_dir_all(&here).expect("here");
    let updated = update(
        &root,
        &layout,
        "studio",
        "Studio 2".into(),
        Some(here.display().to_string()),
    )
    .expect("update");
    assert_eq!(updated.site_repo_state, SiteRepoState::Here);
    assert_eq!(updated.site_repo_name.as_deref(), Some("my-checkout"));
    let saved = on_disk(&file);
    assert_eq!(
        saved["siteRepo"], OTHER_COMPUTER,
        "still the other's to move"
    );
    assert_eq!(saved["siteRepoName"], "my-checkout");
}

#[test]
fn a_new_project_saves_only_the_folder_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("ws");
    let repo = dir.path().join("site");
    fs::create_dir_all(&repo).expect("repo");
    let layout = FolderLayout::default();
    let made = create(
        &root,
        &layout,
        NewProject {
            name: "Studio".into(),
            template: ProjectTemplate::Room,
            target: Default::default(),
            site_repo: Some(repo.display().to_string()),
            source_folder: None,
            game: None,
        },
    )
    .expect("create");
    assert_eq!(made.site_repo_state, SiteRepoState::Here);
    let file = root.join("projects/studio").join(PROJECT_FILE);
    let saved = on_disk(&file);
    assert_eq!(saved["siteRepo"], serde_json::Value::Null);
    assert_eq!(saved["siteRepoName"], "site");

    // Another computer (an empty map for this root) is asked.
    install(&root, RepoMap::new());
    let there = load(&root, &layout, "studio").expect("load");
    assert_eq!(there.site_repo, None);
    assert_eq!(there.site_repo_state, SiteRepoState::Elsewhere);

    // A path that went away says so.
    install(
        &root,
        RepoMap::from([(
            "studio".into(),
            dir.path().join("gone").display().to_string(),
        )]),
    );
    assert_eq!(
        load(&root, &layout, "studio")
            .expect("load")
            .site_repo_state,
        SiteRepoState::Gone
    );
}

#[test]
fn folder_names_and_root_keys_read_either_separator() {
    assert_eq!(folder_name("/Users/me/site/").as_deref(), Some("site"));
    assert_eq!(folder_name(r"C:\Users\me\game").as_deref(), Some("game"));
    assert_eq!(folder_name("/"), None);
    assert_eq!(root_key(Path::new("/ws/")), "/ws");
    assert_eq!(root_key(Path::new("/")), "/");
}
