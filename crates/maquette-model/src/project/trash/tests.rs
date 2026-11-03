use super::*;
use crate::project::{NewProject, create};

fn new(name: &str) -> NewProject {
    NewProject {
        name: name.to_string(),
        template: ProjectTemplate::FloatingDiorama,
        target: ProjectTarget::Website,
        game: None,
        site_repo: None,
        source_folder: None,
    }
}

fn workspace() -> (tempfile::TempDir, FolderLayout) {
    (
        tempfile::tempdir().expect("tempdir"),
        FolderLayout::default(),
    )
}

#[test]
fn a_removed_project_waits_in_the_trash_and_comes_back_whole() {
    let (dir, layout) = workspace();
    let root = dir.path();
    create(root, &layout, new("Moon Island")).expect("create");
    let folder = root.join("projects").join("moon-island");
    fs::write(folder.join("refs").join("bowl.png"), b"png").expect("ref");
    let own = root.join(TRASH_DIR).join("moon-island").join("lamp-1");
    fs::create_dir_all(&own).expect("own trash");
    fs::write(own.join("removed.json"), b"{}").expect("record");
    repos::set(root, "moon-island", Some("/code/site".into()));

    let removed = remove_project(root, &layout, "moon-island").expect("remove");
    assert_eq!(removed.id, "moon-island");
    assert_eq!(removed.name, "Moon Island");
    assert!(removed.path.starts_with("trash/_projects/moon-island-"));
    assert!(!folder.exists());
    assert!(!root.join(TRASH_DIR).join("moon-island").exists());
    assert!(
        crate::project::list(root, &layout)
            .expect("list")
            .is_empty()
    );
    let err = load(root, &layout, "moon-island").expect_err("gone");
    assert_eq!(err.code(), "not-found");
    assert!(err.to_string().contains("removed to the workspace trash"));
    assert_eq!(list_removed(root).expect("trash"), vec![removed.clone()]);

    repos::set(root, "moon-island", None);
    let back = restore_project(root, &layout, &removed.trash).expect("restore");
    assert_eq!(back, "moon-island");
    assert_eq!(
        fs::read(folder.join("refs").join("bowl.png")).expect("ref back"),
        b"png"
    );
    assert!(
        own.join("removed.json").is_file(),
        "its own trash came back"
    );
    assert_eq!(
        repos::get(root, "moon-island").as_deref(),
        Some("/code/site")
    );
    assert!(list_removed(root).expect("trash").is_empty());
    assert!(!root.join(removed.path).exists());
}

#[test]
fn restore_never_writes_over_a_project_that_took_the_id() {
    let (dir, layout) = workspace();
    let root = dir.path();
    create(root, &layout, new("Moon Island")).expect("create");
    let removed = remove_project(root, &layout, "moon-island").expect("remove");
    create(root, &layout, new("Moon Island")).expect("again");

    let err = restore_project(root, &layout, &removed.trash).expect_err("taken");
    assert_eq!(err.code(), "invalid");
    assert!(err.to_string().contains("in the workspace again"));
    assert_eq!(list_removed(root).expect("trash").len(), 1);
    assert!(root.join(&removed.path).join("project").is_dir());
}

#[test]
fn removing_a_missing_project_or_restoring_a_strange_id_fails_cleanly() {
    let (dir, layout) = workspace();
    let root = dir.path();
    assert_eq!(
        remove_project(root, &layout, "nope")
            .expect_err("missing")
            .code(),
        "not-found"
    );
    assert!(
        !root.join(TRASH_DIR).join(PROJECTS_TRASH).exists()
            || list_removed(root).expect("list").is_empty()
    );
    for bad in ["../x", "nope", ""] {
        assert_eq!(
            restore_project(root, &layout, bad).expect_err("bad").code(),
            "not-found",
            "{bad}"
        );
    }
}

#[test]
fn two_removes_of_the_same_id_keep_both_and_list_newest_first() {
    let (dir, layout) = workspace();
    let root = dir.path();
    create(root, &layout, new("Moon Island")).expect("create");
    let first = remove_project(root, &layout, "moon-island").expect("remove");
    create(root, &layout, new("Moon Island")).expect("again");
    let second = remove_project(root, &layout, "moon-island").expect("remove again");
    assert_ne!(first.trash, second.trash);
    let listed = list_removed(root).expect("list");
    assert_eq!(listed.len(), 2);
    assert!(listed[0].removed_at >= listed[1].removed_at);
}

#[test]
fn the_projects_trash_folder_is_no_project_trash() {
    let (dir, layout) = workspace();
    assert_eq!(
        project_trash(dir.path(), &layout, PROJECTS_TRASH)
            .expect_err("reserved")
            .code(),
        "invalid"
    );
}
