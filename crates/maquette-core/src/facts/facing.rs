//! Which way a model faces, and how that was decided. A rigged model says
//! so itself: its skeleton has a left and a right. A prop does not, so its
//! front is the file format's and the answer says it was not measured.
//!
//! The axes, layer by layer. glTF and three.js share one frame: Y up, and a
//! model's front is +Z. Blender imports that as Z up with the front at -Y;
//! X is the same in all three. A turn is in degrees from +Z about Y, towards
//! +X positive, which is also the sense of a layout entry's Y rotation.

use serde::Serialize;

use crate::inbox::analyse::GlbReport;
use crate::inbox::math::point;
use crate::inbox::vrm::Axis;
use crate::layout::{Placement, matrix};

/// How a facing was decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FacingHow {
    /// Read from the skeleton's rest pose: its left crossed with up.
    Rig,
    /// Not measured: the front glTF gives every model.
    Convention,
}

/// How far a facing can be trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sure {
    Measured,
    Assumed,
}

/// A front as an axis and a turn.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Front {
    /// The nearest of +z, -z, +x and -x, in glTF and three.js terms.
    pub axis: Axis,
    /// Degrees from +Z about Y, towards +X positive, from -180 to 180.
    pub degrees: f32,
    /// The same axis as Blender names it once the file is imported.
    pub blender: &'static str,
}

/// Which way a model faces.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Facing {
    /// In the model file's own frame.
    pub in_file: Front,
    pub how: FacingHow,
    pub sure: Sure,
    /// In the scene, once the layout has turned the model; `None` without a
    /// layout entry.
    pub in_scene: Option<Front>,
    /// How it was decided and how to check it.
    pub note: String,
}

/// What Blender calls a glTF axis: its Y is glTF's -Z.
pub fn blender_axis(axis: Axis) -> &'static str {
    match axis {
        Axis::PlusZ => "-y",
        Axis::MinusZ => "+y",
        Axis::PlusX => "+x",
        Axis::MinusX => "-x",
    }
}

fn wrap(degrees: f32) -> f32 {
    let turned = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    if turned == -180.0 { 180.0 } else { turned }
}

/// The front for a turn of `degrees`.
pub fn front(degrees: f32) -> Front {
    let degrees = wrap(degrees);
    let axis = Axis::nearest(degrees);
    Front {
        axis,
        degrees,
        blender: blender_axis(axis),
    }
}

/// Where a front that points `degrees` round in the file points once
/// `placement` has turned the model: the layout's rotation applied to the
/// direction, read again on the ground plane.
pub fn placed(degrees: f32, placement: &Placement) -> Front {
    let (sin, cos) = degrees.to_radians().sin_cos();
    // The same matrix the layout places bounds with: where a point one step
    // along the front lands, less where the origin lands. Its uniform scale
    // and its translation do not turn a direction.
    let m = matrix(placement);
    let tip = point(&m, [sin, 0.0, cos]);
    let origin = point(&m, [0.0; 3]);
    front((tip[0] - origin[0]).atan2(tip[2] - origin[2]).to_degrees())
}

const RIG_NOTE: &str = "Read from the skeleton's rest pose: the character's left (its left upper leg and arm minus the right ones) crossed with up. render_views' front picture looks at the +Z side, so a model facing another axis shows its side or back there.";
const PROP_NOTE: &str = "Not measured: a model without a skeleton carries no front. +Z is the front glTF gives every model (Blender's -Y), and a provider usually builds a model facing its front ref. Look at render_views' front picture, which looks at the +Z side: if it shows the model's side or back, the real front is the axis that picture's camera would have to stand on.";
const BUILT_NOTE: &str = "Not measured: a built script decides its own front. +Z is the front glTF gives every model, which is -Y in the Blender script. Look at render_views' front picture, which looks at the +Z side, to check the script built it that way.";

/// The facing of the model `report` describes, placed by `placement`.
/// `built` says a script of the project made it.
pub fn facing(report: &GlbReport, placement: Option<&Placement>, built: bool) -> Facing {
    let rig = report.vrm.as_ref().and_then(|v| v.pose.facing_deg);
    let (degrees, how, sure, note) = match rig {
        Some(degrees) => (degrees, FacingHow::Rig, Sure::Measured, RIG_NOTE),
        None if built => (0.0, FacingHow::Convention, Sure::Assumed, BUILT_NOTE),
        None => (0.0, FacingHow::Convention, Sure::Assumed, PROP_NOTE),
    };
    Facing {
        in_file: front(degrees),
        how,
        sure,
        in_scene: placement.map(|p| placed(degrees, p)),
        note: note.to_string(),
    }
}
