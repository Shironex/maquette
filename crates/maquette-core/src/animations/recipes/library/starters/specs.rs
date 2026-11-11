//! The plain starters: one clip found by name (or the rest pose) with a
//! change or two of each of the first four kinds laid over it.

use crate::recipes::{BlankKind, LayerKind, LayerSettings, RecipeBlank, RecipeLayer};

/// The clip names an idle goes by, tried in order.
const IDLE: &[&str] = &["Idle", "Breathing Idle", "Standing Idle", "Happy Idle"];
/// The clip names a walk goes by, tried in order.
const WALK: &[&str] = &["Walking", "Walk"];

/// One starter as it is built in.
pub(super) struct Spec {
    pub key: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    /// Empty for the rest pose.
    pub clips: &'static [&'static str],
    /// How long a rest pose starter runs, frames.
    pub frames: u32,
    pub blanks: fn() -> Vec<RecipeBlank>,
    pub layers: fn() -> Vec<RecipeLayer>,
}

fn blank(id: &str, label: &str, point: Option<[f32; 3]>) -> RecipeBlank {
    RecipeBlank {
        id: id.into(),
        label: label.into(),
        kind: if point.is_some() {
            BlankKind::Point
        } else {
            BlankKind::Prop
        },
        prop: None,
        point,
    }
}

fn layer(id: &str, kind: LayerKind, bone: &str, settings: LayerSettings) -> RecipeLayer {
    RecipeLayer::new(id, kind, bone, settings)
}

fn span(start: f32, end: f32) -> LayerSettings {
    LayerSettings {
        start,
        end,
        ..LayerSettings::default()
    }
}

fn aimed(start: f32, end: f32, target: &str) -> LayerSettings {
    LayerSettings {
        target: Some(target.into()),
        ..span(start, end)
    }
}

fn turned(start: f32, end: f32, rotation: [f32; 3]) -> LayerSettings {
    LayerSettings {
        rotation: Some(rotation),
        ..span(start, end)
    }
}

pub(super) const SPECS: [Spec; 5] = [
    Spec {
        key: "wave-hello",
        name: "Wave hello",
        about: "Raises the right arm and waves the forearm, on the rest pose.",
        clips: &[],
        frames: 60,
        blanks: Vec::new,
        layers: || {
            vec![
                layer(
                    "l-1",
                    LayerKind::Nudge,
                    "rightUpperArm",
                    turned(0.0, 1.0, [0.0, 0.0, -60.0]),
                ),
                layer(
                    "l-2",
                    LayerKind::Wave,
                    "rightLowerArm",
                    LayerSettings {
                        cycles: Some(3.0),
                        amount: Some(30.0),
                        ..span(0.15, 0.95)
                    },
                ),
            ]
        },
    },
    Spec {
        key: "look-at-point",
        name: "Look at a point",
        about: "Turns the head toward a point while the idle plays.",
        clips: IDLE,
        frames: 0,
        blanks: || vec![blank("spot", "Where the head looks", Some([0.6, 1.6, 1.2]))],
        layers: || {
            vec![layer(
                "l-1",
                LayerKind::LookAt,
                "head",
                aimed(0.1, 0.9, "spot"),
            )]
        },
    },
    Spec {
        key: "reach-for-prop",
        name: "Reach for a prop",
        about: "The right hand reaches a prop of the project while the idle plays.",
        clips: IDLE,
        frames: 0,
        blanks: || vec![blank("prop", "The prop the hand reaches", None)],
        layers: || {
            vec![layer(
                "l-1",
                LayerKind::Reach,
                "rightHand",
                aimed(0.2, 0.8, "prop"),
            )]
        },
    },
    Spec {
        key: "lean-in",
        name: "Lean in",
        about: "Leans the upper body forward over the idle, as if listening.",
        clips: IDLE,
        frames: 0,
        blanks: Vec::new,
        layers: || {
            vec![
                layer(
                    "l-1",
                    LayerKind::Nudge,
                    "spine",
                    turned(0.1, 0.9, [12.0, 0.0, 0.0]),
                ),
                layer(
                    "l-2",
                    LayerKind::Nudge,
                    "head",
                    turned(0.1, 0.9, [-6.0, 0.0, 0.0]),
                ),
            ]
        },
    },
    Spec {
        key: "point-while-walking",
        name: "Point while walking",
        about: "The right hand points ahead and to the right during a walk.",
        clips: WALK,
        frames: 0,
        blanks: || {
            vec![blank(
                "spot",
                "Where the hand points",
                Some([-0.4, 1.4, 0.8]),
            )]
        },
        layers: || {
            vec![layer(
                "l-1",
                LayerKind::Reach,
                "rightHand",
                aimed(0.25, 0.75, "spot"),
            )]
        },
    },
];
