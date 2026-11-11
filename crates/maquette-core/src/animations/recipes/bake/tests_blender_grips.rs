//! Grips in a real, headless Blender: the fixture model with finger bones
//! takes the dodge clip, whose left arm lifts, with a grip laid in its
//! place on that arm, the right hand and its index finger, and another
//! added to the right upper arm. At every frame of the bake each gripped bone
//! is as far along from the clip to the grip's pose as the shared grip math
//! says (`recipes::grips`), which the rough preview follows too, and a bone
//! the rig lacks is left out and said in the review. A second bake holds a
//! pin, pose keys and a grip in one recipe. Skipped where no Blender is
//! installed.

use super::tests::{Project, project_on};
use super::tests_blender_joined::{Glb, Keys, bake_of, degrees_apart, fps_of, sample};
use super::tests_blender_keys::rest_of;
use super::*;
use crate::animations::list;
use crate::animations::tests_blender::{Setup, fixture, import_fixture, setup};
use crate::recipes::grips::{GripSpan, amount, spans};
use crate::recipes::timeline::timeline;
use crate::recipes::{
    KeyEase, LayerKind, LayerMode, LayerSettings, PoseKey, RecipeLayer, empty_pose_layer,
};
use crate::workspace::WorkspaceConfig;

/// A grip change on shares `range`, easing over `eases` frames.
fn grip(
    id: &str,
    mode: LayerMode,
    range: (f32, f32),
    eases: (u32, u32),
    pose: &[(&str, [f32; 3])],
) -> RecipeLayer {
    RecipeLayer {
        mode: Some(mode),
        pose: Some(pose.iter().map(|(b, t)| ((*b).to_string(), *t)).collect()),
        ..RecipeLayer::new(
            id,
            LayerKind::Grip,
            "rightHand",
            LayerSettings {
                start: range.0,
                end: range.1,
                ease_in: Some(eases.0),
                ease_out: Some(eases.1),
                ..LayerSettings::default()
            },
        )
    }
}

/// A project in `s` on fixture clip `clip` and the fixture model with
/// finger bones, holding a small nudge on the head (a bake needs a change)
/// and then `layers`.
fn project(s: &Setup, clip: (&str, &str), layers: Vec<RecipeLayer>) -> Project {
    let config = WorkspaceConfig::default();
    // A lone FBX take is named after its file.
    let name = clip.1.trim_end_matches(".fbx");
    let known = |clips: &[crate::animations::AnimationClip]| {
        clips.iter().find(|c| c.name == name).map(|c| c.id.clone())
    };
    if known(&list(s.root.path()).expect("clips")).is_none() {
        import_fixture(s, &config, clip.0, clip.1);
    }
    let id = known(&list(s.root.path()).expect("clips")).expect("the clip");
    let file = s.files.join("model_fingers.glb");
    fixture(
        &s.blender,
        &s.root.path().join("scripts"),
        "model_fingers",
        &file,
    );
    let mut p = project_on(
        s.root.path(),
        Some(&id),
        std::fs::read(&file).expect("model"),
    );
    let nudge = RecipeLayer::new(
        "l-1",
        LayerKind::Nudge,
        "head",
        LayerSettings {
            rotation: Some([4.0, 0.0, 0.0]),
            ..LayerSettings::default()
        },
    );
    p.change([vec![nudge], layers].concat(), Vec::new());
    p
}

const DODGE: (&str, &str) = ("clip", "Dodge Left.fbx");
const SLIDE: (&str, &str) = ("slide", "Foot Slide.fbx");

fn turns(glb: &Glb, result: &RecipeBakeResult, node: usize) -> Keys {
    glb.keys(&result.clip, node, "rotation").expect("keys")
}

/// Every frame of `ours`: `node` is as far along from the clip (`theirs`,
/// the same recipe without the grip) to the grip's pose as `grip` shows
/// there. Held whole it is the grip's pose, `turn` degrees from the bone's
/// rest whatever the clip does. Answers how far the clip turns the bone
/// while the grip holds whole.
fn replaces(
    grip: &GripSpan,
    ours: (&Glb, &RecipeBakeResult),
    theirs: (&Glb, &RecipeBakeResult),
    (node, bone, turn): (usize, &str, f64),
) -> f64 {
    let fps = fps_of(ours.0, ours.1);
    let (a, b) = (turns(ours.0, ours.1, node), turns(theirs.0, theirs.1, node));
    let whole: Vec<u32> = (0..=ours.1.frames)
        .filter(|f| amount(grip, f64::from(*f)) >= 1.0 - 1e-9)
        .collect();
    let first = *whole.first().expect("it holds whole on some frame");
    let want = sample(&a, f64::from(first) / fps);
    let from_rest = degrees_apart(&want, &rest_of(ours.0, node));
    assert!(
        (from_rest - turn).abs() < 1.5,
        "{bone} is {from_rest} degrees from its rest, the grip says {turn}"
    );
    for f in 0..=ours.1.frames {
        let t = f64::from(f) / fps;
        let (now, clip) = (sample(&a, t), sample(&b, t));
        let shows = amount(grip, f64::from(f));
        let apart = degrees_apart(&now, &clip);
        let gap = degrees_apart(&clip, &want);
        assert!(
            (apart - shows * gap).abs() < 1.5,
            "frame {f}: {bone} is {apart} degrees from the clip, {shows} of {gap} is {}",
            shows * gap
        );
        if whole.contains(&f) {
            let off = degrees_apart(&now, &want);
            assert!(off < 1.0, "frame {f}: {bone} is {off} degrees off the grip");
        }
    }
    let at = |f: u32| sample(&b, f64::from(f) / fps);
    whole
        .iter()
        .map(|f| degrees_apart(&at(*f), &at(first)))
        .fold(0.0, f64::max)
}

#[test]
fn a_grip_replaces_the_clip_on_its_bones_or_adds_to_it_eased_as_the_shared_math_says() {
    let Some(s) = setup() else { return };
    let bones = [
        "leftUpperArm",
        "rightHand",
        "rightIndexProximal",
        "rightIndexIntermediate",
        "rightUpperArm",
    ];
    let plain = project(&s, DODGE, Vec::new());
    let (base, base_glb, nodes) = bake_of(&s, &plain, &bones);
    assert!(base.grips.is_empty());

    let held = grip(
        "l-2",
        LayerMode::Replace,
        (0.1, 0.9),
        (2, 3),
        &[
            ("leftUpperArm", [0.0, 0.0, 30.0]),
            ("rightHand", [0.0, 25.0, 0.0]),
            ("rightIndexProximal", [0.0, 0.0, 50.0]),
            ("rightIndexIntermediate", [0.0, 0.0, 40.0]),
            // The fixture's right hand has an index finger and no thumb.
            ("rightThumbDistal", [10.0, 0.0, 0.0]),
        ],
    );
    let mut added = grip(
        "l-3",
        LayerMode::Add,
        (0.0, 1.0),
        (0, 0),
        &[("rightUpperArm", [0.0, 36.0, 0.0])],
    );
    added.settings.weight = 0.5;
    let p = project(&s, DODGE, vec![held, added]);
    let (result, glb, _) = bake_of(&s, &p, &bones);
    assert_eq!(result.frames, base.frames, "a grip keeps the length");
    let recipe = &p.recipe.recipe;
    let found = spans(recipe, &timeline(recipe));
    let [replaced, laid_on] = &found[..] else {
        panic!("{found:?}")
    };
    assert_eq!((replaced.first, replaced.last), (3, 27), "30 frames");

    // In place of a turn the clip keeps changing: the arm holds the grip's
    // pose while the clip lifts it.
    let (ours, theirs) = ((&glb, &result), (&base_glb, &base));
    let lifted = replaces(replaced, ours, theirs, (nodes[0], "leftUpperArm", 30.0));
    eprintln!("the clip turns the left upper arm {lifted} degrees while the grip holds whole");
    assert!(lifted > 20.0, "the clip lifts the arm too little to tell");
    // A hand and its finger, parent before child: each from its own rest.
    for (i, bone, turn) in [
        (1, "rightHand", 25.0),
        (2, "rightIndexProximal", 50.0),
        (3, "rightIndexIntermediate", 40.0),
    ] {
        replaces(replaced, ours, theirs, (nodes[i], bone, turn));
    }

    // Added at half weight with no ease: 18 degrees on top of the clip on
    // every frame. The retarget turns this A posed arm up to the clip's, so
    // a turn laid in the clip's place would land somewhere else.
    let fps = fps_of(&glb, &result);
    let (a, b) = (
        turns(&glb, &result, nodes[4]),
        turns(&base_glb, &base, nodes[4]),
    );
    for f in 0..=result.frames {
        let t = f64::from(f) / fps;
        let want = amount(laid_on, f64::from(f)) * 36.0;
        let apart = degrees_apart(&sample(&a, t), &sample(&b, t));
        assert!(
            (want - 18.0).abs() < 1e-9 && (apart - want).abs() < 1.5,
            "frame {f}: the upper arm is {apart} degrees from the clip, the grip adds {want}"
        );
    }

    // The rig has no thumb: the grip leaves it, and the review says so.
    let [one, two] = &result.grips[..] else {
        panic!("{:?}", result.grips)
    };
    assert_eq!((one.bones, one.posed), (5, 4));
    assert_eq!(one.missing, ["rightThumbDistal"]);
    assert_eq!(one.frames, Some([3, 27]));
    assert_eq!((two.bones, two.posed), (1, 1));
    let review = review_of(&result);
    let line = |id: &str| review.lines.iter().find(|l| l.id == id).expect("line");
    assert_eq!(line("grip-l-2").status, RecipeReviewStatus::Warn);
    assert!(
        line("grip-l-2")
            .note
            .as_deref()
            .is_some_and(|n| n.contains("no bone for 1 of its finger bones")),
        "{:?}",
        line("grip-l-2")
    );
    assert_eq!(line("grip-l-3").status, RecipeReviewStatus::Pass);
}

#[test]
fn a_pin_pose_keys_and_a_grip_bake_in_one_recipe_each_doing_its_own() {
    let Some(s) = setup() else { return };
    let bones = ["chest", "rightHand", "rightIndexProximal"];
    let plain = project(&s, SLIDE, Vec::new());
    let (base, base_glb, nodes) = bake_of(&s, &plain, &bones);

    let pin = RecipeLayer::new(
        "l-2",
        LayerKind::Pin,
        "leftFoot",
        LayerSettings {
            start: 0.1,
            end: 0.9,
            ..LayerSettings::default()
        },
    );
    let key = |frame, turn| PoseKey {
        clip: 0,
        frame,
        ease: KeyEase::Linear,
        pose: [("chest".to_string(), turn)].into(),
    };
    let mut keys = empty_pose_layer("l-3", LayerMode::Add);
    keys.keys = vec![key(5, [0.0, 0.0, 0.0]), key(15, [0.0, 20.0, 0.0])];
    let held = grip(
        "l-4",
        LayerMode::Replace,
        (0.0, 1.0),
        (0, 0),
        &[
            ("rightHand", [0.0, 30.0, 0.0]),
            ("rightIndexProximal", [0.0, 0.0, 45.0]),
        ],
    );
    let p = project(&s, SLIDE, vec![pin, keys, held]);
    let (result, glb, _) = bake_of(&s, &p, &bones);
    assert_eq!(result.frames, base.frames);

    // The pin still holds the foot the clip slides, under the other two.
    let [foot] = &result.pins[..] else {
        panic!("{:?}", result.pins)
    };
    assert!(
        foot.drift_cm.expect("drift") < PIN_DRIFT_LIMIT_CM,
        "{foot:?}"
    );
    assert!(foot.base_drift_cm.expect("base drift") > 2.0, "{foot:?}");

    // The keys turn the chest on top of the clip, none at frame 5, 20 by 15.
    let fps = fps_of(&glb, &result);
    let apart = |node: usize, f: u32| {
        let t = f64::from(f) / fps;
        degrees_apart(
            &sample(&turns(&glb, &result, node), t),
            &sample(&turns(&base_glb, &base, node), t),
        )
    };
    assert!(apart(nodes[0], 5) < 1.5, "{}", apart(nodes[0], 5));
    for f in [15, 22, result.frames] {
        let chest = apart(nodes[0], f);
        assert!((chest - 20.0).abs() < 1.5, "frame {f}: the chest {chest}");
    }

    // The grip holds the hand and its finger from their rest on every frame.
    for (i, turn) in [(1, 30.0), (2, 45.0)] {
        let keys = turns(&glb, &result, nodes[i]);
        let rest = rest_of(&glb, nodes[i]);
        for f in [0, 15, result.frames] {
            let from_rest = degrees_apart(&sample(&keys, f64::from(f) / fps), &rest);
            assert!(
                (from_rest - turn).abs() < 1.5,
                "frame {f}: {} is {from_rest} degrees from its rest",
                bones[i]
            );
        }
    }
    let review = review_of(&result);
    let ids: Vec<&str> = review.lines.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["joints", "pin-l-2", "grip-l-4", "feet", "stretch"]);
    for line in &review.lines[..3] {
        assert_eq!(line.status, RecipeReviewStatus::Pass, "{line:?}");
    }
}
