//! Grips in a bake: each grip change as the bake laid it (the frames it
//! holds on, how many of its bones the model has and which it lacks) and
//! the review's line for it. A rig without fingers still takes the hand and
//! the arm; the line says what was left out.

use serde::{Deserialize, Serialize};

use super::RecipeBakeResult;
use super::review::{RecipeReviewLine, RecipeReviewStatus};

/// One grip as the bake laid it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeGripResult {
    /// The change's id.
    pub layer: String,
    /// The hand that holds, by VRM name.
    pub bone: String,
    /// The library grip and revision it was copied from, when it has a link.
    pub entry: Option<String>,
    pub revision: Option<u32>,
    /// The bake frames it holds on, first and last; none when no frame
    /// falls within its range.
    pub frames: Option<[u32; 2]>,
    /// How many bones its pose turns, and how many of those the model has.
    pub bones: u32,
    pub posed: u32,
    /// The bones of its pose the model lacks, by VRM name.
    pub missing: Vec<String>,
    /// A grip on both hands: how far the hands move closer or further apart
    /// than at its first whole frame, cm, and the frame of the most. None
    /// for one hand (and in a record from before).
    #[serde(default)]
    #[specta(optional)]
    pub gap_cm: Option<f32>,
    #[serde(default)]
    #[specta(optional)]
    pub gap_frame: Option<u32>,
}

/// Whether `bone` is a finger bone, by its VRM name.
fn is_finger(bone: &str) -> bool {
    ["Thumb", "Index", "Middle", "Ring", "Little"]
        .iter()
        .any(|finger| bone.contains(finger))
}

fn left_out(result: &RecipeBakeResult, grip: &RecipeGripResult) -> String {
    let fingers = grip.missing.iter().filter(|b| is_finger(b)).count();
    if fingers == grip.missing.len() && grip.posed > 0 {
        return format!(
            "{} has no bone for {} of its finger bones, so the grip poses the hand and arm only there.",
            result.asset_name, fingers
        );
    }
    let named: Vec<&str> = grip.missing.iter().take(4).map(String::as_str).collect();
    let more = grip.missing.len() - named.len();
    format!(
        "{} has no bone for {}{}, so the grip leaves {}.",
        result.asset_name,
        named.join(", "),
        if more > 0 {
            format!(" and {more} more")
        } else {
            String::new()
        },
        if grip.missing.len() == 1 {
            "it"
        } else {
            "them"
        }
    )
}

/// A line per grip: how many of its bones the model takes.
pub(super) fn grip_lines(result: &RecipeBakeResult) -> Vec<RecipeReviewLine> {
    result
        .grips
        .iter()
        .map(|grip| {
            let span = match grip.frames {
                Some([first, last]) => format!("Frames {first} to {last}."),
                None => "No frame falls within its range, so it poses nothing.".to_string(),
            };
            let whole = grip.missing.is_empty() && grip.frames.is_some();
            let note = if grip.missing.is_empty() {
                span
            } else {
                format!("{span} {}", left_out(result, grip))
            };
            RecipeReviewLine {
                id: format!("grip-{}", grip.layer),
                label: "Grip".to_string(),
                about: Some(format!("{} ({})", grip.bone, grip.layer)),
                value: format!("{} of {} bones", grip.posed, grip.bones),
                limit: "every bone".to_string(),
                // More is better here, so no number is held to a limit.
                measured: None,
                max: None,
                unit: None,
                status: if whole {
                    RecipeReviewStatus::Pass
                } else {
                    RecipeReviewStatus::Warn
                },
                note: Some(note),
            }
        })
        .collect()
}
