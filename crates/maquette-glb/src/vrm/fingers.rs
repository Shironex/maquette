//! How far the fingers curl at rest. VRM's T-pose lays the four fingers
//! straight along the hand, nails up, and the thumb straight, 45 degrees
//! between the hand and the front; auto rigs often leave them bent.
//!
//! Each finger is a chain of three humanoid bones (the thumb's metacarpal,
//! proximal and distal; the others' proximal, intermediate and distal)
//! ending at the distal bone's child node, when it has one. Its curl is
//! the sum of the bends between those bones and, for the four fingers, how
//! far the first bone dips out of the palm towards it. The palm is read
//! from the hand: its axis runs from the wrist to the middle knuckle and its
//! normal points to the palm side (the left hand's axis crossed with the
//! little to the index knuckle).
//!
//! A hand whose knuckles all sit nearer the wrist than [`BUNCHED`] of the
//! height has its finger bones bunched up, as a broken auto rig leaves
//! them: its fingers are not measured, and straightening them cannot work.

use serde::{Deserialize, Serialize};

use super::pose::Joints;
use crate::math::Vec3;

/// A hand whose knuckles sit nearer the wrist than this share of the
/// height has its finger bones bunched up.
pub const BUNCHED: f32 = 0.015;

/// The fingers, VRM's names without the side, thumb first.
pub const FINGERS: [&str; 5] = ["Thumb", "Index", "Middle", "Ring", "Little"];

/// One finger's curl at rest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FingerCurl {
    /// The finger with its side, such as `leftIndex`.
    pub finger: String,
    /// Degrees it bends in all, 0 when straight.
    pub curl_deg: f32,
}

/// What the measure found over both hands.
pub(crate) struct Fingers {
    pub(crate) curls: Vec<FingerCurl>,
    /// The sides whose finger bones sit bunched at the wrist.
    pub(crate) bunched: Vec<String>,
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(v: Vec3) -> Option<Vec3> {
    let n = dot(v, v).sqrt();
    (n > 1e-9).then(|| [v[0] / n, v[1] / n, v[2] / n])
}

fn angle(a: Vec3, b: Vec3) -> Option<f32> {
    let (a, b) = (unit(a)?, unit(b)?);
    Some(dot(a, b).clamp(-1.0, 1.0).acos().to_degrees())
}

/// The bones of one finger, VRM names with the side.
pub(crate) fn chain(side: &str, finger: &str) -> [String; 3] {
    let joints = if finger == "Thumb" {
        ["Metacarpal", "Proximal", "Distal"]
    } else {
        ["Proximal", "Intermediate", "Distal"]
    };
    joints.map(|j| format!("{side}{finger}{j}"))
}

/// The hand's palm normal, towards the palm; `None` without the knuckles.
fn palm(joints: &Joints, side: &str) -> Option<Vec3> {
    let wrist = joints.get(&format!("{side}Hand"))?;
    let knuckle = |f: &str| joints.get(&format!("{side}{f}Proximal"));
    let first = knuckle("Index")?;
    let last = knuckle("Little").or_else(|| knuckle("Ring"))?;
    let middle =
        knuckle("Middle").unwrap_or_else(|| [0, 1, 2].map(|i| f32::midpoint(first[i], last[i])));
    let axis = unit(sub(middle, wrist))?;
    let normal = cross(axis, sub(first, last));
    let normal = if side == "left" {
        normal
    } else {
        normal.map(|v| -v)
    };
    unit(normal)
}

/// True when the hand has knuckles and every one sits nearer the wrist
/// than [`BUNCHED`] of `height`.
fn bunched(joints: &Joints, side: &str, height: f32) -> bool {
    let Some(wrist) = joints.get(&format!("{side}Hand")) else {
        return false;
    };
    FINGERS[1..]
        .iter()
        .filter_map(|f| joints.get(&format!("{side}{f}Proximal")))
        .map(|k| dot(sub(k, wrist), sub(k, wrist)).sqrt())
        .reduce(f32::max)
        .is_some_and(|far| far < BUNCHED * height)
}

/// Measures the fingers of both hands. `tip` gives where a bone ends (its
/// child node), `height` the model's, metres.
pub(crate) fn measure(
    joints: &Joints,
    tip: impl Fn(&str) -> Option<Vec3>,
    height: Option<f32>,
) -> Fingers {
    let mut out = Fingers {
        curls: Vec::new(),
        bunched: Vec::new(),
    };
    for side in ["left", "right"] {
        if height.is_some_and(|h| bunched(joints, side, h)) {
            out.bunched.push(side.to_string());
            continue;
        }
        let palm = palm(joints, side);
        for finger in FINGERS {
            let bones = chain(side, finger);
            let Some(points) = bones
                .iter()
                .map(|b| joints.get(b))
                .collect::<Option<Vec<Vec3>>>()
            else {
                continue;
            };
            let mut points = points;
            points.extend(tip(&bones[2]));
            let segs: Vec<Vec3> = points.windows(2).map(|w| sub(w[1], w[0])).collect();
            let mut curl: f32 = segs.windows(2).filter_map(|w| angle(w[0], w[1])).sum();
            if finger != "Thumb"
                && let (Some(normal), Some(first)) = (palm, unit(segs[0]))
            {
                curl += dot(first, normal)
                    .clamp(-1.0, 1.0)
                    .asin()
                    .to_degrees()
                    .max(0.0);
            }
            out.curls.push(FingerCurl {
                finger: format!("{side}{finger}"),
                curl_deg: curl,
            });
        }
    }
    out
}
