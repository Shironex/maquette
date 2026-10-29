use super::*;

#[test]
fn default_json_uses_camel_case_and_enum_spellings() {
    let json = serde_json::to_string(&WorkspaceConfig::default()).expect("json");
    for needle in [
        "\"currentProject\":null",
        "\"removeLighting\":true",
        "\"multiView\":true",
        "\"rigSpec\":\"mixamo\"",
        "\"sceneMb\":3.0",
        "\"trisPerModel\":60000",
        "\"firstViewMb\":3.0",
        "\"textureCodec\":\"etc1s\"",
        "\"meshopt\":\"medium\"",
        "\"mcpPort\":4791",
        "\"blenderHeadlessOnly\":true",
        "\"topology\":\"triangle\"",
        "\"textureSize\":\"2K\"",
        "\"model\":\"v3.1-20260211\"",
    ] {
        assert!(json.contains(needle), "missing {needle} in {json}");
    }
    assert_eq!(
        serde_json::to_string(&TextureSize::K4).expect("json"),
        "\"4K\""
    );
    assert_eq!(
        serde_json::to_string(&TextureCodec::Uastc).expect("json"),
        "\"uastc\""
    );
    assert_eq!(
        serde_json::to_string(&RootState::NotEmpty).expect("json"),
        "\"not-empty\""
    );
}

#[test]
fn default_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = WorkspaceConfig::default();
    save(dir.path(), &config).expect("save");
    let text = fs::read_to_string(dir.path().join(CONFIG_FILE)).expect("read");
    assert!(text.ends_with("}\n"));
    assert_eq!(open(dir.path()).expect("open"), config);
}

#[test]
fn partial_file_fills_defaults_and_ignores_unknown_keys() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join(CONFIG_FILE),
        r#"{ "budgets": { "sceneMb": 5.5 }, "somethingNew": [1, 2] }"#,
    )
    .expect("write");
    let config = open(dir.path()).expect("open");
    let mut expected = WorkspaceConfig::default();
    expected.budgets.scene_mb = 5.5;
    assert_eq!(config, expected);
    for name in ["projects", "inbox", "cache"] {
        assert!(dir.path().join(name).is_dir());
    }
}

#[test]
fn legacy_tripo_block_folds_into_compression_and_presets() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join(CONFIG_FILE),
        r#"{"version":1,"tripo":{"model":"H3.0","hd":true,"topology":"quad","texture":"4K","removeLighting":true,"pbr":false,"character":{"quads":40000,"multiView":false,"autoRig":true,"rigPreset":"tripo"}}}"#,
    )
    .expect("write");
    let config = open(dir.path()).expect("open");
    assert_eq!(config.compression.texture_size, TextureSize::K4);
    let tripo = &config.providers.tripo;
    assert_eq!(tripo.prop.model, "v3.0-20250812");
    assert!(!tripo.prop.pbr);
    assert_eq!(tripo.prop.face_limit, 0);
    assert_eq!(tripo.character.face_limit, 40_000);
    assert!(!tripo.character.multi_view);
    assert!(tripo.character.rig);
    assert_eq!(
        tripo.character.rig_spec,
        crate::settings::presets::TripoRigSpec::Tripo
    );
    assert_eq!(
        config.providers.meshy,
        crate::settings::presets::MeshyPresets::default()
    );
    save(dir.path(), &config).expect("save");
    let text = fs::read_to_string(dir.path().join(CONFIG_FILE)).expect("read");
    assert!(!text.contains("\"tripo\": {\n    \"model\": \"H3.0\""));
    assert!(text.contains("\"providers\""));
}

#[test]
fn future_version_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join(CONFIG_FILE),
        r#"{ "version": 2, "layout": 7 }"#,
    )
    .expect("write");
    let err = open(dir.path()).expect_err("future");
    assert_eq!(err.code(), "unsupported-version");
}

#[test]
fn corrupt_config_is_a_json_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join(CONFIG_FILE), "{ nope").expect("write");
    assert_eq!(open(dir.path()).expect_err("corrupt").code(), "json");
}

#[test]
fn validation_failures() {
    let mut config = WorkspaceConfig::default();
    config.agents.mcp_port = 80;
    assert_eq!(validate(&config).expect_err("port").code(), "invalid");

    let mut config = WorkspaceConfig::default();
    config.agents.mcp_port = 70000;
    assert!(validate(&config).is_err());

    for bad in ["../x", "a/b", "a\\b", "", "..", " x"] {
        let mut config = WorkspaceConfig::default();
        config.layout.inbox = bad.to_string();
        assert!(validate(&config).is_err(), "{bad:?} should fail");
    }

    let mut config = WorkspaceConfig::default();
    config.layout.cache = "projects".to_string();
    assert!(validate(&config).is_err());

    let mut config = WorkspaceConfig::default();
    config.budgets.scene_mb = 0.0;
    assert!(validate(&config).is_err());
    config.budgets.scene_mb = 1001.0;
    assert!(validate(&config).is_err());

    let mut config = WorkspaceConfig::default();
    config.budgets.first_view_mb = 0.0;
    assert!(validate(&config).is_err());
    config.budgets.first_view_mb = f64::NAN;
    assert!(validate(&config).is_err());

    let mut config = WorkspaceConfig::default();
    config.budgets.tris_per_model = 0;
    assert!(validate(&config).is_err());

    let mut config = WorkspaceConfig::default();
    config.providers.tripo.character.face_limit = 3_000_000;
    assert!(validate(&config).is_err());

    let mut config = WorkspaceConfig::default();
    config.providers.tripo.prop.model = " ".to_string();
    assert!(validate(&config).is_err());

    let mut config = WorkspaceConfig::default();
    config.providers.meshy.character.height_meters = 0.0;
    assert!(validate(&config).is_err());

    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = WorkspaceConfig::default();
    config.agents.mcp_port = 80;
    assert!(save(dir.path(), &config).is_err());
    assert!(!dir.path().join(CONFIG_FILE).exists());

    assert!(validate(&WorkspaceConfig::default()).is_ok());
}

#[test]
fn create_on_missing_and_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("new").join("Maquette");
    let config = create(&missing, &WorkspaceConfig::default()).expect("create missing");
    assert_eq!(config, WorkspaceConfig::default());
    for name in ["projects", "inbox", "cache"] {
        assert!(missing.join(name).is_dir());
    }
    assert!(missing.join(CONFIG_FILE).is_file());

    let empty = dir.path().join("empty");
    fs::create_dir(&empty).expect("mkdir");
    fs::write(empty.join(".DS_Store"), b"x").expect("write");
    create(&empty, &WorkspaceConfig::default()).expect("create empty");
    assert!(empty.join(CONFIG_FILE).is_file());

    let mut changed = WorkspaceConfig::default();
    changed.budgets.tris_per_model = 1;
    let reopened = create(&empty, &changed).expect("open existing");
    assert_eq!(reopened.budgets.tris_per_model, 60_000);
}

#[test]
fn create_on_not_empty_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("notes.txt"), b"x").expect("write");
    let err = create(dir.path(), &WorkspaceConfig::default()).expect_err("not empty");
    assert_eq!(err.code(), "not-empty");
    assert!(!dir.path().join(CONFIG_FILE).exists());
}

#[test]
fn open_without_config_is_not_a_workspace() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(
        open(dir.path()).expect_err("no config").code(),
        "not-a-workspace"
    );
}

#[test]
fn inspect_root_states() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path();
    let root = home.join("Maquette");

    let missing = inspect_root(Path::new("~/Maquette"), Some(home)).expect("inspect");
    assert_eq!(missing.state, RootState::Missing);
    assert_eq!(missing.path, root.display().to_string());
    assert_eq!(
        missing.display,
        format!("~{}Maquette", std::path::MAIN_SEPARATOR)
    );

    fs::create_dir(&root).expect("mkdir");
    for junk in IGNORED_ENTRIES {
        fs::write(root.join(junk), b"x").expect("write");
    }
    let empty = inspect_root(&root, Some(home)).expect("inspect");
    assert_eq!(empty.state, RootState::Empty);
    assert_eq!(empty.entries, 0);

    fs::write(root.join("a.txt"), b"x").expect("write");
    let not_empty = inspect_root(&root, Some(home)).expect("inspect");
    assert_eq!(not_empty.state, RootState::NotEmpty);
    assert_eq!(not_empty.entries, 1);

    fs::write(root.join(CONFIG_FILE), b"{}").expect("write");
    assert_eq!(
        inspect_root(&root, Some(home)).expect("inspect").state,
        RootState::Workspace
    );

    assert_eq!(
        inspect_root(&root.join("a.txt"), Some(home))
            .expect_err("file")
            .code(),
        "invalid"
    );
}

#[test]
fn cache_size_and_clear() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = create(dir.path(), &WorkspaceConfig::default()).expect("create");
    let cache = dir.path().join("cache");
    fs::create_dir_all(cache.join("job")).expect("mkdir");
    fs::write(cache.join("a.bin"), [0u8; 7]).expect("write");
    fs::write(cache.join("job").join("b.bin"), [0u8; 3]).expect("write");
    assert_eq!(cache_size(dir.path(), &config), 10);
    assert_eq!(clear_cache(dir.path(), &config).expect("clear"), 10);
    assert_eq!(cache_size(dir.path(), &config), 0);
    assert!(cache.is_dir());
}
