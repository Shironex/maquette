//! Seams in a real, headless Blender: the fixture clip cut into itself at a
//! frame far from where it left off, with nothing lined up, pops where the
//! clips meet; cut into itself where it left off, with the hips lined up,
//! it passes. A hold over with no ease out pops as it lets go. Skipped
//! where no Blender is installed.

use super::tests_blender_joined::{bake_of, project};
use super::*;
use crate::animations::tests_blender::{Setup, import_fixture, setup};
use crate::recipes::seams::SeamKind;
use crate::recipes::{BlendWindow, LineUp, RecipeChange, TimeEdit, TimeKind};
use crate::workspace::WorkspaceConfig;

fn joined_at(
    from: [u32; 2],
    to: [u32; 2],
    line_up: LineUp,
) -> impl FnOnce(&crate::recipes::RecipeFile) -> RecipeChange {
    move |r| {
        let mut first = r.base.clone();
        first.trim = Some(from);
        let mut second = r.base.clone();
        second.trim = Some(to);
        second.blend = Some(BlendWindow {
            frames: 0,
            line_up,
            ..BlendWindow::default()
        });
        RecipeChange {
            clips: Some(vec![first, second]),
            kept: Some(vec![Some(0), Some(0)]),
            ..RecipeChange::default()
        }
    }
}

/// The dodge clip in the library first, so the recipes run on it: its hips
/// travel 0.9 m to the left over 30 frames, 3 cm a frame.
fn dodging(s: &Setup) {
    import_fixture(s, &WorkspaceConfig::default(), "clip", "Dodge Left.fbx");
}

#[test]
fn a_cut_to_a_far_frame_pops_and_a_cut_where_the_clip_left_off_does_not() {
    let Some(s) = setup() else { return };
    dodging(&s);
    let popping = project(&s, joined_at([0, 20], [2, 30], LineUp::Off));
    let (result, _, _) = bake_of(&s, &popping, &[]);
    eprintln!("popping seams: {:#?}", result.seams);
    let join = result
        .seams
        .iter()
        .find(|s| s.kind == SeamKind::Join)
        .expect("a join");
    assert!(join.pops, "{join:?}");
    let review = review_of(&result);
    let line = review
        .lines
        .iter()
        .find(|l| l.id == "seam")
        .expect("seam line");
    assert_eq!(line.status, RecipeReviewStatus::Warn, "{line:?}");
    assert!(
        review.fixes.iter().any(|f| f.line == "seam"),
        "{:?}",
        review.fixes
    );

    let smooth = project(&s, joined_at([0, 20], [20, 30], LineUp::Hips));
    let (result, _, _) = bake_of(&s, &smooth, &[]);
    eprintln!("smooth seams: {:#?}", result.seams);
    let join = result
        .seams
        .iter()
        .find(|s| s.kind == SeamKind::Join)
        .expect("a join");
    assert!(!join.pops, "{join:?}");
    let review = review_of(&result);
    let line = review
        .lines
        .iter()
        .find(|l| l.id == "seam")
        .expect("seam line");
    assert_eq!(line.status, RecipeReviewStatus::Pass, "{line:?}");
}

#[test]
fn a_hold_over_that_lets_go_at_once_pops_and_a_freeze_does_not() {
    let Some(s) = setup() else { return };
    dodging(&s);
    let held = |kind: TimeKind| {
        move |_: &crate::recipes::RecipeFile| RecipeChange {
            time: Some(vec![TimeEdit::hold(kind, 0, 4, 14)]),
            ..RecipeChange::default()
        }
    };
    let (result, _, _) = bake_of(&s, &project(&s, held(TimeKind::HoldOver)), &[]);
    eprintln!("held seams: {:#?}", result.seams);
    let out = result
        .seams
        .iter()
        .find(|s| s.kind == SeamKind::HoldOut)
        .expect("lets go");
    assert!(out.pops, "{out:?}");
    assert!(!out.freeze);
    let review = review_of(&result);
    let line = review
        .lines
        .iter()
        .find(|l| l.id == "hold-seam")
        .expect("hold seam line");
    assert_eq!(line.status, RecipeReviewStatus::Warn, "{line:?}");
    let fix = review
        .fixes
        .iter()
        .find(|f| f.line == "hold-seam")
        .expect("a fix");
    assert!(
        matches!(
            fix.edit,
            RecipeFixEdit::HoldEase {
                ease_out: Some(_),
                ..
            }
        ),
        "{fix:?}"
    );

    // A freeze stops on the pose and goes on from it: nothing jumps.
    let (result, _, _) = bake_of(&s, &project(&s, held(TimeKind::Freeze)), &[]);
    eprintln!("frozen seams: {:#?}", result.seams);
    assert!(
        result.seams.iter().all(|s| s.freeze && !s.pops),
        "{:?}",
        result.seams
    );
    assert!(!result.seams.is_empty());
}
