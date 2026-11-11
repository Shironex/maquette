//! The seam checks of a bake: what the bake script reports of the motion
//! (where the hips and hands are each frame and how far each body bone
//! turns), judged for seam pops where clips meet and where a hold eases in
//! and out (`crate::recipes::seams`), a two handed grip's hand gap, and a
//! mirror's unswapped bones, with the review's line for each.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::RecipeBakeResult;
use super::grips::RecipeGripResult;
use super::job::Prepared;
use super::review::{RecipeReviewLine, RecipeReviewStatus};
use crate::recipes::grips::hands_of_pose;
use crate::recipes::grips::{GripHands, span_on};
use crate::recipes::seams::{
    SEAM_HIPS_CM, SEAM_TURN_DEGREES, SeamCheck, SeamKind, SeamMotion, checks,
};
use crate::recipes::timeline::{Timeline, timeline};
use crate::recipes::{LayerKind, RecipeFile};

/// The two hands of a grip may move this far closer or further apart than
/// at its first whole frame, cm.
pub const GRIP_GAP_LIMIT_CM: f32 = 2.0;

/// `motion` in the report: places by VRM name (`hips`, `leftHand`,
/// `rightHand`), metres per frame, and each body bone's turn from the frame
/// before, degrees.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct MotionRun {
    pub tracks: BTreeMap<String, Vec<[f32; 3]>>,
    pub turns: BTreeMap<String, Vec<f32>>,
}

/// A mirrored recipe as the bake swapped it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeMirrorResult {
    /// How many left and right bones it swapped, in pairs.
    pub pairs: u32,
    /// The side bones the model has without their other side, by VRM name,
    /// which stay where they are.
    pub unpaired: Vec<String>,
}

/// The timeline `prepared` bakes on: the one it carries for a joined or
/// timed bake, else the plain recipe's own. Seams, grip gaps and the sheet
/// all read this one.
pub(crate) fn baked_timeline(prepared: &Prepared) -> Timeline {
    prepared
        .timeline
        .clone()
        .unwrap_or_else(|| timeline(&prepared.doc.recipe))
}

/// Every seam of the recipe `prepared` bakes on its timeline `t`, judged on
/// the motion `run` reports.
pub(crate) fn seam_checks(prepared: &Prepared, t: &Timeline, run: &MotionRun) -> Vec<SeamCheck> {
    let recipe = &prepared.doc.recipe;
    let motion = SeamMotion {
        hips: run.tracks.get("hips").cloned().unwrap_or_default(),
        turns: run.turns.clone(),
    };
    checks(recipe, t, &motion)
}

fn apart(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f32>().sqrt()
}

/// `grips` with each two handed grip's hand gap: how far the hands move
/// closer or further apart than at its first whole frame, over the frames
/// it holds whole, on the timeline `t` it was baked on.
pub(crate) fn with_gaps(
    recipe: &RecipeFile,
    t: &Timeline,
    grips: &[RecipeGripResult],
    run: &MotionRun,
) -> Vec<RecipeGripResult> {
    let (Some(left), Some(right)) = (run.tracks.get("leftHand"), run.tracks.get("rightHand"))
    else {
        return grips.to_vec();
    };
    grips
        .iter()
        .map(|grip| {
            let mut grip = grip.clone();
            let layer = recipe
                .layers
                .iter()
                .find(|l| l.kind == LayerKind::Grip && l.id == grip.layer);
            let both = layer.and_then(|l| l.pose.as_ref()).and_then(hands_of_pose)
                == Some(GripHands::Both);
            let span = layer.and_then(|l| span_on(t, l));
            if let (true, Some(span)) = (both, span) {
                let first = span.first + span.ease_in;
                let last = span.last.saturating_sub(span.ease_out);
                let gap = |i: u32| {
                    let i = i as usize;
                    left.get(i).zip(right.get(i)).map(|(l, r)| apart(*l, *r))
                };
                if let Some(start) = (first <= last).then(|| gap(first)).flatten() {
                    let worst = (first..=last)
                        .filter_map(|i| gap(i).map(|g| (i, (g - start).abs())))
                        .fold((first, 0.0_f32), |b, x| if x.1 > b.1 { x } else { b });
                    grip.gap_cm = Some((worst.1 * 1000.0).round() / 10.0);
                    grip.gap_frame = Some(worst.0);
                }
            }
            grip
        })
        .collect()
}

fn status(warn: bool) -> RecipeReviewStatus {
    if warn {
        RecipeReviewStatus::Warn
    } else {
        RecipeReviewStatus::Pass
    }
}

/// The worst seam of `kinds`, by how far past its limits.
fn worst<'a>(result: &'a RecipeBakeResult, kinds: &[SeamKind]) -> Vec<&'a SeamCheck> {
    let mut found: Vec<&SeamCheck> = result
        .seams
        .iter()
        .filter(|s| kinds.contains(&s.kind))
        .collect();
    found.sort_by(|a, b| b.ratio().total_cmp(&a.ratio()));
    found
}

/// What `seam` measured, in words, and its number against the limit: the
/// hips when they go further past theirs, else the bone.
fn measured(seam: &SeamCheck) -> (String, String, f32, f32, &'static str) {
    if seam.hips_worse() {
        (
            format!("{:.1} cm", seam.hips_over_cm),
            format!("{SEAM_HIPS_CM:.0} cm"),
            seam.hips_over_cm,
            SEAM_HIPS_CM,
            "cm",
        )
    } else {
        (
            format!("{:.0} deg", seam.turn_over_degrees),
            format!("{SEAM_TURN_DEGREES:.0} deg"),
            seam.turn_over_degrees,
            SEAM_TURN_DEGREES,
            "deg",
        )
    }
}

/// The step of `seam` in a sentence: "Hips move 3.1 cm in one frame, 1.0 cm
/// more than ... ".
fn step_words(seam: &SeamCheck) -> String {
    if seam.hips_worse() {
        format!(
            "Hips move {:.1} cm in one frame; the clips around move {:.1} cm.",
            seam.hips_cm, seam.hips_around_cm
        )
    } else {
        format!(
            "{} turns {:.0} deg in one frame; around it, {:.0} deg.",
            seam.bone.as_deref().unwrap_or("A bone"),
            seam.turn_degrees,
            seam.turn_around_degrees
        )
    }
}

fn seam_line(
    id: &str,
    label: &str,
    seams: &[&SeamCheck],
    place: impl Fn(&SeamCheck) -> String,
) -> Option<RecipeReviewLine> {
    let seam = seams.first()?;
    if !seam.measured {
        return Some(RecipeReviewLine {
            id: id.to_string(),
            label: label.to_string(),
            about: Some(seam.about.clone()),
            value: "not measured".to_string(),
            limit: format!("{SEAM_HIPS_CM:.0} cm or {SEAM_TURN_DEGREES:.0} deg"),
            measured: None,
            max: None,
            unit: None,
            status: RecipeReviewStatus::NotMeasured,
            note: Some("The bake reported no hips motion, so no seam was measured.".to_string()),
        });
    }
    let (value, limit, number, max, unit) = measured(seam);
    let more = seams.len() - 1;
    let others = if more == 0 {
        String::new()
    } else {
        match seams.iter().skip(1).filter(|s| s.pops).count() {
            0 => format!(" {more} more, none past the limit."),
            popping => format!(" {more} more, {popping} of them past the limit."),
        }
    };
    Some(RecipeReviewLine {
        id: id.to_string(),
        label: label.to_string(),
        about: Some(seam.about.clone()),
        value,
        limit,
        measured: Some(number),
        max: Some(max),
        unit: Some(unit.to_string()),
        status: status(seam.pops),
        note: Some(format!("{}{}{others}", place(seam), step_words(seam))),
    })
}

/// The seam pop line (where clips meet) and the hold seam line (where a
/// freeze or hold over eases in and out), each for its worst seam.
pub(super) fn seam_lines(
    result: &RecipeBakeResult,
) -> (Vec<RecipeReviewLine>, Vec<RecipeReviewLine>) {
    let joins = worst(result, &[SeamKind::Join]);
    let holds = worst(result, &[SeamKind::HoldIn, SeamKind::HoldOut]);
    let pop = seam_line("seam", "Seam pop", &joins, |s| {
        format!("{}, frame {}. ", s.about, s.frame)
    });
    let hold = seam_line("hold-seam", "Hold seam", &holds, |s| {
        let what = if s.freeze { "freeze" } else { "hold" };
        let way = if s.kind == SeamKind::HoldIn {
            "eases in"
        } else {
            "eases out"
        };
        format!("The {what} on {} {way} at frame {}. ", s.about, s.frame)
    });
    (pop.into_iter().collect(), hold.into_iter().collect())
}

/// A line per two handed grip: how far its hands drift apart.
pub(super) fn gap_lines(result: &RecipeBakeResult) -> Vec<RecipeReviewLine> {
    result
        .grips
        .iter()
        .filter_map(|grip| {
            let gap = grip.gap_cm?;
            let warn = gap > GRIP_GAP_LIMIT_CM;
            Some(RecipeReviewLine {
                id: format!("grip-gap-{}", grip.layer),
                label: "Grip hand gap".to_string(),
                about: Some(format!("both hands ({})", grip.layer)),
                value: format!("{gap:.1} cm"),
                limit: format!("{GRIP_GAP_LIMIT_CM:.0} cm"),
                measured: Some(gap),
                max: Some(GRIP_GAP_LIMIT_CM),
                unit: Some("cm".to_string()),
                status: status(warn),
                note: Some(match grip.gap_frame {
                    Some(f) if warn => format!(
                        "Both hands on one grip: they move {gap:.1} cm from where they sit at its first whole frame, most at frame {f}."
                    ),
                    _ => "Both hands on one grip stay as far apart as at its first whole frame."
                        .to_string(),
                }),
            })
        })
        .collect()
}

/// The mirror line of a mirrored recipe: the side bones it could not swap.
pub(super) fn mirror_line(result: &RecipeBakeResult) -> Option<RecipeReviewLine> {
    let mirror = result.mirror.as_ref()?;
    let left = u32::try_from(mirror.unpaired.len()).unwrap_or(u32::MAX);
    let named: Vec<&str> = mirror.unpaired.iter().take(3).map(String::as_str).collect();
    Some(RecipeReviewLine {
        id: "mirror".to_string(),
        label: "Mirror".to_string(),
        about: None,
        value: format!("{} pairs swapped", mirror.pairs),
        limit: "every side bone".to_string(),
        measured: None,
        max: None,
        unit: None,
        status: status(left > 0),
        note: Some(if left == 0 {
            "Every left and right bone of the model has its other side, so all of them swap."
                .to_string()
        } else {
            format!(
                "{} has no other side for {}{}, so {} where {} {}.",
                result.asset_name,
                named.join(", "),
                if left > 3 {
                    format!(" and {} more", left - 3)
                } else {
                    String::new()
                },
                if left == 1 { "it stays" } else { "they stay" },
                if left == 1 { "it" } else { "they" },
                if left == 1 { "is" } else { "are" },
            )
        }),
    })
}
