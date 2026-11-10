//! Laying a library grip on a recipe: which of its hands a recipe takes,
//! the change that holds the copy, and taking a newer revision into a
//! change that is there already.
//!
//! A grip that poses both hands gives both, or one of them on its own. A
//! grip that poses one hand gives that hand as it is, the other hand as
//! its mirror, or both hands, the second the mirror of the first.

use super::span::GRIP_EASE_FRAMES;
use super::{GripFile, GripHands, GripSide, mirror_pose, side_pose, sides_of};
use crate::recipes::{BonePose, GripFrom, LayerKind, LayerMode, LayerSettings, RecipeLayer};

/// The hands a recipe takes from `grip` unless told: both of a grip that
/// poses both, else the one it poses.
pub fn default_hands(grip: &GripFile) -> GripHands {
    match grip.hands.supports {
        Some(_) => GripHands::Both,
        None => GripHands::of(grip.hands.holds),
    }
}

/// The hands `pose` turns: both, one, or none for a pose with no bone on
/// either side.
pub fn hands_of_pose(pose: &BonePose) -> Option<GripHands> {
    match sides_of(pose) {
        (true, true) => Some(GripHands::Both),
        (true, false) => Some(GripHands::Left),
        (false, true) => Some(GripHands::Right),
        (false, false) => None,
    }
}

/// The bones of `pose` on `side`, or the other side's in a mirror when it
/// turns none there.
fn on_side(pose: &BonePose, side: GripSide) -> BonePose {
    let own = side_pose(pose, side);
    if own.is_empty() {
        mirror_pose(&side_pose(pose, side.other()))
    } else {
        own
    }
}

/// The pose a recipe takes from `grip` for `hands` (its own unless told),
/// and the hand that holds in it.
pub fn pose_for(grip: &GripFile, hands: Option<GripHands>) -> (GripSide, BonePose) {
    match hands.unwrap_or_else(|| default_hands(grip)) {
        GripHands::Right => (GripSide::Right, on_side(&grip.pose, GripSide::Right)),
        GripHands::Left => (GripSide::Left, on_side(&grip.pose, GripSide::Left)),
        GripHands::Both => {
            let mut pose = on_side(&grip.pose, GripSide::Left);
            pose.extend(on_side(&grip.pose, GripSide::Right));
            (grip.hands.holds, pose)
        }
    }
}

/// A new grip change `id` holding a copy of `grip` for `hands` (its own
/// unless told), linked back with `from`: on the hand that holds, in place
/// of the clip's own turns, across the whole recipe, easing in and out
/// over [`GRIP_EASE_FRAMES`] frames.
pub fn layer_of(
    id: &str,
    grip: &GripFile,
    from: GripFrom,
    hands: Option<GripHands>,
) -> RecipeLayer {
    let (holds, pose) = pose_for(grip, hands);
    let settings = LayerSettings {
        ease_in: Some(GRIP_EASE_FRAMES),
        ease_out: Some(GRIP_EASE_FRAMES),
        ..LayerSettings::default()
    };
    RecipeLayer {
        mode: Some(LayerMode::Replace),
        pose: Some(pose),
        from: Some(from),
        ..RecipeLayer::new(id, LayerKind::Grip, holds.hand(), settings)
    }
}

/// `layer`, a grip change, taking `grip` (a newer revision, or another
/// grip) with the link `from`: its id, where it sits, its eases, weight and
/// what it holds stay; its pose is the grip's for `hands`, or for the hands
/// it posed before when none is given.
pub fn updated(
    layer: &RecipeLayer,
    grip: &GripFile,
    from: GripFrom,
    hands: Option<GripHands>,
) -> RecipeLayer {
    let hands = hands.or_else(|| layer.pose.as_ref().and_then(hands_of_pose));
    let (holds, pose) = pose_for(grip, hands);
    RecipeLayer {
        bone: holds.hand().to_string(),
        pose: Some(pose),
        from: Some(from),
        ..layer.clone()
    }
}
