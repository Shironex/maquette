//! Baking a recipe with grips: it is taken like any other change, the job
//! carries each grip on the bake's frames with its pose, and the review
//! says how many of a grip's bones the model has.

use std::fs;

use super::job::{self, JOB_FILE, REPORT_FILE};
use super::result::{read_run, result_of};
use super::tests::{humanoid, project_with};
use super::*;
use crate::inbox::fixture::{self, HumanoidSpec, SkinFlaw};
use crate::recipes::grips::{GRIP_VERSION, GripFile, GripHands, GripHolds, GripSide, layer_of};
use crate::recipes::{GripFrom, LayerMode, RecipeLayer};

/// A two handed grip: both hands, a forearm and the right hand's index and
/// little fingers.
fn longsword() -> GripFile {
    GripFile {
        version: GRIP_VERSION,
        name: "Longsword, two hands".into(),
        revision: 2,
        weapon: "longsword".into(),
        hands: GripHolds {
            holds: GripSide::Right,
            supports: Some(GripSide::Left),
        },
        pose: [
            ("rightLowerArm", [0.0, 15.0, 0.0]),
            ("rightHand", [4.0, 0.0, -8.0]),
            ("rightIndexProximal", [0.0, 0.0, 62.0]),
            ("rightLittleProximal", [0.0, 0.0, 70.0]),
            ("rightLittleIntermediate", [0.0, 0.0, 80.0]),
            ("rightLittleDistal", [0.0, 0.0, 40.0]),
            ("leftHand", [0.0, 6.0, 0.0]),
        ]
        .into_iter()
        .map(|(bone, turn)| (bone.to_string(), turn))
        .collect(),
    }
}

pub(super) fn grips() -> Vec<RecipeLayer> {
    let from = |revision| GripFrom {
        entry: "longsword-two-hands".into(),
        uid: Some("0a1b2c3d4e5f6071".into()),
        revision,
    };
    let mut both = layer_of("l-1", &longsword(), from(2), None);
    both.settings.start = 0.2;
    both.settings.end = 0.7;
    let mut left = layer_of("l-2", &longsword(), from(1), Some(GripHands::Left));
    left.settings.weight = 0.5;
    left.settings.ease_in = Some(0);
    left.mode = Some(LayerMode::Add);
    left.from = None;
    vec![both, left]
}

#[test]
fn a_recipe_with_grips_bakes_and_stages_each_grip_on_the_bake_frames() {
    let mut p = project_with(humanoid());
    p.change(grips(), Vec::new());
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("a grip bakes");
    for vrm in [
        "rightLowerArm",
        "rightHand",
        "rightIndexProximal",
        "leftHand",
    ] {
        assert!(prepared.bones.contains_key(vrm), "{vrm} is posed");
    }
    assert_eq!(
        prepared.bones["rightLittleDistal"], "mixamorig:RightHandPinky3",
        "a finger goes by its own name on the rig"
    );
    let [both, left] = &prepared.grip_results[..] else {
        panic!("{:?}", prepared.grip_results)
    };
    assert_eq!((both.bones, both.posed), (7, 7));
    assert!(both.missing.is_empty());
    assert_eq!(
        (both.entry.as_deref(), both.revision),
        (Some("longsword-two-hands"), Some(2))
    );
    assert_eq!(
        (left.bone.as_str(), left.entry.as_deref()),
        ("leftHand", None)
    );

    let work = p.root().join("work");
    job::stage(p.root(), &prepared, &work).expect("stage");
    let job: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join(JOB_FILE)).expect("job")).expect("json");
    let staged = &job["recipe"]["grips"];
    let frames = p.recipe.recipe.base.frames;
    let at = |share: f64| share * f64::from(frames);
    assert_eq!(staged["l-1"]["first"].as_f64(), Some(at(0.2).ceil()));
    assert_eq!(staged["l-1"]["last"].as_f64(), Some(at(0.7).floor()));
    assert_eq!(staged["l-1"]["easeIn"], 4);
    assert_eq!(staged["l-1"]["mode"], "replace");
    assert_eq!(staged["l-1"]["pose"]["rightIndexProximal"][2], 62.0);
    assert_eq!(
        staged["l-1"]["pose"].as_object().map(serde_json::Map::len),
        Some(7)
    );
    assert_eq!(staged["l-2"]["first"], 0);
    assert_eq!(staged["l-2"]["last"].as_u64(), Some(u64::from(frames)));
    assert_eq!(staged["l-2"]["easeIn"], 0);
    assert_eq!(staged["l-2"]["mode"], "add");
    assert_eq!(staged["l-2"]["weight"], 0.5);
    assert_eq!(
        staged["l-2"]["pose"].as_object().map(serde_json::Map::len),
        Some(1)
    );
    // The changes still go to the script in the order they are laid.
    let kinds: Vec<&str> = job["recipe"]["layers"]
        .as_array()
        .expect("layers")
        .iter()
        .filter_map(|l| l["kind"].as_str())
        .collect();
    assert_eq!(kinds, ["grip", "grip"]);
}

const REPORT: &str = r#"{
  "clip": "reach_out", "frames": 30, "facing": [0, 0, 1],
  "feet": {}, "baseFeet": {}, "reaches": [], "pins": [], "seconds": 1.0
}"#;

#[test]
fn each_grip_reads_as_a_review_line_saying_the_bones_the_rig_lacks() {
    // This rig's right little finger has names no rig uses.
    let one_sided = fixture::humanoid(&HumanoidSpec {
        flaw: SkinFlaw::OneSidedFinger,
        ..HumanoidSpec::default()
    });
    let mut p = project_with(one_sided);
    p.change(grips(), Vec::new());
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("it still bakes");
    let work = p.root().join("work");
    fs::create_dir_all(&work).expect("work");
    fs::write(work.join(REPORT_FILE), REPORT).expect("report");
    let result = result_of(&read_run(&work).expect("run"), &prepared, &work, 2.0);
    let [both, left] = &result.grips[..] else {
        panic!("{:?}", result.grips)
    };
    assert_eq!((both.bones, both.posed), (7, 4));
    assert_eq!(
        both.missing,
        [
            "rightLittleDistal",
            "rightLittleIntermediate",
            "rightLittleProximal"
        ]
    );
    assert_eq!(both.frames, Some([8, 26]), "38 frames: 0.2 to 0.7");
    assert_eq!((left.bones, left.posed), (1, 1));

    let review = review_of(&result);
    let line = |id: &str| {
        review
            .lines
            .iter()
            .find(|l| l.id == id)
            .unwrap_or_else(|| panic!("no {id}"))
    };
    let short = line("grip-l-1");
    assert_eq!(
        (short.label.as_str(), short.value.as_str()),
        ("Grip", "4 of 7 bones")
    );
    assert_eq!(short.status, RecipeReviewStatus::Warn);
    assert_eq!(short.about.as_deref(), Some("rightHand (l-1)"));
    assert_eq!(
        short.note.as_deref(),
        Some(
            "Frames 8 to 26. Elsheria has no bone for 3 of its finger bones, so the grip poses the hand and arm only there."
        )
    );
    let whole = line("grip-l-2");
    assert_eq!(
        (whole.value.as_str(), whole.status),
        ("1 of 1 bones", RecipeReviewStatus::Pass)
    );
    assert_eq!(whole.note.as_deref(), Some("Frames 0 to 38."));
    assert_eq!(review.warns, 1, "only the grip with bones left out");
    // Grips sit after the pins and joint limits, before the feet.
    let ids: Vec<&str> = review.lines.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["grip-l-1", "grip-l-2", "feet", "stretch"]);
}

#[test]
fn a_grip_no_frame_falls_within_poses_nothing_and_says_so() {
    let mut p = project_with(humanoid());
    let mut layers = grips();
    layers.truncate(1);
    // 38 frames: no frame's share falls between 0.41 and 0.42.
    layers[0].settings.start = 0.41;
    layers[0].settings.end = 0.42;
    p.change(layers, Vec::new());
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    assert!(prepared.grips.is_empty(), "nothing goes to the script");
    assert_eq!(prepared.grip_results[0].frames, None);
    let work = p.root().join("work");
    fs::create_dir_all(&work).expect("work");
    fs::write(work.join(REPORT_FILE), REPORT).expect("report");
    let result = result_of(&read_run(&work).expect("run"), &prepared, &work, 2.0);
    let review = review_of(&result);
    let line = &review.lines[0];
    assert_eq!(line.status, RecipeReviewStatus::Warn);
    assert_eq!(
        line.note.as_deref(),
        Some("No frame falls within its range, so it poses nothing.")
    );
}
