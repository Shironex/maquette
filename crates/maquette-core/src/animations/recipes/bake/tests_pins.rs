//! Baking a recipe with pins: it is no longer refused, the job carries each
//! pin on the bake's frames with the limb it solves, and the report's drift
//! reads as the review's pin lines.

use std::fs;

use super::job::{self, JOB_FILE, REPORT_FILE};
use super::result::{read_run, result_of};
use super::tests::{point, reaching};
use super::*;
use crate::recipes::{LayerKind, LayerSettings, PinFlat, RecipeLayer};

fn pin(id: &str, bone: &str, target: Option<&str>) -> RecipeLayer {
    RecipeLayer::new(
        id,
        LayerKind::Pin,
        bone,
        LayerSettings {
            start: 0.2,
            end: 0.7,
            target: target.map(str::to_string),
            ..LayerSettings::default()
        },
    )
}

fn pinned() -> tests::Project {
    let mut p = reaching();
    let mut heel = pin("l-3", "rightFoot", None);
    heel.settings.flat = Some(PinFlat::Heel);
    p.change(
        vec![
            pin("l-1", "leftFoot", None),
            pin("l-2", "leftHand", Some("b-2")),
            heel,
        ],
        vec![point("b-2", Some([0.3, 1.0, 0.3]))],
    );
    p
}

#[test]
fn a_recipe_with_pins_bakes_and_stages_each_pin_on_the_bake_frames() {
    let p = pinned();
    assert_eq!(not_bakeable(&p.recipe), None, "pins bake now");
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    for vrm in [
        "leftUpperLeg",
        "leftLowerLeg",
        "leftUpperArm",
        "leftLowerArm",
    ] {
        assert!(prepared.bones.contains_key(vrm), "{vrm} is solved");
    }
    let work = p.root().join("work");
    job::stage(p.root(), &prepared, &work).expect("stage");
    assert!(work.join("recipe_pins.py").is_file());
    let job: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join(JOB_FILE)).expect("job")).expect("json");
    let pins = job["recipe"]["pins"].as_array().expect("pins");
    assert_eq!(pins.len(), 3);
    let frames = p.recipe.recipe.base.frames;
    // From the first frame its start reaches to the last within its end.
    let at = |share: f64| share * f64::from(frames);
    assert_eq!(pins[0]["first"].as_f64(), Some(at(0.2).ceil()));
    assert_eq!(pins[0]["last"].as_f64(), Some(at(0.7).floor()));
    assert_eq!(
        pins[0]["flat"], "toe-and-heel",
        "a foot keeps toe and heel flat"
    );
    assert_eq!(pins[0]["easeIn"], 3);
    assert!(
        pins[0].get("target").is_none(),
        "it stays where the foot is"
    );
    assert_eq!(pins[1]["target"], "b-2");
    assert!(pins[1].get("flat").is_none(), "a hand has no flat");
    assert_eq!(pins[2]["flat"], "heel");
    assert!(job["recipe"]["targets"]["b-2"]["point"].is_array());
}

const REPORT: &str = r#"{
  "clip": "reach_out", "frames": 30, "facing": [0, 0, 1],
  "feet": {}, "baseFeet": {}, "reaches": [],
  "pins": [
    {"layer": "l-1", "bone": "leftFoot", "first": 6, "last": 21,
     "drift": 0.0042, "frame": 14, "baseDrift": 0.068},
    {"layer": "l-2", "bone": "leftHand", "first": 6, "last": 21,
     "drift": 0.031, "frame": 18, "baseDrift": 0.2},
    {"layer": "l-3", "bone": "rightFoot", "first": 6, "last": 8,
     "drift": null, "frame": null, "baseDrift": null}
  ],
  "seconds": 1.0
}"#;

#[test]
fn each_pin_s_drift_reads_as_a_review_line_held_to_one_centimetre() {
    let p = pinned();
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let work = p.root().join("work");
    fs::create_dir_all(&work).expect("work");
    fs::write(work.join(REPORT_FILE), REPORT).expect("report");
    let result = result_of(&read_run(&work).expect("run"), &prepared, &work, 2.0);
    let [foot, hand, short] = &result.pins[..] else {
        panic!("{:?}", result.pins)
    };
    assert_eq!((foot.drift_cm, foot.base_drift_cm), (Some(0.42), Some(6.8)));
    assert_eq!((hand.drift_cm, hand.frame), (Some(3.1), Some(18)));
    assert_eq!((short.drift_cm, short.frame), (None, None));

    let review = review_of(&result);
    let line = |id: &str| {
        review
            .lines
            .iter()
            .find(|l| l.id == id)
            .unwrap_or_else(|| panic!("no {id}"))
    };
    let foot = line("pin-l-1");
    assert_eq!(foot.label, "Pin drift");
    assert_eq!(
        (foot.value.as_str(), foot.limit.as_str()),
        ("0.4 cm", "1 cm")
    );
    assert_eq!(foot.status, RecipeReviewStatus::Pass);
    assert_eq!(
        foot.note.as_deref(),
        Some("Frames 6 to 21. Without the pin it moves 6.8 cm.")
    );
    let hand = line("pin-l-2");
    assert_eq!(hand.status, RecipeReviewStatus::Warn);
    assert!(hand.note.as_deref().is_some_and(|n| n.contains("frame 18")));
    assert_eq!(line("pin-l-3").status, RecipeReviewStatus::NotMeasured);
    assert_eq!(review.warns, 1, "only the hand drifts too far");
    // Pins sit after the reaches and before the feet.
    let ids: Vec<&str> = review.lines.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["pin-l-1", "pin-l-2", "pin-l-3", "feet", "stretch"]);
}
