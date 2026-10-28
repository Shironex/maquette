use std::collections::BTreeMap;
use std::fs;

use super::*;
use crate::assets::{AssetKind, AssetStatus, AssetVersion, Stage};
use crate::inbox::fixture;
use crate::pipeline::template::for_template;
use crate::project::ProjectTemplate;

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, bytes).expect("write");
}

fn manifest(assets: Vec<Asset>) -> ProjectManifest {
    ProjectManifest {
        version: 1,
        id: "moon-island".into(),
        name: "Moon Island".into(),
        template: ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: None,
        site_repo_name: None,
        site_repo_state: Default::default(),
        source_folder: None,
        created_at: "2026-10-20T10:00:00.000Z".into(),
        assets,
    }
}

fn character_asset() -> Asset {
    let version = |n: u32| AssetVersion {
        version: n,
        file: format!("models/character_v{n}.glb"),
        created_at: format!("2026-10-2{n}T10:00:00.000Z"),
        stats: None,
    };
    Asset {
        id: "character".into(),
        name: "Character".into(),
        kind: AssetKind::Character,
        stage: Stage::Blender,
        status: AssetStatus::Review,
        // v3 was never written, so it is left out of the comparison.
        versions: vec![version(1), version(2), version(3)],
        thumbnail: None,
        created_at: "2026-10-21T10:00:00.000Z".into(),
        updated_at: "2026-10-22T10:00:00.000Z".into(),
        origin: None,
        refs: Vec::new(),
    }
}

fn defaults() -> CompressDefaults {
    CompressDefaults {
        texture_size: 1024,
        codec: TextureCodec::Etc1s,
        meshopt: MeshoptLevel::Medium,
    }
}

#[test]
fn lists_scene_folders_and_asset_versions_with_their_numbers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path();
    write(&project.join("web/raw/crate.glb"), &fixture::clean_prop());
    write(&project.join("web/dist/crate.glb"), &fixture::clean_prop());
    write(
        &project.join("web/dist/character.glb"),
        &fixture::character(false),
    );
    write(
        &project.join("models/character_v1.glb"),
        &fixture::character(true),
    );
    write(
        &project.join("models/character_v2.glb"),
        &fixture::character(false),
    );
    let mut def = for_template(ProjectTemplate::FloatingDiorama);
    def.presets = vec![crate::pipeline::template::character_preset()];
    let manifest = manifest(vec![character_asset()]);
    let limits = Budgets::default();
    let view = inspect(&Sources {
        project_dir: project,
        manifest: &manifest,
        def: &def,
        defaults: defaults(),
        limits: &limits,
    });

    let ids: Vec<&str> = view.models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "raw/crate",
            "shipped/character",
            "shipped/crate",
            "asset/character/v1",
            "asset/character/v2",
        ]
    );
    let scene = &view.comparisons[0];
    assert_eq!(scene.kind, CompareKind::Scene);
    let labels: Vec<&str> = scene.sides.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, ["Raw", "Shipped"]);
    assert_eq!(scene.sides[1].members.len(), 2);
    let asset = &view.comparisons[1];
    assert_eq!(asset.kind, CompareKind::Asset);
    let notes: Vec<Option<&str>> = asset.sides.iter().map(|s| s.note.as_deref()).collect();
    assert_eq!(notes, [Some("2026-10-21"), Some("2026-10-22")]);

    let crate_model = view
        .models
        .iter()
        .find(|m| m.id == "shipped/crate")
        .expect("crate");
    assert_eq!(crate_model.triangles, 12);
    assert_eq!(crate_model.label, "Crate");
    assert_eq!(crate_model.textures.len(), 3);
    let texture = &crate_model.textures[0];
    assert_eq!(texture.codec, "PNG");
    assert_eq!((texture.width, texture.height), (Some(64), Some(64)));
    assert!((texture.gpu_bytes - 64.0 * 64.0 * 4.0 * 4.0 / 3.0).abs() < 1e-6);
    assert!(crate_model.gpu_bytes > texture.gpu_bytes * 3.0);
    assert!(!crate_model.meshopt && !crate_model.ktx2);
    assert!(!crate_model.preset.has_row);

    // The character's preset row, with its lock.
    let character = view
        .models
        .iter()
        .find(|m| m.id == "asset/character/v2")
        .expect("character v2");
    assert_eq!(character.model, "character");
    assert!(character.preset.has_row);
    assert!(character.preset.lock.is_some());
    assert_eq!(character.joints, 65);
    assert_eq!(character.version, Some(2));

    // The scene is the shipped build.
    assert_eq!(view.scene.models, 2);
    let budgets: Vec<(&str, f64, bool)> = view
        .budgets
        .iter()
        .map(|b| (b.model.as_str(), b.target_mb, b.explicit))
        .collect();
    assert_eq!(budgets, [("character", 1.5, false), ("crate", 1.5, false)]);
    assert_eq!(view.stops.source, StopsSource::None);
}

#[test]
fn budgets_file_rows_come_first_and_the_rest_share_the_remainder() {
    let models: Vec<String> = ["character", "island", "props"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let mut explicit = BTreeMap::new();
    explicit.insert("character".to_string(), 1.0);
    let split = split_budget(3.0, &models, &explicit);
    assert_eq!(
        split,
        vec![
            ("character".to_string(), 1.0, true),
            ("island".to_string(), 1.0, false),
            ("props".to_string(), 1.0, false),
        ]
    );
    explicit.insert("island".to_string(), 2.5);
    let over = split_budget(3.0, &models, &explicit);
    assert!(over[2].1.abs() < f64::EPSILON, "nothing is left for props");

    let dir = tempfile::tempdir().expect("tempdir");
    write(
        &dir.path().join(BUDGETS_FILE),
        br#"{ "models": { "character": 1.2, "bad": -1 } }"#,
    );
    let read = read_budgets(dir.path());
    assert_eq!(read.len(), 1);
    assert!((read["character"] - 1.2).abs() < f64::EPSILON);
}

#[test]
fn labels_from_file_stems() {
    assert_eq!(model_label("shop_house"), "Shop house");
    assert_eq!(model_label("island-rock"), "Island rock");
    assert_eq!(model_label("crate"), "Crate");
}

#[test]
fn an_export_of_an_asset_version_reads_as_the_asset() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path();
    write(&project.join("models/clay_v1.glb"), &fixture::clean_prop());
    write(&project.join("web/raw/clay_v1.glb"), &fixture::clean_prop());
    write(
        &project.join("web/dist/clay_v1.glb"),
        &fixture::clean_prop(),
    );
    let clay = Asset {
        id: "clay".into(),
        name: "clay".into(),
        kind: AssetKind::Prop,
        stage: Stage::Blender,
        status: AssetStatus::Review,
        versions: vec![AssetVersion {
            version: 1,
            file: "models/clay_v1.glb".into(),
            created_at: "2026-10-24T19:14:19.000Z".into(),
            stats: None,
        }],
        thumbnail: None,
        created_at: "2026-10-24T19:00:00.000Z".into(),
        updated_at: "2026-10-24T19:14:19.000Z".into(),
        origin: None,
        refs: Vec::new(),
    };
    let def = for_template(ProjectTemplate::FloatingDiorama);
    let manifest = manifest(vec![clay]);
    let limits = Budgets::default();
    let view = inspect(&Sources {
        project_dir: project,
        manifest: &manifest,
        def: &def,
        defaults: defaults(),
        limits: &limits,
    });
    let shipped = view
        .models
        .iter()
        .find(|m| m.id == "shipped/clay_v1")
        .expect("shipped");
    assert_eq!(shipped.label, "clay");
    assert_eq!(shipped.version, Some(1));
    assert_eq!(shipped.model, "clay_v1", "the file stem stays for presets");
    assert!(shipped.path.ends_with("clay_v1.glb"));
    assert_eq!(view.budgets[0].label, "clay");
}
