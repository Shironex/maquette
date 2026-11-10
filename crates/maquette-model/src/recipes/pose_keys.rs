//! Pose keys as they play: where each key stands on the recipe's timeline
//! and the turn each keyed bone has between keys, the one source of that
//! math for the bake and for agents. The web app keeps a copy
//! (`state/recipe-keys.ts`); `fixtures/pose_keys.json` holds cases both
//! must agree on, and the bake script blends the same way.
//!
//! A key sits on a clip's own frame, so it stands where round 1's changes
//! stand: at the share of the clips' own frames laid end to end (before the
//! clip, plus its frame, over all of them). Each frame of the timeline has
//! its own share ([`super::timeline`]), so a trim, retime, freeze or blend
//! carries a key with the motion it sits on, and on the rest pose a key's
//! frame is the recipe's own frame.
//!
//! Each bone blends only between the keys that turn it. Before its first
//! key it holds that key's turn, after its last it holds the last, and
//! between two it moves from one to the next as the later key's ease says.
//! A turn is degrees about the model's own axes (X to its left, Y up, Z
//! ahead), as the bone stands under its parent: added to the clip's own
//! turn (add) or in place of it, from the rest pose (replace).

use std::collections::BTreeSet;

use serde::Serialize;

use super::timeline::{Timeline, plain, timeline, wide};
use super::{KeyEase, LayerKind, LayerMode, PoseKey, RecipeFile, RecipeLayer};

const EPS: f64 = 1e-9;

/// How far along `t` (0 to 1, between the key before and this one) the
/// motion is, as the later key's ease says.
pub fn ease(kind: KeyEase, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    match kind {
        KeyEase::Smooth => t * t * (3.0 - 2.0 * t),
        KeyEase::In => t * t,
        KeyEase::Out => 1.0 - (1.0 - t) * (1.0 - t),
        KeyEase::Linear => t,
        KeyEase::Step => {
            if t >= 1.0 - EPS {
                1.0
            } else {
                0.0
            }
        }
    }
}

/// Where a key on frame `frame` of clip `clip` stands: the share of the
/// clips' own frames laid end to end, 0 to 1.
pub fn key_share(recipe: &RecipeFile, clip: u32, frame: u32) -> f64 {
    let all: f64 = recipe.clips().map(|c| f64::from(c.frames)).sum();
    if all <= 0.0 {
        return 0.0;
    }
    let before: f64 = recipe
        .clips()
        .take(clip as usize)
        .map(|c| f64::from(c.frames))
        .sum();
    ((before + f64::from(frame)) / all).clamp(0.0, 1.0)
}

/// The share frame `frame` of the bake stands on: on one clip played whole,
/// its frame over the clip's count, as round 1 reads it; otherwise the
/// timeline's.
pub fn share_at(recipe: &RecipeFile, t: &Timeline, frame: u32) -> f64 {
    if plain(recipe) {
        let frames = f64::from(recipe.base.frames);
        return if frames > 0.0 {
            (f64::from(frame) / frames).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }
    t.keys
        .get(frame.min(t.frames) as usize)
        .map_or(0.0, |k| k.share)
}

/// The clip and its own frame that frame `frame` of the bake shows: in a
/// blend window the clip that shows more, a frame between two rounded.
pub fn clip_frame_at(t: &Timeline, frame: u32) -> (u32, u32) {
    let Some(key) = t.keys.get(frame.min(t.frames) as usize) else {
        return (0, frame);
    };
    let side = match &key.to {
        Some(to) if key.blend >= 0.5 => to,
        _ => &key.from,
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let at = side.at.round().max(0.0) as u32;
    (side.clip, at)
}

/// The bake frame nearest frame `frame` of clip `clip`, or none when its
/// trim cuts it out.
pub fn key_frame(t: &Timeline, clip: u32, frame: u32) -> Option<u32> {
    let at = f64::from(frame);
    let mut best: Option<usize> = None;
    let mut gap = f64::INFINITY;
    for (i, k) in t.keys.iter().enumerate() {
        for s in std::iter::once(&k.from).chain(k.to.as_ref()) {
            if s.clip != clip {
                continue;
            }
            let d = (s.at - at).abs();
            if d < gap - 1e-6 {
                gap = d;
                best = Some(i);
            }
        }
    }
    best.filter(|_| gap <= 1.0 + 1e-6)
        .and_then(|i| u32::try_from(i).ok())
}

/// One bone's keys on a layer, by share: where each stands, its ease and
/// its turn.
type Track = Vec<(f64, KeyEase, [f64; 3])>;

fn sample(track: &Track, share: f64) -> Option<[f64; 3]> {
    let first = track.first()?;
    let Some(i) = track.iter().rposition(|p| p.0 <= share + EPS) else {
        return Some(first.2);
    };
    let Some(b) = track.get(i + 1) else {
        return Some(track[i].2);
    };
    let a = &track[i];
    let span = b.0 - a.0;
    let t = if span > EPS {
        (share - a.0) / span
    } else {
        1.0
    };
    let e = ease(b.1, t);
    Some(std::array::from_fn(|k| a.2[k] + (b.2[k] - a.2[k]) * e))
}

/// The keys of `layer` by where they stand, first to last.
fn by_share<'a>(recipe: &RecipeFile, layer: &'a RecipeLayer) -> Vec<(f64, &'a PoseKey)> {
    let mut keys: Vec<(f64, &PoseKey)> = layer
        .keys
        .iter()
        .map(|k| (key_share(recipe, k.clip, k.frame), k))
        .collect();
    keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    keys
}

/// Every bone `layer` keys, by name.
pub fn keyed_bones(layer: &RecipeLayer) -> BTreeSet<&str> {
    layer
        .keys
        .iter()
        .flat_map(|k| k.pose.keys().map(String::as_str))
        .collect()
}

/// The turn each bone `layer` keys has at `share`, degrees, by bone name.
pub fn layer_turns(
    recipe: &RecipeFile,
    layer: &RecipeLayer,
    share: f64,
) -> Vec<(String, [f64; 3])> {
    let keys = by_share(recipe, layer);
    keyed_bones(layer)
        .into_iter()
        .filter_map(|bone| {
            let track: Track = keys
                .iter()
                .filter_map(|(s, k)| k.pose.get(bone).map(|t| (*s, k.ease, t.map(wide))))
                .collect();
            sample(&track, share).map(|turn| (bone.to_string(), turn))
        })
        .collect()
}

/// A bone's turn from one pose key change at one frame.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyedTurn {
    pub layer: String,
    pub bone: String,
    pub mode: LayerMode,
    /// How much of it shows, 0 to 1.
    pub weight: f32,
    /// Degrees about the model's own axes, X then Y then Z.
    pub turn: [f64; 3],
}

/// The keyed turns at frame `frame` of the bake.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoseAt {
    pub frame: u32,
    /// Where the frame stands on the clips' own frames.
    pub share: f64,
    /// The clip and its own frame the frame shows.
    pub clip: u32,
    pub at: u32,
    /// Every pose key change's turns, in the order they are laid on.
    pub turns: Vec<KeyedTurn>,
}

/// The keyed turns of every pose key change of `recipe` at frame `frame`
/// of its bake (kept within its length).
pub fn pose_at(recipe: &RecipeFile, frame: u32) -> PoseAt {
    let t = timeline(recipe);
    let frame = frame.min(t.frames);
    let share = share_at(recipe, &t, frame);
    let (clip, at) = if plain(recipe) {
        (0, frame)
    } else {
        clip_frame_at(&t, frame)
    };
    let turns = recipe
        .layers
        .iter()
        .filter(|l| l.kind == LayerKind::Pose)
        .flat_map(|layer| {
            let mode = layer.mode.unwrap_or(LayerMode::Add);
            let weight = layer.settings.weight.clamp(0.0, 1.0);
            layer_turns(recipe, layer, share)
                .into_iter()
                .map(move |(bone, turn)| KeyedTurn {
                    layer: layer.id.clone(),
                    bone,
                    mode,
                    weight,
                    turn,
                })
        })
        .collect();
    PoseAt {
        frame,
        share,
        clip,
        at,
        turns,
    }
}
