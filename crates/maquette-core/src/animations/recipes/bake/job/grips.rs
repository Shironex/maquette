//! Grips as the bake script reads them: each grip change on the bake's
//! frames (its span, from [`crate::recipes::grips`]) with its pose, so the
//! script lays the pose at every frame by how much of the grip shows there,
//! as the rough preview does. Also the model bones the grips turn: a bone
//! the model lacks is left out and the review says so, since a rig without
//! fingers still takes the hand and the arm.

use std::collections::BTreeMap;

use serde::Serialize;

use super::vrm::model_bone;
use crate::animations::recipes::bake::grips::RecipeGripResult;
use crate::animations::retarget::{BonePair, Model};
use crate::recipes::grips::span_on;
use crate::recipes::timeline::Timeline;
use crate::recipes::{BonePose, LayerKind, LayerMode, RecipeFile};

/// One grip change: the bake frames it holds on, how it eases, how much of
/// it shows, how it is laid on and its pose.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GripSpec {
    first: u32,
    last: u32,
    ease_in: u32,
    ease_out: u32,
    weight: f64,
    mode: LayerMode,
    pose: BonePose,
}

/// Every grip change of `recipe` that holds on a frame of its timeline `t`,
/// by change id.
pub(crate) fn grip_specs(recipe: &RecipeFile, t: &Timeline) -> BTreeMap<String, GripSpec> {
    recipe
        .layers
        .iter()
        .filter(|l| l.kind == LayerKind::Grip)
        .filter_map(|layer| {
            let span = span_on(t, layer)?;
            let spec = GripSpec {
                first: span.first,
                last: span.last,
                ease_in: span.ease_in,
                ease_out: span.ease_out,
                weight: span.weight,
                mode: span.mode,
                pose: layer.pose.clone().unwrap_or_default(),
            };
            Some((layer.id.clone(), spec))
        })
        .collect()
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The model bone of every bone the grips of `recipe` turn, by VRM name,
/// and each grip as the review reads it: the frames it holds on, how many
/// of its bones the model has and which it lacks.
pub(crate) fn grip_bones(
    recipe: &RecipeFile,
    t: &Timeline,
    map: &[BonePair],
    model: &Model,
) -> (BTreeMap<String, String>, Vec<RecipeGripResult>) {
    let mut bones = BTreeMap::new();
    let mut results = Vec::new();
    for layer in recipe.layers.iter().filter(|l| l.kind == LayerKind::Grip) {
        let mut missing = Vec::new();
        let pose = layer.pose.iter().flat_map(BTreeMap::keys);
        let mut turned = 0;
        for vrm in pose {
            turned += 1;
            match model_bone(vrm, map, &model.joints) {
                Some(bone) => {
                    bones.insert(vrm.clone(), bone);
                }
                None => missing.push(vrm.clone()),
            }
        }
        let span = span_on(t, layer);
        results.push(RecipeGripResult {
            layer: layer.id.clone(),
            bone: layer.bone.clone(),
            entry: layer.from.as_ref().map(|f| f.entry.clone()),
            revision: layer.from.as_ref().map(|f| f.revision),
            frames: span.map(|s| [s.first, s.last]),
            bones: count(turned),
            posed: count(turned - missing.len()),
            missing,
            gap_cm: None,
            gap_frame: None,
        });
    }
    (bones, results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipes::grips::{GRIP_VERSION, GripFile, GripHolds, GripSide, layer_of};
    use crate::recipes::timeline::timeline;
    use crate::recipes::{GripFrom, RecipeBase};

    fn held() -> RecipeFile {
        let grip = GripFile {
            version: GRIP_VERSION,
            name: "Dagger".into(),
            revision: 2,
            weapon: "dagger".into(),
            hands: GripHolds {
                holds: GripSide::Right,
                supports: None,
            },
            pose: [
                ("rightHand".to_string(), [0.0, 30.0, 0.0]),
                ("rightIndexProximal".to_string(), [0.0, 0.0, 60.0]),
            ]
            .into(),
        };
        let from = GripFrom {
            entry: "dagger".into(),
            uid: None,
            revision: 2,
        };
        let mut r = RecipeFile::new("Held", RecipeBase::rest_pose(40));
        let mut layer = layer_of("l-1", &grip, from, None);
        layer.settings.start = 0.25;
        layer.settings.end = 0.75;
        layer.settings.weight = 0.5;
        r.layers = vec![layer];
        r
    }

    #[test]
    fn a_grip_goes_to_the_script_on_the_bakes_frames_with_its_pose() {
        let r = held();
        let specs = grip_specs(&r, &timeline(&r));
        let spec = &specs["l-1"];
        assert_eq!((spec.first, spec.last), (10, 30));
        assert_eq!((spec.ease_in, spec.ease_out), (4, 4));
        assert_eq!(spec.mode, LayerMode::Replace);
        assert!((spec.weight - 0.5).abs() < 1e-9);
        assert_eq!(spec.pose.len(), 2);
        let text = serde_json::to_string(spec).expect("json");
        assert!(
            text.contains("\"easeIn\":4") && text.contains("\"mode\":\"replace\""),
            "{text}"
        );
    }
}
