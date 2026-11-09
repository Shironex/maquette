//! Pins in a bake: what the bake script measures of each (how far the
//! pinned hand or foot drifts from its spot while it holds whole, and how
//! far it would have moved without the pin), the result the review reads,
//! and the review's drift line (1 cm).

use serde::{Deserialize, Serialize};

use super::RecipeBakeResult;
use super::review::{RecipeReviewLine, RecipeReviewStatus};

/// A pinned hand or foot may drift this far from its spot while the pin
/// holds whole, cm.
pub const PIN_DRIFT_LIMIT_CM: f32 = 1.0;

/// One pin, as the bake script reports it.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct PinRun {
    pub layer: String,
    pub bone: String,
    pub first: u32,
    pub last: u32,
    /// The worst drift from its spot while it holds whole, metres, and its
    /// frame; none when its eases leave no frame held whole.
    pub drift: Option<f32>,
    pub frame: Option<u32>,
    /// How far the bone moved from the spot over the same frames before the
    /// pin held it, metres.
    pub base_drift: Option<f32>,
}

/// One pin as the bake measured it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipePinResult {
    /// The change's id.
    pub layer: String,
    /// The hand or foot, by VRM name.
    pub bone: String,
    /// The bake frames it holds on, first and last.
    pub first: u32,
    pub last: u32,
    /// The worst drift from its spot while it holds whole, cm, and the frame
    /// of it; none when its eases leave no frame held whole.
    pub drift_cm: Option<f32>,
    pub frame: Option<u32>,
    /// How far the bone moves from the spot over those frames without the
    /// pin, cm.
    pub base_drift_cm: Option<f32>,
}

fn cm(metres: Option<f32>) -> Option<f32> {
    metres.map(|m| (m * 10_000.0).round() / 100.0)
}

pub(crate) fn pin_result(run: &PinRun) -> RecipePinResult {
    RecipePinResult {
        layer: run.layer.clone(),
        bone: run.bone.clone(),
        first: run.first,
        last: run.last,
        drift_cm: cm(run.drift),
        frame: run.drift.and(run.frame),
        base_drift_cm: cm(run.base_drift),
    }
}

/// A drift line per pin.
pub(super) fn pin_lines(result: &RecipeBakeResult) -> Vec<RecipeReviewLine> {
    result
        .pins
        .iter()
        .map(|pin| {
            let span = format!("Frames {} to {}.", pin.first, pin.last);
            let without = pin
                .base_drift_cm
                .map(|b| format!(" Without the pin it moves {b:.1} cm."))
                .unwrap_or_default();
            let (value, status, note) = match pin.drift_cm {
                None => (
                    "not measured".to_string(),
                    RecipeReviewStatus::NotMeasured,
                    format!("{span} Its eases leave no frame where it holds whole."),
                ),
                Some(d) => (
                    format!("{d:.1} cm"),
                    if d > PIN_DRIFT_LIMIT_CM {
                        RecipeReviewStatus::Warn
                    } else {
                        RecipeReviewStatus::Pass
                    },
                    match pin.frame {
                        Some(f) if d > PIN_DRIFT_LIMIT_CM => {
                            format!("{span} It leaves its spot most at frame {f}.{without}")
                        }
                        _ => format!("{span}{without}"),
                    },
                ),
            };
            RecipeReviewLine {
                id: format!("pin-{}", pin.layer),
                label: "Pin drift".to_string(),
                about: Some(format!("{} ({})", pin.bone, pin.layer)),
                value,
                limit: format!("{PIN_DRIFT_LIMIT_CM:.0} cm"),
                measured: pin.drift_cm,
                max: Some(PIN_DRIFT_LIMIT_CM),
                unit: Some("cm".to_string()),
                status,
                note: Some(note),
            }
        })
        .collect()
}
