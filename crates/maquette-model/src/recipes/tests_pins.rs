//! Pins on the timeline: the frames each holds on, how it eases, what of a
//! foot stays flat, checked against `fixtures/pins.json`, which the web
//! app's copy of the rules reads too. Run with `MAQUETTE_WRITE_PINS=1` to
//! write the fixture again after a deliberate change to the rules.

use serde_json::{Value, json};

use super::pins::{PIN_EASE_FRAMES, PinSpan, amount, flat_of, held, spans};
use super::tests_v2::{clip, pin, slash};
use super::timeline::timeline;
use super::*;

const FIXTURE: &str = "src/recipes/fixtures/pins.json";

fn on(name: &str, base: RecipeBase) -> RecipeFile {
    let mut r = RecipeFile::new(name, base);
    r.created = "2026-11-09T00:00:00.000Z".into();
    r.edited = r.created.clone();
    r
}

fn ranged(id: &str, bone: &str, start: f32, end: f32) -> RecipeLayer {
    let mut l = pin(id, bone);
    l.settings.start = start;
    l.settings.end = end;
    l
}

fn cases() -> Vec<RecipeFile> {
    let mut plain = on("Plain foot", clip("Standing Idle", 60));
    plain.layers = vec![ranged("l-1", "leftFoot", 0.2, 0.7)];

    let mut joined = slash();
    joined.name = "Slash, both pins".into();
    joined.created = "2026-11-09T00:00:00.000Z".into();
    joined.edited = joined.created.clone();
    joined.blanks = vec![RecipeBlank {
        id: "shaft".into(),
        label: "The shaft the hand stays on".into(),
        kind: BlankKind::Point,
        prop: None,
        point: Some([0.2, 1.1, 0.3]),
    }];
    let mut hand = ranged("l-5", "rightHand", 0.5, 0.9);
    hand.settings.target = Some("shaft".into());
    hand.settings.ease_in = Some(0);
    hand.settings.ease_out = Some(6);
    hand.settings.weight = 0.5;
    let mut heel = ranged("l-6", "rightFoot", 0.6, 1.0);
    heel.settings.flat = Some(PinFlat::Heel);
    joined.layers = vec![joined.layers[0].clone(), hand, heel];

    let mut rest = on("Rest freeze", RecipeBase::rest_pose(60));
    rest.time = vec![TimeEdit::hold(TimeKind::Freeze, 0, 30, 12)];
    let mut long = ranged("l-1", "leftFoot", 0.4, 0.6);
    long.settings.ease_in = Some(10);
    long.settings.ease_out = Some(10);
    rest.layers = vec![long];

    let mut tight = on("Eases overlap", clip("Standing Idle", 60));
    let mut late = ranged("l-1", "leftFoot", 0.0, 0.034);
    late.settings.ease_in = Some(0);
    late.settings.ease_out = Some(5);
    let mut early = ranged("l-2", "rightFoot", 0.5, 0.534);
    early.settings.ease_in = Some(5);
    early.settings.ease_out = Some(0);
    tight.layers = vec![late, early];
    vec![plain, joined, rest, tight]
}

fn rounded(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

fn probes(pin: &PinSpan) -> Vec<u32> {
    let mut at = vec![
        pin.first.saturating_sub(1),
        pin.first,
        pin.first + 1,
        pin.first + pin.ease_in / 2,
        pin.first + pin.ease_in,
        u32::midpoint(pin.first, pin.last),
        pin.last.saturating_sub(pin.ease_out),
        pin.last.saturating_sub(1),
        pin.last,
        pin.last + 1,
    ];
    at.sort_unstable();
    at.dedup();
    at
}

fn expected(recipe: &RecipeFile) -> Value {
    let t = timeline(recipe);
    let pins = spans(recipe, &t);
    let amounts: Vec<Value> = pins
        .iter()
        .flat_map(|p| {
            probes(p).into_iter().map(move |f| {
                json!({ "layer": p.layer, "frame": f, "amount": rounded(amount(p, f64::from(f))) })
            })
        })
        .collect();
    json!({
        "name": recipe.name,
        "recipe": serde_json::from_str::<Value>(&serde_json::to_string(recipe).expect("recipe"))
            .expect("json"),
        "spans": pins,
        "held": pins.iter().map(held).collect::<Vec<_>>(),
        "amounts": amounts,
    })
}

#[test]
fn pins_match_the_fixture_the_web_app_reads_too() {
    for case in cases() {
        validate(&case).unwrap_or_else(|e| panic!("{}: {e}", case.name));
    }
    let want = json!({ "cases": cases().iter().map(expected).collect::<Vec<_>>() });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    if std::env::var_os("MAQUETTE_WRITE_PINS").is_some() {
        let text = serde_json::to_string_pretty(&want).expect("json");
        std::fs::write(&path, format!("{text}\n")).expect("write the fixture");
    }
    let have: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("fixture")).expect("json");
    assert_eq!(have, want, "the pin rules moved: write the fixture again");
}

#[test]
fn a_pin_holds_from_where_its_start_falls_to_where_its_end_falls() {
    let recipe = &cases()[0];
    let t = timeline(recipe);
    let p = &spans(recipe, &t)[0];
    // 60 frames: 0.2 lands on frame 12, 0.7 on frame 42.
    assert_eq!((p.first, p.last), (12, 42));
    assert_eq!((p.ease_in, p.ease_out), (PIN_EASE_FRAMES, PIN_EASE_FRAMES));
    assert!(amount(p, 11.0) == 0.0 && amount(p, 43.0) == 0.0);
    assert!(amount(p, 12.0) == 0.0, "it eases in from where the bone is");
    assert!((amount(p, 15.0) - 1.0).abs() < 1e-9);
    assert!((amount(p, 27.0) - 1.0).abs() < 1e-9);
    assert!(amount(p, 13.0) < amount(p, 14.0), "it eases in");
    assert_eq!(held(p), Some([15, 39]));
}

#[test]
fn a_pin_whose_eases_overlap_holds_no_frame_whole() {
    let recipe = &cases()[3];
    let t = timeline(recipe);
    let pins = spans(recipe, &t);
    assert_eq!(pins.len(), 2);
    for p in &pins {
        assert!(p.last - p.first < 5, "frames {} to {}", p.first, p.last);
        assert_eq!(held(p), None, "{}", p.layer);
    }
}

#[test]
fn a_freeze_inside_a_pin_stays_pinned() {
    let recipe = &cases()[2];
    let t = timeline(recipe);
    let p = &spans(recipe, &t)[0];
    // The freeze's 12 added frames sit on clip frame 30 (share 0.5), within 0.4 to 0.6.
    let frozen: Vec<usize> = t
        .keys
        .iter()
        .enumerate()
        .filter(|(_, k)| k.from.hold.is_some())
        .map(|(i, _)| i)
        .collect();
    assert!(!frozen.is_empty());
    for f in frozen {
        let f = u32::try_from(f).expect("frame");
        assert!(p.first <= f && f <= p.last, "frame {f} outside {p:?}");
    }
}

#[test]
fn a_foot_keeps_toe_and_heel_flat_unless_told_and_a_hand_keeps_its_turn() {
    let foot = pin("l-1", "leftFoot");
    assert_eq!(flat_of(&foot), Some(PinFlat::ToeAndHeel));
    let mut heel = pin("l-1", "rightFoot");
    heel.settings.flat = Some(PinFlat::Heel);
    assert_eq!(flat_of(&heel), Some(PinFlat::Heel));
    assert_eq!(flat_of(&pin("l-1", "leftHand")), None);
}

#[test]
fn an_ease_of_none_holds_at_once_and_weight_scales_it() {
    let recipe = &cases()[1];
    let t = timeline(recipe);
    let pins = spans(recipe, &t);
    let hand = pins.iter().find(|p| p.layer == "l-5").expect("hand");
    assert_eq!(hand.target.as_deref(), Some("shaft"));
    assert!((amount(hand, f64::from(hand.first)) - 0.5).abs() < 1e-9);
    assert_eq!(hand.flat, None);
    let heel = pins.iter().find(|p| p.layer == "l-6").expect("heel");
    assert_eq!(
        heel.last, t.frames,
        "a pin to the end holds to the last frame"
    );
    assert_eq!(heel.flat, Some(PinFlat::Heel));
}

#[test]
fn a_pin_needs_version_2() {
    let mut recipe = on("Pinned", clip("Standing Idle", 60));
    recipe.layers = vec![pin("l-1", "leftFoot")];
    let recipe = normalized(&recipe);
    assert_eq!(recipe.version, RECIPE_VERSION, "a pin needs version 2");
}

#[test]
fn a_write_refuses_two_ik_changes_holding_one_bone_at_once() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, _) = create(dir.path(), "Pinned", clip("Standing Idle", 60)).expect("create");
    let layers = |layers: Vec<RecipeLayer>| RecipeChange {
        layers: Some(layers),
        blanks: Some(vec![RecipeBlank {
            id: "shaft".into(),
            label: "The shaft".into(),
            kind: BlankKind::Point,
            prop: None,
            point: Some([0.2, 1.1, 0.3]),
        }]),
        ..RecipeChange::default()
    };
    let refused = |l: Vec<RecipeLayer>| {
        update(dir.path(), &tests_v2::project(), &id, layers(l))
            .expect_err("overlap")
            .to_string()
    };
    let err = refused(vec![
        ranged("l-1", "leftFoot", 0.1, 0.5),
        ranged("l-2", "leftFoot", 0.4, 0.8),
    ]);
    assert!(
        err.contains("pin l-1 and the pin l-2 both hold leftFoot"),
        "{err}"
    );
    let mut reach = ranged("l-2", "rightHand", 0.3, 0.6);
    reach.kind = LayerKind::Reach;
    reach.settings.target = Some("shaft".into());
    let err = refused(vec![ranged("l-1", "rightHand", 0.5, 0.9), reach.clone()]);
    assert!(err.contains("the reach l-2"), "{err}");

    // Apart in time, on two bones, or two reaches as before: all fine.
    let apart = vec![
        ranged("l-1", "leftFoot", 0.1, 0.4),
        ranged("l-2", "leftFoot", 0.4, 0.8),
        ranged("l-3", "rightFoot", 0.1, 0.8),
        RecipeLayer {
            id: "l-4".into(),
            ..reach.clone()
        },
        RecipeLayer {
            id: "l-5".into(),
            ..reach
        },
    ];
    update(dir.path(), &tests_v2::project(), &id, layers(apart)).expect("apart");
}
