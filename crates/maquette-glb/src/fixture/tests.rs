use super::*;

#[test]
fn the_rig_is_the_65_joint_mixamo_set() {
    let joints = skeleton();
    assert_eq!(joints.len(), 65);
    assert!(joints.iter().all(|(n, _, _)| n.starts_with("mixamorig:")));
    let world = world_positions(&joints);
    let head = joints
        .iter()
        .position(|(n, _, _)| n == "mixamorig:Head")
        .expect("head");
    assert!((world[head][1] - 1.58).abs() < 1e-4);
}

#[test]
fn fixtures_parse_as_gltf() {
    for (name, bytes) in all() {
        let gltf = gltf::Gltf::from_slice(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(gltf.blob.is_some(), "{name} has a BIN chunk");
        assert_eq!(gltf.meshes().count(), 1);
    }
}

#[test]
fn the_messy_prop_is_dense_scaled_and_has_a_speck() {
    let bytes = messy_prop();
    let report = crate::analyse::analyse_bytes(&bytes, std::path::Path::new(".")).expect("analyse");
    assert_eq!(report.triangles, 19_200 + 12 + 12);
    assert_eq!(report.meshes, 3);
    assert_eq!(report.max_texture, 1024);
    // The root's half scale: the speck at x = 2 lands at 0.3 + 1.0.
    assert!(
        (report.size[0] - (1.0 + 0.25 + 0.0025)).abs() < 1e-3,
        "{:?}",
        report.size
    );
}
