//! The standard camera set: three quarter, front, top down and two close
//! ups. Angles are in degrees: `azimuth` round from the front towards the
//! model's +X (the right of the picture in the front view), `elevation` up.
//! glTF's front is +Z, so the front view stands on +Z and looks along -Z;
//! three.js uses the same axes, and Blender imports them as -Y for the front
//! with Z up, which is where `render_views.py` puts its cameras.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// The three quarter view: front right, a little above. The Board's
/// thumbnails stand there, and object refs are asked for "a front
/// three-quarter view", so a render from here lines up with a ref.
pub const THREE_QUARTER: ViewAngle = ViewAngle {
    azimuth: 38.0,
    elevation: 22.0,
};
/// Straight in front, a little above, as a front ref is drawn.
pub const FRONT: ViewAngle = ViewAngle {
    azimuth: 0.0,
    elevation: 6.0,
};
/// Straight down, the front at the bottom of the picture.
pub const TOP: ViewAngle = ViewAngle {
    azimuth: 0.0,
    elevation: 90.0,
};
/// How much of the frame a whole model fills, as in a thumbnail.
pub const WHOLE_FILL: f32 = 0.74;
/// How much of the frame a close up's target fills.
pub const CLOSE_FILL: f32 = 0.9;
pub const MAX_CLOSE_UPS: usize = 2;
/// The steepest a three quarter view may look down.
pub const MAX_ELEVATION: f32 = 80.0;
/// The smallest side of a close up's box, as a share of the bounds.
const MIN_BOX_SHARE: f32 = 0.02;
/// Blender keeps object names to 63 bytes.
const MAX_NODE_CHARS: usize = 63;

/// Where a camera stands around a model.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewAngle {
    pub azimuth: f32,
    pub elevation: f32,
}

/// What a view frames.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewTarget {
    Whole,
    /// One half of the model along its longest side: 0 the left, the top or
    /// the front, 1 the other.
    Half(u8),
    /// The object of this name with everything under it.
    Node(String),
    /// Two corners in shares of the bounding box: x from left to right, y
    /// from bottom to top, z from back to front.
    Box([[f32; 3]; 2]),
}

/// One view of the set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardView {
    /// `three-quarter`, `front`, `top`, `close-up-1` or `close-up-2`: the
    /// picture's file name too.
    pub id: String,
    pub azimuth: f32,
    pub elevation: f32,
    pub fill: f32,
    pub target: ViewTarget,
}

/// The ids of the set, in the order the pictures are made and listed.
pub const VIEW_IDS: [&str; 5] = ["three-quarter", "front", "top", "close-up-1", "close-up-2"];

fn trim(v: f32) -> String {
    crate::prepare::trim_number(v)
}

/// Where a camera at `azimuth` and `elevation` stands, in words an agent can
/// check a picture by.
pub fn stands(azimuth: f32, elevation: f32) -> String {
    if elevation >= 89.5 {
        return "straight above, looking down, the model's front (+Z) at the bottom of the picture"
            .to_string();
    }
    let round = if azimuth.abs() < 0.5 {
        "straight in front, on the model's +Z side".to_string()
    } else {
        let side = if azimuth > 0.0 {
            "+X, the right of the front view"
        } else {
            "-X, the left of the front view"
        };
        format!(
            "{} degrees round from the front towards {side}",
            trim(azimuth.abs())
        )
    };
    format!("{round}, {} degrees up", trim(elevation))
}

impl StandardView {
    /// Where its camera stands (see [`stands`]).
    pub fn stands(&self) -> String {
        stands(self.azimuth, self.elevation)
    }
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

fn check_angle(angle: ViewAngle) -> CoreResult<()> {
    let ok = angle.azimuth.is_finite()
        && angle.elevation.is_finite()
        && (-180.0..=180.0).contains(&angle.azimuth)
        && (0.0..=MAX_ELEVATION).contains(&angle.elevation);
    if ok {
        return Ok(());
    }
    Err(invalid(format!(
        "The three quarter angle takes an azimuth from -180 to 180 degrees (0 is the front, positive towards the model's +X) and an elevation from 0 to {MAX_ELEVATION} degrees, not {} and {}.",
        angle.azimuth, angle.elevation
    )))
}

fn check_target(target: &ViewTarget) -> CoreResult<()> {
    match target {
        ViewTarget::Whole => Ok(()),
        ViewTarget::Half(half) if *half <= 1 => Ok(()),
        ViewTarget::Half(half) => Err(invalid(format!(
            "A close up of a half takes 0 or 1, not {half}."
        ))),
        ViewTarget::Node(name) => {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > MAX_NODE_CHARS {
                return Err(invalid(
                    "A close up of a part takes the object's name, 1 to 63 characters.",
                ));
            }
            Ok(())
        }
        ViewTarget::Box([a, b]) => {
            let shares = a.iter().chain(b).all(|v| (0.0..=1.0).contains(v));
            let sized = a.iter().zip(b).all(|(x, y)| (y - x).abs() >= MIN_BOX_SHARE);
            if shares && sized {
                return Ok(());
            }
            Err(invalid(
                "A close up's box is two corners in shares of the bounding box, each from 0 to 1 (x left to right, y bottom to top, z back to front), at least 0.02 apart on every side.",
            ))
        }
    }
}

/// The five views of a model. `angle` moves the three quarter view (and the
/// close ups, which look from the same side) to match a ref drawn from
/// elsewhere; `close_ups` names what the close ups frame, the two halves of
/// the model when left out.
///
/// # Errors
///
/// Refuses an angle out of range, more than two close ups and a close up
/// target that names nothing.
pub fn standard_set(
    angle: Option<ViewAngle>,
    close_ups: &[ViewTarget],
) -> CoreResult<Vec<StandardView>> {
    let quarter = angle.unwrap_or(THREE_QUARTER);
    check_angle(quarter)?;
    if close_ups.len() > MAX_CLOSE_UPS {
        return Err(invalid(format!(
            "The set has {MAX_CLOSE_UPS} close ups, not {}.",
            close_ups.len()
        )));
    }
    let mut targets = close_ups.to_vec();
    for half in 0..2 {
        if targets.len() < MAX_CLOSE_UPS {
            // The halves a named close up leaves free, the first one first.
            let free = if targets.contains(&ViewTarget::Half(half)) {
                1 - half
            } else {
                half
            };
            targets.push(ViewTarget::Half(free));
        }
    }
    let whole = |id: &str, at: ViewAngle| StandardView {
        id: id.to_string(),
        azimuth: at.azimuth,
        elevation: at.elevation,
        fill: WHOLE_FILL,
        target: ViewTarget::Whole,
    };
    let mut set = vec![
        whole(VIEW_IDS[0], quarter),
        whole(VIEW_IDS[1], FRONT),
        whole(VIEW_IDS[2], TOP),
    ];
    for (n, target) in targets.into_iter().enumerate() {
        check_target(&target)?;
        if matches!(target, ViewTarget::Whole) {
            return Err(invalid(
                "A close up frames a half, a part or a box, not the whole model.",
            ));
        }
        let target = match target {
            ViewTarget::Node(name) => ViewTarget::Node(name.trim().to_string()),
            other => other,
        };
        set.push(StandardView {
            id: VIEW_IDS[3 + n].to_string(),
            azimuth: quarter.azimuth,
            elevation: quarter.elevation,
            fill: CLOSE_FILL,
            target,
        });
    }
    Ok(set)
}
