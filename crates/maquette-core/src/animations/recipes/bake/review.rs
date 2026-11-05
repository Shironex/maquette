//! A finished bake's numbers against their limits, as the review shows
//! them and agents read them: each reach's gap to its target (2 cm) and its
//! elbow or knee (no flip), the planted feet's slide (1.5 cm over the base
//! clip's own on this model) and the rig's stretch (5 mm over the base
//! clip's). A warn asks for a look; it never stops the bake being added.

use serde::{Deserialize, Serialize};

use super::{GAP_LIMIT_CM, RecipeBakeResult};

/// A foot may slide this much more than the base clip's own, cm.
pub const SLIDE_OVER_BASE_CM: f32 = 1.5;
/// The rig may stretch this much more than the base clip's own, mm at a
/// model one metre tall.
pub const STRETCH_OVER_BASE_MM: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RecipeReviewStatus {
    Pass,
    /// Worth a look before adding; it does not stop the add.
    Warn,
    /// The bake could not measure it (`note` says why).
    NotMeasured,
}

/// One number of the review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeReviewLine {
    /// `gap-l-1`, `flip-l-1`, `feet` or `stretch`.
    pub id: String,
    /// "Hand gap", "Elbow", "Foot slide", "Rig stretch".
    pub label: String,
    /// The bone and change it is about, for a reach.
    pub about: Option<String>,
    /// What was measured, in words: "1.4 cm", "flips at frame 18".
    pub value: String,
    /// The limit it is held to, in words: "2 cm", "no flip", "1.9 cm".
    pub limit: String,
    /// The measured number and its limit, in `unit`, when there are numbers.
    pub measured: Option<f32>,
    pub max: Option<f32>,
    pub unit: Option<String>,
    pub status: RecipeReviewStatus,
    pub note: Option<String>,
}

/// The review of a finished bake.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBakeReview {
    pub lines: Vec<RecipeReviewLine>,
    /// How many lines warn.
    pub warns: u32,
}

fn round(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

fn status(warn: bool) -> RecipeReviewStatus {
    if warn {
        RecipeReviewStatus::Warn
    } else {
        RecipeReviewStatus::Pass
    }
}

fn reach_lines(result: &RecipeBakeResult) -> Vec<RecipeReviewLine> {
    let mut out = Vec::new();
    for reach in &result.reaches {
        let foot = reach.bone.ends_with("Foot");
        let about = Some(format!("{} ({})", reach.bone, reach.layer));
        out.push(RecipeReviewLine {
            id: format!("gap-{}", reach.layer),
            label: if foot { "Foot gap" } else { "Hand gap" }.to_string(),
            about: about.clone(),
            value: reach
                .gap_cm
                .map_or_else(|| "not measured".to_string(), |cm| format!("{cm:.1} cm")),
            limit: format!("{GAP_LIMIT_CM:.0} cm"),
            measured: reach.gap_cm,
            max: Some(GAP_LIMIT_CM),
            unit: Some("cm".to_string()),
            // A reach whose end was never measured did not reach.
            status: status(!reach.reached),
            note: match reach.gap_cm {
                None => Some("Its end frame was not measured, so it counts as short.".to_string()),
                Some(_) => Some(format!("At frame {}, the reach's end.", reach.frame)),
            },
        });
        let joint = if foot { "Knee" } else { "Elbow" };
        out.push(RecipeReviewLine {
            id: format!("flip-{}", reach.layer),
            label: joint.to_string(),
            about,
            value: reach.flip_frame.map_or_else(
                || "bends as the clip has it".to_string(),
                |f| format!("flips at frame {f}"),
            ),
            limit: "no flip".to_string(),
            measured: None,
            max: None,
            unit: None,
            status: status(reach.flip_frame.is_some()),
            note: reach.flip_frame.map(|_| {
                format!(
                    "The {} bends the other way than the clip has it from there.",
                    joint.to_lowercase()
                )
            }),
        });
    }
    out
}

fn feet_line(result: &RecipeBakeResult) -> RecipeReviewLine {
    let max = round(result.base_slide_cm + SLIDE_OVER_BASE_CM);
    let foot = result.slide_foot.as_deref().map(|f| format!(" ({f})"));
    RecipeReviewLine {
        id: "feet".to_string(),
        label: "Foot slide".to_string(),
        about: None,
        value: format!("{:.1} cm{}", result.slide_cm, foot.unwrap_or_default()),
        limit: format!("{max:.1} cm"),
        measured: Some(result.slide_cm),
        max: Some(max),
        unit: Some("cm".to_string()),
        status: status(result.slide_cm > max),
        note: Some(format!(
            "The base clip slides {:.1} cm on this model; up to {SLIDE_OVER_BASE_CM} cm more passes.",
            result.base_slide_cm
        )),
    }
}

fn stretch_line(result: &RecipeBakeResult) -> RecipeReviewLine {
    let base = |mm: f32| round(mm + STRETCH_OVER_BASE_MM);
    let Some(mm) = result.stretch_mm else {
        return RecipeReviewLine {
            id: "stretch".to_string(),
            label: "Rig stretch".to_string(),
            about: None,
            value: "not measured".to_string(),
            limit: result
                .base_stretch_mm
                .map_or_else(|| "none".to_string(), |b| format!("{:.1} mm", base(b))),
            measured: None,
            max: result.base_stretch_mm.map(base),
            unit: Some("mm".to_string()),
            status: RecipeReviewStatus::NotMeasured,
            note: result.stretch_note.clone(),
        };
    };
    let max = result.base_stretch_mm.map(base);
    RecipeReviewLine {
        id: "stretch".to_string(),
        label: "Rig stretch".to_string(),
        about: None,
        value: format!("{mm:.1} mm"),
        limit: max.map_or_else(|| "none".to_string(), |m| format!("{m:.1} mm")),
        measured: Some(mm),
        max,
        unit: Some("mm".to_string()),
        status: status(max.is_some_and(|m| mm > m)),
        note: Some(match result.base_stretch_mm {
            Some(b) => format!(
                "The base clip stretches {b:.1} mm; up to {STRETCH_OVER_BASE_MM} mm more passes."
            ),
            None => {
                "The base clip's stretch was not measured, so nothing to hold it to.".to_string()
            }
        }),
    }
}

/// The review of `result`: each reach's gap and joint, then the feet and
/// the rig.
pub fn review_of(result: &RecipeBakeResult) -> RecipeBakeReview {
    let mut lines = reach_lines(result);
    lines.push(feet_line(result));
    lines.push(stretch_line(result));
    let warns = lines
        .iter()
        .filter(|l| l.status == RecipeReviewStatus::Warn)
        .count();
    RecipeBakeReview {
        warns: u32::try_from(warns).unwrap_or(u32::MAX),
        lines,
    }
}
