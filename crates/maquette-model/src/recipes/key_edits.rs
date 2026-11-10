//! Edits to one pose key change by bone and frame, as an agent sends them:
//! set turns on a key (making it when its clip and frame have none), move a
//! key or some of its bones to another frame, or take a key or some of its
//! bones out. Removes go first, then moves, then sets, and a key left with
//! no bones goes. The result is checked whole on the write, like any
//! change.

use serde::{Deserialize, Serialize};

use super::{
    BonePose, KeyEase, LayerKind, LayerMode, LayerSettings, MAX_KEY_BONES, MAX_KEYS, PoseKey,
    RecipeLayer,
};
use crate::error::{CoreError, CoreResult};
use crate::plans::trimmed;

/// Turns to set on the key at a clip's frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeySet {
    pub clip: u32,
    pub frame: u32,
    /// None keeps the key's ease (smooth for a new key).
    pub ease: Option<KeyEase>,
    /// Degrees per bone; each replaces that bone's turn on the key.
    pub pose: BonePose,
}

/// A key, or some of its bones, moved to another clip's frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyMove {
    pub clip: u32,
    pub frame: u32,
    pub to_clip: u32,
    pub to_frame: u32,
    /// None moves every bone of the key.
    pub bones: Option<Vec<String>>,
}

/// A key, or some of its bones, taken out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyRemove {
    pub clip: u32,
    pub frame: u32,
    /// None takes the whole key out.
    pub bones: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyEdits {
    pub set: Vec<KeySet>,
    pub moves: Vec<KeyMove>,
    pub remove: Vec<KeyRemove>,
}

impl KeyEdits {
    pub fn is_empty(&self) -> bool {
        self.set.is_empty() && self.moves.is_empty() && self.remove.is_empty()
    }
}

/// A new pose key change `id`, laid on as `mode`, with no keys yet.
pub fn empty_pose_layer(id: &str, mode: LayerMode) -> RecipeLayer {
    let mut layer = RecipeLayer::new(id, LayerKind::Pose, "", LayerSettings::default());
    layer.mode = Some(mode);
    layer
}

fn at(keys: &[PoseKey], clip: u32, frame: u32) -> Option<usize> {
    keys.iter().position(|k| k.clip == clip && k.frame == frame)
}

fn missing(id: &str, clip: u32, frame: u32) -> CoreError {
    CoreError::Invalid(format!(
        "{id} has no key on frame {frame} of clip {}. Read the recipe again for its keys.",
        u64::from(clip) + 1
    ))
}

/// The bones of `pose` named in `bones` (all of them for none), taken out.
fn take(pose: &mut BonePose, bones: Option<&Vec<String>>) -> BonePose {
    match bones {
        None => std::mem::take(pose),
        Some(names) => names
            .iter()
            .filter_map(|b| {
                let b = trimmed(b);
                pose.remove(b).map(|turn| (b.to_string(), turn))
            })
            .collect(),
    }
}

fn put(keys: &mut Vec<PoseKey>, clip: u32, frame: u32, ease: Option<KeyEase>, pose: BonePose) {
    match at(keys, clip, frame) {
        Some(i) => {
            let key = &mut keys[i];
            key.pose.extend(pose);
            if let Some(ease) = ease {
                key.ease = ease;
            }
        }
        None => keys.push(PoseKey {
            clip,
            frame,
            ease: ease.unwrap_or_default(),
            pose,
        }),
    }
}

fn too_long(what: &str, len: usize, most: usize) -> CoreError {
    CoreError::Invalid(format!(
        "{len} {what} in one edit; send at most {most}. Split it into several edits."
    ))
}

/// Refuses lists longer than a pose key change can hold, before any scan:
/// each edit scans the keys, so the work grows with the product.
fn check_sizes(edits: &KeyEdits) -> CoreResult<()> {
    for (what, len) in [
        ("keys to set", edits.set.len()),
        ("keys to move", edits.moves.len()),
        ("keys to remove", edits.remove.len()),
    ] {
        if len > MAX_KEYS {
            return Err(too_long(what, len, MAX_KEYS));
        }
    }
    let bones = edits
        .set
        .iter()
        .map(|s| s.pose.len())
        .chain(
            edits
                .moves
                .iter()
                .filter_map(|m| m.bones.as_ref())
                .map(Vec::len),
        )
        .chain(
            edits
                .remove
                .iter()
                .filter_map(|r| r.bones.as_ref())
                .map(Vec::len),
        );
    for len in bones {
        if len > MAX_KEY_BONES {
            return Err(too_long("bones on one key", len, MAX_KEY_BONES));
        }
    }
    Ok(())
}

/// `layer` (a pose key change) with `edits` made.
///
/// # Errors
///
/// [`CoreError::Invalid`] for a change that is not pose keys, an edit list
/// longer than [`MAX_KEYS`] (or a bone list longer than [`MAX_KEY_BONES`]),
/// or a move or remove of a key or bone it does not have.
pub fn edit_keys(layer: &RecipeLayer, edits: &KeyEdits) -> CoreResult<RecipeLayer> {
    if layer.kind != LayerKind::Pose {
        return Err(CoreError::Invalid(format!(
            "{} is not a pose key change. Send the id of one, or a new id to add one.",
            layer.id
        )));
    }
    check_sizes(edits)?;
    let id = layer.id.as_str();
    let mut keys = layer.keys.clone();
    for r in &edits.remove {
        let i = at(&keys, r.clip, r.frame).ok_or_else(|| missing(id, r.clip, r.frame))?;
        let gone = take(&mut keys[i].pose, r.bones.as_ref());
        if gone.is_empty() {
            return Err(CoreError::Invalid(format!(
                "The key on frame {} of {id} turns none of those bones.",
                r.frame
            )));
        }
    }
    keys.retain(|k| !k.pose.is_empty());
    for m in &edits.moves {
        let i = at(&keys, m.clip, m.frame).ok_or_else(|| missing(id, m.clip, m.frame))?;
        let ease = keys[i].ease;
        let moved = take(&mut keys[i].pose, m.bones.as_ref());
        if moved.is_empty() {
            return Err(CoreError::Invalid(format!(
                "The key on frame {} of {id} turns none of those bones.",
                m.frame
            )));
        }
        keys.retain(|k| !k.pose.is_empty());
        // Bones moved onto a key keep that key's ease; a new key takes theirs.
        let ease = at(&keys, m.to_clip, m.to_frame).is_none().then_some(ease);
        put(&mut keys, m.to_clip, m.to_frame, ease, moved);
    }
    for s in &edits.set {
        let pose = s
            .pose
            .iter()
            .map(|(b, turn)| (trimmed(b).to_string(), *turn))
            .collect();
        put(&mut keys, s.clip, s.frame, s.ease, pose);
    }
    keys.retain(|k| !k.pose.is_empty());
    keys.sort_by_key(|k| (k.clip, k.frame));
    Ok(RecipeLayer {
        keys,
        ..layer.clone()
    })
}
