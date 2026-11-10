//! Grips: the cases the web app's copy of the rules reads too
//! (`fixtures/grips.json`): which hands a recipe takes from a grip, a grip
//! change on the timeline and a grip made from pose keys. Run with
//! `MAQUETTE_WRITE_GRIPS=1` to write the fixture again after a deliberate
//! change to the rules. The builders here are shared with
//! `tests_grips_apply` and `tests_grips_file`.

use serde_json::{Value, json};

use super::grips::{
    GRIP_VERSION, GripFile, GripHands, GripHolds, GripSide, amount, grip_of, hands_words,
    keyed_pose, layer_of, mirror_pose, pose_for, spans, weapon_group,
};
use super::tests_v2::{clip, key, pose_layer, slash};
use super::timeline::timeline;
use super::*;

const FIXTURE: &str = "src/recipes/fixtures/grips.json";

pub(super) fn pose(bones: &[(&str, [f32; 3])]) -> BonePose {
    bones.iter().map(|(b, t)| ((*b).to_string(), *t)).collect()
}

pub(super) fn grip(
    name: &str,
    weapon: &str,
    hands: (GripSide, Option<GripSide>),
    pose: BonePose,
) -> GripFile {
    GripFile {
        version: GRIP_VERSION,
        name: name.into(),
        revision: 1,
        weapon: weapon.into(),
        hands: GripHolds {
            holds: hands.0,
            supports: hands.1,
        },
        pose,
    }
}

pub(super) fn longsword() -> GripFile {
    grip(
        "Longsword, two hands",
        "longsword",
        (GripSide::Right, Some(GripSide::Left)),
        pose(&[
            ("rightHand", [4.0, 0.0, -8.0]),
            ("rightLowerArm", [0.0, 12.0, 0.0]),
            ("rightIndexProximal", [0.0, 0.0, 62.0]),
            ("rightThumbDistal", [10.0, -20.0, 5.0]),
            ("leftHand", [-2.0, 6.0, 9.0]),
            ("leftIndexProximal", [0.0, 0.0, -58.0]),
        ]),
    )
}

pub(super) fn dagger() -> GripFile {
    grip(
        "Dagger, reverse",
        "dagger",
        (GripSide::Right, None),
        pose(&[
            ("rightHand", [0.0, 30.0, 0.0]),
            ("rightMiddleIntermediate", [0.0, 0.0, 80.0]),
        ]),
    )
}

pub(super) fn relaxed() -> GripFile {
    grip(
        "Empty hand, relaxed",
        "empty",
        (GripSide::Left, None),
        pose(&[
            ("leftHand", [0.0, 0.0, 0.0]),
            ("leftLittleDistal", [3.0, 4.0, -18.5]),
        ]),
    )
}

pub(super) fn link(revision: u32) -> GripFrom {
    GripFrom {
        entry: "longsword-two-hands".into(),
        uid: Some("0a1b2c3d4e5f6071".into()),
        revision,
    }
}

pub(super) fn on(name: &str, base: RecipeBase) -> RecipeFile {
    let mut r = RecipeFile::new(name, base);
    r.created = "2026-11-10T00:00:00.000Z".into();
    r.edited = r.created.clone();
    r
}

pub(super) fn held(
    id: &str,
    bone: &str,
    range: (f32, f32),
    eases: (Option<u32>, Option<u32>),
) -> RecipeLayer {
    let mut layer = layer_of(id, &dagger(), link(1), None);
    layer.bone = bone.into();
    layer.settings.start = range.0;
    layer.settings.end = range.1;
    layer.settings.ease_in = eases.0;
    layer.settings.ease_out = eases.1;
    layer
}

/// Recipes with grip changes on a clip played whole, on joined clips with a
/// freeze, and on the rest pose with eases that overlap.
pub(super) fn span_cases() -> Vec<RecipeFile> {
    let mut plain = on("Plain grip", clip("Standing Idle", 60));
    plain.layers = vec![layer_of("l-1", &longsword(), link(2), None)];

    let mut joined = slash();
    joined.name = "Slash, gripped".into();
    joined.created = "2026-11-10T00:00:00.000Z".into();
    joined.edited = joined.created.clone();
    let mut half = held("l-5", "rightHand", (0.3, 0.8), (Some(0), Some(6)));
    half.settings.weight = 0.5;
    half.mode = Some(LayerMode::Add);
    joined.layers = vec![half, held("l-6", "leftHand", (0.5, 1.0), (None, None))];

    let mut rest = on("Rest, short grip", RecipeBase::rest_pose(40));
    rest.time = vec![TimeEdit::hold(TimeKind::Freeze, 0, 20, 10)];
    rest.layers = vec![
        held("l-1", "rightHand", (0.4, 0.6), (Some(12), Some(12))),
        held("l-2", "leftHand", (0.0, 0.05), (Some(5), Some(0))),
    ];
    vec![plain, normalized(&joined), rest]
}

/// A recipe whose pose keys turn both hands, the chest and a finger.
pub(super) fn keyed_recipe() -> RecipeFile {
    let mut r = on("Keyed hands", clip("Sword Idle", 90));
    r.layers = vec![pose_layer(
        "l-2",
        vec![
            key(
                0,
                10,
                &[
                    ("chest", [0.0, -18.0, 0.0]),
                    ("rightHand", [0.0, 0.0, 0.0]),
                    ("rightIndexProximal", [0.0, 0.0, 10.0]),
                    ("leftHand", [5.0, 0.0, 0.0]),
                ],
            ),
            key(
                0,
                40,
                &[
                    ("rightHand", [12.0, 0.0, -9.0]),
                    ("rightIndexProximal", [0.0, 0.0, 70.0]),
                    ("leftHand", [5.0, 20.0, 0.0]),
                ],
            ),
        ],
    )];
    normalized(&r)
}

fn rounded(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

fn value(of: &impl serde::Serialize) -> Value {
    serde_json::from_str(&serde_json::to_string(of).expect("json")).expect("value")
}

fn grip_case(grip: &GripFile) -> Value {
    let applies: Vec<Value> = [
        None,
        Some(GripHands::Both),
        Some(GripHands::Right),
        Some(GripHands::Left),
    ]
    .into_iter()
    .map(|hands| {
        let (holds, pose) = pose_for(grip, hands);
        json!({ "hands": hands, "bone": holds.hand(), "pose": value(&pose) })
    })
    .collect();
    json!({
        "grip": value(grip),
        "words": hands_words(grip),
        "group": weapon_group(&grip.weapon),
        "mirrored": value(&mirror_pose(&grip.pose)),
        "applies": applies,
    })
}

fn span_case(recipe: &RecipeFile) -> Value {
    let t = timeline(recipe);
    let found = spans(recipe, &t);
    let amounts: Vec<Value> = found
        .iter()
        .flat_map(|g| {
            let mut at = vec![
                g.first.saturating_sub(1),
                g.first,
                g.first + 1,
                g.first + g.ease_in / 2,
                g.first + g.ease_in,
                u32::midpoint(g.first, g.last),
                g.last.saturating_sub(g.ease_out),
                g.last.saturating_sub(1),
                g.last,
                g.last + 1,
            ];
            at.sort_unstable();
            at.dedup();
            at.into_iter().map(move |f| {
                json!({ "layer": g.layer, "frame": f, "amount": rounded(amount(g, f64::from(f))) })
            })
        })
        .collect();
    json!({ "name": recipe.name, "recipe": value(recipe), "spans": found, "amounts": amounts })
}

fn keyed_cases() -> Vec<Value> {
    let recipe = keyed_recipe();
    let layer = &recipe.layers[0];
    let cases: [(u32, Option<Vec<String>>); 4] = [
        (10, None),
        (25, None),
        (
            40,
            Some(vec!["rightHand".into(), "rightIndexProximal".into()]),
        ),
        (90, Some(vec!["leftHand".into()])),
    ];
    cases
        .into_iter()
        .map(|(frame, keep)| {
            let pose = keyed_pose(&recipe, layer, frame, keep.as_deref()).expect("keyed");
            json!({ "recipe": value(&recipe), "layer": layer.id, "frame": frame, "keep": keep, "pose": value(&pose) })
        })
        .collect()
}

fn made_cases() -> Vec<Value> {
    let both = longsword().pose;
    let right = dagger().pose;
    let cases: [(&BonePose, Option<GripHands>, Option<GripSide>); 7] = [
        (&both, None, None),
        (&both, Some(GripHands::Both), Some(GripSide::Left)),
        (&both, Some(GripHands::Right), None),
        (&both, Some(GripHands::Left), None),
        (&right, None, None),
        (&right, Some(GripHands::Both), None),
        (&right, Some(GripHands::Left), None),
    ];
    cases
        .into_iter()
        .map(|(pose, hands, holds)| {
            let made = grip_of("Made", "sword", hands, holds, pose).ok();
            json!({
                "pose": value(pose), "hands": hands, "holds": holds,
                "made": made.map(|g| json!({ "hands": g.hands, "pose": value(&g.pose) })),
            })
        })
        .collect()
}

#[test]
fn grips_match_the_fixture_the_web_app_reads_too() {
    for grip in [longsword(), dagger(), relaxed()] {
        grips::validate(&grip).unwrap_or_else(|e| panic!("{}: {e}", grip.name));
    }
    for recipe in span_cases() {
        validate(&recipe).unwrap_or_else(|e| panic!("{}: {e}", recipe.name));
    }
    let files: Vec<Value> = [longsword(), dagger(), relaxed()]
        .iter()
        .map(grip_case)
        .collect();
    let on_timelines: Vec<Value> = span_cases().iter().map(span_case).collect();
    let want = json!({
        "grips": files,
        "spans": on_timelines,
        "keyed": keyed_cases(),
        "made": made_cases(),
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    if std::env::var_os("MAQUETTE_WRITE_GRIPS").is_some() {
        let text = serde_json::to_string_pretty(&want).expect("json");
        std::fs::write(&path, format!("{text}\n")).expect("write the fixture");
    }
    let have: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("fixture")).expect("json");
    assert_eq!(have, want, "the grip rules moved: write the fixture again");
}
