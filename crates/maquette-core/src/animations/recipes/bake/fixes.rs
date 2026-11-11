//! Fixes, best first: for each review line that warns and has a change that
//! answers it, the change, what it should come to and whether that passes.
//! A seam pop widens its blend window, a hold seam eases its hold in or out
//! over more frames, a key past a joint limit is held to the limit. The
//! guess for a seam spreads the same extra move over more frames: a pop of
//! `x` over a window of `n` frames (1 for a cut) is about `x * n / m` over
//! `m`. Nothing is written: the editor lays a fix on the person's draft and
//! the next bake measures it.

use serde::{Deserialize, Serialize};

use super::RecipeBakeResult;
use crate::recipes::seams::{SEAM_HIPS_CM, SEAM_TURN_DEGREES, SeamCheck, SeamKind};

/// The most frames a fix eases a hold over.
const MOST_EASE: u32 = 12;
/// A fix aims this far under the limit, so the next bake has room.
const AIM: f32 = 0.8;

/// What a fix changes in the recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RecipeFixEdit {
    /// Clip `clip`'s blend window from the clip before, frames.
    Blend { clip: u32, frames: u32 },
    /// The freeze or hold over on frame `at` of clip `clip`: how many frames
    /// it eases in or out over (the side the fix is about).
    #[serde(rename_all = "camelCase")]
    HoldEase {
        clip: u32,
        at: u32,
        ease_in: Option<u32>,
        ease_out: Option<u32>,
    },
    /// The turn on `axis` (`x`, `y` or `z`) of `bone` in the key on frame
    /// `frame` of clip `clip` of change `layer`, degrees.
    KeyTurn {
        layer: String,
        clip: u32,
        frame: u32,
        bone: String,
        axis: String,
        degrees: f32,
    },
}

/// One fix of the review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeReviewFix {
    pub id: String,
    /// The review line it answers.
    pub line: String,
    /// "Widen the blend to 12 f".
    pub title: String,
    /// "Spreads the hip move over more frames."
    pub detail: String,
    /// What it should come to, in words: "1.4 cm".
    pub after: String,
    /// Whether that is within the limit.
    pub passes: bool,
    pub edit: RecipeFixEdit,
}

/// The fewest frames from `from` up to `most` that bring `seam` under its
/// limits, else `most`; and the hips and turn it should come to there.
fn frames_for(seam: &SeamCheck, from: u32, most: u32) -> Option<(u32, f32, f32)> {
    let now = from.max(1) as f32;
    let hips = seam.hips_over_cm * now;
    let turn = seam.turn_over_degrees * now;
    let after = |m: u32| (hips / m as f32, turn / m as f32);
    let fits = |m: u32| {
        let (h, t) = after(m);
        h <= SEAM_HIPS_CM * AIM && t <= SEAM_TURN_DEGREES * AIM
    };
    let start = (from + 2).max(2);
    if start > most {
        return None;
    }
    let m = (start..=most).find(|m| fits(*m)).unwrap_or(most);
    let (h, t) = after(m);
    Some((m, h, t))
}

/// The guess after a seam fix, in words, and whether it passes.
fn after_words(seam: &SeamCheck, hips: f32, turn: f32) -> (String, bool) {
    let passes = hips <= SEAM_HIPS_CM && turn <= SEAM_TURN_DEGREES;
    let words = if seam.hips_worse() {
        format!("about {hips:.1} cm")
    } else {
        format!("about {turn:.0} deg")
    };
    (words, passes)
}

fn what_moves(seam: &SeamCheck) -> String {
    if seam.hips_worse() {
        "the hip move".to_string()
    } else {
        format!("the {} turn", seam.bone.as_deref().unwrap_or("bone"))
    }
}

fn seam_fix(seam: &SeamCheck) -> Option<RecipeReviewFix> {
    match seam.kind {
        SeamKind::Join => {
            let from = seam.blend.unwrap_or(0);
            let (m, hips, turn) = frames_for(seam, from, seam.blend_most?)?;
            let (after, passes) = after_words(seam, hips, turn);
            Some(RecipeReviewFix {
                id: format!("blend-{}", seam.clip),
                line: "seam".to_string(),
                title: format!("Widen the blend to {m} f"),
                detail: format!("Spreads {} over more frames.", what_moves(seam)),
                after,
                passes,
                edit: RecipeFixEdit::Blend {
                    clip: seam.clip,
                    frames: m,
                },
            })
        }
        SeamKind::HoldIn | SeamKind::HoldOut => {
            let at = seam.at?;
            let from = seam.ease.unwrap_or(0);
            let most = seam.ease_most.unwrap_or(MOST_EASE).min(MOST_EASE);
            let (m, hips, turn) = frames_for(seam, from, most)?;
            let (after, passes) = after_words(seam, hips, turn);
            let held = if seam.freeze { "freeze" } else { "hold" };
            let into = seam.kind == SeamKind::HoldIn;
            Some(RecipeReviewFix {
                id: format!(
                    "ease-{}-{at}-{}",
                    seam.clip,
                    if into { "in" } else { "out" }
                ),
                line: "hold-seam".to_string(),
                title: format!(
                    "Ease the {held} {} over {m} f",
                    if into { "in" } else { "out" }
                ),
                detail: format!("Spreads {} over more frames.", what_moves(seam)),
                after,
                passes,
                edit: RecipeFixEdit::HoldEase {
                    clip: seam.clip,
                    at,
                    ease_in: into.then_some(m),
                    ease_out: (!into).then_some(m),
                },
            })
        }
    }
}

/// Holds the worst keyed turn past its limit to the limit.
fn joint_fix(result: &RecipeBakeResult) -> Option<RecipeReviewFix> {
    let worst = result.joints.as_ref()?.hits.first()?;
    let hit = &worst.hit;
    let at = worst.bake_frame.map_or_else(
        || format!("frame {} of clip {}", worst.frame, worst.clip + 1),
        |f| format!("frame {f}"),
    );
    Some(RecipeReviewFix {
        id: format!(
            "limit-{}-{}-{}-{}",
            worst.layer, worst.clip, worst.frame, hit.bone
        ),
        line: "joints".to_string(),
        title: format!("Hold the {} key to its limit", hit.bone),
        detail: format!(
            "Sets the {} to {:.0} deg at {at}.",
            hit.word,
            hit.limit.abs()
        ),
        after: format!("{:.0} deg", hit.limit.abs()),
        passes: true,
        edit: RecipeFixEdit::KeyTurn {
            layer: worst.layer.clone(),
            clip: worst.clip,
            frame: worst.frame,
            bone: hit.bone.clone(),
            axis: hit.axis.clone(),
            degrees: hit.limit,
        },
    })
}

/// The fixes for `result`'s warnings, best first: those that should pass,
/// then by how far past its limit the warning is.
pub(super) fn fixes_of(result: &RecipeBakeResult) -> Vec<RecipeReviewFix> {
    let mut found: Vec<(f32, RecipeReviewFix)> = result
        .seams
        .iter()
        .filter(|s| s.pops)
        .filter_map(|s| Some((s.ratio(), seam_fix(s)?)))
        .collect();
    if let Some(fix) = joint_fix(result) {
        let over = result
            .joints
            .as_ref()
            .and_then(|j| j.hits.first())
            .map_or(1.0, |h| h.hit.degrees.abs() / h.hit.limit.abs().max(1.0));
        found.push((over, fix));
    }
    found.sort_by(|a, b| b.1.passes.cmp(&a.1.passes).then(b.0.total_cmp(&a.0)));
    found.into_iter().map(|(_, f)| f).collect()
}
