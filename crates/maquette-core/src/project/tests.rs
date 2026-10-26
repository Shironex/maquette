use super::*;
use std::time::Duration;

fn new_project(name: &str) -> NewProject {
    NewProject {
        name: name.to_string(),
        template: ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: None,
        source_folder: None,
    }
}

#[test]
fn slugify_cases() {
    assert_eq!(slugify("Moon Island"), "moon-island");
    assert_eq!(slugify("  Moon   Island!! "), "moon-island");
    assert_eq!(slugify("Zażółć gęślą jaźń"), "zazolc-gesla-jazn");
    assert_eq!(slugify("Café Straße"), "cafe-strasse");
    assert_eq!(slugify("moon_island_v2"), "moon-island-v2");
    assert_eq!(slugify("月の島"), "project");
    assert_eq!(slugify(""), "project");
    assert_eq!(slugify("---"), "project");
    assert_eq!(slugify(&"a ".repeat(100)).chars().count(), 59);
}

#[test]
fn rfc3339_formatting() {
    assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
    let t = UNIX_EPOCH + Duration::from_millis(1_709_210_096_789);
    assert_eq!(rfc3339_utc(t), "2024-02-29T12:34:56.789Z");
    assert_eq!(civil_from_days(-1), (1969, 12, 31));
}

#[test]
fn create_makes_folders_and_manifest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let layout = FolderLayout::default();
    let mut new = new_project("  Moon Island ");
    new.site_repo = Some("  ".to_string());
    new.source_folder = Some("/tmp/src".to_string());
    let manifest = create(dir.path(), &layout, new).expect("create");
    assert_eq!(manifest.id, "moon-island");
    assert_eq!(manifest.name, "Moon Island");
    assert_eq!(manifest.site_repo, None);
    assert_eq!(manifest.source_folder.as_deref(), Some("/tmp/src"));
    assert!(manifest.created_at.ends_with('Z'));
    let root = dir.path().join("projects").join("moon-island");
    for sub in PROJECT_DIRS {
        assert!(root.join(sub).is_dir(), "{sub}");
    }
    let json = fs::read_to_string(root.join(PROJECT_FILE)).expect("read");
    assert!(json.contains("\"createdAt\""));
    assert!(json.contains("\"template\": \"floating-diorama\""));
    assert_eq!(
        load(dir.path(), &layout, "moon-island").expect("load"),
        manifest
    );
}

#[test]
fn create_rejects_bad_names_and_suffixes_collisions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let layout = FolderLayout::default();
    assert_eq!(
        create(dir.path(), &layout, new_project("   "))
            .expect_err("empty")
            .code(),
        "invalid"
    );
    assert!(create(dir.path(), &layout, new_project(&"x".repeat(81))).is_err());
    assert!(create(dir.path(), &layout, new_project(&"x".repeat(80))).is_ok());

    let a = create(dir.path(), &layout, new_project("Moon Island")).expect("a");
    let b = create(dir.path(), &layout, new_project("moon island")).expect("b");
    let c = create(dir.path(), &layout, new_project("Moon-Island")).expect("c");
    assert_eq!(
        [a.id.as_str(), b.id.as_str(), c.id.as_str()],
        ["moon-island", "moon-island-2", "moon-island-3"]
    );
}

#[test]
fn update_and_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let layout = FolderLayout::default();
    assert!(list(dir.path(), &layout).expect("empty list").is_empty());
    let first = create(dir.path(), &layout, new_project("First")).expect("first");
    let second = create(dir.path(), &layout, new_project("Second")).expect("second");
    fs::create_dir_all(dir.path().join("projects").join("stray")).expect("mkdir");

    let updated = update(
        dir.path(),
        &layout,
        &first.id,
        " Renamed ".to_string(),
        Some("/sites/moon".to_string()),
    )
    .expect("update");
    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.id, "first");
    assert_eq!(load(dir.path(), &layout, "first").expect("load"), updated);

    let ids: Vec<String> = list(dir.path(), &layout)
        .expect("list")
        .into_iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(ids, vec![first.id, second.id]);

    assert_eq!(
        load(dir.path(), &layout, "nope")
            .expect_err("missing")
            .code(),
        "not-found"
    );
    assert_eq!(
        load(dir.path(), &layout, "../x")
            .expect_err("traversal")
            .code(),
        "invalid"
    );
    assert!(update(dir.path(), &layout, "second", String::new(), None).is_err());
}

#[test]
fn modify_keeps_assets_and_old_files_load_without_them() {
    use crate::assets::{self, AssetKind, NewAsset, Stage};
    let dir = tempfile::tempdir().expect("tempdir");
    let layout = FolderLayout::default();
    let project = create(dir.path(), &layout, new_project("Moon Island")).expect("create");
    modify(dir.path(), &layout, &project.id, |m| {
        assets::add(
            m,
            NewAsset {
                name: "Torii".into(),
                kind: AssetKind::Prop,
                stage: Stage::Blender,
            },
        )
    })
    .expect("add asset");
    let renamed = update(dir.path(), &layout, &project.id, "Moon".into(), None).expect("update");
    assert_eq!(renamed.assets.len(), 1);
    assert_eq!(renamed.assets[0].id, "torii");

    let failed = modify(dir.path(), &layout, &project.id, |m| {
        m.assets.clear();
        Err::<(), _>(CoreError::Invalid("no".into()))
    });
    assert!(failed.is_err());
    assert_eq!(
        load(dir.path(), &layout, &project.id)
            .expect("load")
            .assets
            .len(),
        1
    );

    let old = dir.path().join("projects").join("old");
    fs::create_dir_all(&old).expect("mkdir");
    fs::write(
        old.join(PROJECT_FILE),
        r#"{"version":1,"id":"old","name":"Old","template":"room","siteRepo":null,"sourceFolder":null,"createdAt":"2026-01-01T00:00:00.000Z"}"#,
    )
    .expect("write");
    assert!(
        load(dir.path(), &layout, "old")
            .expect("old")
            .assets
            .is_empty()
    );
}

#[test]
fn summaries_count_assets_and_pick_a_picture() {
    use crate::assets::{self, AssetKind, NewAsset, Stage};
    let dir = tempfile::tempdir().expect("tempdir");
    let layout = FolderLayout::default();
    let moon = create(dir.path(), &layout, new_project("Moon Island")).expect("moon");
    let mut room = new_project("Rooftop cinema");
    room.template = ProjectTemplate::Room;
    room.site_repo = Some("/sites/cinema".into());
    let cinema = create(dir.path(), &layout, room).expect("cinema");

    let empty = summaries(dir.path(), &layout).expect("list");
    assert_eq!(empty.len(), 2);
    assert_eq!(empty[0].id, moon.id);
    assert_eq!(empty[0].asset_count, 0);
    assert_eq!(empty[0].thumb, None);
    assert_eq!(empty[0].activity, ProjectActivity::default());
    assert_eq!(empty[1].template, ProjectTemplate::Room);
    assert_eq!(empty[1].site_repo.as_deref(), Some("/sites/cinema"));
    assert!(empty[1].dir.ends_with("rooftop-cinema"));
    assert_eq!(empty[1].created_at, cinema.created_at);

    let moon_dir = dir.path().join("projects").join("moon-island");
    fs::create_dir_all(moon_dir.join("thumbs")).expect("mkdir");
    fs::write(moon_dir.join("thumbs/torii_v1.png"), b"png").expect("thumb");
    fs::write(moon_dir.join("thumbs/island_v7.png"), b"png").expect("thumb");
    modify(dir.path(), &layout, &moon.id, |m| {
        for (name, kind, thumb) in [
            ("Torii", AssetKind::Prop, Some("thumbs/torii_v1.png")),
            ("Island", AssetKind::Scene, Some("thumbs/island_v7.png")),
            ("Lantern", AssetKind::Prop, Some("thumbs/gone.png")),
            ("Bench", AssetKind::Prop, None),
        ] {
            let a = assets::add(
                m,
                NewAsset {
                    name: name.into(),
                    kind,
                    stage: Stage::Blender,
                },
            )?;
            let slot = m.assets.iter_mut().find(|x| x.id == a.id).expect("asset");
            slot.thumbnail = thumb.map(str::to_string);
        }
        Ok(())
    })
    .expect("assets");
    let moon_row = summary(
        dir.path(),
        &layout,
        &load(dir.path(), &layout, &moon.id).expect("load"),
    );
    assert_eq!(moon_row.asset_count, 4);
    assert_eq!(moon_row.thumb.as_deref(), Some("thumbs/island_v7.png"));

    fs::create_dir_all(moon_dir.join("web/posters")).expect("mkdir");
    fs::write(moon_dir.join("web/posters/stops.json"), b"{}").expect("json");
    fs::write(moon_dir.join("web/posters/stop_02.png"), b"png").expect("poster");
    fs::write(moon_dir.join("web/posters/stop_01.png"), b"png").expect("poster");
    let first = &summaries(dir.path(), &layout).expect("list")[0];
    assert_eq!(first.thumb.as_deref(), Some("web/posters/stop_01.png"));
}

#[test]
fn switch_persists_the_current_project_and_refuses_unknown_ones() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = crate::workspace::create(dir.path(), &WorkspaceConfig::default()).expect("ws");
    let moon = create(dir.path(), &config.layout, new_project("Moon Island")).expect("moon");
    let cinema = create(dir.path(), &config.layout, new_project("Cinema")).expect("cinema");

    let (next, manifest) = switch(dir.path(), &config, &cinema.id).expect("switch");
    assert_eq!(manifest.id, cinema.id);
    assert_eq!(next.current_project.as_deref(), Some("cinema"));
    let reopened = crate::workspace::open(dir.path()).expect("reopen");
    assert_eq!(reopened.current_project.as_deref(), Some("cinema"));

    let (back, _) = switch(dir.path(), &reopened, &moon.id).expect("back");
    assert_eq!(back.current_project.as_deref(), Some("moon-island"));

    for bad in ["nope", "../moon-island", ""] {
        assert!(switch(dir.path(), &back, bad).is_err(), "{bad:?}");
    }
    assert_eq!(
        crate::workspace::open(dir.path())
            .expect("reopen")
            .current_project
            .as_deref(),
        Some("moon-island")
    );
}

#[test]
fn site_repo_inspection() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("site");
    let missing = inspect_site_repo(&repo);
    assert!(!missing.exists && !missing.is_git && !missing.has_public_dir);

    fs::create_dir_all(repo.join(".git")).expect("mkdir");
    fs::create_dir_all(repo.join("public")).expect("mkdir");
    let found = inspect_site_repo(&repo);
    assert!(found.exists && found.is_git && found.has_public_dir);

    let worktree = dir.path().join("wt");
    fs::create_dir_all(&worktree).expect("mkdir");
    fs::write(worktree.join(".git"), b"gitdir: /x").expect("write");
    assert!(inspect_site_repo(&worktree).is_git);
}

#[test]
fn blend_file_names() {
    let cases = [
        ("moon_island_v7.blend", "moon_island", Some(7)),
        ("moon_island-v7.blend", "moon_island", Some(7)),
        ("moon island v7.blend", "moon island", Some(7)),
        ("moon_island_V12.blend", "moon_island", Some(12)),
        ("moon_island.blend", "moon_island", None),
        ("moon7.blend", "moon7", None),
        ("v7.blend", "v7", None),
        ("sub/room_v2.blend", "room", Some(2)),
    ];
    for (file, stem, version) in cases {
        let parsed = BlendFile::parse(file);
        assert_eq!(parsed.stem, stem, "{file}");
        assert_eq!(parsed.version, version, "{file}");
        assert_eq!(parsed.file, file);
    }
}

#[test]
fn scan_import_finds_blends_and_scripts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    for file in [
        "moon_v2.blend",
        "moon_v2.blend1",
        "notes.txt",
        "scenes/room_v1.blend",
        "scenes/deep/too_deep.blend",
        ".hidden/secret.blend",
        "tools/bake.py",
        "tools/ship.mjs",
        "tools/run.sh",
        "tools/readme.md",
        "tools/nested/x.py",
    ] {
        let path = root.join(file);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, b"x").expect("write");
    }
    let scan = scan_import(root).expect("scan");
    let files: Vec<&str> = scan.blend_files.iter().map(|b| b.file.as_str()).collect();
    assert_eq!(files, vec!["moon_v2.blend", "scenes/room_v1.blend"]);
    assert_eq!(scan.blend_files[0].version, Some(2));
    assert_eq!(scan.tool_scripts, vec!["bake.py", "run.sh", "ship.mjs"]);

    assert_eq!(
        scan_import(&root.join("missing"))
            .expect_err("missing")
            .code(),
        "not-found"
    );
}
