//! A recipe that joins clips or edits its base: each library clip goes onto
//! the model once, under a name of its own, and the bake script builds the
//! recipe's motion from them frame by frame as its timeline says
//! ([`crate::recipes::timeline`]), before the changes are laid over it.

use std::path::Path;

use serde::Serialize;

use crate::animations::recipes::{BaseMatch, BaseStatus, RecipeDoc};
use crate::animations::retarget::{ClipSetting, settings};
use crate::animations::{AnimationClip, load as load_clip};
use crate::error::{CoreError, CoreResult};

/// What a part's action is called on the model while the bake runs; the
/// bake removes every part before it writes the model.
const PART_NAME: &str = "maquette_recipe_part";

/// One clip of the recipe as the bake script reads it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PartSpec {
    /// The library clip, as the retarget job names it; none for the rest pose.
    pub clip: Option<String>,
    /// The rate its keys were read at, frames a second.
    pub fps: f32,
}

/// The library clips the joined clips match, in order (clip 2 first).
///
/// # Errors
///
/// `invalid` naming the clip for one the library does not have; as reading
/// a clip does otherwise.
pub(crate) fn joined_clips(root: &Path, doc: &RecipeDoc) -> CoreResult<Vec<AnimationClip>> {
    doc.joined
        .iter()
        .zip(&doc.recipe.joined)
        .enumerate()
        .map(|(i, (found, clip))| match found {
            BaseMatch {
                status: BaseStatus::Found | BaseStatus::HashDiffers,
                clip: Some(id),
                ..
            } => load_clip(root, id),
            _ => Err(CoreError::Invalid(format!(
                "Clip {} ({}) is not in the animation library, so the recipe cannot be baked. {}",
                i + 2,
                clip.name,
                found.warning.clone().unwrap_or_default()
            ))),
        })
        .collect()
}

/// The library clips to put on the model, each once, with the settings
/// that bake each whole under a part name, and every recipe clip's part.
pub(crate) fn parts_of(
    base: Option<&AnimationClip>,
    joined: &[AnimationClip],
) -> (Vec<AnimationClip>, Vec<ClipSetting>, Vec<PartSpec>) {
    let mut clips: Vec<AnimationClip> = Vec::new();
    let mut specs = Vec::new();
    let rest = PartSpec {
        clip: None,
        fps: crate::recipes::REST_FPS,
    };
    for clip in base.into_iter().chain(joined) {
        if !clips.iter().any(|c| c.id == clip.id) {
            clips.push(clip.clone());
        }
        specs.push(PartSpec {
            clip: Some(clip.id.clone()),
            fps: clip.fps,
        });
    }
    if base.is_none() {
        specs.insert(0, rest);
    }
    let settings = clips
        .iter()
        .enumerate()
        .map(|(i, clip)| ClipSetting {
            name: format!("{PART_NAME}_{i}"),
            ..settings::default_for(clip)
        })
        .collect();
    (clips, settings, specs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(id: &str) -> AnimationClip {
        crate::animations::tests::library_clip(id, id, "Pack", id)
    }

    #[test]
    fn each_library_clip_goes_on_once_and_every_recipe_clip_names_its_part() {
        let (a, b) = (clip("a"), clip("b"));
        let (clips, settings, specs) = parts_of(Some(&a), &[b.clone(), a.clone()]);
        assert_eq!(
            clips.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(settings.len(), 2);
        assert!(settings.iter().all(|s| s.name.starts_with(PART_NAME)));
        assert_eq!(settings[0].end, a.frames, "whole: the timeline trims");
        let named: Vec<_> = specs.iter().map(|s| s.clip.as_deref()).collect();
        assert_eq!(named, [Some("a"), Some("b"), Some("a")]);
        let (_, _, rest) = parts_of(None, &[]);
        assert_eq!(rest[0].clip, None, "the rest pose has no clip");
    }
}
