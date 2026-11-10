//! How far a humanoid bone may turn under its parent before a pose looks
//! broken: one fixed table, the same for every model, read in the turns a
//! pose key holds (degrees about the model's own axes, X to its left, Y up,
//! Z ahead). It only warns, and only keyed poses are judged: the clips
//! break these on purpose now and then. The web app keeps a copy
//! (`state/recipe-keys.ts`), checked against `fixtures/pose_keys.json`.
//!
//! The table names the left side; the right side is its mirror (the X
//! range as it is, the Y and Z ranges flipped).

use serde::{Deserialize, Serialize};

use super::pose_keys::key_frame;
use super::timeline::{plain, timeline};
use super::{LayerKind, RecipeFile};

/// Degrees each way, low then high, per axis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct JointLimits {
    pub x: [f32; 2],
    pub y: [f32; 2],
    pub z: [f32; 2],
}

const fn l(x: [f32; 2], y: [f32; 2], z: [f32; 2]) -> JointLimits {
    JointLimits { x, y, z }
}

/// The body, middle and left side. Hips move the whole body, so they have
/// no limits.
const BODY: &[(&str, JointLimits)] = &[
    ("spine", l([-30.0, 45.0], [-40.0, 40.0], [-30.0, 30.0])),
    ("chest", l([-25.0, 40.0], [-45.0, 45.0], [-30.0, 30.0])),
    ("upperChest", l([-20.0, 30.0], [-30.0, 30.0], [-20.0, 20.0])),
    ("neck", l([-40.0, 50.0], [-60.0, 60.0], [-40.0, 40.0])),
    ("head", l([-40.0, 40.0], [-70.0, 70.0], [-35.0, 35.0])),
    ("jaw", l([-5.0, 30.0], [-10.0, 10.0], [-10.0, 10.0])),
    ("leftEye", l([-25.0, 25.0], [-35.0, 35.0], [-5.0, 5.0])),
    (
        "leftShoulder",
        l([-20.0, 20.0], [-30.0, 30.0], [-15.0, 30.0]),
    ),
    (
        "leftUpperArm",
        l([-90.0, 90.0], [-110.0, 50.0], [-95.0, 110.0]),
    ),
    (
        "leftLowerArm",
        l([-90.0, 90.0], [-150.0, 5.0], [-15.0, 15.0]),
    ),
    ("leftHand", l([-40.0, 40.0], [-80.0, 80.0], [-70.0, 70.0])),
    (
        "leftUpperLeg",
        l([-120.0, 30.0], [-45.0, 45.0], [-30.0, 60.0]),
    ),
    (
        "leftLowerLeg",
        l([-5.0, 150.0], [-15.0, 15.0], [-15.0, 15.0]),
    ),
    ("leftFoot", l([-50.0, 30.0], [-30.0, 30.0], [-30.0, 30.0])),
    ("leftToes", l([-40.0, 60.0], [-10.0, 10.0], [-10.0, 10.0])),
];

/// A left finger joint past the thumb: it curls down (Z), spreads a little
/// (Y) and barely twists (X).
const FINGER: JointLimits = l([-15.0, 15.0], [-25.0, 25.0], [-100.0, 20.0]);
/// A left thumb joint, which moves every way.
const THUMB: JointLimits = l([-60.0, 60.0], [-60.0, 60.0], [-60.0, 60.0]);

const FINGERS: [&str; 4] = ["Index", "Middle", "Ring", "Little"];
const SEGMENTS: [&str; 3] = ["Proximal", "Intermediate", "Distal"];
const THUMB_SEGMENTS: [&str; 3] = ["Metacarpal", "Proximal", "Distal"];

fn left_limits(bone: &str) -> Option<JointLimits> {
    if let Some((_, limits)) = BODY.iter().find(|(name, _)| *name == bone) {
        return Some(*limits);
    }
    let finger = bone.strip_prefix("left")?;
    if let Some(segment) = finger.strip_prefix("Thumb") {
        return THUMB_SEGMENTS.contains(&segment).then_some(THUMB);
    }
    FINGERS.iter().find_map(|f| {
        let segment = finger.strip_prefix(f)?;
        SEGMENTS.contains(&segment).then_some(FINGER)
    })
}

/// How far `bone` (a VRM humanoid bone name) may turn, or none for a bone
/// the table leaves free (the hips, and names it does not know).
pub fn joint_limits(bone: &str) -> Option<JointLimits> {
    if let Some(rest) = bone.strip_prefix("right") {
        let flip = |[a, b]: [f32; 2]| [-b, -a];
        return left_limits(&format!("left{rest}")).map(|m| JointLimits {
            x: m.x,
            y: flip(m.y),
            z: flip(m.z),
        });
    }
    left_limits(bone)
}

/// What each axis does to `bone`, in words: an upright bone bends, twists
/// and leans to the side; an arm, hand or finger, lying along X at rest,
/// twists, swings and raises.
pub fn axis_words(bone: &str) -> [&'static str; 3] {
    let arm = [
        "Shoulder", "Arm", "Hand", "Thumb", "Index", "Middle", "Ring", "Little",
    ]
    .iter()
    .any(|part| bone.contains(part));
    if arm {
        ["twist", "swing", "raise"]
    } else {
        ["bend", "twist", "side"]
    }
}

/// One axis of a bone turned past its limit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LimitHit {
    pub bone: String,
    /// `x`, `y` or `z`.
    pub axis: String,
    /// What the axis does to the bone: "twist", "bend", "raise".
    pub word: String,
    /// The turn, and the limit on that side, degrees.
    pub degrees: f32,
    pub limit: f32,
}

impl LimitHit {
    /// How far past its limit, degrees.
    pub fn over(&self) -> f32 {
        (self.degrees.abs() - self.limit.abs()).max(0.0)
    }
}

/// Each axis of `turn` (degrees, X then Y then Z) past `bone`'s limits.
pub fn over_limits(bone: &str, turn: [f32; 3]) -> Vec<LimitHit> {
    let Some(limits) = joint_limits(bone) else {
        return Vec::new();
    };
    let words = axis_words(bone);
    [limits.x, limits.y, limits.z]
        .into_iter()
        .zip(turn)
        .enumerate()
        .filter_map(|(i, ([low, high], degrees))| {
            let limit = if degrees < low {
                low
            } else if degrees > high {
                high
            } else {
                return None;
            };
            Some(LimitHit {
                bone: bone.to_string(),
                axis: ["x", "y", "z"][i].to_string(),
                word: words[i].to_string(),
                degrees,
                limit,
            })
        })
        .collect()
}

/// A pose key that turns a bone past its limit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KeyLimitHit {
    /// The pose key change.
    pub layer: String,
    /// The key's clip and its own frame.
    pub clip: u32,
    pub frame: u32,
    /// Where it shows in the bake; none when a trim cuts it out.
    pub bake_frame: Option<u32>,
    pub hit: LimitHit,
}

/// Every keyed pose of `recipe` judged against the table.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KeyedLimits {
    /// How many keyed bone turns were judged.
    pub judged: u32,
    /// The turns past a limit, the furthest past first.
    pub hits: Vec<KeyLimitHit>,
}

/// Judges every key of every pose key change of `recipe`, as written: a
/// key's own turns, not the clip under them.
pub fn keyed_limits(recipe: &RecipeFile) -> KeyedLimits {
    let t = timeline(recipe);
    let whole = plain(recipe);
    let mut out = KeyedLimits::default();
    for layer in recipe.layers.iter().filter(|l| l.kind == LayerKind::Pose) {
        for key in &layer.keys {
            let bake_frame = if whole {
                (key.frame <= t.frames).then_some(key.frame)
            } else {
                key_frame(&t, key.clip, key.frame)
            };
            for (bone, turn) in &key.pose {
                out.judged += 1;
                out.hits
                    .extend(over_limits(bone, *turn).into_iter().map(|hit| KeyLimitHit {
                        layer: layer.id.clone(),
                        clip: key.clip,
                        frame: key.frame,
                        bake_frame,
                        hit,
                    }));
            }
        }
    }
    out.hits
        .sort_by(|a, b| b.hit.over().total_cmp(&a.hit.over()));
    out
}
