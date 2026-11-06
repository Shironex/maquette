//! The recipe timeline: frame counts and which clip frame each frame shows,
//! checked against `fixtures/timeline.json`, which the web app's copy of
//! the rules reads too. Run with `MAQUETTE_WRITE_TIMELINE=1` to write the
//! fixture again after a deliberate change to the rules.

use serde_json::{Value, json};

use super::tests_v2::clip;
use super::timeline::{Timeline, plain, timeline};
use super::*;

const FIXTURE: &str = "src/recipes/fixtures/timeline.json";
/// The timeline frames each case is probed at, besides its first and last.
const PROBES: usize = 12;

fn blend(frames: u32, curve: BlendCurve, line_up: LineUp) -> Option<BlendWindow> {
    Some(BlendWindow {
        frames,
        curve,
        line_up,
    })
}

fn trimmed(mut c: RecipeBase, from: u32, to: u32) -> RecipeBase {
    c.trim = Some([from, to]);
    c
}

fn on(name: &str, base: RecipeBase, joined: Vec<RecipeBase>) -> RecipeFile {
    let mut r = RecipeFile::new(name, base);
    r.created = "2026-11-06T00:00:00.000Z".into();
    r.edited = r.created.clone();
    r.joined = joined;
    r
}

/// The slash into thrust of the design: three clips, a retime and a freeze.
fn slash_into_thrust() -> RecipeFile {
    let mut thrust = trimmed(clip("Great Sword Thrust", 64), 4, 60);
    thrust.blend = blend(8, BlendCurve::Smooth, LineUp::HipsAndFacing);
    let mut idle = trimmed(clip("Sword Idle", 90), 0, 40);
    idle.blend = blend(6, BlendCurve::Smooth, LineUp::HipsAndFacing);
    let mut r = on(
        "Slash into thrust",
        trimmed(clip("Great Sword Slash", 72), 6, 58),
        vec![thrust, idle],
    );
    r.time = vec![
        TimeEdit::retime(0, 20, 34, 1.6),
        TimeEdit::hold(TimeKind::Freeze, 1, 56, 18),
    ];
    r
}

fn cases() -> Vec<RecipeFile> {
    let plain_clip = on("Plain", clip("Standing Idle", 60), Vec::new());
    let mut mirrored = slash_into_thrust();
    mirrored.name = "Slash into thrust, left".into();
    mirrored.mirror = true;
    let mut eased = on("Eased", clip("Charge", 80), Vec::new());
    let mut retime = TimeEdit::retime(0, 10, 40, 0.5);
    retime.ease = Some(6);
    let mut hold = TimeEdit::hold(TimeKind::HoldOver, 0, 50, 20);
    hold.ease_in = Some(4);
    hold.ease_out = Some(5);
    hold.breathe = Some(0.5);
    let mut freeze = TimeEdit::hold(TimeKind::Freeze, 0, 0, 6);
    freeze.ease_out = Some(3);
    eased.time = vec![retime, hold, freeze];
    let mut slow = clip("Slow Walk", 48);
    slow.length = 2.0;
    slow.blend = blend(0, BlendCurve::Linear, LineUp::Hips);
    let mut rates = on("Two rates", clip("Run", 72), vec![slow]);
    rates.time = vec![TimeEdit::retime(1, 6, 30, 2.5)];
    let mut rest = on("Rest hold", RecipeBase::rest_pose(60), Vec::new());
    rest.time = vec![TimeEdit::hold(TimeKind::Freeze, 0, 30, 12)];
    vec![
        plain_clip,
        slash_into_thrust(),
        mirrored,
        eased,
        rates,
        rest,
    ]
}

fn rounded(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

fn probe(t: &Timeline, at: usize) -> Value {
    let k = &t.keys[at];
    let side = |s: &super::timeline::ClipSample| {
        json!({
            "clip": s.clip,
            "at": rounded(s.at),
            "hold": s.hold.as_ref().map(|h| json!({
                "at": h.at, "weight": rounded(h.weight), "breathe": rounded(h.breathe)
            })),
        })
    };
    json!({
        "t": at,
        "from": side(&k.from),
        "to": k.to.as_ref().map(side),
        "blend": rounded(k.blend),
        "share": rounded(k.share),
    })
}

fn expected(recipe: &RecipeFile) -> Value {
    let t = timeline(recipe);
    let n = t.keys.len();
    let mut at: Vec<usize> = (0..PROBES).map(|i| i * (n - 1) / (PROBES - 1)).collect();
    // Every seam's window, start and middle.
    for s in &t.seams {
        at.push(s.frame as usize);
        at.push((s.frame + s.frames / 2) as usize);
    }
    at.sort_unstable();
    at.dedup();
    json!({
        "name": recipe.name,
        "recipe": recipe,
        "fps": t.fps,
        "frames": t.frames,
        "seams": t.seams,
        "clips": t.clips,
        "probes": at.into_iter().map(|i| probe(&t, i)).collect::<Vec<_>>(),
    })
}

#[test]
fn the_timeline_matches_the_fixture_the_web_app_reads_too() {
    let want = json!({ "cases": cases().iter().map(expected).collect::<Vec<_>>() });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    if std::env::var_os("MAQUETTE_WRITE_TIMELINE").is_some() {
        let text = serde_json::to_string_pretty(&want).expect("json");
        std::fs::write(&path, format!("{text}\n")).expect("write the fixture");
    }
    let have: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("fixture")).expect("json");
    assert_eq!(
        have, want,
        "the timeline rules moved: write the fixture again"
    );
}

#[test]
fn the_design_combo_runs_147_frames() {
    let t = timeline(&slash_into_thrust());
    // 52 + 56 + 40 kept, less blends of 8 and 6, less 5 for the retime,
    // plus the freeze's 18.
    assert_eq!(t.frames, 147);
    assert_eq!(t.keys.len(), 148);
    assert_eq!(t.seams[0].frames, 8);
    let frozen = t
        .keys
        .iter()
        .filter(|k| k.from.hold.as_ref().is_some_and(|h| h.weight >= 1.0) && k.from.clip == 1)
        .count();
    assert_eq!(frozen, 19, "the freeze frame and its 18 added frames");
}

#[test]
fn one_clip_with_nothing_new_plays_every_frame_once() {
    let r = on("Plain", clip("Standing Idle", 60), Vec::new());
    assert!(plain(&r));
    let t = timeline(&r);
    assert_eq!(t.frames, 60);
    for (i, k) in t.keys.iter().enumerate() {
        assert!((k.from.at - i as f64).abs() < 1e-9);
        assert!(
            (k.share - i as f64 / 60.0).abs() < 1e-9,
            "shares as round 1"
        );
    }
}

#[test]
fn a_freeze_adds_frames_and_a_hold_over_keeps_the_length() {
    let mut r = on("Hold", clip("Charge", 80), Vec::new());
    r.time = vec![TimeEdit::hold(TimeKind::HoldOver, 0, 10, 20)];
    assert_eq!(timeline(&r).frames, 80);
    r.time = vec![TimeEdit::hold(TimeKind::Freeze, 0, 10, 20)];
    assert_eq!(timeline(&r).frames, 100);
    r.time = vec![TimeEdit::retime(0, 0, 40, 2.0)];
    assert_eq!(timeline(&r).frames, 60, "40 frames at 2x play in 20");
}
