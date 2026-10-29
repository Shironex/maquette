use std::fs;
use std::path::Path;

use super::*;
use crate::assets::{Asset, AssetStatus, AssetVersion, Stage};
use crate::inbox::analyse::RigKind;
use crate::inbox::fixture;
use crate::layout::{ClipSettings, ObjectState, PartPose};
use crate::pipeline::template::for_template;
use crate::project::{PROJECT_VERSION, ProjectTemplate};
use crate::test_blender::{blender, run};

fn asset(id: &str, file: &str) -> Asset {
    Asset {
        id: id.into(),
        name: id.replace('-', " "),
        kind: AssetKind::Prop,
        stage: Stage::Blender,
        status: AssetStatus::Review,
        versions: vec![AssetVersion {
            version: 1,
            file: file.into(),
            created_at: "2026-10-24T09:30:00.000Z".into(),
            stats: None,
        }],
        thumbnail: None,
        created_at: "2026-10-24T09:30:00.000Z".into(),
        updated_at: "2026-10-24T09:30:00.000Z".into(),
        origin: None,
        refs: Vec::new(),
    }
}

fn project(dir: &Path) -> ProjectManifest {
    let models = dir.join("models");
    fs::create_dir_all(&models).expect("models");
    fs::write(models.join("crate_v1.glb"), fixture::clean_prop()).expect("crate");
    fs::write(models.join("figure_v1.glb"), fixture::character(false)).expect("figure");
    ProjectManifest {
        version: PROJECT_VERSION,
        id: "maquette".into(),
        name: "Maquette".into(),
        template: ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: None,
        site_repo_name: None,
        site_repo_state: Default::default(),
        source_folder: None,
        created_at: "2026-10-24T09:30:00.000Z".into(),
        assets: vec![
            asset("crate", "models/crate_v1.glb"),
            Asset {
                kind: AssetKind::Character,
                ..asset("figure", "models/figure_v1.glb")
            },
        ],
    }
}

#[test]
fn the_view_lists_every_model_and_counts_motion_inside_the_scene_budget() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = project(dir.path());
    let def = for_template(ProjectTemplate::FloatingDiorama);
    let mut layout = SceneLayout::default();
    layout.clips.insert(
        "figure".into(),
        BTreeMap::from([("Idle".to_string(), ClipSettings { ship: false })]),
    );
    layout::save(dir.path(), &layout).expect("layout");
    let budgets = Budgets::default();
    let view = view(dir.path(), &manifest, &def, &budgets, None).expect("view");
    let ids: Vec<&str> = view.models.iter().map(|m| m.asset.as_str()).collect();
    assert_eq!(ids, ["crate", "figure"]);
    let figure = &view.models[1];
    assert_eq!(figure.measured, Measured::Source);
    assert!((figure.motion_bytes - 48.0).abs() < f64::EPSILON);
    assert!(view.models[0].clips.is_empty() && view.models[0].rig.is_none());
    let total: f64 = view.models.iter().map(|m| m.bytes).sum();
    assert!((view.budget.scene_bytes - total).abs() < f64::EPSILON);
    assert!((view.budget.motion_bytes - 48.0).abs() < f64::EPSILON);
    assert!((view.budget.scene_target_mb - 3.0).abs() < f64::EPSILON);
    assert_eq!(view.clips_off["figure"], ["Idle"]);
    assert!(!view.clips_off_supported, "no tools/build_scene.py");

    // A shipped build weighs the clip as it ships.
    fs::create_dir_all(dir.path().join("web/dist")).expect("dist");
    fs::write(
        dir.path().join("web/dist/figure_v1.glb"),
        fixture::character(false),
    )
    .expect("dist");
    fs::create_dir_all(dir.path().join("tools")).expect("tools");
    fs::write(
        dir.path().join("tools/build_scene.py"),
        crate::pipeline::starter::scripts(ProjectTemplate::FloatingDiorama)[0].text,
    )
    .expect("starter");
    let view = super::view(dir.path(), &manifest, &def, &budgets, None).expect("view");
    let figure = &view.models[1];
    assert_eq!(figure.measured, Measured::Shipped);
    assert_eq!(figure.clips[0].shipped_bytes, Some(48.0));
    assert!(view.clips_off_supported, "the starter drops clips");
}

#[test]
fn unknown_parts_are_named_with_their_asset() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = project(dir.path());
    let mut parts = BTreeMap::new();
    parts.insert("Crate".to_string(), PartPose::default());
    parts.insert("Lid".to_string(), PartPose::default());
    let states = BTreeMap::from([(
        "crate".to_string(),
        ObjectStates {
            states: vec![ObjectState {
                id: "open".into(),
                name: "Open".into(),
                parts,
            }],
            ..ObjectStates::default()
        },
    )]);
    assert_eq!(
        unknown_parts(dir.path(), &manifest, &states),
        [("crate".to_string(), "Lid".to_string())]
    );
}

#[test]
fn saving_motion_keeps_the_layout_and_refuses_unknown_parts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = project(dir.path());
    let mut layout = SceneLayout::default();
    layout
        .models
        .insert("crate".into(), layout::Placement::default());
    layout::save(dir.path(), &layout).expect("layout");
    let rest = ObjectState {
        id: "closed".into(),
        name: "Closed".into(),
        parts: BTreeMap::new(),
    };
    let mut parts = BTreeMap::new();
    parts.insert("Crate".to_string(), PartPose::default());
    let lifted = ObjectState {
        id: "lifted".into(),
        name: "Lifted".into(),
        parts,
    };
    let states = BTreeMap::from([(
        "crate".to_string(),
        ObjectStates {
            states: vec![rest, lifted.clone()],
            ..ObjectStates::default()
        },
    )]);
    let off = BTreeMap::from([("figure".to_string(), vec!["Idle".to_string()])]);
    save(dir.path(), &manifest, states.clone(), &off).expect("save");
    let back = layout::load(dir.path()).expect("load");
    assert!(back.models.contains_key("crate"), "placements stay");
    assert_eq!(back.states["crate"].states[1], lifted);
    assert_eq!(layout::states::clips_off(&back, "figure"), ["Idle"]);

    // Clips back on and the states emptied leave nothing behind.
    let empty = BTreeMap::from([("crate".to_string(), ObjectStates::default())]);
    save(dir.path(), &manifest, empty, &BTreeMap::new()).expect("save");
    let back = layout::load(dir.path()).expect("load");
    assert!(back.states.is_empty() && back.clips.is_empty());

    let mut wrong = states;
    if let Some(o) = wrong.get_mut("crate") {
        o.states[1].parts.insert("Lid".into(), PartPose::default());
    }
    let err = save(dir.path(), &manifest, wrong, &BTreeMap::new())
        .expect_err("lid")
        .to_string();
    assert!(err.contains("no part named Lid"), "{err}");
    let err = save(
        dir.path(),
        &manifest,
        BTreeMap::new(),
        &BTreeMap::from([("nope".to_string(), vec![])]),
    )
    .expect_err("nope")
    .to_string();
    assert!(err.contains("no model for the asset nope"), "{err}");
}

/// The Blender made fixtures the mock UI shows, read back headless.
#[test]
fn clips_rigs_and_parts_of_models_made_in_blender() {
    let Some(blender) = blender() else { return };
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("motion_models.py");
    fs::write(&script, include_str!("fixtures/motion_models.py")).expect("script");
    let make = |which: &str| {
        let out = dir.path().join(format!("{which}.glb"));
        run(
            &blender,
            &[
                "--python",
                &script.display().to_string(),
                "--",
                which,
                &out.display().to_string(),
            ],
        );
        read_file(&out).expect(which)
    };
    let figure = make("figure");
    let names: Vec<&str> = figure.clips.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Idle breathe", "Walk loop", "Wave"]);
    let walk = &figure.clips[1];
    assert_eq!((walk.frames, walk.keys), (32, 33));
    assert!((walk.fps - 30.0).abs() < f32::EPSILON);
    let travel = walk.root_travel.expect("walks");
    assert!((travel - 1.42).abs() < 0.02, "{travel}");
    assert_eq!(walk.loops, Some(true), "the legs end where they start");
    assert_eq!(figure.clips[0].frames, 90);
    assert_eq!(figure.clips[0].root_travel, None);
    assert_eq!(figure.clips[2].frames, 48);
    let rig = figure.rig.expect("rig");
    assert_eq!(rig.joints, 20);
    assert!(rig.humanoid);
    assert_eq!(rig.kind, RigKind::Other);
    assert_eq!(rig.max_influences, Some(1));
    let missing: Vec<&str> = rig
        .map
        .iter()
        .filter(|m| m.how == BoneMatch::Missing)
        .map(|m| m.bone.as_str())
        .collect();
    assert_eq!(missing, ["LeftToeBase", "RightToeBase"]);

    let cabinet = make("cabinet");
    assert!(cabinet.rig.is_none() && cabinet.clips.is_empty());
    assert_eq!(cabinet.parts.len(), 8);
    assert!(cabinet.parts.iter().any(|p| p == "Door"));

    let turntable = make("turntable");
    let spin = &turntable.clips[0];
    assert_eq!((spin.name.as_str(), spin.frames), ("Spin", 120));
    assert_eq!(spin.loops, Some(true), "a full turn ends where it starts");
}
