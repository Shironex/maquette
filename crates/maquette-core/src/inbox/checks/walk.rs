//! The walk row: a walk or run clip that carries the model backwards, its
//! root or hips travelling against the way it faces. A clip like that plays
//! as a moonwalk in a scene that moves the model forward.

use super::{Check, CheckStatus, check};
use crate::inbox::analyse::GlbReport;
use crate::inbox::vrm::Axis;

/// Travel at more than this angle from the face reads as against it: the
/// cosine of 120 degrees.
const AGAINST: f32 = -0.5;

/// A clip whose name says it walks or runs.
fn walks(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    ["walk", "run", "jog", "sprint"]
        .iter()
        .any(|w| name.contains(w))
}

/// The axis a flat direction (x, z) points along, as the findings write it.
fn axis(along: [f32; 2]) -> &'static str {
    Axis::nearest(along[0].atan2(along[1]).to_degrees()).label()
}

/// One row when the model has a walk or run clip that travels and its
/// facing can be read from the rig: a warning for each clip that goes
/// against the face, else a pass. Nothing is changed, so it never fails.
pub(super) fn walk_check(report: &GlbReport) -> Option<Check> {
    let deg = report.vrm.as_ref()?.pose.facing_deg?;
    let (s, c) = deg.to_radians().sin_cos();
    let face = [s, c];
    let facing = Axis::nearest(deg).label();
    let read: Vec<(&str, [f32; 2], f32, bool)> = report
        .clip_travel
        .iter()
        .filter(|t| walks(&t.clip))
        .map(|t| {
            let length = t.along[0].hypot(t.along[1]);
            let cos = (t.along[0] * face[0] + t.along[1] * face[1]) / length.max(f32::EPSILON);
            (t.clip.as_str(), t.along, length, cos < AGAINST)
        })
        .collect();
    let line = |(clip, along, length, _): &(&str, [f32; 2], f32, bool)| {
        format!("{clip} moves {length:.2} m along {}", axis(*along))
    };
    let against: Vec<String> = read.iter().filter(|r| r.3).map(line).collect();
    if against.is_empty() {
        let first = read.first()?;
        return Some(check(
            "walk-facing",
            CheckStatus::Pass,
            "Walks the way it faces",
            format!("{}, the way the model faces ({facing}).", line(first)),
        ));
    }
    Some(check(
        "walk-facing",
        CheckStatus::Warning,
        "Walk travels against the face",
        format!(
            "{} while the model faces {facing}: it walks backwards. Turn the model to face its walk, or keep the clip as a backward walk.",
            against.join("; ")
        ),
    ))
}
