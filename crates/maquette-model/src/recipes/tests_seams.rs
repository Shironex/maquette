//! Seams: which steps of the timeline each join and hold is judged on, and
//! the seam pop limits (hips 2 cm, any bone 25 degrees more in one frame
//! than the clips around it), each at its edge.

use std::collections::BTreeMap;

use super::seams::{
    AROUND_FRAMES, SEAM_HIPS_CM, SEAM_TURN_DEGREES, SeamKind, SeamMotion, SeamWindow, checks,
    judge, windows,
};
use super::tests_v2::clip;
use super::timeline::timeline;
use super::*;

fn joined(blend: u32) -> RecipeFile {
    let mut second = clip("Thrust", 40);
    second.blend = Some(BlendWindow {
        frames: blend,
        ..BlendWindow::default()
    });
    let mut r = RecipeFile::new("Two clips", clip("Slash", 40));
    r.joined = vec![second];
    r
}

/// Hips that walk `walk` cm a frame, and one bone that turns `turn`
/// degrees a frame, over `frames` frames.
fn steady(frames: u32, walk: f32, turn: f32) -> SeamMotion {
    let n = frames as usize + 1;
    let hips = (0..n)
        .map(|i| [0.0, 1.0, i as f32 * walk / 100.0])
        .collect();
    let mut steps = vec![turn; n];
    steps[0] = 0.0;
    SeamMotion {
        hips,
        turns: BTreeMap::from([("chest".to_string(), steps)]),
    }
}

/// `motion` with the hips jumping `cm` more at step `frame`, and every
/// later frame carried along.
fn jump(mut motion: SeamMotion, frame: usize, cm: f32) -> SeamMotion {
    for p in &mut motion.hips[frame..] {
        p[2] += cm / 100.0;
    }
    motion
}

fn window(first: u32, last: u32) -> SeamWindow {
    SeamWindow {
        kind: SeamKind::Join,
        clip: 1,
        at: None,
        freeze: false,
        first,
        last,
    }
}

#[test]
fn a_blend_is_judged_from_its_first_frame_to_the_frame_after_it() {
    let r = joined(8);
    let t = timeline(&r);
    let w = windows(&r, &t);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].kind, SeamKind::Join);
    assert_eq!(
        (w[0].first, w[0].last),
        (32, 40),
        "40 frames less an 8 frame blend"
    );
}

#[test]
fn a_cut_is_judged_on_its_one_step() {
    let r = joined(0);
    let t = timeline(&r);
    let w = windows(&r, &t);
    assert_eq!((w[0].first, w[0].last), (40, 40));
}

#[test]
fn a_hold_over_is_judged_where_it_eases_in_and_out() {
    let mut r = RecipeFile::new("Held", clip("Charge", 60));
    let mut hold = TimeEdit::hold(TimeKind::HoldOver, 0, 20, 10);
    hold.ease_in = Some(3);
    hold.ease_out = Some(2);
    r.time = vec![hold];
    let t = timeline(&r);
    let w = windows(&r, &t);
    let kinds: Vec<(SeamKind, u32, u32)> = w.iter().map(|w| (w.kind, w.first, w.last)).collect();
    assert_eq!(
        kinds,
        vec![(SeamKind::HoldIn, 21, 23), (SeamKind::HoldOut, 29, 30)],
        "frames 21 to 29 hold some of frame 20, 23 to 28 all of it: in up to the first whole one, out from after the last"
    );
    assert!(w.iter().all(|w| w.at == Some(20) && !w.freeze));
}

#[test]
fn a_freeze_at_the_very_end_has_no_way_out_to_judge() {
    let mut r = RecipeFile::new("Frozen", clip("Charge", 30));
    r.time = vec![TimeEdit::hold(TimeKind::Freeze, 0, 30, 6)];
    let t = timeline(&r);
    let w = windows(&r, &t);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].kind, SeamKind::HoldIn);
    assert!(w[0].freeze);
}

#[test]
fn hips_pop_only_past_two_cm_more_than_the_clips_around() {
    let base = steady(80, 1.0, 2.0);
    let w = window(40, 40);
    let at_limit = judge(&w, &jump(base.clone(), 40, SEAM_HIPS_CM), 80);
    assert!((at_limit.hips_over_cm - SEAM_HIPS_CM).abs() < 0.05);
    assert!(!at_limit.pops, "at the limit passes: {at_limit:?}");
    let past = judge(&w, &jump(base.clone(), 40, SEAM_HIPS_CM + 0.2), 80);
    assert!(past.pops, "{past:?}");
    assert_eq!(past.frame, 40);
    assert!(
        (past.hips_cm - 3.2).abs() < 0.05,
        "the step itself: {past:?}"
    );
    assert!((past.hips_around_cm - 1.0).abs() < 0.05);
}

#[test]
fn a_fast_motion_is_held_to_itself_not_to_zero() {
    // Hips that travel 5 cm a frame on both sides never pop at a seam that
    // keeps the pace.
    let fast = steady(80, 5.0, 2.0);
    let check = judge(&window(36, 44), &fast, 80);
    assert!(!check.pops, "{check:?}");
    assert!(check.hips_over_cm < 0.05);
}

#[test]
fn a_bone_pops_only_past_25_degrees_more_than_its_own_around() {
    let mut motion = steady(80, 1.0, 4.0);
    let steps = motion.turns.get_mut("chest").expect("chest");
    steps[50] = 4.0 + SEAM_TURN_DEGREES;
    let w = window(48, 52);
    let at_limit = judge(&w, &motion, 80);
    assert!(!at_limit.pops, "{at_limit:?}");
    motion.turns.get_mut("chest").expect("chest")[50] = 4.0 + SEAM_TURN_DEGREES + 0.5;
    let past = judge(&w, &motion, 80);
    assert!(past.pops);
    assert_eq!(past.bone.as_deref(), Some("chest"));
    assert_eq!(past.frame, 50);
    assert!(!past.hips_worse());
}

#[test]
fn the_clips_around_are_six_frames_either_side_within_the_timeline() {
    let mut motion = steady(80, 1.0, 1.0);
    // A big step just outside the window's around does not raise the bar.
    let far = 40 - AROUND_FRAMES as usize - 1;
    motion.turns.get_mut("chest").expect("chest")[far] = 40.0;
    motion.turns.get_mut("chest").expect("chest")[40] = 30.0;
    let check = judge(&window(40, 40), &motion, 80);
    assert!(check.pops, "{check:?}");
    // One inside it does.
    motion.turns.get_mut("chest").expect("chest")[far + 1] = 40.0;
    let check = judge(&window(40, 40), &motion, 80);
    assert!(!check.pops, "{check:?}");
}

#[test]
fn every_seam_of_a_recipe_is_judged() {
    let mut r = joined(8);
    r.time = vec![TimeEdit::hold(TimeKind::HoldOver, 1, 20, 6)];
    let t = timeline(&r);
    let motion = jump(steady(t.frames, 1.0, 2.0), 35, 3.0);
    let found = checks(&r, &t, &motion);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found[0].pops, "the join pops at its jump");
    assert_eq!(found[0].frame, 35);
    assert!(!found[1].pops && !found[2].pops, "the steady hold does not");
}
