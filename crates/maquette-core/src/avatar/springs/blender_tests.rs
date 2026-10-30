//! Finding and chaining in a real, headless Blender, then writing the VRM
//! with its springs. Ignored by default, so CI never needs Blender: run
//! with `--ignored`. `MAQUETTE_AVATAR_PROBE` names a rigged `.glb` in
//! VRM's rest pose (the box humanoid with a strip hanging from its head
//! otherwise), and `MAQUETTE_AVATAR_OUT` a folder to copy the outputs into
//! for a look.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::avatar::SpringSpec;
use crate::avatar::normalize::NormalizeState;
use crate::inbox::fixture::{self, HumanoidSpec};
use crate::inbox::vrm::{self, BoneOverrides, SpringKind, WriteSpec};
use crate::test_blender;

#[test]
#[ignore = "runs Blender; MAQUETTE_AVATAR_PROBE picks the model"]
fn find_chain_and_write_springs_in_blender() {
    let Some(blender) = test_blender::blender() else {
        return;
    };
    let probe = std::env::var_os("MAQUETTE_AVATAR_PROBE").map(PathBuf::from);
    let model = match &probe {
        Some(path) => fs::read(path).expect("read the probe model"),
        None => fixture::humanoid(&HumanoidSpec {
            tail: true,
            animation: false,
            ..HumanoidSpec::default()
        }),
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let glb = dir.path().join("model.glb");
    fs::write(&glb, &model).expect("model");
    let none = BoneOverrides::new();
    let readiness = vrm::readiness(&model, dir.path(), &none).expect("readiness");
    let input = SpringsInput {
        project: "probe".to_string(),
        asset: "probe".to_string(),
        version: 1,
        dir: dir.path().join("springs"),
        glb: glb.clone(),
        bones: readiness
            .bones
            .iter()
            .filter_map(|b| b.joint.clone().map(|j| (b.bone.clone(), j)))
            .collect(),
        cache_root: dir.path().join("hashes"),
    };
    let out = std::env::var_os("MAQUETTE_AVATAR_OUT").map(PathBuf::from);
    if let Some(out) = &out {
        fs::create_dir_all(out).expect("out");
        fs::write(out.join("model.glb"), &model).expect("model");
    }
    let runner = test_blender::runner();
    let tools = test_blender::toolbox(&blender);
    let found = find::find(&runner, &tools, &input).expect("find");
    for c in &found.candidates {
        println!(
            "{} {:?} from {} conf {:.2} len {:.2} verts {} joints {}",
            c.id, c.kind, c.root_bone, c.confidence, c.length, c.vertices, c.joints
        );
    }
    let first = found.candidates.first().expect("a candidate");
    if probe.is_none() {
        assert_eq!(
            (first.kind, first.root_bone.as_str()),
            (SpringKind::Hair, "head")
        );
        assert_eq!(found.candidates.len(), 1, "the strip is one part");
    }
    let specs: Vec<SpringSpec> = found
        .candidates
        .iter()
        .filter(|c| c.confidence >= 0.5)
        .map(|c| SpringSpec {
            id: c.id.clone(),
            kind: c.kind,
            root_bone: c.root_bone.clone(),
            line: c.line.clone(),
            joints: None,
            preset: None,
            stiffness: None,
            gravity_power: None,
            drag_force: None,
            hit_radius: None,
        })
        .collect();
    let plan = chain::plan(&input, &specs).expect("plan");
    let chained = chain::run(&runner, &tools, &plan).expect("chain");
    println!(
        "{}",
        serde_json::to_string_pretty(&chained.report).expect("json")
    );
    assert_eq!(chained.state, NormalizeState::Pending);
    assert_eq!(chained.report.chains.len(), specs.len());
    assert_eq!(chained.report.weights.unweighted, 0);
    assert!(chained.report.weights.max_influences <= 4);
    for sheet in [&chained.sheets.before, &chained.sheets.after] {
        assert!(fs::metadata(sheet).is_ok_and(|m| m.len() > 1000), "{sheet}");
    }

    chain::approve(&plan.dir).expect("approve");
    let bytes = chain::approved_glb(&plan.dir).expect("approved");
    let spec = WriteSpec {
        meta: vrm::VrmMeta {
            name: "Probe".to_string(),
            authors: vec!["Test".to_string()],
            ..vrm::VrmMeta::default()
        },
        ..WriteSpec::default()
    };
    let mut written = vrm::write(&bytes, &spec, &plan.dir).expect("write");
    let bones = chain::spring_bones(&chained.report, &specs);
    written.bytes = vrm::add_springs(&written.bytes, &bones).expect("springs");
    let back = vrm::round_trip(&written.bytes);
    assert!(back.passed, "{:?}", back.problems);
    assert_eq!(back.springs as usize, specs.len());

    if let Some(out) = out {
        fs::write(out.join("springs.vrm"), &written.bytes).expect("vrm");
        let copy = |from: &str, to: &str| {
            fs::copy(Path::new(from), out.join(to)).expect("copy");
        };
        copy(&found.sheet, "candidates.png");
        copy(&chained.sheets.before, "sheet_before.png");
        copy(&chained.sheets.after, "sheet_after.png");
    }
}

#[test]
#[ignore = "runs Blender"]
fn a_charm_close_to_the_body_is_a_small_piece_with_a_short_chain_in_blender() {
    let Some(blender) = test_blender::blender() else {
        return;
    };
    let model = fixture::humanoid(&HumanoidSpec {
        tail: true,
        charm: true,
        animation: false,
        ..HumanoidSpec::default()
    });
    let dir = tempfile::tempdir().expect("tempdir");
    let glb = dir.path().join("model.glb");
    fs::write(&glb, &model).expect("model");
    if let Some(out) = std::env::var_os("MAQUETTE_AVATAR_OUT") {
        fs::create_dir_all(&out).expect("out");
        fs::write(Path::new(&out).join("charm.glb"), &model).expect("model");
    }
    let readiness = vrm::readiness(&model, dir.path(), &BoneOverrides::new()).expect("readiness");
    let input = SpringsInput {
        project: "probe".to_string(),
        asset: "probe".to_string(),
        version: 1,
        dir: dir.path().join("springs"),
        glb,
        bones: readiness
            .bones
            .iter()
            .filter_map(|b| b.joint.clone().map(|j| (b.bone.clone(), j)))
            .collect(),
        cache_root: dir.path().join("hashes"),
    };
    let runner = test_blender::runner();
    let tools = test_blender::toolbox(&blender);
    let found = find::find(&runner, &tools, &input).expect("find");
    let ids: Vec<(&str, bool)> = found
        .candidates
        .iter()
        .map(|c| (c.id.as_str(), c.small))
        .collect();
    // The tail hangs off the body as before; the charm is too near it for that.
    assert_eq!(found.candidates[0].id, "hair-1", "{ids:?}");
    assert!(!found.candidates[0].small);
    let charms: Vec<_> = found.candidates.iter().filter(|c| c.small).collect();
    assert_eq!(charms.len(), 1, "{ids:?}");
    let charm = charms[0];
    assert_eq!(charm.id, "charm-1");
    assert_eq!(charm.kind, SpringKind::Accessory);
    assert_eq!(charm.root_bone, "spine");
    assert!((2..=4).contains(&charm.joints), "{}", charm.joints);
    assert!(charm.confidence > 0.5, "{}", charm.confidence);

    let spec = SpringSpec {
        id: charm.id.clone(),
        kind: charm.kind,
        root_bone: charm.root_bone.clone(),
        line: charm.line.clone(),
        joints: None,
        preset: None,
        stiffness: None,
        gravity_power: None,
        drag_force: None,
        hit_radius: None,
    };
    let plan = chain::plan(&input, std::slice::from_ref(&spec)).expect("plan");
    let chained = chain::run(&runner, &tools, &plan).expect("chain");
    let built = &chained.report.chains[0];
    // Its joints plus the tip.
    assert_eq!(built.joints.len(), charm.joints as usize + 1);
    assert_eq!(built.root_bone, "spine");
    assert!(built.vertices >= 24, "{}", built.vertices);
    assert_eq!(chained.report.weights.unweighted, 0);
    assert!(chained.report.weights.max_influences <= 4);
}
