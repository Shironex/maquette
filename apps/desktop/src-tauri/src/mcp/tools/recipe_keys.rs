//! Pose keys for agents: add, move or remove keys on one pose key change of
//! a recipe by bone and frame, and read the keyed turns at a frame with the
//! humanoid joint limits they pass. Writes send expected like update_recipe
//! and go through the same checks; nothing here bakes or runs Blender.

use std::collections::BTreeMap;

use maquette_core::recipes::joint_limits::{joint_limits, over_limits};
use maquette_core::recipes::pose_keys::{keyed_bones, pose_at};
use maquette_core::recipes::timeline::timeline;
use maquette_core::recipes::{
    KeyEase, KeyEdits, KeyMove, KeyRemove, KeySet, LayerKind, LayerMode, RecipeChange, edit_keys,
    empty_pose_layer,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::hints::{step, with_next};
use super::recipes::change_view;
use super::{McpTools, Out, ToolError, respond, to_short_value, to_value};
use crate::commands::recipes as commands;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct KeySetArg {
    /// The clip the key sits on: 0 the base (always 0 on the rest pose),
    /// then the joined clips. Defaults to 0.
    pub clip: Option<u32>,
    /// The frame of that clip before its trim, from 0 (on the rest pose,
    /// the recipe's own frame).
    pub frame: u32,
    /// How the motion eases into this key: smooth, in, out, linear or step.
    /// Left out, a key keeps its ease (smooth for a new one).
    pub ease: Option<String>,
    /// Degrees per humanoid bone, [x, y, z] about the model's own axes (X to
    /// its left, Y up, Z ahead), within 180 each way: {"chest": [0, -18, 0]}.
    /// Each replaces that bone's turn on the key; other bones stay.
    pub pose: BTreeMap<String, [f32; 3]>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct KeyMoveArg {
    /// The key's clip (defaults to 0) and frame.
    pub clip: Option<u32>,
    pub frame: u32,
    /// Where it goes: a clip (defaults to its own) and frame.
    pub to_clip: Option<u32>,
    pub to_frame: u32,
    /// Only these bones of the key move; left out, the whole key does.
    pub bones: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct KeyRemoveArg {
    /// The key's clip (defaults to 0) and frame.
    pub clip: Option<u32>,
    pub frame: u32,
    /// Only these bones come out of the key; left out, the whole key does.
    pub bones: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct SetRecipeKeysArgs {
    /// The project id.
    pub project: String,
    /// The recipe id.
    pub recipe: String,
    /// The pose key change to edit, by id. Left out, or an id the recipe
    /// has not got, adds a new pose key change at the end (with that id, or
    /// the lowest free l-N).
    pub layer: Option<String>,
    /// The version get_recipe (or the last write) answered with; a recipe
    /// changed since is refused (moved).
    pub expected: String,
    /// add (on top of the clip, the default for a new one) or replace (in
    /// place of the clip's own turn, from the rest pose). Left out, it stays.
    pub mode: Option<String>,
    /// How much of the keys shows, 0 to 1. Left out, it stays (1 for new).
    pub weight: Option<f32>,
    /// Turns to set, each on the key at its clip and frame (made when there
    /// is none).
    pub set: Option<Vec<KeySetArg>>,
    /// Keys, or some of their bones, moved to another frame. Done before set.
    #[serde(rename = "move")]
    pub moves: Option<Vec<KeyMoveArg>>,
    /// Keys, or some of their bones, taken out. Done first.
    pub remove: Option<Vec<KeyRemoveArg>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct GetRecipePoseArgs {
    /// The project id.
    pub project: String,
    /// The recipe id.
    pub recipe: String,
    /// A frame of the bake, 0 to its length (get_recipe's frames); later
    /// frames read the last.
    pub frame: u32,
    /// Only this pose key change; left out, all of them.
    pub layer: Option<String>,
}

fn ease_of(given: &str) -> Result<KeyEase, ToolError> {
    match given
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '_'], "-")
        .as_str()
    {
        "smooth" | "in-out" => Ok(KeyEase::Smooth),
        "in" => Ok(KeyEase::In),
        "out" => Ok(KeyEase::Out),
        "linear" => Ok(KeyEase::Linear),
        "step" => Ok(KeyEase::Step),
        _ => Err(ToolError::from(format!(
            "{given} is not a key ease. Use smooth, in, out, linear or step."
        ))),
    }
}

fn mode_of(given: &str) -> Result<LayerMode, ToolError> {
    match given.trim().to_ascii_lowercase().as_str() {
        "add" => Ok(LayerMode::Add),
        "replace" => Ok(LayerMode::Replace),
        _ => Err(ToolError::from(format!(
            "{given} is not how keys are laid on. Use add or replace."
        ))),
    }
}

fn edits_of(args: &SetRecipeKeysArgs) -> Result<KeyEdits, ToolError> {
    let set = args
        .set
        .iter()
        .flatten()
        .map(|s| {
            Ok(KeySet {
                clip: s.clip.unwrap_or(0),
                frame: s.frame,
                ease: s.ease.as_deref().map(ease_of).transpose()?,
                pose: s.pose.clone(),
            })
        })
        .collect::<Result<_, ToolError>>()?;
    let moves = args
        .moves
        .iter()
        .flatten()
        .map(|m| KeyMove {
            clip: m.clip.unwrap_or(0),
            frame: m.frame,
            to_clip: m.to_clip.or(m.clip).unwrap_or(0),
            to_frame: m.to_frame,
            bones: m.bones.clone(),
        })
        .collect();
    let remove = args
        .remove
        .iter()
        .flatten()
        .map(|r| KeyRemove {
            clip: r.clip.unwrap_or(0),
            frame: r.frame,
            bones: r.bones.clone(),
        })
        .collect();
    Ok(KeyEdits { set, moves, remove })
}

/// Each axis of a turn past `bone`'s joint limits.
fn over_view(bone: &str, turn: [f32; 3]) -> Value {
    let over: Vec<Value> = over_limits(bone, turn)
        .into_iter()
        .map(|h| json!({ "axis": h.axis, "word": h.word, "degrees": to_short_value(h.degrees), "limit": h.limit }))
        .collect();
    json!(over)
}

#[allow(clippy::cast_possible_truncation)]
fn narrow(turn: [f64; 3]) -> [f32; 3] {
    turn.map(|v| v as f32)
}

#[tool_router(router = recipe_keys_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Add, move or remove pose keys on one pose key change of an animation recipe, by bone and frame, as the Motion view's pose keys do. A key sits on a clip's own frame (clip 0 the base, then the joined clips; on the rest pose the recipe's own frame) and turns only the bones sent, degrees [x, y, z] about the model's own axes (X to its left, Y up, Z ahead) as the bone stands under its parent: added to the clip (add, the default) or in place of its turn, from the rest pose (replace). Each bone blends between the keys that turn it, as the later key's ease says (smooth, in, out, linear or step); before its first key and after its last it holds. remove goes first, then move, then set; a key left with no bones goes. layer names the change; left out or new, a pose key change is added at the end. Needs expected, the version get_recipe answered with; a recipe changed since is refused (moved). Checked like update_recipe (64 keys a change, 32 bones a key, 256 keys and 1024 bone turns a recipe, turns within 180). Answers the change, its keys, bones and the keyed turns past the humanoid joint limits (warn only). Pose keys bake. Nothing is baked or spent."
    )]
    async fn set_recipe_keys(&self, Parameters(args): Parameters<SetRecipeKeysArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let edits = edits_of(&args)?;
                let mode = args.mode.as_deref().map(mode_of).transpose()?;
                if edits.is_empty() && mode.is_none() && args.weight.is_none() {
                    return Err(ToolError::from(
                        "Send set, move or remove (or a new mode or weight) to change the keys.",
                    ));
                }
                let doc = commands::get(host.session(), &args.project, &args.recipe)?;
                let mut layers = doc.recipe.layers.clone();
                let at = args
                    .layer
                    .as_deref()
                    .and_then(|id| layers.iter().position(|l| l.id == id.trim()));
                let index = match at {
                    Some(i) => i,
                    None => {
                        let id = args.layer.as_deref().map_or("", str::trim);
                        layers.push(empty_pose_layer(id, LayerMode::Add));
                        layers.len() - 1
                    }
                };
                let mut layer = edit_keys(&layers[index], &edits)?;
                if let Some(mode) = mode {
                    layer.mode = Some(mode);
                }
                if let Some(weight) = args.weight {
                    layer.settings.weight = weight;
                }
                layers[index] = layer;
                let change = RecipeChange {
                    layers: Some(layers),
                    expected: Some(args.expected.clone()),
                    ..RecipeChange::default()
                };
                let doc = commands::update(host.session(), &args.project, &args.recipe, change)?;
                host.recipe_changed(&args.project);
                let layer = doc
                    .recipe
                    .layers
                    .get(index)
                    .filter(|l| l.kind == LayerKind::Pose)
                    .ok_or_else(|| ToolError::from("The pose keys were written but not read back."))?;
                let over: Vec<Value> = layer
                    .keys
                    .iter()
                    .flat_map(|k| {
                        k.pose.iter().filter_map(move |(bone, turn)| {
                            let over = over_view(bone, *turn);
                            (over.as_array().is_some_and(|a| !a.is_empty())).then(|| {
                                json!({ "clip": k.clip, "frame": k.frame, "bone": bone, "over": over })
                            })
                        })
                    })
                    .collect();
                let out = json!({
                    "recipe": doc.id, "version": doc.version, "frames": timeline(&doc.recipe).frames,
                    "layer": change_view(layer),
                    "keys": layer.keys.len(),
                    "bones": keyed_bones(layer).into_iter().collect::<Vec<_>>(),
                    "overLimits": over,
                });
                Ok(with_next(
                    out,
                    vec![
                        step("get_recipe_pose", "Check the turns at a frame against the joint limits."),
                        step("bake_recipe", "Bake it onto a rigged model to see it exactly."),
                    ],
                ))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read the bone turns an animation recipe's pose keys make at one frame of its bake, from the same math the rough preview blends them with, and how each stands against the humanoid joint limits (a fixed table per bone; warn only). Answers the clip and its own frame that frame shows, and per keyed bone: the pose key change, add or replace, weight, its turn [x, y, z] in degrees about the model's own axes, its limits per axis (none for the hips) and each axis past them. An add key's turn sits on the clip's own, which is not counted; the bake review judges the keys the same way. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn get_recipe_pose(&self, Parameters(args): Parameters<GetRecipePoseArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let doc = commands::get(host.session(), &args.project, &args.recipe)?;
                let recipe = &doc.recipe;
                let pose = pose_at(recipe, args.frame);
                let bones: Vec<Value> = pose
                    .turns
                    .iter()
                    .filter(|t| args.layer.as_deref().is_none_or(|l| l.trim() == t.layer))
                    .map(|t| {
                        json!({
                            "bone": t.bone, "layer": t.layer, "mode": to_value(t.mode),
                            "weight": to_short_value(t.weight),
                            "turn": t.turn.map(|v| (v * 100.0).round() / 100.0),
                            "limits": joint_limits(&t.bone),
                            "over": over_view(&t.bone, narrow(t.turn)),
                        })
                    })
                    .collect();
                let keyed = recipe.layers.iter().any(|l| l.kind == LayerKind::Pose);
                let out = json!({
                    "recipe": doc.id, "frame": pose.frame, "frames": timeline(recipe).frames,
                    "clip": pose.clip, "at": pose.at, "bones": bones,
                    "rough": "From the keys alone, as the rough preview blends them; the bake solves the pose exactly.",
                });
                Ok(if keyed {
                    out
                } else {
                    with_next(
                        out,
                        vec![step("set_recipe_keys", "The recipe has no pose keys yet: add some.")],
                    )
                })
            })
            .await;
        respond(found)
    }
}
