//! Grips: a hand pose kept once in the grip library and laid on a recipe as
//! a copy with a link back. A grip file is one revision of a library grip,
//! plain JSON written once and never changed:
//!
//! ```json
//! {
//!   "version": 1,
//!   "name": "Longsword, two hands",
//!   "revision": 2,
//!   "weapon": "longsword",
//!   "hands": { "holds": "right", "supports": "left" },
//!   "pose": { "rightHand": [4, 0, -8], "rightIndexProximal": [62, 0, 0] }
//! }
//! ```
//!
//! `pose` is degrees per VRM humanoid bone about the model's own axes, X
//! then Y then Z, as a pose key's turn is, with no frames. It holds only
//! what a hand poses: the hand, its fingers and the arm above it
//! ([`is_grip_bone`]). `hands` says which hand holds and which, if any,
//! supports, and always agrees with the sides the pose turns. `weapon` is
//! what the grip holds ([`WEAPONS`]), which the library sorts it by.
//!
//! A recipe's grip change keeps its own copy of the pose ([`layer_of`]) and
//! a [`GripFrom`](super::GripFrom) link, so the recipe stays whole when it
//! is exported and the library can still offer a newer revision
//! ([`updated`]). The web app keeps step for step copies of these rules
//! (`state/recipe-grips.ts`); `fixtures/grips.json` holds the cases both
//! must agree on.

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::check::{check_line, invalid};
use super::check_edits::check_bone_pose;
use super::{BonePose, MAX_GRIP_BONES, NAME_MAX_CHARS};
use crate::error::{CoreError, CoreResult};
use crate::plans::{is_item_id, trimmed};

mod apply;
mod save;
mod span;

pub use apply::{default_hands, hands_of_pose, layer_of, pose_for, updated};
pub use save::{grip_of, keyed_pose, same_pose};
pub use span::{GRIP_EASE_FRAMES, GripSpan, amount, span_on, spans};

/// The newest grip file version this Maquette reads and writes.
pub const GRIP_VERSION: u32 = 1;
pub const GRIP_SUFFIX: &str = ".grip.json";
/// The biggest grip file read or written. A grip at every limit is a few KB.
pub const MAX_GRIP_FILE_BYTES: u64 = 64 * 1024;

/// A side of the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum GripSide {
    Left,
    Right,
}

impl GripSide {
    pub fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    /// `left` or `right`, as VRM bone names start.
    pub fn word(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    /// The hand on this side, by VRM name.
    pub fn hand(self) -> &'static str {
        match self {
            Self::Left => "leftHand",
            Self::Right => "rightHand",
        }
    }
}

/// Which hand of a grip holds and which, if any, supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GripHolds {
    pub holds: GripSide,
    /// The other hand, when the grip poses both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub supports: Option<GripSide>,
}

/// One revision of a library grip: `r<revision>.grip.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GripFile {
    pub version: u32,
    pub name: String,
    /// From 1; a save to the library adds the next one.
    pub revision: u32,
    /// What it holds, one of [`WEAPONS`].
    pub weapon: String,
    pub hands: GripHolds,
    /// Degrees per hand, finger and arm bone, X then Y then Z; no frames.
    pub pose: BonePose,
}

/// Which hands of a grip a recipe takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum GripHands {
    Both,
    Right,
    Left,
}

impl GripHands {
    pub fn of(side: GripSide) -> Self {
        match side {
            GripSide::Left => Self::Left,
            GripSide::Right => Self::Right,
        }
    }
}

/// What a grip holds: its id in the file, how it reads and the group the
/// library sorts it under ([`WEAPON_GROUPS`]).
pub const WEAPONS: [(&str, &str, &str); 11] = [
    ("longsword", "Longsword", "swords"),
    ("sword", "Sword", "swords"),
    ("rapier", "Rapier", "swords"),
    ("scythe", "Scythe", "polearms"),
    ("staff", "Staff", "polearms"),
    ("spear", "Spear", "polearms"),
    ("dagger", "Dagger", "daggers"),
    ("shield", "Shield", "shields"),
    ("bow", "Bow", "other"),
    ("empty", "Empty hand", "empty"),
    ("other", "Something else", "other"),
];

/// The groups the library sorts grips under, in order.
pub const WEAPON_GROUPS: [(&str, &str); 6] = [
    ("swords", "Swords"),
    ("polearms", "Polearms"),
    ("daggers", "Daggers"),
    ("shields", "Shields"),
    ("empty", "Empty"),
    ("other", "Other"),
];

/// The group `weapon` sorts under; `other` for one this Maquette does not
/// know (a grip from a newer one still lists).
pub fn weapon_group(weapon: &str) -> &'static str {
    WEAPONS
        .iter()
        .find(|(id, _, _)| *id == weapon)
        .map_or("other", |(_, _, group)| group)
}

/// Checks `weapon` is one a grip can be saved with.
///
/// # Errors
///
/// [`CoreError::Invalid`] naming the weapons there are.
pub fn check_weapon(weapon: &str) -> CoreResult<()> {
    if WEAPONS.iter().any(|(id, _, _)| *id == weapon) {
        return Ok(());
    }
    let known: Vec<&str> = WEAPONS.iter().map(|(id, _, _)| *id).collect();
    invalid(format!(
        "{weapon:?} is not a weapon a grip holds. Use one of {}.",
        known.join(", ")
    ))
}

const ARM: [&str; 3] = ["UpperArm", "LowerArm", "Hand"];
const FINGERS: [(&str, [&str; 3]); 5] = [
    ("Thumb", ["Metacarpal", "Proximal", "Distal"]),
    ("Index", ["Proximal", "Intermediate", "Distal"]),
    ("Middle", ["Proximal", "Intermediate", "Distal"]),
    ("Ring", ["Proximal", "Intermediate", "Distal"]),
    ("Little", ["Proximal", "Intermediate", "Distal"]),
];

/// The side `bone` is on, by its VRM name; none for a bone in the middle.
pub fn bone_side(bone: &str) -> Option<GripSide> {
    if bone.starts_with("left") {
        Some(GripSide::Left)
    } else if bone.starts_with("right") {
        Some(GripSide::Right)
    } else {
        None
    }
}

/// `bone` without its side: `rightIndexProximal` reads `IndexProximal`.
fn part(bone: &str) -> Option<&str> {
    bone.strip_prefix("left")
        .or_else(|| bone.strip_prefix("right"))
}

/// Whether a grip may pose `bone`: a hand, one of its fifteen finger bones,
/// or the lower or upper arm above it.
pub fn is_grip_bone(bone: &str) -> bool {
    part(bone).is_some_and(|p| {
        ARM.contains(&p)
            || FINGERS.iter().any(|(finger, segments)| {
                p.strip_prefix(finger)
                    .is_some_and(|s| segments.contains(&s))
            })
    })
}

/// Every bone a grip may pose on `side`, the arm first, then each finger
/// from its base.
pub fn grip_bones(side: GripSide) -> Vec<String> {
    let arm = ARM.iter().map(|p| format!("{}{p}", side.word()));
    let fingers = FINGERS.iter().flat_map(|(finger, segments)| {
        segments
            .iter()
            .map(move |s| format!("{}{finger}{s}", side.word()))
    });
    arm.chain(fingers).collect()
}

/// `bone` on the other side: `leftHand` reads `rightHand`. A bone in the
/// middle stays.
pub fn other_side(bone: &str) -> String {
    match (bone_side(bone), part(bone)) {
        (Some(side), Some(rest)) => format!("{}{rest}", side.other().word()),
        _ => bone.to_string(),
    }
}

fn flipped(v: f32) -> f32 {
    if v == 0.0 { 0.0 } else { -v }
}

/// `pose` on the other side of the body: each bone's partner, turned the
/// same about the model's left axis and the other way about its up and
/// ahead axes, which is the pose in a mirror.
pub fn mirror_pose(pose: &BonePose) -> BonePose {
    pose.iter()
        .map(|(bone, [x, y, z])| (other_side(bone), [*x, flipped(*y), flipped(*z)]))
        .collect()
}

/// Whether `pose` turns a bone on the left, and on the right.
pub fn sides_of(pose: &BonePose) -> (bool, bool) {
    let on = |side| pose.keys().any(|b| bone_side(b) == Some(side));
    (on(GripSide::Left), on(GripSide::Right))
}

/// The bones of `pose` on `side`.
pub(super) fn side_pose(pose: &BonePose, side: GripSide) -> BonePose {
    pose.iter()
        .filter(|(bone, _)| bone_side(bone) == Some(side))
        .map(|(bone, turn)| (bone.clone(), *turn))
        .collect()
}

/// How a grip's hands read on its card: `both hands`, `right hand`,
/// `left hand`, or `either hand` for an empty hand, which holds nothing a
/// side belongs to.
pub fn hands_words(grip: &GripFile) -> &'static str {
    match (grip.hands.supports, grip.hands.holds) {
        (Some(_), _) => "both hands",
        (None, _) if grip.weapon == "empty" => "either hand",
        (None, GripSide::Right) => "right hand",
        (None, GripSide::Left) => "left hand",
    }
}

/// The grip as it is written: a trimmed name, weapon and bone names, and
/// this Maquette's file version.
pub fn normalized(grip: &GripFile) -> GripFile {
    GripFile {
        version: GRIP_VERSION,
        name: trimmed(&grip.name).to_string(),
        revision: grip.revision,
        weapon: trimmed(&grip.weapon).to_lowercase(),
        hands: grip.hands,
        pose: grip
            .pose
            .iter()
            .map(|(bone, turn)| (trimmed(bone).to_string(), *turn))
            .collect(),
    }
}

/// Checks everything a grip file holds: its name, revision, weapon, a pose
/// of at most [`MAX_GRIP_BONES`] hand, finger and arm bones each turned
/// within 180 degrees, and hands that agree with the sides the pose turns.
///
/// # Errors
///
/// [`CoreError::Invalid`] saying what to fix.
pub fn validate(grip: &GripFile) -> CoreResult<()> {
    check_line(&grip.name, "grip", NAME_MAX_CHARS)?;
    if grip.revision == 0 {
        return invalid("A grip's revision starts at 1.".into());
    }
    if !is_item_id(&grip.weapon) {
        return invalid(format!("{:?} is not a weapon a grip holds.", grip.weapon));
    }
    check_bone_pose(&grip.pose, MAX_GRIP_BONES, &format!("grip {}", grip.name))?;
    if let Some(bone) = grip.pose.keys().find(|b| !is_grip_bone(b)) {
        return invalid(format!(
            "A grip poses a hand, its fingers and the arm above it, not {bone}."
        ));
    }
    let (left, right) = sides_of(&grip.pose);
    let turns = |side| match side {
        GripSide::Left => left,
        GripSide::Right => right,
    };
    let GripHolds { holds, supports } = grip.hands;
    if !turns(holds) {
        return invalid(format!(
            "The grip {} says the {} hand holds, but its pose turns no bone on that side.",
            grip.name,
            holds.word()
        ));
    }
    match supports {
        Some(side) if side == holds || !turns(side) => invalid(format!(
            "The grip {} says the {} hand supports, but its pose turns no bone on that side.",
            grip.name,
            side.word()
        )),
        None if turns(holds.other()) => invalid(format!(
            "The grip {} poses both hands, so say which holds and which supports.",
            grip.name
        )),
        _ => Ok(()),
    }
}

/// Reads and checks a grip file, normalised ([`normalized`]).
///
/// # Errors
///
/// Fails when the file is missing, unreadable or over
/// [`MAX_GRIP_FILE_BYTES`], comes from a newer Maquette or holds values
/// [`validate`] refuses.
pub fn load(file: &Path) -> CoreResult<GripFile> {
    let text = match crate::plans::read_capped(file, MAX_GRIP_FILE_BYTES) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!(
                "the grip file {}",
                file.display()
            )));
        }
        Err(err) if err.kind() == io::ErrorKind::FileTooLarge => {
            return invalid(format!(
                "The grip file is over {} KB, too big to be a grip.",
                MAX_GRIP_FILE_BYTES / 1024
            ));
        }
        Err(err) => return Err(CoreError::io(file, err)),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| CoreError::json(file, e))?;
    // The version first, so a newer file whose shape changed says so.
    let found = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if found > u64::from(GRIP_VERSION) {
        return Err(CoreError::UnsupportedVersion {
            found: u32::try_from(found).unwrap_or(u32::MAX),
            supported: GRIP_VERSION,
        });
    }
    let grip: GripFile = serde_json::from_value(value).map_err(|e| CoreError::json(file, e))?;
    let grip = normalized(&grip);
    validate(&grip)?;
    Ok(grip)
}

/// The text a grip file holds: `grip` normalised and checked, as pretty
/// JSON.
///
/// # Errors
///
/// [`CoreError::Invalid`] for a grip [`validate`] refuses or one too big
/// to open again.
pub fn text(grip: &GripFile) -> CoreResult<String> {
    let grip = normalized(grip);
    validate(&grip)?;
    let mut text = serde_json::to_string_pretty(&grip)
        .map_err(|e| CoreError::Invalid(format!("The grip could not be written: {e}")))?;
    text.push('\n');
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > MAX_GRIP_FILE_BYTES {
        return invalid(format!(
            "The grip would be over {} KB, too big to save.",
            MAX_GRIP_FILE_BYTES / 1024
        ));
    }
    Ok(text)
}

pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<GripFile>()
        .register::<GripHands>()
        .register::<GripHolds>()
        .register::<GripSide>()
}
