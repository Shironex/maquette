//! The retarget review: each baked clip as good, to look at or failed,
//! from what the Blender run measured on the model. A failed clip is left
//! out of the new version; one to look at goes in unless the person takes
//! it out.

use serde::{Deserialize, Serialize};

/// A planted foot may drift this far before the clip is flagged, cm.
pub const PLANTED_CM: f32 = 1.5;
/// Hips that end farther than this from where they start have keys in
/// the wrong units, metres.
const FAR_M: f32 = 25.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewStatus {
    Good,
    Look,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipReview {
    /// The library clip's id.
    pub clip: String,
    /// Its name in the model.
    pub name: String,
    pub status: ReviewStatus,
    /// One line: "Looks right", "Foot slides 4.2 cm while planted, frames 9 to 14".
    pub note: String,
    pub frames: u32,
    /// How far the root travels in the baked clip, metres; none in place.
    pub travel: Option<f32>,
    /// The worst slide of a planted foot, cm, and the frames it spans.
    pub slide_cm: f32,
    pub slide_frames: Option<[u32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RetargetReview {
    pub clips: Vec<ClipReview>,
    /// How long Blender took, seconds.
    pub seconds: f32,
    /// Model bones the clips drive.
    pub driven: u32,
    /// Humanoid bones with no model bone, which stay at rest.
    pub at_rest: Vec<String>,
    /// The model's limbs were turned this far, at most, to line up with the
    /// clip's rest pose, degrees.
    pub aligned_deg: f32,
    /// What the hips' moves were scaled by: the model's hips height over
    /// the clip's.
    pub hips_scale: f32,
}

impl RetargetReview {
    /// The clips that go into the new version.
    pub fn baked(&self) -> impl Iterator<Item = &ClipReview> {
        self.clips
            .iter()
            .filter(|c| c.status != ReviewStatus::Failed)
    }
}

/// One clip as the Blender run measured it.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ClipRun {
    pub id: String,
    pub name: String,
    pub error: Option<String>,
    pub frames: u32,
    pub travel: f32,
    pub farthest: f32,
    pub slide_cm: f32,
    pub slide_from: u32,
    pub slide_to: u32,
    pub foot: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct RunReport {
    pub clips: Vec<ClipRun>,
    pub seconds: f32,
    pub driven: u32,
    pub aligned_deg: f32,
    pub hips_scale: f32,
}

fn review_clip(run: &ClipRun, root_motion: bool) -> ClipReview {
    let travel = (root_motion && run.travel > 0.05).then_some(run.travel);
    let (status, note) = if let Some(error) = &run.error {
        (ReviewStatus::Failed, format!("Failed: {error}"))
    } else if !run.farthest.is_finite() || run.farthest > FAR_M {
        (
            ReviewStatus::Failed,
            format!(
                "Failed: the hips leave by {:.0} m, so its keys are likely in the wrong units",
                run.farthest
            ),
        )
    } else if run.slide_cm > PLANTED_CM {
        (
            ReviewStatus::Look,
            format!(
                "The {} foot slides {:.1} cm while planted, frames {} to {}",
                run.foot, run.slide_cm, run.slide_from, run.slide_to
            ),
        )
    } else {
        let note = travel.map_or_else(
            || "Looks right".to_string(),
            |m| format!("Looks right, {m:.2} m root motion"),
        );
        (ReviewStatus::Good, note)
    };
    ClipReview {
        clip: run.id.clone(),
        name: run.name.clone(),
        status,
        note,
        frames: run.frames,
        travel,
        slide_cm: run.slide_cm,
        slide_frames: (run.slide_cm > PLANTED_CM).then_some([run.slide_from, run.slide_to]),
    }
}

/// The review of a run. `root_motion` says, per clip id, whether it keeps
/// its travel; `at_rest` names the humanoid bones left unmapped.
pub(crate) fn review(
    report: &RunReport,
    root_motion: impl Fn(&str) -> bool,
    at_rest: Vec<String>,
) -> RetargetReview {
    RetargetReview {
        clips: report
            .clips
            .iter()
            .map(|c| review_clip(c, root_motion(&c.id)))
            .collect(),
        seconds: report.seconds,
        driven: report.driven,
        at_rest,
        aligned_deg: report.aligned_deg,
        hips_scale: report.hips_scale,
    }
}
