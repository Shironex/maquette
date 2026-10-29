use super::*;
use serde_json::json;

fn tweak(
    tint: Option<&str>,
    roughness: Option<(FactorMode, f32)>,
    metal: Option<f32>,
) -> MaterialTweak {
    MaterialTweak {
        tint: tint.map(str::to_string),
        roughness: roughness.map(|(mode, value)| RoughnessTweak { mode, value }),
        metalness: metal,
        emissive_strength: None,
    }
}

fn look(all: MaterialTweak) -> ModelMaterials {
    ModelMaterials {
        all,
        materials: BTreeMap::new(),
    }
}

fn scene(asset: &str, model: ModelMaterials) -> SceneMaterials {
    SceneMaterials {
        version: MATERIALS_VERSION,
        models: BTreeMap::from([(asset.to_string(), model)]),
    }
}

#[test]
fn a_missing_file_is_no_tweaks_and_a_saved_one_comes_back_clean() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(
        load(dir.path()).expect("missing"),
        SceneMaterials::default()
    );
    let mut laptop = look(tweak(
        Some("#3A3936"),
        Some((FactorMode::Override, 0.62)),
        Some(0.25),
    ));
    laptop
        .materials
        .insert("Screen".into(), MaterialTweak::default());
    let mut file = scene("laptop", laptop);
    file.models
        .insert("bench".into(), ModelMaterials::default());
    let stored = save(dir.path(), &file).expect("save");
    // Empty entries go and the hex is lowercased.
    assert_eq!(stored.models.len(), 1);
    assert!(stored.models["laptop"].materials.is_empty());
    assert_eq!(stored.models["laptop"].all.tint.as_deref(), Some("#3a3936"));
    let text = fs::read_to_string(path(dir.path())).expect("read");
    assert!(text.contains("\"emissiveStrength\": null"), "{text}");
    assert!(text.contains("\"mode\": \"override\""));
    assert_eq!(load(dir.path()).expect("load"), stored);

    // A newer file is refused rather than misread.
    fs::write(path(dir.path()), r#"{ "version": 9, "models": {} }"#).expect("newer");
    assert_eq!(
        load(dir.path()).expect_err("newer").code(),
        "unsupported-version"
    );
}

#[test]
fn validation_names_the_value_that_is_out_of_range() {
    let refused = |t: MaterialTweak| {
        validate(&scene("lamp", look(t)))
            .expect_err("refused")
            .to_string()
    };
    assert!(refused(tweak(Some("olive"), None, None)).contains("#rrggbb"));
    assert!(refused(tweak(Some("#12345"), None, None)).contains("not a color"));
    assert!(
        refused(tweak(None, Some((FactorMode::Override, 1.2)), None)).contains("between 0 and 1")
    );
    assert!(
        refused(tweak(None, Some((FactorMode::Multiply, 2.5)), None)).contains("between 0 and 2")
    );
    assert!(refused(tweak(None, None, Some(-0.1))).contains("metalness"));
    assert!(refused(tweak(None, None, Some(f32::NAN))).contains("metalness"));
    let bright = MaterialTweak {
        emissive_strength: Some(80.0),
        ..MaterialTweak::default()
    };
    assert!(refused(bright).contains("emissive strength"));
    assert!(
        validate(&scene(
            "lamp",
            look(tweak(None, Some((FactorMode::Multiply, 1.5)), Some(1.0)))
        ))
        .is_ok()
    );

    let mut named = ModelMaterials::default();
    named
        .materials
        .insert(" ".into(), tweak(None, None, Some(0.5)));
    assert!(validate(&scene("lamp", named)).is_err());
}

#[test]
fn a_named_material_wins_field_by_field() {
    let mut model = look(tweak(
        Some("#808080"),
        Some((FactorMode::Override, 0.9)),
        None,
    ));
    model.materials.insert(
        "Top".into(),
        tweak(None, Some((FactorMode::Multiply, 0.5)), Some(0.0)),
    );
    let top = model.for_material(Some("Top"));
    assert_eq!(top.tint.as_deref(), Some("#808080"));
    assert_eq!(top.roughness.map(|r| r.mode), Some(FactorMode::Multiply));
    assert_eq!(top.metalness, Some(0.0));
    assert_eq!(model.for_material(Some("Base")), model.all);
    assert_eq!(model.for_material(None), model.all);
}

#[test]
fn every_version_of_an_asset_with_tweaks_is_found_by_its_stem() {
    use crate::assets::{self, AssetKind, NewAsset, Stage};
    let mut manifest = ProjectManifest {
        version: crate::project::PROJECT_VERSION,
        id: "bench".into(),
        name: "Bench".into(),
        template: crate::project::ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: None,
        site_repo_name: None,
        site_repo_state: Default::default(),
        source_folder: None,
        created_at: "2026-10-25T08:00:00.000Z".into(),
        assets: Vec::new(),
    };
    for (name, files) in [
        (
            "Laptop",
            &["models/laptop_v1.glb", "models/Laptop_v2.glb"][..],
        ),
        ("Lamp", &["models/lamp_v1.glb"][..]),
    ] {
        let asset = assets::add(
            &mut manifest,
            NewAsset {
                name: name.into(),
                kind: AssetKind::Prop,
                stage: Stage::Blender,
            },
        )
        .expect("asset");
        for file in files {
            assets::add_version(&mut manifest, &asset.id, (*file).to_string(), None)
                .expect("version");
        }
    }
    let mut file = scene("laptop", look(tweak(None, None, Some(0.2))));
    file.models
        .insert("gone".into(), look(tweak(None, None, Some(1.0))));
    let stems = by_model_stem(&manifest, &file);
    assert_eq!(
        stems.keys().cloned().collect::<Vec<_>>(),
        vec!["laptop_v1".to_string(), "laptop_v2".to_string()]
    );
}

#[test]
fn a_patch_changes_only_what_it_names() {
    let current = scene(
        "turntable",
        look(tweak(
            Some("#ffffff"),
            Some((FactorMode::Override, 0.8)),
            None,
        )),
    );
    let names = vec!["Top".to_string(), "Base".to_string()];
    let change: MaterialsPatch = serde_json::from_value(json!({
        "all": { "tint": null, "metalness": 0 },
        "materials": { "Base": { "tint": "#26231F", "metalness": 0.75 } }
    }))
    .expect("patch");
    let next = apply_patch(&current, "turntable", &change, &names).expect("apply");
    let turntable = &next.models["turntable"];
    assert_eq!(turntable.all.tint, None);
    assert_eq!(turntable.all.metalness, Some(0.0));
    assert_eq!(turntable.all.roughness.map(|r| r.value), Some(0.8));
    assert_eq!(turntable.materials["Base"].tint.as_deref(), Some("#26231f"));

    let unknown: MaterialsPatch =
        serde_json::from_value(json!({ "materials": { "Lid": { "metalness": 1 } } }))
            .expect("patch");
    let err = apply_patch(&next, "turntable", &unknown, &names).expect_err("unknown");
    assert!(err.to_string().contains("Top, Base"), "{err}");

    let bad: MaterialsPatch =
        serde_json::from_value(json!({ "all": { "metalness": 3 } })).expect("patch");
    assert!(apply_patch(&next, "turntable", &bad, &names).is_err());

    let removed: MaterialsPatch =
        serde_json::from_value(json!({ "materials": { "Base": null } })).expect("patch");
    let after = apply_patch(&next, "turntable", &removed, &names).expect("remove");
    assert!(after.models["turntable"].materials.is_empty());

    let reset: MaterialsPatch = serde_json::from_value(json!({ "reset": true })).expect("patch");
    let cleared = apply_patch(&after, "turntable", &reset, &names).expect("reset");
    assert!(!cleared.models.contains_key("turntable"));
}

#[test]
fn saving_looks_keeps_every_other_asset_on_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let laptop = look(tweak(Some("#3a3936"), None, None));
    save(dir.path(), &scene("laptop", laptop.clone())).expect("save");

    // The panel only knows about the bench; the laptop on disk stays.
    let bench = look(tweak(None, None, Some(0.5)));
    let looks = BTreeMap::from([("bench".to_string(), bench.clone())]);
    let stored = save_looks(dir.path(), &looks).expect("looks");
    assert_eq!(stored.models.get("laptop"), Some(&laptop));
    assert_eq!(stored.models.get("bench"), Some(&bench));
    assert_eq!(load(dir.path()).expect("load"), stored);

    // An empty look removes only that asset.
    let cleared = BTreeMap::from([("bench".to_string(), ModelMaterials::default())]);
    let stored = save_looks(dir.path(), &cleared).expect("clear");
    assert_eq!(stored.models.keys().collect::<Vec<_>>(), vec!["laptop"]);

    // Agents go through the same path.
    let change: MaterialsPatch =
        serde_json::from_value(json!({ "all": { "metalness": 0.25 } })).expect("patch");
    let stored = update(dir.path(), |current| {
        apply_patch(current, "bench", &change, &[])
    })
    .expect("patch");
    assert_eq!(stored.models.get("laptop"), Some(&laptop));
    assert_eq!(stored.models["bench"].all.metalness, Some(0.25));
}

#[test]
fn a_newer_or_unreadable_file_on_disk_is_never_overwritten() {
    let dir = tempfile::tempdir().expect("tempdir");
    let looks = BTreeMap::from([(
        "bench".to_string(),
        look(tweak(Some("#ffffff"), None, None)),
    )]);

    let newer = r#"{ "version": 9, "models": { "laptop": {} } }"#;
    fs::write(path(dir.path()), newer).expect("newer");
    let err = save_looks(dir.path(), &looks).expect_err("newer");
    assert_eq!(err.code(), "unsupported-version");
    assert_eq!(fs::read_to_string(path(dir.path())).expect("read"), newer);

    let broken = "{ not json";
    fs::write(path(dir.path()), broken).expect("broken");
    let err = save_looks(dir.path(), &looks).expect_err("broken");
    assert!(
        err.to_string().contains("nothing was saved over it"),
        "{err}"
    );
    assert_eq!(fs::read_to_string(path(dir.path())).expect("read"), broken);
}
