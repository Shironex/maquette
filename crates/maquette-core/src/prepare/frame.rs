//! The scale maths: how far the real size and the origin fix-up move and
//! scale a model in its own frame, so the layout editor and the first view
//! measure the model the way the Build scene step will place it.

use serde::{Deserialize, Serialize};

use super::{FixupKind, ModelPrep, RealSize, SizeAxis};
use crate::layout::Bounds;

/// What preparing does to a model's points, before its placement:
/// `p' = scale * (p + offset)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PrepFrame {
    /// The real size scale, 1 without a real size.
    pub scale: f32,
    /// The origin fix-up's shift, metres in the model's own frame.
    pub offset: [f32; 3],
}

impl Default for PrepFrame {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset: [0.0; 3],
        }
    }
}

impl PrepFrame {
    pub fn is_identity(&self) -> bool {
        *self == Self::default()
    }

    /// `bounds` once prepared.
    pub fn apply(&self, (min, max): Bounds) -> Bounds {
        let point = |p: [f32; 3]| {
            [
                self.scale * (p[0] + self.offset[0]),
                self.scale * (p[1] + self.offset[1]),
                self.scale * (p[2] + self.offset[2]),
            ]
        };
        (point(min), point(max))
    }
}

/// Width (X), height (Y) and depth (Z).
pub fn extent((min, max): Bounds) -> [f32; 3] {
    [max[0] - min[0], max[1] - min[1], max[2] - min[2]]
}

/// The side `axis` measures, in metres.
pub fn measured_along(axis: SizeAxis, extent: [f32; 3]) -> f32 {
    match axis {
        SizeAxis::Width => extent[0],
        SizeAxis::Height => extent[1],
        SizeAxis::Depth => extent[2],
        SizeAxis::Longest => extent[0].max(extent[1]).max(extent[2]),
    }
}

/// The uniform scale that makes a model of `extent` measure `size`, or
/// `None` when that side is flat or the numbers are not usable.
pub fn scale_for(size: &RealSize, extent: [f32; 3]) -> Option<f32> {
    let measured = measured_along(size.axis, extent);
    let target = size.metres();
    let scale = target / measured;
    (measured > 1e-6 && target > 0.0 && scale.is_finite()).then_some(scale)
}

/// The frame `prep` gives a model with `bounds`. The origin fix-up moves
/// the bottom centre to the origin; the real size scales about it. Other
/// fix-ups keep the bounds (a removed loose part can shrink them a little,
/// which the Build scene step measures after the fact).
pub fn frame_of(prep: Option<&ModelPrep>, bounds: Option<Bounds>) -> PrepFrame {
    let (Some(prep), Some(bounds)) = (prep, bounds) else {
        return PrepFrame::default();
    };
    let (min, max) = bounds;
    let offset = if prep.fixup(FixupKind::OriginBottom).is_some() {
        [-(min[0] + max[0]) / 2.0, -min[1], -(min[2] + max[2]) / 2.0]
    } else {
        [0.0; 3]
    };
    let scale = prep
        .size
        .and_then(|size| scale_for(&size, extent(bounds)))
        .unwrap_or(1.0);
    PrepFrame { scale, offset }
}
