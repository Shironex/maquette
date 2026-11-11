//! The seam checks of a bake: the report's motion read as seam pop and
//! hold seam lines, a two handed grip's hand gap, a mirror's swapped bones,
//! the fixes, best first, and the pose sheet's frames.

use std::fs;

use serde_json::json;

use super::job::REPORT_FILE;
use super::result::{read_run, result_of};
use super::tests::{humanoid, project_with};
use super::tests_grips::grips;
use super::tests_joined::joined;
use super::*;
use crate::recipes::seams::SeamKind;
use crate::recipes::timeline::timeline;

/// Hips that walk 1 cm a frame and a chest that turns 2 degrees a frame,
/// with the hips jumping `jump` cm more at frame `at`.
fn motion(frames: u32, at: u32, jump: f32) -> serde_json::Value {
    let hips: Vec<[f32; 3]> = (0..=frames)
        .map(|i| {
            let z = i as f32 * 0.01 + if i >= at { jump / 100.0 } else { 0.0 };
            [0.0, 1.0, z]
        })
        .collect();
    let mut chest = vec![2.0_f32; frames as usize + 1];
    chest[0] = 0.0;
    let hand = |x: f32| -> Vec<[f32; 3]> { (0..=frames).map(|_| [x, 1.2, 0.3]).collect() };
    json!({
        "tracks": {"hips": hips, "leftHand": hand(0.1), "rightHand": hand(-0.1)},
        "turns": {"chest": chest},
    })
}

fn report(frames: u32, motion: serde_json::Value) -> String {
    json!({
        "clip": "reach_out", "frames": frames, "facing": [0, 0, 1],
        "feet": {}, "baseFeet": {}, "reaches": [], "pins": [],
        "motion": motion, "seconds": 1.0,
    })
    .to_string()
}

fn baked(p: &tests::Project, report: &str) -> RecipeBakeResult {
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let work = p.root().join("work");
    fs::create_dir_all(&work).expect("work");
    fs::write(work.join(REPORT_FILE), report).expect("report");
    result_of(&read_run(&work).expect("run"), &prepared, &work, 2.0)
}

#[test]
fn a_hip_jump_where_clips_meet_reads_as_a_seam_pop_with_a_wider_blend_to_fix_it() {
    let p = joined();
    let t = timeline(&p.recipe.recipe);
    let seam = t.seams[0].frame + 2;
    let result = baked(&p, &report(t.frames, motion(t.frames, seam, 4.0)));
    let kinds: Vec<SeamKind> = result.seams.iter().map(|s| s.kind).collect();
    assert_eq!(
        kinds,
        [SeamKind::Join, SeamKind::HoldIn, SeamKind::HoldOut],
        "the join, then the freeze in and out"
    );
    let join = &result.seams[0];
    assert!(join.pops);
    assert_eq!(join.frame, seam);
    assert_eq!(join.about, "Standing Still into Turn Around");

    let review = review_of(&result);
    let line = |id: &str| {
        review
            .lines
            .iter()
            .find(|l| l.id == id)
            .unwrap_or_else(|| panic!("no {id} in {:?}", review.lines))
    };
    let pop = line("seam");
    assert_eq!(pop.label, "Seam pop");
    assert_eq!((pop.value.as_str(), pop.limit.as_str()), ("4.0 cm", "2 cm"));
    assert_eq!(pop.status, RecipeReviewStatus::Warn);
    let note = pop.note.as_deref().expect("note");
    assert!(
        note.starts_with(&format!(
            "Standing Still into Turn Around, frame {seam}. Hips move 5.0 cm"
        )),
        "{note}"
    );
    let hold = line("hold-seam");
    assert_eq!(hold.status, RecipeReviewStatus::Pass, "{hold:?}");
    assert!(
        hold.note
            .as_deref()
            .is_some_and(|n| n.starts_with("The freeze on Turn Around")),
        "{hold:?}"
    );
    assert_eq!(line("mirror").label, "Mirror");
    let ids: Vec<&str> = review.lines.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids[..2], ["seam", "hold-seam"], "seams lead the review");

    let fix = review.fixes.first().expect("a fix");
    assert_eq!(fix.line, "seam");
    let RecipeFixEdit::Blend { clip, frames } = fix.edit else {
        panic!("{fix:?}")
    };
    assert_eq!(clip, 1);
    assert!(
        frames > 6 && frames <= join.blend_most.expect("most"),
        "{fix:?}"
    );
    assert_eq!(fix.title, format!("Widen the blend to {frames} f"));
    assert!(fix.passes, "{fix:?}");

    let sheet = &result.sheet;
    let first = sheet
        .iter()
        .find(|f| f.frame == seam)
        .expect("the seam's frame");
    assert_eq!(first.label, "seam 1");
    assert!(first.warn);
    assert_eq!(first.detail.as_deref(), Some("hips jump 5.0 cm"));
    assert!(
        sheet.iter().any(|f| f.label == "freeze starts"),
        "{sheet:?}"
    );
    assert!(sheet.windows(2).all(|w| w[0].frame < w[1].frame));
}

#[test]
fn a_seam_that_keeps_the_clips_pace_passes_with_no_fix() {
    let p = joined();
    let t = timeline(&p.recipe.recipe);
    let result = baked(&p, &report(t.frames, motion(t.frames, 0, 0.0)));
    let review = review_of(&result);
    let pop = review.lines.iter().find(|l| l.id == "seam").expect("seam");
    assert_eq!(pop.status, RecipeReviewStatus::Pass);
    assert_eq!(pop.value, "0.0 cm");
    assert!(review.fixes.is_empty(), "{:?}", review.fixes);
}

#[test]
fn a_two_handed_grip_s_hands_drifting_apart_reads_as_its_hand_gap() {
    let mut p = project_with(humanoid());
    p.change(grips(), Vec::new());
    let frames = p.recipe.recipe.base.frames;
    let mut m = motion(frames, 0, 0.0);
    // The left hand slides 0.5 cm a frame away from the right.
    let left: Vec<[f32; 3]> = (0..=frames)
        .map(|i| [0.1 + i as f32 * 0.005, 1.2, 0.3])
        .collect();
    m["tracks"]["leftHand"] = json!(left);
    let result = baked(&p, &report(frames, m));
    let both = &result.grips[0];
    let gap = both.gap_cm.expect("the two handed grip has a gap");
    assert!(gap > 2.0, "{both:?}");
    assert_eq!(result.grips[1].gap_cm, None, "one hand has none");
    let review = review_of(&result);
    let line = review
        .lines
        .iter()
        .find(|l| l.id == "grip-gap-l-1")
        .expect("gap line");
    assert_eq!(line.label, "Grip hand gap");
    assert_eq!(line.status, RecipeReviewStatus::Warn);
    assert!(review.lines.iter().all(|l| l.id != "grip-gap-l-2"));
    assert!(
        review
            .lines
            .iter()
            .all(|l| l.id != "seam" && l.id != "mirror")
    );

    // Hands that keep their distance pass.
    let result = baked(&p, &report(frames, motion(frames, 0, 0.0)));
    assert_eq!(result.grips[0].gap_cm, Some(0.0));
}

#[test]
fn a_record_from_before_the_seam_checks_still_reads() {
    let p = joined();
    let t = timeline(&p.recipe.recipe);
    let result = baked(&p, &report(t.frames, motion(t.frames, 0, 0.0)));
    let mut value = serde_json::to_value(&result).expect("json");
    for key in ["seams", "mirror", "sheet"] {
        value.as_object_mut().expect("object").remove(key);
    }
    let old: RecipeBakeResult = serde_json::from_value(value).expect("an older record reads");
    assert!(old.seams.is_empty() && old.sheet.is_empty() && old.mirror.is_none());
    let review = review_of(&old);
    assert!(review.lines.iter().all(|l| l.id != "seam"));
}

#[test]
fn a_recipe_with_no_seam_key_or_hold_sheets_where_each_change_starts_and_ends() {
    let p = super::tests::reaching();
    let frames = p.recipe.recipe.base.frames;
    let result = baked(&p, &report(frames, motion(frames, 0, 0.0)));
    assert!(result.seams.is_empty());
    let labels: Vec<&str> = result.sheet.iter().map(|f| f.label.as_str()).collect();
    assert_eq!(labels, ["rightHand starts", "rightHand ends"]);
    assert!(
        result
            .sheet
            .iter()
            .all(|f| f.kind == SheetKind::Change && !f.warn)
    );
}
