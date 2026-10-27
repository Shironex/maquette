use super::*;
use crate::assets::{AssetKind, NewAsset, Stage, add, add_version, rename};
use crate::project::{PROJECT_VERSION, ProjectTemplate};

fn manifest() -> ProjectManifest {
    ProjectManifest {
        version: PROJECT_VERSION,
        id: "bench".into(),
        name: "Bench".into(),
        template: ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: None,
        site_repo_name: None,
        site_repo_state: Default::default(),
        source_folder: None,
        created_at: "2026-10-25T09:00:00.000Z".into(),
        assets: Vec::new(),
    }
}

fn prop(m: &mut ProjectManifest, name: &str, versions: u32) -> String {
    let new = NewAsset {
        name: name.into(),
        kind: AssetKind::Prop,
        stage: Stage::Tripo,
    };
    let id = add(m, new).expect("asset").id;
    for n in 1..=versions {
        let file = format!("models/{}_v{n}.glb", id.replace('-', "_"));
        add_version(m, &id, file, None).expect("version");
    }
    id
}

#[test]
fn a_site_without_a_config_gets_the_folders_ship_always_used() {
    let dir = tempfile::tempdir().expect("dir");
    assert_eq!(
        load_config(dir.path()).expect("config"),
        SiteConfig::legacy()
    );
    assert_eq!(models_dir(dir.path()), dir.path().join("public/models"));
}

#[test]
fn a_config_sets_the_folders_and_leaves_the_rest_out() {
    let dir = tempfile::tempdir().expect("dir");
    fs::write(
        dir.path().join(CONFIG_FILE),
        r#"{ "version": 1, "models": "public/scene/models", "data": "src/scene/shipped.json",
             "preview": { "command": ["pnpm", "dev"], "url": "http://localhost:15292/" } }"#,
    )
    .expect("write");
    let config = load_config(dir.path()).expect("config");
    assert_eq!(config.models, "public/scene/models");
    assert_eq!(config.posters, None);
    assert_eq!(config.data.as_deref(), Some("src/scene/shipped.json"));
    assert_eq!(
        config.preview.expect("preview").command,
        vec!["pnpm".to_string(), "dev".to_string()]
    );
    assert_eq!(
        models_dir(dir.path()),
        dir.path().join("public/scene/models")
    );
}

#[test]
fn a_config_cannot_point_outside_the_site_or_come_from_a_newer_maquette() {
    for bad in ["../elsewhere", "/abs/models", "a/../../b", " "] {
        let config = SiteConfig {
            models: bad.into(),
            ..SiteConfig::legacy()
        };
        assert!(validate(&config).is_err(), "{bad}");
    }
    let data = SiteConfig {
        data: Some("../shipped.json".into()),
        ..SiteConfig::legacy()
    };
    assert!(validate(&data).is_err());
    let newer = SiteConfig {
        version: CONFIG_VERSION + 1,
        ..SiteConfig::legacy()
    };
    assert_eq!(
        validate(&newer).expect_err("newer").code(),
        "unsupported-version"
    );
    let dir = tempfile::tempdir().expect("dir");
    fs::write(dir.path().join(CONFIG_FILE), "{ nope").expect("write");
    assert!(load_config(dir.path()).is_err());
    assert_eq!(models_dir(dir.path()), dir.path().join(LEGACY_MODELS));
}

#[test]
fn the_index_lists_each_newest_model_under_its_shipped_name() {
    let mut m = manifest();
    let lamp = prop(&mut m, "Desk lamp", 2);
    let rock = prop(&mut m, "Rock", 1);
    prop(&mut m, "Not modelled yet", 0);
    rename(&mut m, &rock, "Mossy rock").expect("rename");
    let index = ship_index(&m, &ShipSettings::default());
    assert_eq!(index.project, "bench");
    assert_eq!(index.models.len(), 2);
    assert_eq!(
        index.models.get(&lamp),
        Some(&IndexEntry {
            name: "Desk lamp".into(),
            file: "desk_lamp_v2.glb".into(),
            version: 2,
            choice: ShipChoice::Auto,
        })
    );
    assert_eq!(
        index.models.get(&rock).map(|e| e.file.as_str()),
        Some("mossy_rock_v1.glb")
    );
}

#[test]
fn a_hand_built_site_gets_stable_names_and_the_choices() {
    let mut m = manifest();
    let lamp = prop(&mut m, "Desk lamp", 2);
    let rock = prop(&mut m, "Rock", 1);
    let mut ship = ShipSettings {
        style: crate::ship::ShipStyle::Models,
        ..ShipSettings::default()
    };
    ship.assets.insert(
        rock.clone(),
        crate::ship::AssetShip {
            choice: ShipChoice::Never,
            file: Some("boulder".into()),
        },
    );
    let index = ship_index(&m, &ship);
    assert_eq!(
        index.models[&lamp].file, "desk_lamp.glb",
        "no version in the name"
    );
    assert_eq!(index.models[&rock].file, "boulder.glb");
    assert_eq!(index.models[&rock].choice, ShipChoice::Never);
    assert_eq!(index.origin, "own");
    assert!(index.size_guard);
    let text = serde_json::to_value(ship_index(&m, &ShipSettings::default())).expect("json");
    assert_eq!(text["origin"], "scene");
    assert!(
        text["models"][&lamp].get("choice").is_none(),
        "auto is left out"
    );
}

#[test]
fn the_index_is_written_into_the_cache() {
    let dir = tempfile::tempdir().expect("dir");
    let mut m = manifest();
    prop(&mut m, "Lamp", 1);
    let path =
        write_ship_index(&dir.path().join("cache"), &m, &ShipSettings::default()).expect("write");
    let text = fs::read_to_string(&path).expect("read");
    let back: ShipIndex = serde_json::from_str(&text).expect("json");
    assert_eq!(back, ship_index(&m, &ShipSettings::default()));
    assert!(path.ends_with(INDEX_FILE));
}
