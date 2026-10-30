use std::path::Path;

use serde_json::{Value, json};

use super::*;
use crate::fixture::{self, HumanoidSpec};

fn written() -> Vec<u8> {
    let spec = WriteSpec {
        meta: VrmMeta {
            name: "Hero".to_string(),
            authors: vec!["Test Author".to_string()],
            ..VrmMeta::default()
        },
        ..WriteSpec::default()
    };
    write(
        &fixture::humanoid(&HumanoidSpec::default()),
        &spec,
        Path::new("."),
    )
    .expect("write")
    .bytes
}

/// The written fixture with its JSON chunk changed by `edit`.
fn edited(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let (mut json, bin) = container::split(&written()).expect("split");
    edit(&mut json);
    container::join(&json, bin).expect("join")
}

fn vrm(json: &mut Value) -> &mut Value {
    &mut json["extensions"][VRMC_VRM]
}

fn node_of(json: &Value, bone: &str) -> Value {
    json["extensions"][VRMC_VRM]["humanoid"]["humanBones"][bone]["node"].clone()
}

fn problems(bytes: &[u8]) -> String {
    let found = round_trip(bytes);
    assert!(!found.passed, "expected a failure");
    found.problems.join(" ")
}

#[test]
fn the_written_fixture_reads_back() {
    let found = round_trip(&written());
    assert!(found.passed, "{:?}", found.problems);
    assert!(found.bones >= 15);
}

#[test]
fn a_file_that_is_not_gltf_fails() {
    assert!(problems(b"not a model").contains("valid glTF"));
}

#[test]
fn a_wrong_spec_version_fails() {
    let bytes = edited(|json| vrm(json)["specVersion"] = json!("0.0"));
    assert!(problems(&bytes).contains("spec version 0.0"));
}

#[test]
fn a_bone_past_the_nodes_or_twice_fails() {
    let bytes = edited(|json| vrm(json)["humanoid"]["humanBones"]["head"]["node"] = json!(9999));
    assert!(problems(&bytes).contains("head points at node 9999"));
    let bytes = edited(|json| {
        let hips = node_of(json, "hips");
        vrm(json)["humanoid"]["humanBones"]["leftToes"] = json!({ "node": hips });
    });
    assert!(problems(&bytes).contains("both point at node"));
}

#[test]
fn a_missing_required_bone_fails() {
    let bytes = edited(|json| {
        if let Some(bones) = vrm(json)["humanoid"]["humanBones"].as_object_mut() {
            bones.remove("leftHand");
        }
    });
    assert!(problems(&bytes).contains("Required bones missing: leftHand."));
}

#[test]
fn a_broken_parent_chain_fails() {
    let bytes = edited(|json| {
        let upper = node_of(json, "leftUpperArm");
        let lower = node_of(json, "leftLowerArm");
        let bones = &mut vrm(json)["humanoid"]["humanBones"];
        bones["leftUpperArm"]["node"] = lower;
        bones["leftLowerArm"]["node"] = upper;
    });
    assert!(problems(&bytes).contains("in the node tree; VRM puts it under"));
}

#[test]
fn a_meta_without_its_required_fields_fails() {
    let bytes = edited(|json| {
        let meta = &mut vrm(json)["meta"];
        meta["name"] = json!("");
        meta["authors"] = json!([]);
        if let Some(m) = meta.as_object_mut() {
            m.remove("licenseUrl");
        }
    });
    let text = problems(&bytes);
    assert!(text.contains("no name"), "{text}");
    assert!(text.contains("no author"), "{text}");
    assert!(text.contains("no licence URL"), "{text}");
}

#[test]
fn an_extension_missing_from_extensions_used_fails() {
    let bytes = edited(|json| json["extensionsUsed"] = json!([]));
    assert!(problems(&bytes).contains("extensionsUsed does not list"));
}

#[test]
fn animations_left_in_the_file_fail() {
    let source = fixture::humanoid(&HumanoidSpec::default());
    let (from, _) = container::split(&source).expect("split");
    assert!(from["animations"].is_array(), "the fixture has a clip");
    let bytes = edited(|json| json["animations"] = from["animations"].clone());
    assert!(problems(&bytes).contains("animations"));
}

#[test]
fn an_mtoon_texture_that_is_not_there_fails() {
    let bytes = edited(|json| {
        json["materials"][0]["extensions"][VRMC_MATERIALS_MTOON]["shadeMultiplyTexture"] =
            json!({ "index": 7 });
    });
    assert!(problems(&bytes).contains("shadeMultiplyTexture points at a texture"));
}

/// Reads back the `.vrm` at `MAQUETTE_VRM_ROUND_TRIP`, such as one the app
/// wrote: `MAQUETTE_VRM_ROUND_TRIP=/path/to/a.vrm cargo test -p
/// maquette-glb round_trip_on_a_file -- --ignored --nocapture`.
#[test]
#[ignore = "reads a file named by MAQUETTE_VRM_ROUND_TRIP"]
fn round_trip_on_a_file() {
    let path = std::env::var("MAQUETTE_VRM_ROUND_TRIP").expect("MAQUETTE_VRM_ROUND_TRIP");
    let bytes = std::fs::read(Path::new(&path)).expect("read the file");
    let found = round_trip(&bytes);
    println!("{}", serde_json::to_string_pretty(&found).expect("json"));
    assert!(found.passed, "{:?}", found.problems);
}
