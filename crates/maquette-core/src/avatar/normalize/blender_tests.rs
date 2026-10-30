//! Normalising in a real, headless Blender. Ignored by default, so CI never
//! needs Blender: run with `--ignored`. `MAQUETTE_AVATAR_PROBE` names a
//! rigged `.glb` to normalise (the box humanoid otherwise), and
//! `MAQUETTE_AVATAR_OUT` a folder to copy the outputs into for a look.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::assets::{AssetKind, NewAsset, Stage};
use crate::inbox::fixture::{self, HumanoidSpec};
use crate::inbox::vrm::WriteSpec;
use crate::project::{NewProject, ProjectTemplate};
use crate::test_blender;
use crate::workspace;

#[test]
#[ignore = "runs Blender; MAQUETTE_AVATAR_PROBE picks the model"]
fn normalise_a_model_in_blender() {
    let Some(blender) = test_blender::blender() else {
        return;
    };
    let model = match std::env::var_os("MAQUETTE_AVATAR_PROBE") {
        Some(path) => fs::read(PathBuf::from(path)).expect("read the probe model"),
        None => fixture::humanoid(&HumanoidSpec {
            arm_drop: 55.0,
            yaw: 90.0,
            offset: [0.03, 0.2],
            animation: false,
            ..HumanoidSpec::default()
        }),
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Pets".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    fs::create_dir_all(project_dir.join("models")).expect("models");
    fs::write(project_dir.join("models/hero_v1.glb"), &model).expect("model");
    let mut asset = String::new();
    project::modify(&root, &config.layout, &manifest.id, |m| {
        let added = assets::add(
            m,
            NewAsset {
                name: "Hero".into(),
                kind: AssetKind::Character,
                stage: Stage::Blender,
            },
        )?;
        assets::add_version(m, &added.id, "models/hero_v1.glb".into(), None)?;
        asset = added.id;
        Ok(())
    })
    .expect("asset");

    let none = BoneOverrides::new();
    let planned = plan(&NormalizeRequest {
        workspace_root: &root,
        config: &config,
        project: &manifest.id,
        asset: &asset,
        version: None,
        overrides: &none,
        elbows: Elbows::Keep,
        fingers: Fingers::Straighten,
    })
    .expect("plan");
    let runner = test_blender::runner();
    let found = run(&runner, &test_blender::toolbox(&blender), &planned).expect("normalise");
    println!(
        "{}",
        serde_json::to_string_pretty(&found.report).expect("json")
    );
    for f in &found.readiness.findings {
        println!("[{:?}] {}", f.level, f.title);
    }
    assert_eq!(found.state, NormalizeState::Pending);
    assert!(found.readiness.ready, "{:#?}", found.readiness.findings);
    let pose = &found.readiness.pose;
    assert!(pose.ground_y.is_some_and(|y| y.abs() < 0.01), "{pose:?}");
    assert!(pose.hips_offset.is_some_and(|o| o < 0.01), "{pose:?}");
    for sheet in [&found.sheets.before, &found.sheets.after] {
        assert!(fs::metadata(sheet).is_ok_and(|m| m.len() > 1000), "{sheet}");
    }
    assert!(found.report.weights.max_influences <= 4);
    assert!(
        write_vrm(&planned.dir, &WriteSpec::default()).is_err(),
        "not approved"
    );
    approve(&planned.dir).expect("approve");
    assert!(approved_glb(&planned.dir).is_ok());

    if let Some(out) = std::env::var_os("MAQUETTE_AVATAR_OUT") {
        let out = PathBuf::from(out);
        fs::create_dir_all(&out).expect("out");
        for name in [OUTPUT_FILE, REPORT_FILE, SHEET_BEFORE, SHEET_AFTER] {
            fs::copy(planned.dir.join(name), out.join(name)).expect("copy");
        }
    }
}

/// The PNG's height in pixels, from its header.
fn png_height(path: &str) -> u32 {
    let bytes = fs::read(path).expect("sheet");
    u32::from_be_bytes(bytes[20..24].try_into().expect("IHDR height"))
}

#[test]
#[ignore = "runs Blender"]
fn curled_fingers_come_out_straight_in_blender() {
    let Some(blender) = test_blender::blender() else {
        return;
    };
    let model = fixture::humanoid(&HumanoidSpec {
        arm_drop: 55.0,
        yaw: 90.0,
        finger_curl: 20.0,
        animation: false,
        ..HumanoidSpec::default()
    });
    let w = super::tests::world_with(&model);
    let none = BoneOverrides::new();
    let request = |fingers| NormalizeRequest {
        workspace_root: &w.root,
        config: &w.config,
        project: &w.project,
        asset: &w.asset,
        version: None,
        overrides: &none,
        elbows: Elbows::Straighten,
        fingers,
    };
    let planned = plan(&request(Fingers::Straighten)).expect("plan");
    assert!(
        planned.readiness.findings.iter().any(|f| f.id == "fingers"),
        "{:#?}",
        planned.readiness.findings
    );
    let runner = test_blender::runner();
    let tools = test_blender::toolbox(&blender);
    let found = run(&runner, &tools, &planned).expect("normalise");
    println!(
        "{}",
        serde_json::to_string_pretty(&found.report).expect("json")
    );
    let report = &found.report;
    assert!(
        report.before.finger_curl_deg.is_some_and(|d| d > 50.0),
        "{report:?}"
    );
    assert!(
        report.after.finger_curl_deg.is_some_and(|d| d < 2.0),
        "{report:?}"
    );
    for side in ["left", "right"] {
        assert_eq!(report.straightened[side].share, 1.0, "{report:?}");
    }
    assert!(
        found.readiness.findings.iter().all(|f| f.id != "fingers"),
        "{:#?}",
        found.readiness.findings
    );
    assert!(
        found
            .readiness
            .pose
            .finger_curl_deg
            .is_some_and(|d| d < 2.0)
    );
    // Four pose rows and the hands row, 400 pixels each.
    assert_eq!(png_height(&found.sheets.after), 2000);

    // Kept, they stay as they were.
    let kept = plan(&request(Fingers::Keep)).expect("plan");
    let found = run(&runner, &tools, &kept).expect("normalise");
    assert!(found.report.straightened.is_empty());
    assert!(found.report.after.finger_curl_deg.is_some_and(|d| d > 50.0));
    assert!(found.readiness.findings.iter().any(|f| f.id == "fingers"));
}

#[test]
#[ignore = "runs Blender"]
fn bunched_finger_joints_are_placed_along_the_fingers_in_blender() {
    let Some(blender) = test_blender::blender() else {
        return;
    };
    // The right hand's finger joints sit at the wrist, its fingers weighted
    // to the hand alone: the joints go along the fingers the mesh shows, and
    // then the fingers straighten like any other.
    let model = fixture::humanoid(&HumanoidSpec {
        arm_drop: 55.0,
        yaw: 90.0,
        bunched_hand: true,
        animation: false,
        ..HumanoidSpec::default()
    });
    let w = super::tests::world_with(&model);
    let none = BoneOverrides::new();
    let planned = plan(&NormalizeRequest {
        workspace_root: &w.root,
        config: &w.config,
        project: &w.project,
        asset: &w.asset,
        version: None,
        overrides: &none,
        elbows: Elbows::Straighten,
        fingers: Fingers::Straighten,
    })
    .expect("plan");
    assert_eq!(
        planned.readiness.pose.bunched_hands,
        vec!["right".to_string()]
    );
    let runner = test_blender::runner();
    let found = run(&runner, &test_blender::toolbox(&blender), &planned).expect("normalise");
    assert!(found.readiness.pose.bunched_hands.is_empty());
    assert!(
        found
            .readiness
            .findings
            .iter()
            .all(|f| f.id != "finger-bones"),
        "{:#?}",
        found.readiness.findings
    );
    let report = &found.report;
    println!("{}", serde_json::to_string_pretty(report).expect("json"));
    assert!(!report.repaired.contains_key("left"), "{report:?}");
    let repaired = &report.repaired["right"];
    assert!(repaired.vertices > 100, "{repaired:?}");
    assert!(
        repaired.reach_after > 5.0 * repaired.reach_before,
        "{repaired:?}"
    );
    assert!(report.straightened["right"].share > 0.0, "{report:?}");
    assert!(
        report.after.fingers.contains_key("rightMiddle"),
        "{report:?}"
    );
    assert_eq!(report.weights.over_influences, 0);
    assert_eq!(report.weights.unweighted, 0);
    assert_eq!(png_height(&found.sheets.after), 2000);
}
