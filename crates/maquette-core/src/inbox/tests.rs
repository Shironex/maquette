use super::*;
use crate::project::{NewProject, ProjectTemplate};

fn workspace() -> (tempfile::TempDir, WorkspaceConfig) {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = WorkspaceConfig::default();
    fs::create_dir_all(inbox_dir(dir.path(), &config)).expect("inbox");
    (dir, config)
}

fn drop_in(root: &Path, config: &WorkspaceConfig, name: &str, bytes: &[u8]) {
    fs::write(inbox_dir(root, config).join(name), bytes).expect("write");
}

#[test]
fn names_and_kinds() {
    assert_eq!(display_name("character_tails_v2.glb"), "Character tails");
    assert_eq!(display_name("Sakura-Tree.glb"), "Sakura tree");
    assert_eq!(display_name("v2.glb"), "V2");
    assert_eq!(guess_kind(None, "moon_island.glb"), AssetKind::Scene);
    assert_eq!(guess_kind(None, "lantern.glb"), AssetKind::Prop);
}

#[test]
fn lists_analyses_and_caches() {
    let (dir, config) = workspace();
    let root = dir.path();
    assert!(
        list(&root.join("nowhere"), &config)
            .expect("missing inbox")
            .is_empty()
    );
    drop_in(root, &config, "crate_clean.glb", &fixture::clean_prop());
    drop_in(
        root,
        &config,
        "character_tails.glb",
        &fixture::character(true),
    );
    drop_in(root, &config, "broken.glb", b"half a download");
    drop_in(root, &config, "readme.txt", b"ignored");
    let items = list(root, &config).expect("list");
    assert_eq!(items.len(), 3);
    let character = items
        .iter()
        .find(|i| i.id == "character_tails.glb")
        .expect("character");
    assert_eq!(character.kind, AssetKind::Character);
    assert!(character.kind_guessed);
    assert_eq!(character.worst, CheckStatus::Fail);
    assert_eq!(character.state, InboxState::Ready);
    let broken = items.iter().find(|i| i.id == "broken.glb").expect("broken");
    assert_eq!(broken.state, InboxState::Failed);
    assert!(broken.error.is_some());
    assert!(
        cache_dir(root, &config)
            .join("character_tails.glb.json")
            .is_file()
    );
    assert!(!cache_dir(root, &config).join("broken.glb.json").exists());

    // The cache is used: a doctored record comes back as is.
    let cache_file = cache_dir(root, &config).join("crate_clean.glb.json");
    let mut record: CachedReport =
        serde_json::from_str(&fs::read_to_string(&cache_file).expect("read")).expect("json");
    record.report.triangles = 7;
    write_json(&cache_file, &record).expect("write");
    assert_eq!(
        item(root, &config, "crate_clean.glb")
            .expect("item")
            .report
            .expect("r")
            .triangles,
        7
    );
    assert_eq!(
        reprocess(root, &config, "crate_clean.glb")
            .expect("again")
            .report
            .expect("r")
            .triangles,
        12
    );

    let prop = set_kind(root, &config, "character_tails.glb", AssetKind::Prop).expect("kind");
    assert!(!prop.kind_guessed);
    assert_eq!(prop.kind, AssetKind::Prop);
    assert!(item(root, &config, "../escape.glb").is_err());
    assert!(item(root, &config, "readme.txt").is_err());
    assert_eq!(
        item(root, &config, "gone.glb").expect_err("missing").code(),
        "not-found"
    );
}

#[test]
fn accept_moves_the_file_and_registers_versions() {
    let (dir, config) = workspace();
    let root = dir.path();
    let manifest = project::create(
        root,
        &config.layout,
        NewProject {
            name: "Moon Island".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    drop_in(
        root,
        &config,
        "character_tails.glb",
        &fixture::character(true),
    );
    let request = AcceptRequest {
        item: "character_tails.glb".into(),
        project: manifest.id.clone(),
        asset: None,
        name: Some("Moon girl".into()),
        prep: None,
    };
    let first = accept(root, &config, &request).expect("accept");
    assert_eq!(first.asset.id, "moon-girl");
    assert_eq!(first.asset.kind, AssetKind::Character);
    assert_eq!(first.asset.stage, Stage::Blender);
    assert_eq!(first.asset.status, AssetStatus::Review);
    assert_eq!(first.version.version, 1);
    assert_eq!(first.version.file, "models/moon_girl_v1.glb");
    assert_eq!(first.version.stats.as_ref().expect("stats").joints, 65);
    assert!(Path::new(&first.file).is_file());
    assert!(
        !inbox_dir(root, &config)
            .join("character_tails.glb")
            .exists()
    );

    drop_in(
        root,
        &config,
        "character_tails.glb",
        &fixture::character(false),
    );
    let second = accept(
        root,
        &config,
        &AcceptRequest {
            asset: Some("moon-girl".into()),
            name: None,
            ..request.clone()
        },
    )
    .expect("again");
    assert_eq!(second.version.version, 2);
    assert_eq!(second.version.file, "models/moon_girl_v2.glb");
    let loaded = project::load(root, &config.layout, &manifest.id).expect("load");
    assert_eq!(loaded.assets.len(), 1);
    assert_eq!(loaded.assets[0].versions.len(), 2);

    assert!(accept(root, &config, &request).is_err(), "the file is gone");
    drop_in(
        root,
        &config,
        "scene.gltf",
        br#"{"asset":{"version":"2.0"},"buffers":[{"uri":"scene.bin","byteLength":4}]}"#,
    );
    let err = accept(
        root,
        &config,
        &AcceptRequest {
            item: "scene.gltf".into(),
            ..request
        },
    )
    .expect_err("external");
    assert!(err.to_string().contains("single .glb"));
}

#[test]
fn open_in_blender_imports_models_and_opens_blends() {
    assert_eq!(
        open_in_blender_args(Path::new("/w/a.blend")),
        vec!["/w/a.blend".to_string()]
    );
    let args = open_in_blender_args(Path::new("/w/inbox/a.glb"));
    assert_eq!(args[0], "--python-expr");
    assert!(args[1].contains("import_scene.gltf"));
    assert_eq!(&args[2..], ["--".to_string(), "/w/inbox/a.glb".to_string()]);
    assert!(
        !args.iter().any(|a| a == "-b"),
        "this one opens the window on purpose"
    );
}

#[test]
fn accept_saves_the_real_size_and_fixups_or_refuses_them_before_moving() {
    use crate::prepare::{Fixup, ModelPrep, RealSize, SizeAxis, SizeUnit};
    let (dir, config) = workspace();
    let root = dir.path();
    let manifest = project::create(
        root,
        &config.layout,
        NewProject {
            name: "Workbench".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    drop_in(root, &config, "desk_lamp.glb", &fixture::messy_prop());
    let mut prep = ModelPrep {
        skip_bake: None,
        pad_edges: None,
        parts: None,
        size: Some(RealSize {
            axis: SizeAxis::Height,
            value: -4.0,
            unit: SizeUnit::Cm,
        }),
        fixups: vec![Fixup::OriginBottom],
    };
    let mut request = AcceptRequest {
        item: "desk_lamp.glb".into(),
        project: manifest.id.clone(),
        asset: None,
        name: None,
        prep: Some(prep.clone()),
    };
    assert!(accept(root, &config, &request).is_err(), "a bad size");
    assert!(
        inbox_dir(root, &config).join("desk_lamp.glb").is_file(),
        "not moved"
    );

    prep.size = Some(RealSize {
        axis: SizeAxis::Height,
        value: 45.0,
        unit: SizeUnit::Cm,
    });
    request.prep = Some(prep.clone());
    let done = accept(root, &config, &request).expect("accept");
    assert_eq!(done.note, None);
    let project_dir = project::dir(root, &config.layout, &manifest.id).expect("dir");
    let saved = crate::prepare::load(&project_dir).expect("prepare");
    // Nobody chose a bake or padding, so the scan skips the bake and pads.
    let want = ModelPrep {
        skip_bake: Some(true),
        pad_edges: Some(true),
        ..prep.clone()
    };
    assert_eq!(saved.models[&done.asset.id], want);
    let report = done.version.stats.expect("stats");
    assert_eq!(report.triangles, 19_224, "the source is kept as sent");

    // A choice the person made, in the Inbox or saved before, is kept.
    crate::prepare::save_entry(
        &project_dir,
        &done.asset.id,
        &ModelPrep {
            skip_bake: Some(false),
            pad_edges: Some(false),
            ..ModelPrep::default()
        },
    )
    .expect("bake it");
    drop_in(root, &config, "desk_lamp.glb", &fixture::messy_prop());
    request.asset = Some(done.asset.id.clone());
    request.prep = None;
    let again = accept(root, &config, &request).expect("second version");
    let saved = crate::prepare::load(&project_dir).expect("prepare");
    assert_eq!(saved.models[&again.asset.id].skip_bake, Some(false));
    assert_eq!(saved.models[&again.asset.id].pad_edges, Some(false));
}

#[test]
fn discard_moves_the_model_aside_and_forgets_what_the_inbox_kept() {
    let (dir, config) = workspace();
    let root = dir.path();
    drop_in(root, &config, "lamp.glb", &fixture::clean_prop());
    set_kind(root, &config, "lamp.glb", AssetKind::Scene).expect("kind");
    assert!(cache_dir(root, &config).join("lamp.glb.json").is_file());

    let done = discard(root, &config, "lamp.glb").expect("discard");
    let kept = inbox_dir(root, &config)
        .join(DISCARDED_DIR)
        .join("lamp.glb");
    assert_eq!(done.file, kept.display().to_string());
    assert!(kept.is_file(), "moved, not deleted");
    assert!(list(root, &config).expect("list").is_empty());
    assert!(!cache_dir(root, &config).join("lamp.glb.json").exists());
    assert!(load_kinds(&cache_dir(root, &config)).is_empty());
    assert!(item(root, &config, "lamp.glb").is_err());

    // The same name again: the first discard is kept.
    drop_in(root, &config, "lamp.glb", b"second");
    let again = discard(root, &config, "lamp.glb").expect("second discard");
    assert!(again.file.ends_with("lamp-2.glb"));
    assert!(kept.is_file());

    assert!(discard(root, &config, "lamp.glb").is_err(), "already gone");
    assert!(discard(root, &config, "../lamp.glb").is_err());
    assert!(discard(root, &config, "notes.txt").is_err());
}

#[test]
fn restore_puts_a_discarded_model_back_aimed_at_its_asset() {
    let (dir, config) = workspace();
    let root = dir.path();
    drop_in(root, &config, "lamp.glb", &fixture::clean_prop());
    drop_in(root, &config, "old.glb", &fixture::clean_prop());
    let goal = InboxAim {
        project: "harbour".into(),
        asset: "lamp".into(),
        asset_name: "Lamp".into(),
    };
    let done = discard(root, &config, "lamp.glb").expect("discard");
    remember_discard(root, &config, &done, Some(goal.clone()), Some(2), 5_000.0);
    // A discard from before the notes: listed by its file, last.
    discard(root, &config, "old.glb").expect("discard old");

    let listed = list_discarded(root, &config).expect("listed");
    let names: Vec<&str> = listed.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names.len(), 2);
    let lamp = listed.iter().find(|d| d.name == "lamp.glb").expect("lamp");
    assert_eq!(lamp.asset_name.as_deref(), Some("Lamp"));
    assert_eq!(lamp.replaced_by, Some(2));
    assert_eq!(lamp.discarded_ms, 5_000.0);
    let old = listed.iter().find(|d| d.name == "old.glb").expect("old");
    assert_eq!(old.id, "old.glb");
    assert!(old.asset_name.is_none());

    // The name was taken meanwhile: the restored one gets a number.
    drop_in(root, &config, "lamp.glb", b"a newer download");
    let back = restore(root, &config, "lamp.glb").expect("restore");
    assert_eq!(back.id, "lamp-2.glb");
    let restored = item(root, &config, "lamp-2.glb").expect("in the inbox");
    assert_eq!(restored.aim, Some(goal));
    assert_eq!(list_discarded(root, &config).expect("left").len(), 1);

    assert!(restore(root, &config, "lamp.glb").is_err(), "already back");
    assert!(restore(root, &config, "../old.glb").is_err());
    assert!(restore(root, &config, "discarded.json").is_err());
}

#[test]
fn place_reads_the_next_version_and_a_newer_one_made_after_arrival() {
    let (dir, config) = workspace();
    let root = dir.path();
    let manifest = project::create(
        root,
        &config.layout,
        NewProject {
            name: "Harbour".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    drop_in(root, &config, "lamp_a.glb", &fixture::clean_prop());
    drop_in(root, &config, "lamp_b.glb", &fixture::clean_prop());
    let request = AcceptRequest {
        item: "lamp_a.glb".into(),
        project: manifest.id.clone(),
        asset: None,
        name: Some("Lamp".into()),
        prep: None,
    };
    let first = accept(root, &config, &request).expect("accept");
    let goal = InboxAim {
        project: manifest.id.clone(),
        asset: first.asset.id.clone(),
        asset_name: "Lamp".into(),
    };
    set_aim(root, &config, "lamp_b.glb", Some(goal)).expect("aim");
    drop_in(root, &config, "free.glb", &fixture::clean_prop());

    let mut items = list(root, &config).expect("list");
    // lamp_b arrived before v1 was accepted, so v1 replaced it.
    for i in &mut items {
        if i.id == "lamp_b.glb" {
            i.modified_ms = 0.0;
        }
    }
    place(root, &config, &mut items);
    let b = items.iter().find(|i| i.id == "lamp_b.glb").expect("b");
    assert_eq!(b.next_version, Some(2));
    assert_eq!(b.replaced_by, Some(1));
    let free = items.iter().find(|i| i.id == "free.glb").expect("free");
    assert_eq!(free.next_version, None);
    assert_eq!(free.replaced_by, None);

    // Arrived after v1: nothing replaced it.
    for i in &mut items {
        i.modified_ms = f64::MAX;
    }
    place(root, &config, &mut items);
    let b = items.iter().find(|i| i.id == "lamp_b.glb").expect("b");
    assert_eq!(b.replaced_by, None);
}
