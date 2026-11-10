//! A grip read from a project's recipe as it is on disk: the hand pose one
//! of its pose key changes holds at a frame of the bake. This is what an
//! agent's proposal names; the app's own save sheet reads the pose from
//! the recipe as the person is editing it instead.

use std::path::Path;

use super::GripSource;
use crate::error::{CoreError, CoreResult};
use crate::project;
use crate::recipes::grips::{self, GripFile, GripHands, GripSide, is_grip_bone};
use crate::recipes::pose_keys::{key_frame, key_share};
use crate::recipes::timeline::{plain, timeline};
use crate::recipes::{self, LayerMode, RecipeFile, RecipeLayer};
use crate::workspace::FolderLayout;

/// A grip to read from a recipe's pose keys.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GripFromKeys {
    pub project: String,
    pub recipe: String,
    /// The pose key change's id.
    pub layer: String,
    /// The frame of the bake to read; none takes the frame its first key
    /// on a hand stands on.
    pub frame: Option<u32>,
    pub name: String,
    /// What it holds, one of [`grips::WEAPONS`].
    pub weapon: String,
    /// The hands to keep; none keeps the sides the keys turn.
    pub hands: Option<GripHands>,
    /// For both hands: the one that holds, the right unless told.
    pub holds: Option<GripSide>,
    /// Only these bones; none keeps every hand, finger and arm bone keyed.
    pub bones: Option<Vec<String>>,
}

/// A grip read from pose keys, with where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct GripKeyed {
    pub grip: GripFile,
    pub source: GripSource,
    pub recipe_name: String,
    /// What to know before saving it, a plain line each.
    pub notes: Vec<String>,
}

/// The frame of the bake the first key of `layer` that turns a hand,
/// finger or arm bone stands on; 0 when its trim cuts that key out.
fn first_hand_frame(recipe: &RecipeFile, layer: &RecipeLayer) -> u32 {
    let mut keys: Vec<_> = layer
        .keys
        .iter()
        .filter(|k| k.pose.keys().any(|b| is_grip_bone(b)))
        .collect();
    keys.sort_by(|a, b| {
        key_share(recipe, a.clip, a.frame).total_cmp(&key_share(recipe, b.clip, b.frame))
    });
    let Some(first) = keys.first() else { return 0 };
    if plain(recipe) {
        return first.frame;
    }
    key_frame(&timeline(recipe), first.clip, first.frame).unwrap_or(0)
}

/// Reads the grip `request` names from the recipe's file.
///
/// # Errors
///
/// Fails for an unknown project, recipe or change, a change that is not
/// pose keys, keys that turn no hand, finger or arm bone (of those named),
/// hands the keys do not turn, or a name or weapon a grip cannot have.
pub fn from_keys(
    root: &Path,
    layout: &FolderLayout,
    request: &GripFromKeys,
) -> CoreResult<GripKeyed> {
    let manifest = project::load(root, layout, &request.project)?;
    let dir = project::dir(root, layout, &request.project)?;
    let recipe = recipes::get(&dir, &manifest, &request.recipe)?.recipe;
    let id = request.layer.trim();
    let Some(layer) = recipe.layers.iter().find(|l| l.id == id) else {
        return Err(CoreError::NotFound(format!(
            "the change {id} of {}",
            recipe.name
        )));
    };
    let frame = request
        .frame
        .unwrap_or_else(|| first_hand_frame(&recipe, layer))
        .min(timeline(&recipe).frames);
    let pose = grips::keyed_pose(&recipe, layer, frame, request.bones.as_deref())?;
    let weapon = request.weapon.trim().to_lowercase();
    grips::check_weapon(&weapon)?;
    let grip = grips::grip_of(&request.name, &weapon, request.hands, request.holds, &pose)?;
    grips::validate(&grip)?;
    let mut notes = Vec::new();
    if layer.mode.unwrap_or(LayerMode::Add) == LayerMode::Add {
        notes.push(format!(
            "The pose keys {id} add to the clip, so on a model these turns sat on top of the clip's own. A grip replaces the clip's turns on its bones: it shows these turns from the rest pose."
        ));
    }
    Ok(GripKeyed {
        grip,
        source: GripSource {
            project: request.project.clone(),
            recipe: request.recipe.clone(),
            layer: id.to_string(),
            frame,
        },
        recipe_name: recipe.name,
        notes,
    })
}
