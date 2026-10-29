use std::collections::BTreeMap;
use std::fs;

use serde_json::{Value, json};

use super::*;

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

fn close(value: &Value, want: f64) -> bool {
    value.as_f64().is_some_and(|v| (v - want).abs() < 1e-4)
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
fn the_smaller_group_is_marked_for_resize_and_unmarked_after() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("crate_v1.glb");
    let marked = dir.path().join("crate_v1.split.glb");
    fs::write(&input, crate::inbox::fixture::clean_prop()).expect("fixture");
    let names = |path: &std::path::Path| -> Vec<Option<String>> {
        let json = crate::test_blender::glb_json(path);
        json["images"]
            .as_array()
            .expect("images")
            .iter()
            .map(|i| i["name"].as_str().map(str::to_string))
            .collect()
    };
    let before = names(&input);
    // The data and normal maps are the smaller group: the base color stays.
    let count = mark_small_maps(&input, &marked, false).expect("mark");
    assert!(count >= 1);
    let after = names(&marked);
    assert_eq!(after.len(), before.len());
    let suffix = crate::pipeline::presets::SMALL_MARK;
    assert_eq!(
        after
            .iter()
            .flatten()
            .filter(|n| n.ends_with(suffix))
            .count(),
        count
    );
    assert!(gltf::Gltf::from_slice(&fs::read(&marked).expect("read")).is_ok());
    assert_eq!(unmark_small_maps(&marked).expect("unmark"), count);
    assert_eq!(names(&marked), before, "the names are as they were");
    // The color group alone.
    let color = mark_small_maps(&input, &marked, true).expect("mark color");
    assert_eq!(color + count, before.len(), "every image is in one group");
}
