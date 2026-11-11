//! Pose keys in a real, headless Blender: the fixture model takes the
//! fixture clip with keys added to it and in its place, keys on a joined
//! clip, and keys on the rest pose. At every frame of each bake a keyed
//! bone is turned as far from the clip (added) or from its rest (replaced)
//! as the shared pose key math says (`recipes::pose_keys`), which the rough
//! preview follows too. Skipped where no Blender is installed.

use std::path::Path;

use super::tests::project_on;
use super::tests_blender_joined::{Glb, bake_of, degrees_apart, fps_of, project, sample};
use super::*;
use crate::animations::tests_blender::{fixture, setup};
use crate::recipes::pose_keys::pose_at;
use crate::recipes::{
    BlendWindow, KeyEase, LayerMode, PoseKey, RecipeChange, RecipeFile, RecipeLayer,
    empty_pose_layer,
};

fn key(clip: u32, frame: u32, ease: KeyEase, bone: &str, turn: [f32; 3]) -> PoseKey {
    PoseKey {
        clip,
        frame,
        ease,
        pose: [(bone.to_string(), turn)].into(),
    }
}

fn keys(id: &str, mode: LayerMode, keys: Vec<PoseKey>) -> RecipeLayer {
    let mut layer = empty_pose_layer(id, mode);
    layer.keys = keys;
    layer
}

/// How far the keys of `layer` turn `bone` at frame `frame`, degrees: the
/// size of a turn about one axis.
fn keyed(recipe: &RecipeFile, frame: u32, layer: &str, bone: &str) -> f64 {
    pose_at(recipe, frame)
        .turns
        .iter()
        .find(|t| t.layer == layer && t.bone == bone)
        .map_or(0.0, |t| t.turn.iter().map(|v| v * v).sum::<f64>().sqrt())
}

pub(super) fn rest_of(glb: &Glb, node: usize) -> Vec<f64> {
    glb.json["nodes"][node]["rotation"].as_array().map_or_else(
        || vec![0.0, 0.0, 0.0, 1.0],
        |a| a.iter().filter_map(serde_json::Value::as_f64).collect(),
    )
}

/// Every frame of `ours`: `node`'s local turn is as far from `theirs`
/// (the same recipe without the keys) as the keys turn `bone`.
fn adds_the_keys(
    recipe: &RecipeFile,
    ours: (&Glb, &RecipeBakeResult),
    theirs: (&Glb, &RecipeBakeResult),
    (node, layer, bone): (usize, &str, &str),
) {
    assert_eq!(ours.1.frames, theirs.1.frames);
    let fps = fps_of(ours.0, ours.1);
    let a = ours.0.keys(&ours.1.clip, node, "rotation").expect("keys");
    let b = theirs
        .0
        .keys(&theirs.1.clip, node, "rotation")
        .expect("keys");
    let mut turned: f64 = 0.0;
    for f in 0..=ours.1.frames {
        let t = f64::from(f) / fps;
        let want = keyed(recipe, f, layer, bone);
        let apart = degrees_apart(&sample(&a, t), &sample(&b, t));
        assert!(
            (apart - want).abs() < 1.5,
            "frame {f}: {bone} {apart} degrees from the clip, the keys say {want}"
        );
        turned = turned.max(want);
    }
    assert!(turned > 20.0, "the keys turn {bone} {turned} at most");
}

/// Every frame of `ours`: `node`'s local turn is as far from its rest as
/// the keys turn `bone`, since replace keys turn from the rest pose.
fn replaces_with_the_keys(
    recipe: &RecipeFile,
    ours: (&Glb, &RecipeBakeResult),
    (node, layer, bone): (usize, &str, &str),
    from: u32,
) {
    let fps = fps_of(ours.0, ours.1);
    let a = ours.0.keys(&ours.1.clip, node, "rotation").expect("keys");
    let rest = rest_of(ours.0, node);
    for f in from..=ours.1.frames {
        let want = keyed(recipe, f, layer, bone);
        let apart = degrees_apart(&sample(&a, f64::from(f) / fps), &rest);
        assert!(
            (apart - want).abs() < 1.5,
            "frame {f}: {bone} {apart} degrees from its rest, the keys say {want}"
        );
    }
}

#[test]
fn keys_add_to_the_clip_or_replace_it_as_the_shared_math_blends_them() {
    let Some(s) = setup() else { return };
    let bones = ["rightLowerArm", "leftUpperArm"];
    let plain = project(&s, |_| RecipeChange::default());
    let (base, base_glb, nodes) = bake_of(&s, &plain, &bones);
    let added = keys(
        "l-k1",
        LayerMode::Add,
        vec![
            key(0, 4, KeyEase::Smooth, "rightLowerArm", [0.0, 0.0, 0.0]),
            key(0, 15, KeyEase::Smooth, "head", [0.0, 80.0, 0.0]),
            key(0, 20, KeyEase::Linear, "rightLowerArm", [0.0, 40.0, 0.0]),
        ],
    );
    let replaced = keys(
        "l-k2",
        LayerMode::Replace,
        vec![
            key(0, 10, KeyEase::Smooth, "leftUpperArm", [0.0, 0.0, 0.0]),
            key(0, 26, KeyEase::Out, "leftUpperArm", [0.0, 0.0, 50.0]),
        ],
    );
    let p = project(&s, |r| RecipeChange {
        layers: Some([r.layers.clone(), vec![added, replaced]].concat()),
        ..RecipeChange::default()
    });
    let (result, glb, _) = bake_of(&s, &p, &bones);
    let recipe = &p.recipe.recipe;
    adds_the_keys(
        recipe,
        (&glb, &result),
        (&base_glb, &base),
        (nodes[0], "l-k1", "rightLowerArm"),
    );
    replaces_with_the_keys(
        recipe,
        (&glb, &result),
        (nodes[1], "l-k2", "leftUpperArm"),
        0,
    );

    // The head's 80 degree twist is past its 70: the review warns, and says
    // where.
    let joints = result.joints.as_ref().expect("judged");
    assert_eq!(joints.judged, 5);
    assert_eq!(joints.hits.len(), 1, "{joints:?}");
    assert_eq!(joints.hits[0].hit.bone, "head");
    assert_eq!(joints.hits[0].bake_frame, Some(15));
    let review = review_of(&result);
    let line = review
        .lines
        .iter()
        .find(|l| l.id == "joints")
        .expect("line");
    assert_eq!(line.status, RecipeReviewStatus::Warn, "{line:?}");
    assert_eq!(line.max, Some(70.0));
}

#[test]
fn a_key_on_a_joined_clip_rides_with_it() {
    let Some(s) = setup() else { return };
    let bones = ["rightLowerArm"];
    let join = |r: &RecipeFile| {
        let mut first = r.base.clone();
        first.trim = Some([0, 20]);
        let mut second = r.base.clone();
        second.trim = Some([5, 25]);
        second.blend = Some(BlendWindow {
            frames: 4,
            ..BlendWindow::default()
        });
        RecipeChange {
            clips: Some(vec![first, second]),
            kept: Some(vec![Some(0), Some(0)]),
            ..RecipeChange::default()
        }
    };
    let joined = project(&s, join);
    let (base, base_glb, nodes) = bake_of(&s, &joined, &bones);
    let keyed_on = project(&s, |r| RecipeChange {
        layers: Some(
            [
                r.layers.clone(),
                vec![keys(
                    "l-k1",
                    LayerMode::Add,
                    vec![
                        key(0, 10, KeyEase::Linear, "rightLowerArm", [0.0, 0.0, 0.0]),
                        key(1, 15, KeyEase::Linear, "rightLowerArm", [0.0, 35.0, 0.0]),
                    ],
                )],
            ]
            .concat(),
        ),
        ..join(r)
    });
    let (result, glb, _) = bake_of(&s, &keyed_on, &bones);
    assert_eq!(result.frames, 36);
    adds_the_keys(
        &keyed_on.recipe.recipe,
        (&glb, &result),
        (&base_glb, &base),
        (nodes[0], "l-k1", "rightLowerArm"),
    );
}

#[test]
fn keys_on_the_rest_pose_use_its_own_frames() {
    let Some(s) = setup() else { return };
    let file = s.files.join("model.glb");
    fixture(&s.blender, &s.root.path().join("scripts"), "model", &file);
    let mut p = project_on(
        s.root.path(),
        None,
        std::fs::read(Path::new(&file)).expect("model"),
    );
    p.change(
        vec![keys(
            "l-1",
            LayerMode::Replace,
            vec![
                key(0, 0, KeyEase::Smooth, "leftUpperArm", [0.0, 0.0, -40.0]),
                key(0, 40, KeyEase::Smooth, "leftUpperArm", [0.0, 0.0, 20.0]),
            ],
        )],
        Vec::new(),
    );
    let (result, glb, nodes) = bake_of(&s, &p, &["leftUpperArm"]);
    assert_eq!(result.frames, 60);
    replaces_with_the_keys(
        &p.recipe.recipe,
        (&glb, &result),
        (nodes[0], "l-1", "leftUpperArm"),
        0,
    );
    let joints = result.joints.expect("judged");
    assert_eq!((joints.judged, joints.hits.len()), (2, 0));
}
