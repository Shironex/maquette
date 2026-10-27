use super::rig::MIXAMO_CORE;
use super::source::{base64_decode, percent_decode};
use super::*;
use crate::inbox::fixture;

fn report(bytes: &[u8]) -> GlbReport {
    analyse_bytes(bytes, Path::new(".")).expect("analyse")
}

#[test]
fn quantized_positions_measure_in_metres() {
    // A normalized int16 position accessor, as KHR_mesh_quantization
    // writes it, on a node scaled by two: the bounds are 2 m wide.
    let json = serde_json::json!({
        "asset": { "version": "2.0" },
        "extensionsUsed": ["KHR_mesh_quantization", "EXT_meshopt_compression"],
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "scale": [2.0, 2.0, 2.0] }],
        "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
        "accessors": [{
            "componentType": 5122, "normalized": true, "count": 3, "type": "VEC3",
            "min": [-16384, 0, 0], "max": [16384, 32767, 0],
        }],
    });
    let r = report(&serde_json::to_vec(&json).expect("json"));
    let [x, y, _] = r.size;
    assert!((x - 2.0).abs() < 1e-3, "width {x}");
    assert!((y - 2.0).abs() < 1e-3, "height {y}");
}

#[test]
fn bounds_read_the_corners_without_the_buffers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("prop.glb");
    fs::write(&file, fixture::clean_prop()).expect("write");
    let (min, max) = bounds(&file).expect("bounds");
    let size = report(&fixture::clean_prop()).size;
    for k in 0..3 {
        assert!((max[k] - min[k] - size[k]).abs() < 1e-5, "axis {k}");
    }
    fs::write(&file, b"not a model").expect("junk");
    assert_eq!(bounds(&file), None);
    assert_eq!(bounds(&dir.path().join("missing.glb")), None);
}

#[test]
fn a_clean_prop_measures_cleanly() {
    let r = report(&fixture::clean_prop());
    assert_eq!(
        (r.triangles, r.vertices, r.meshes, r.primitives, r.materials),
        (12, 24, 1, 1, 1)
    );
    assert_eq!(r.textures.len(), 3);
    assert_eq!(r.max_texture, 64);
    assert_eq!(r.textures[0].uses, vec!["base color".to_string()]);
    assert_eq!(r.textures[0].mime.as_deref(), Some("image/png"));
    assert!(r.pbr.base_color && r.pbr.metallic_roughness && r.pbr.normal);
    assert!(!r.pbr.occlusion);
    assert_eq!(r.rig, RigKind::None);
    assert!(r.animations.is_empty());
    assert!((r.size[1] - 0.5).abs() < 1e-5);
    let islands = r.islands.expect("islands");
    assert_eq!((islands.islands, islands.groups), (1, 1));
    assert!(r.finger_weights.is_none());
    assert_eq!(r.generator.as_deref(), Some("Maquette fixtures"));
}

#[test]
fn merged_objects_show_as_separate_groups() {
    let r = report(&fixture::merged_islands());
    let islands = r.islands.expect("islands");
    assert_eq!(islands.islands, 3, "three boxes");
    assert_eq!(islands.groups, 2, "the touching pair is one object");
    assert_eq!(islands.mesh, "Props");
    assert!(!r.pbr.base_color);
}

#[test]
fn tails_weighted_to_fingers_are_found() {
    let r = report(&fixture::character(true));
    assert_eq!(r.rig, RigKind::Mixamo);
    assert_eq!(r.joints, 65);
    assert_eq!(r.skins, 1);
    assert_eq!(r.animations, vec!["Idle".to_string()]);
    let fw = r.finger_weights.expect("checked");
    // Two tails of 24 vertices each.
    assert_eq!(fw.vertices, 48);
    let names: Vec<&str> = fw.joints.iter().map(|j| j.name.as_str()).collect();
    assert_eq!(
        names,
        ["mixamorig:LeftHandIndex1", "mixamorig:RightHandIndex1"]
    );
    assert!(fw.joints.iter().all(|j| j.vertices == 24));
    // The real finger boxes stay unflagged.
    let islands = r.islands.expect("islands");
    assert_eq!(islands.groups, 1, "a character is one object");

    let clean = report(&fixture::character(false));
    assert_eq!(clean.finger_weights.expect("checked").vertices, 0);
}

#[test]
fn sleeves_and_chains_hanging_below_the_wrists_are_not_flagged() {
    let r = report(&fixture::character_with_cuffs());
    let fw = r.finger_weights.expect("checked");
    assert_eq!(fw.vertices, 0, "{:?}", fw.joints);
    assert!(fw.reach > 0.2 && fw.reach < 0.3, "{}", fw.reach);
}

#[test]
fn joint_names_and_rig_detection() {
    assert_eq!(
        bare_joint_name("mixamorig:LeftHandIndex1"),
        "LeftHandIndex1"
    );
    assert_eq!(bare_joint_name("mixamorig1_LeftHand"), "LeftHand");
    assert_eq!(bare_joint_name("Spine"), "Spine");
    assert!(is_hand_joint("mixamorig:RightHandPinky3"));
    assert!(is_hand_joint("LeftHand"));
    assert!(!is_hand_joint("mixamorig:LeftForeArm"));
    let names: Vec<String> = MIXAMO_CORE.iter().map(|n| n.to_string()).collect();
    assert_eq!(rig_kind(&names), RigKind::Mixamo);
    assert_eq!(
        rig_kind(&["Bone".to_string(), "Bone.001".to_string()]),
        RigKind::Other
    );
    assert_eq!(rig_kind(&[]), RigKind::None);
}

#[test]
fn files_and_bad_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("crate.glb");
    fs::write(&path, fixture::clean_prop()).expect("write");
    let r = analyse(&path).expect("file");
    assert_eq!(r.bytes, fs::metadata(&path).expect("meta").len() as f64);
    fs::write(&path, b"nope").expect("write");
    assert_eq!(analyse(&path).expect_err("bad").code(), "invalid");
    assert!(analyse(&dir.path().join("missing.glb")).is_err());
}

#[test]
fn base64_and_percent_decoding() {
    assert_eq!(base64_decode("TWFu").as_deref(), Some(&b"Man"[..]));
    assert_eq!(base64_decode("TWE=").as_deref(), Some(&b"Ma"[..]));
    assert_eq!(base64_decode("TQ==").as_deref(), Some(&b"M"[..]));
    assert!(base64_decode("T!==").is_none());
    assert_eq!(percent_decode("my%20model.bin"), "my model.bin");
}
