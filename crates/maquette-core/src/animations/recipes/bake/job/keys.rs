//! Pose keys as the bake script reads them: each pose key change's keys
//! with where each stands on the recipe's timeline (its share, from
//! [`crate::recipes::pose_keys`]), so the script blends each bone between
//! its keys at every frame's share as the rough preview does. Also the
//! model bones the keys turn: a bone the model lacks is left out with a
//! warning, not refused, since a recipe made on one rig may key a bone
//! (an upper chest, a toe) another has not got.

use std::collections::BTreeMap;

use serde::Serialize;

use super::vrm::model_bone;
use crate::animations::retarget::{BonePair, Model};
use crate::recipes::pose_keys::{key_share, keyed_bones};
use crate::recipes::{BonePose, KeyEase, LayerKind, LayerMode, RecipeFile};

/// One key: where it stands, how the motion eases into it, its turns.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct KeySpec {
    share: f64,
    ease: KeyEase,
    pose: BonePose,
}

/// One pose key change: how it is laid on, how much of it shows and its
/// keys first to last.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct PoseLayerSpec {
    mode: LayerMode,
    weight: f32,
    keys: Vec<KeySpec>,
}

/// Every pose key change of `recipe`, by change id.
pub(crate) fn pose_specs(recipe: &RecipeFile) -> BTreeMap<String, PoseLayerSpec> {
    recipe
        .layers
        .iter()
        .filter(|l| l.kind == LayerKind::Pose)
        .map(|layer| {
            let mut keys: Vec<KeySpec> = layer
                .keys
                .iter()
                .map(|k| KeySpec {
                    share: key_share(recipe, k.clip, k.frame),
                    ease: k.ease,
                    pose: k.pose.clone(),
                })
                .collect();
            keys.sort_by(|a, b| a.share.total_cmp(&b.share));
            let spec = PoseLayerSpec {
                mode: layer.mode.unwrap_or(LayerMode::Add),
                weight: layer.settings.weight.clamp(0.0, 1.0),
                keys,
            };
            (layer.id.clone(), spec)
        })
        .collect()
}

/// The model bone of every bone the pose keys of `recipe` turn, by VRM
/// name, and a warning per pose key change for the bones the model lacks.
pub(crate) fn pose_bones(
    recipe: &RecipeFile,
    map: &[BonePair],
    model: &Model,
) -> (BTreeMap<String, String>, Vec<String>) {
    let mut bones = BTreeMap::new();
    let mut warnings = Vec::new();
    for layer in recipe.layers.iter().filter(|l| l.kind == LayerKind::Pose) {
        let mut missing = Vec::new();
        for vrm in keyed_bones(layer) {
            match model_bone(vrm, map, &model.joints) {
                Some(bone) => {
                    bones.insert(vrm.to_string(), bone);
                }
                None => missing.push(vrm),
            }
        }
        if !missing.is_empty() {
            warnings.push(format!(
                "{} has no bone for {}, so the pose keys {} leave {}.",
                model.name,
                missing.join(", "),
                layer.id,
                if missing.len() == 1 { "it" } else { "them" }
            ));
        }
    }
    (bones, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipes::{PoseKey, RecipeBase, empty_pose_layer};

    #[test]
    fn keys_go_to_the_script_by_where_they_stand() {
        let mut r = RecipeFile::new("Keys", RecipeBase::rest_pose(40));
        let mut layer = empty_pose_layer("l-1", LayerMode::Replace);
        layer.settings.weight = 0.5;
        layer.keys = vec![
            PoseKey {
                clip: 0,
                frame: 30,
                ease: KeyEase::Step,
                pose: [("head".to_string(), [0.0, 20.0, 0.0])].into(),
            },
            PoseKey {
                clip: 0,
                frame: 10,
                ease: KeyEase::Smooth,
                pose: [("head".to_string(), [0.0, 0.0, 0.0])].into(),
            },
        ];
        r.layers = vec![layer];
        let specs = pose_specs(&r);
        let spec = &specs["l-1"];
        assert_eq!(spec.mode, LayerMode::Replace);
        assert!((spec.weight - 0.5).abs() < 1e-6);
        let shares: Vec<f64> = spec.keys.iter().map(|k| k.share).collect();
        assert_eq!(shares, [0.25, 0.75]);
        assert_eq!(spec.keys[1].ease, KeyEase::Step);
    }
}
