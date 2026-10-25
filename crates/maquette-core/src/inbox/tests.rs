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
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    drop_in(root, &config, "desk_lamp.glb", &fixture::messy_prop());
    let mut prep = ModelPrep {
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
    assert_eq!(saved.models[&done.asset.id], prep);
    let report = done.version.stats.expect("stats");
    assert_eq!(report.triangles, 19_224, "the source is kept as sent");
}
