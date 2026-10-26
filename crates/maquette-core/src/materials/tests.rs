use super::*;
use serde_json::{Value, json};

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

fn close(value: &Value, want: f64) -> bool {
    value.as_f64().is_some_and(|v| (v - want).abs() < 1e-4)
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

fn two_materials() -> Value {
    json!({
        "asset": { "version": "2.0" },
        "materials": [
            {
                "name": "Wood",
                "pbrMetallicRoughness": {
                    "baseColorFactor": [0.5, 0.5, 0.5, 1.0],
                    "baseColorTexture": { "index": 0 },
                    "metallicRoughnessTexture": { "index": 1 },
                    "roughnessFactor": 0.4
                },
                "normalTexture": { "index": 2 },
                "extras": { "lightmap": 2.5 }
            },
            {
                "name": "Iron",
                "pbrMetallicRoughness": { "metallicRoughnessTexture": { "index": 1 } }
            }
        ],
        "textures": [{ "source": 0 }, { "source": 1 }, { "source": 2 }],
        "images": [{ "bufferView": 0 }, { "bufferView": 1 }, { "bufferView": 2 }]
    })
}

#[test]
fn tweaks_change_the_factors_and_keep_the_originals() {
    let mut doc = two_materials();
    let mut model = look(tweak(
        Some("#ff8000"),
        Some((FactorMode::Multiply, 2.0)),
        None,
    ));
    model.materials.insert(
        "Iron".into(),
        MaterialTweak {
            emissive_strength: Some(3.0),
            ..MaterialTweak::default()
        },
    );
    let applied = apply_to_json(&mut doc, &model);
    assert_eq!(
        applied,
        Applied {
            materials: 2,
            maps_dropped: 0
        }
    );
    let wood = &doc["materials"][0];
    let base = &wood["pbrMetallicRoughness"]["baseColorFactor"];
    // sRGB #ff8000 is linear (1, 0.2158, 0): a multiply of the factor.
    assert!(close(&base[0], 0.5) && close(&base[1], 0.5 * 0.215_861) && close(&base[2], 0.0));
    assert!(close(&base[3], 1.0));
    // A multiplier scales the factor and stops at 1.
    assert!(close(&wood["pbrMetallicRoughness"]["roughnessFactor"], 0.8));
    assert!(close(
        &doc["materials"][1]["pbrMetallicRoughness"]["roughnessFactor"],
        1.0
    ));
    // The map stays with a multiplier, and the export's extras survive.
    assert_eq!(
        wood["pbrMetallicRoughness"]["metallicRoughnessTexture"]["index"],
        1
    );
    assert!(close(&wood["extras"]["lightmap"], 2.5));
    let original = &wood["extras"]["maquette"]["original"];
    assert!(close(&original["roughnessFactor"], 0.4));
    assert!(close(&original["metallicFactor"], 1.0));
    assert!(close(&original["baseColorFactor"][0], 0.5));
    let iron = &doc["materials"][1];
    assert!(close(
        &iron["extensions"]["KHR_materials_emissive_strength"]["emissiveStrength"],
        3.0
    ));
    assert_eq!(
        doc["extensionsUsed"],
        json!(["KHR_materials_emissive_strength"])
    );
}

#[test]
fn an_override_drops_the_map_and_the_textures_only_it_used() {
    let mut doc = two_materials();
    let model = look(tweak(None, Some((FactorMode::Override, 0.82)), Some(0.0)));
    let applied = apply_to_json(&mut doc, &model);
    assert_eq!(
        applied,
        Applied {
            materials: 2,
            maps_dropped: 2
        }
    );
    let wood = &doc["materials"][0];
    assert!(
        wood["pbrMetallicRoughness"]
            .get("metallicRoughnessTexture")
            .is_none()
    );
    assert!(close(
        &wood["pbrMetallicRoughness"]["roughnessFactor"],
        0.82
    ));
    assert!(close(&wood["pbrMetallicRoughness"]["metallicFactor"], 0.0));
    // Texture 1 and image 1 are gone; the normal map moved down to 1.
    assert_eq!(doc["textures"], json!([{ "source": 0 }, { "source": 1 }]));
    assert_eq!(
        doc["images"],
        json!([{ "bufferView": 0 }, { "bufferView": 2 }])
    );
    assert_eq!(wood["normalTexture"]["index"], 1);
    assert_eq!(wood["pbrMetallicRoughness"]["baseColorTexture"]["index"], 0);
}

#[test]
fn a_shared_map_stays_while_another_material_still_uses_it() {
    let mut doc = two_materials();
    let mut model = ModelMaterials::default();
    model
        .materials
        .insert("Iron".into(), tweak(Some("#26231f"), None, Some(0.75)));
    let applied = apply_to_json(&mut doc, &model);
    assert_eq!(
        applied,
        Applied {
            materials: 1,
            maps_dropped: 1
        }
    );
    assert_eq!(doc["textures"].as_array().map(Vec::len), Some(3));
    assert_eq!(
        doc["materials"][0]["pbrMetallicRoughness"]["metallicRoughnessTexture"]["index"],
        1
    );
    // Wood had no tweak, so it carries no marker either.
    assert!(doc["materials"][0]["extras"].get("maquette").is_none());
}

#[test]
fn a_fixture_glb_takes_the_tweaks_and_stays_a_valid_glb() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("crate_v1.glb");
    let output = dir.path().join("crate_v1.look.glb");
    let source = crate::inbox::fixture::clean_prop();
    fs::write(&input, &source).expect("fixture");
    assert_eq!(material_names(&input), vec!["Crate".to_string()]);
    assert_eq!(map_groups(&input), crate::pipeline::presets::MapGroups::ALL);
    let mut model = ModelMaterials::default();
    model.materials.insert(
        "Crate".into(),
        tweak(Some("#6f7446"), Some((FactorMode::Override, 0.95)), None),
    );
    let applied = apply_file(&input, &output, &model).expect("apply");
    assert_eq!(
        applied,
        Applied {
            materials: 1,
            maps_dropped: 1
        }
    );
    assert_eq!(
        fs::read(&input).expect("source"),
        source,
        "the source never changes"
    );

    let bytes = fs::read(&output).expect("output");
    let gltf = gltf::Gltf::from_slice(&bytes).expect("a valid glTF");
    let material = gltf.document.materials().next().expect("material");
    let pbr = material.pbr_metallic_roughness();
    assert!((pbr.roughness_factor() - 0.95).abs() < 1e-5);
    assert!(pbr.metallic_roughness_texture().is_none());
    let base = pbr.base_color_factor();
    assert!((base[0] - 0.158_961).abs() < 1e-4, "{base:?}");
    assert_eq!(gltf.document.textures().count(), 2);
    assert_eq!(
        material.normal_texture().map(|t| t.texture().index()),
        Some(1)
    );
    // The mesh and its buffer are untouched.
    assert_eq!(gltf.document.meshes().count(), 1);
    assert_eq!(gltf.blob.as_ref().map(Vec::len).unwrap_or(0) % 4, 0);
    assert!(crate::inbox::analyse::analyse(&output).is_ok());
}

#[test]
fn a_broken_file_is_refused_with_a_reason() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("broken.glb");
    fs::write(&input, b"glb").expect("write");
    let err = apply_file(
        &input,
        &dir.path().join("out.glb"),
        &ModelMaterials::default(),
    )
    .expect_err("broken");
    assert!(err.to_string().contains("not a binary glTF"), "{err}");
    assert!(material_names(&input).is_empty());
    assert_eq!(map_groups(&input), crate::pipeline::presets::MapGroups::ALL);
}

#[test]
fn every_version_of_an_asset_with_tweaks_is_found_by_its_stem() {
    use crate::assets::{self, AssetKind, NewAsset, Stage};
    let mut manifest = ProjectManifest {
        version: crate::project::PROJECT_VERSION,
        id: "bench".into(),
        name: "Bench".into(),
        template: crate::ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: None,
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
