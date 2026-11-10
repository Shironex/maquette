//! The pieces of the version 2 changes: pose keys on a clip's own frames,
//! how a pose key or grip is laid on the clip, a foot pin's flat contact,
//! and the library grip a grip change was copied from.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::normal::settings_for;
use super::{LayerKind, RecipeLayer};
use crate::plans::trimmed;

/// Degrees per VRM humanoid bone, X then Y then Z: `{"chest": [0, -18, 0]}`.
pub type BonePose = BTreeMap<String, [f32; 3]>;

/// How a pose key or a grip is laid on the clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum LayerMode {
    /// Turned on top of the clip, which still shows through.
    Add,
    /// In place of the clip's own turn on those bones.
    Replace,
}

/// How the motion eases into a pose key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum KeyEase {
    /// Slow out of the key before and slow into this one.
    #[default]
    Smooth,
    /// Slow out of the key before, fast into this one.
    In,
    /// Fast out of the key before, slow into this one.
    Out,
    /// An even pace.
    Linear,
    /// Holds the key before until this frame, then jumps.
    Step,
}

/// What of a pinned foot stays flat on the ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PinFlat {
    /// The toe and the heel.
    ToeAndHeel,
    /// The heel only, the toe free to roll.
    Heel,
    /// Neither: only where the foot is stays.
    Off,
}

/// One pose key: the bones a person turned at one frame of one clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PoseKey {
    /// The clip it sits on: 0 the base, then the joined clips. On the rest
    /// pose, always 0.
    #[serde(default)]
    #[specta(optional)]
    pub clip: u32,
    /// The frame of that clip, before its trim (on the rest pose, the
    /// recipe's own frame), from 0.
    pub frame: u32,
    #[serde(default)]
    #[specta(optional)]
    pub ease: KeyEase,
    /// Only the bones turned at this frame.
    pub pose: BonePose,
}

/// The library grip and revision a grip change was copied from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GripFrom {
    /// The library grip's id.
    pub entry: String,
    /// The library grip's uid, made once with it. An entry of the same id
    /// with another uid is another grip (the first was removed and its id
    /// taken again), so the link reads as gone. A link written by hand may
    /// leave it out; it then goes by the id alone.
    #[serde(default)]
    #[specta(optional)]
    pub uid: Option<String>,
    /// Its revision, from 1.
    pub revision: u32,
}

/// Bone names trimmed in a pose.
fn pose_for(pose: &BonePose) -> BonePose {
    pose.iter()
        .map(|(bone, turn)| (trimmed(bone).to_string(), *turn))
        .collect()
}

/// A change as it is written: a trimmed id and bone, and only the settings
/// and fields its kind reads. Pose keys add to the clip unless told and
/// keep no bone of their own; grips replace unless told.
pub(super) fn layer_for(layer: &RecipeLayer) -> RecipeLayer {
    let kind = layer.kind;
    let mut keys: Vec<PoseKey> = if kind == LayerKind::Pose {
        layer
            .keys
            .iter()
            .map(|k| PoseKey {
                pose: pose_for(&k.pose),
                ..k.clone()
            })
            .collect()
    } else {
        Vec::new()
    };
    keys.sort_by_key(|k| (k.clip, k.frame));
    let grip = kind == LayerKind::Grip;
    RecipeLayer {
        id: trimmed(&layer.id).to_string(),
        kind,
        bone: if kind == LayerKind::Pose {
            String::new()
        } else {
            trimmed(&layer.bone).to_string()
        },
        settings: settings_for(kind, &layer.settings),
        mode: match kind {
            LayerKind::Pose => Some(layer.mode.unwrap_or(LayerMode::Add)),
            LayerKind::Grip => Some(layer.mode.unwrap_or(LayerMode::Replace)),
            _ => None,
        },
        keys,
        pose: layer.pose.as_ref().filter(|_| grip).map(pose_for),
        from: layer.from.as_ref().filter(|_| grip).map(|f| GripFrom {
            entry: trimmed(&f.entry).to_string(),
            uid: f.uid.as_deref().map(|u| trimmed(u).to_string()),
            revision: f.revision,
        }),
    }
}
