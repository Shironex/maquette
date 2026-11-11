//! Making a grip from a recipe's pose keys: the turns a pose key change
//! holds on the hands at one frame of the bake, and the grip file they
//! make. Only hand, finger and arm bones are taken; the rest of the keys
//! stay the recipe's own.

use super::{
    GRIP_VERSION, GripFile, GripHands, GripHolds, GripSide, is_grip_bone, side_pose, sides_of,
};
use crate::error::{CoreError, CoreResult};
use crate::plans::trimmed;
use crate::recipes::pose_keys::{layer_turns, share_at};
use crate::recipes::timeline::timeline;
use crate::recipes::{BonePose, LayerKind, RecipeFile, RecipeLayer};

/// A turn kept to a hundredth of a degree, as a grip file holds it.
#[allow(clippy::cast_possible_truncation)]
fn stored(turn: [f64; 3]) -> [f32; 3] {
    turn.map(|v| {
        let v = ((v * 100.0).round() / 100.0) as f32;
        if v == 0.0 { 0.0 } else { v }
    })
}

/// The hand pose the pose key change `layer` of `recipe` holds at frame
/// `frame` of the bake: each hand, finger and arm bone it keys, as blended
/// between its keys there. With `keep`, only the bones it names.
///
/// # Errors
///
/// [`CoreError::Invalid`] for a change that is not pose keys, or one whose
/// keys turn no such bone (of `keep`, when given).
pub fn keyed_pose(
    recipe: &RecipeFile,
    layer: &RecipeLayer,
    frame: u32,
    keep: Option<&[String]>,
) -> CoreResult<BonePose> {
    if layer.kind != LayerKind::Pose {
        return Err(CoreError::Invalid(format!(
            "{} is not a pose key change. A grip is saved from the pose keys on the hands.",
            layer.id
        )));
    }
    let t = timeline(recipe);
    let share = share_at(recipe, &t, frame.min(t.frames));
    let pose: BonePose = layer_turns(recipe, layer, share)
        .into_iter()
        .filter(|(bone, _)| is_grip_bone(bone))
        .filter(|(bone, _)| keep.is_none_or(|k| k.iter().any(|b| trimmed(b) == bone)))
        .map(|(bone, turn)| (bone, stored(turn)))
        .collect();
    if pose.is_empty() {
        return Err(CoreError::Invalid(format!(
            "The pose keys {} turn no hand, finger or arm bone{}. Key the hand first, then save it as a grip.",
            layer.id,
            if keep.is_some() {
                " of those named"
            } else {
                ""
            }
        )));
    }
    Ok(pose)
}

/// The grip called `name` holding `weapon` that `pose` makes for `hands`:
/// one hand keeps only its side's bones; both needs bones on both sides,
/// the right hand holding unless `holds` says the left. Left out, the hands
/// are the sides the pose turns. Its revision is 1 until the library sets it.
///
/// # Errors
///
/// [`CoreError::Invalid`] when the pose turns no bone on a side `hands`
/// asks for.
pub fn grip_of(
    name: &str,
    weapon: &str,
    hands: Option<GripHands>,
    holds: Option<GripSide>,
    pose: &BonePose,
) -> CoreResult<GripFile> {
    let (left, right) = sides_of(pose);
    let hands = hands.unwrap_or(match (left, right) {
        (true, true) => GripHands::Both,
        (true, false) => GripHands::Left,
        _ => GripHands::Right,
    });
    let missing = |side: GripSide| {
        CoreError::Invalid(format!(
            "The pose turns no bone on the {} hand, so it cannot be a grip for it.",
            side.word()
        ))
    };
    let need = |side: GripSide| match side {
        GripSide::Left if !left => Err(missing(side)),
        GripSide::Right if !right => Err(missing(side)),
        _ => Ok(()),
    };
    let (hands, pose) = match hands {
        GripHands::Both => {
            need(GripSide::Left)?;
            need(GripSide::Right)?;
            let holds = holds.unwrap_or(GripSide::Right);
            let hands = GripHolds {
                holds,
                supports: Some(holds.other()),
            };
            (hands, pose.clone())
        }
        one => {
            let side = if one == GripHands::Left {
                GripSide::Left
            } else {
                GripSide::Right
            };
            need(side)?;
            let hands = GripHolds {
                holds: side,
                supports: None,
            };
            (hands, side_pose(pose, side))
        }
    };
    Ok(GripFile {
        version: GRIP_VERSION,
        name: trimmed(name).to_string(),
        revision: 1,
        weapon: trimmed(weapon).to_lowercase(),
        hands,
        pose,
    })
}

/// Whether two grips pose the same hands the same way and hold the same
/// weapon: what the library would save twice. The name does not count.
pub fn same_pose(a: &GripFile, b: &GripFile) -> bool {
    a.weapon == b.weapon && a.hands == b.hands && a.pose == b.pose
}
