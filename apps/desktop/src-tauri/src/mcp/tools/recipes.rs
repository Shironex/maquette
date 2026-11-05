//! Animation recipes for agents: a project's recipes, each a base clip from
//! the animation library with changes laid over it on humanoid bones.
//! Listing, reading, making one from a library clip and changing its
//! changes, blanks or name. Nothing here bakes, runs Blender or spends
//! anything; a later update bakes a recipe into a model's next version.

use maquette_core::animations::recipes::{BaseStatus, RecipeDoc};
use maquette_core::recipes::{
    BlankKind, LayerKind, LayerSettings, RecipeBlank, RecipeChange, RecipeLayer,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::hints::{step, with_next};
use super::{McpTools, Out, ProjectArgs, ToolError, respond, to_short_value, to_value};
use crate::commands::recipes as commands;

/// The code of an update_recipe that sends lists without the version it read.
pub(super) const NEEDS_VERSION: &str = "needs-version";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecipeArgs {
    /// The project id.
    pub project: String,
    /// The recipe id, as list_recipes returns it (its file name without .recipe.json).
    pub recipe: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecipeArgs {
    /// The project id.
    pub project: String,
    /// The library clip it starts from, by id as list_animations returns it.
    pub clip: String,
    /// The recipe name, one line, up to 120 characters. Defaults to the clip's name.
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeArg {
    /// Keep it to edit a change; left out, a free one is made (l-1, l-2).
    pub id: Option<String>,
    /// reach (a hand or foot to a blank's target with IK), nudge (a pose
    /// offset on a bone across the clip), look-at (head or neck follows a
    /// blank's target) or wave (a layered arm gesture).
    pub kind: String,
    /// A VRM humanoid bone name. reach: leftHand, rightHand, leftFoot or
    /// rightFoot. look-at: head or neck. wave: leftUpperArm, rightUpperArm,
    /// leftLowerArm, rightLowerArm, leftHand or rightHand. nudge: any
    /// humanoid bone (hips, spine, chest, head, leftUpperArm and so on).
    pub bone: String,
    /// How much shows, 0 to 1. Defaults to 1.
    pub weight: Option<f32>,
    /// Where in the clip it starts, as a share of its length from 0 to 1. Defaults to 0.
    pub start: Option<f32>,
    /// Where it ends, 0 to 1 and after start. Defaults to 1.
    pub end: Option<f32>,
    /// reach and look-at: the id of the blank holding the target.
    pub target: Option<String>,
    /// nudge: degrees added to the bone's rotation, [x, y, z], within 180 each way.
    pub rotation: Option<[f32; 3]>,
    /// nudge: metres added to the bone's position, [x, y, z] with y up, within 2 each way.
    pub offset: Option<[f32; 3]>,
    /// wave: how many waves between start and end, more than 0 and at most 50.
    pub cycles: Option<f32>,
    /// wave: how far each wave swings in degrees, more than 0 and at most 90.
    pub amount: Option<f32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BlankArg {
    /// Letters, digits, dashes and underscores; changes name it as their target.
    pub id: String,
    /// What fills it, one line, such as "The prop the hand reaches".
    pub label: String,
    /// prop (an asset of the project) or point (a place in the scene).
    pub kind: String,
    /// A prop blank's asset id, as list_assets returns it. Left out, the blank stays open.
    pub prop: Option<String>,
    /// A point blank's point, [x, y, z] in metres with y up. Left out, the blank stays open.
    pub point: Option<[f32; 3]>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecipeArgs {
    /// The project id.
    pub project: String,
    /// The recipe id.
    pub recipe: String,
    /// A new name; the file and id stay.
    pub name: Option<String>,
    /// Every change, in the order they are laid over the clip. Replaces them whole.
    pub changes: Option<Vec<ChangeArg>>,
    /// Every blank. Replaces them whole.
    pub blanks: Option<Vec<BlankArg>>,
    /// The `version` get_recipe answered with. Needed whenever changes or
    /// blanks are sent; a recipe that changed since is refused (moved) and
    /// nothing is written. A name only update may leave it out.
    pub expected: Option<String>,
    /// Prop blank ids to empty for good, even when the asset they name is gone.
    pub clear: Option<Vec<String>>,
}

fn layer_kind(given: &str) -> Result<LayerKind, ToolError> {
    match given
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '_'], "-")
        .as_str()
    {
        "reach" => Ok(LayerKind::Reach),
        "nudge" => Ok(LayerKind::Nudge),
        "look-at" | "lookat" => Ok(LayerKind::LookAt),
        "wave" => Ok(LayerKind::Wave),
        _ => Err(ToolError::from(format!(
            "{given} is not a kind of change. Use reach, nudge, look-at or wave."
        ))),
    }
}

fn blank_kind(given: &str) -> Result<BlankKind, ToolError> {
    match given.trim().to_ascii_lowercase().as_str() {
        "prop" => Ok(BlankKind::Prop),
        "point" => Ok(BlankKind::Point),
        _ => Err(ToolError::from(format!(
            "{given} is not a kind of blank. Use prop or point."
        ))),
    }
}

fn layer_of(arg: ChangeArg) -> Result<RecipeLayer, ToolError> {
    let defaults = LayerSettings::default();
    Ok(RecipeLayer {
        id: arg.id.unwrap_or_default(),
        kind: layer_kind(&arg.kind)?,
        bone: arg.bone,
        settings: LayerSettings {
            weight: arg.weight.unwrap_or(defaults.weight),
            start: arg.start.unwrap_or(defaults.start),
            end: arg.end.unwrap_or(defaults.end),
            target: arg.target,
            rotation: arg.rotation,
            offset: arg.offset,
            cycles: arg.cycles,
            amount: arg.amount,
        },
    })
}

fn blank_of(arg: BlankArg) -> Result<RecipeBlank, ToolError> {
    Ok(RecipeBlank {
        kind: blank_kind(&arg.kind)?,
        id: arg.id,
        label: arg.label,
        prop: arg.prop,
        point: arg.point,
    })
}

/// One change in a few fields; settings its kind does not use are left out.
fn change_view(layer: &RecipeLayer) -> Value {
    let mut out = Map::new();
    out.insert("id".into(), json!(layer.id));
    out.insert("kind".into(), to_value(layer.kind));
    out.insert("bone".into(), json!(layer.bone));
    let settings = to_short_value(&layer.settings);
    if let Value::Object(fields) = settings {
        out.extend(fields.into_iter().filter(|(_, v)| !v.is_null()));
    }
    Value::Object(out)
}

/// A recipe as agents read it.
fn recipe_view(doc: &RecipeDoc) -> Value {
    let recipe = &doc.recipe;
    let mut base = json!({
        "name": recipe.base.name, "frames": recipe.base.frames,
        "seconds": to_short_value(recipe.base.length), "hash": recipe.base.hash,
        "library": to_value(doc.base.status),
    });
    if let Some(clip) = &doc.base.clip {
        base["clip"] = json!(clip);
    }
    if let Some(warning) = &doc.base.warning {
        base["warning"] = json!(warning);
    }
    let blanks: Vec<Value> = recipe
        .blanks
        .iter()
        .map(|b| {
            json!({
                "id": b.id, "label": b.label, "kind": to_value(b.kind),
                "prop": b.prop, "point": b.point.map(to_short_value), "filled": b.filled(),
            })
        })
        .collect();
    let mut out = json!({
        "id": doc.id, "file": doc.file, "name": recipe.name, "revision": recipe.revision,
        "edited": recipe.edited, "version": doc.version, "mixamo": doc.mixamo, "base": base,
        "blanks": blanks, "changes": recipe.layers.iter().map(change_view).collect::<Vec<_>>(),
    });
    if !doc.warnings.is_empty() {
        out["warnings"] = json!(doc.warnings);
    }
    out
}

/// What to do next with a recipe: fill its blanks, add changes, or import
/// its base clip first.
fn after(doc: &RecipeDoc) -> Vec<Value> {
    let mut next = Vec::new();
    if doc.base.status == BaseStatus::Missing {
        next.push(step(
            "import_animations",
            "Import the base clip into the library before the recipe can be baked.",
        ));
    }
    if doc.recipe.blanks.iter().any(|b| !b.filled()) {
        next.push(step(
            "update_recipe",
            "Fill the open blanks: a prop's asset id or a point.",
        ));
    } else if doc.recipe.layers.is_empty() {
        next.push(step(
            "update_recipe",
            "Add changes: reach, nudge, look-at or wave on a humanoid bone.",
        ));
    }
    next
}

#[tool_router(router = recipes_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "List a project's animation recipes: each id (for the other recipe tools), name, base clip name and frame count, whether the base clip is Mixamo's, its change and blank counts with how many blanks are still open, and its version to send as expected (a prop blank whose asset the project no longer has counts as open, with a warning). A recipe file that cannot be read lists with its problem. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn list_recipes(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let listed = commands::list(host.session(), &args.project)?;
                let recipes: Vec<Value> = listed
                    .iter()
                    .map(|r| match &r.problem {
                        Some(problem) => json!({ "id": r.id, "problem": problem }),
                        None => {
                            let mut line = json!({
                                "id": r.id, "name": r.name, "base": r.base, "frames": r.frames,
                                "mixamo": r.mixamo, "changes": r.layers, "blanks": r.blanks,
                                "open": r.open, "edited": r.edited, "version": r.version,
                            });
                            if !r.warnings.is_empty() {
                                line["warnings"] = json!(r.warnings);
                            }
                            line
                        }
                    })
                    .collect();
                let out = json!({ "recipes": recipes });
                Ok(if listed.is_empty() {
                    with_next(
                        out,
                        vec![
                            step("list_animations", "Pick a base clip from the library."),
                            step("create_recipe", "Make a recipe on that clip."),
                        ],
                    )
                } else {
                    out
                })
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read an animation recipe: its name, revision, base clip (name, frames, seconds, hash, and library: found, hash-differs or missing, as matched in this computer's animation library by name and frame count, with the clip id and a warning when not exact), mixamo (a bake counts as Mixamo's: kept out of export packs and never saved to the clip library), its blanks (targets filled per project: a prop's asset id or a point; a prop the project no longer has reads as an open blank, with a warning) and its changes in order (kind, humanoid bone, weight, start and end as shares of the clip, and the settings its kind uses). Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn get_recipe(&self, Parameters(args): Parameters<RecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let doc = commands::get(host.session(), &args.project, &args.recipe)?;
                Ok(with_next(recipe_view(&doc), after(&doc)))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Make an animation recipe in a project on a clip of the animation library, as the Motion view's Recipes tab does; writes one file, recipes/<id>.recipe.json, holding the base clip's name, frames, length and hash, never keys or a mesh. It starts with no changes or blanks; add them with update_recipe. A recipe on a Mixamo clip is marked mixamo: its bake will count as Mixamo's. Website and game projects both take recipes. Nothing is baked and nothing is spent."
    )]
    async fn create_recipe(&self, Parameters(args): Parameters<CreateRecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let doc = commands::create(
                    host.session(),
                    &args.project,
                    &args.clip,
                    args.name.as_deref(),
                )?;
                host.recipe_changed(&args.project);
                let next = vec![
                    step(
                        "update_recipe",
                        "Add blanks for the targets, then the changes: reach, nudge, look-at or wave.",
                    ),
                    step("get_recipe", "Read it back."),
                ];
                Ok(with_next(recipe_view(&doc), next))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Change an animation recipe: a new name (the file and id stay), its changes, its blanks, or any of them. changes and blanks each replace the list whole, so read get_recipe first and send every entry you keep. A change without an id gets a free one. Everything is checked before anything is written: each kind's bones, weight 0 to 1, start before end, a reach or look-at aiming at a blank the recipe has, a nudge with a rotation or offset, a wave with cycles and amount, and a prop blank naming an asset of the project. When blanks is left out, the blanks stay as they are: a prop blank whose asset the project no longer has keeps its prop in the file (restoring the asset fills it again), reads as open, and warnings says which. A prop blank sent open whose stored prop is such a gone asset keeps it the same way; name it in clear to empty it for good. Sending changes, blanks or clear needs expected, the version get_recipe answered with: a recipe changed since is refused (moved), never overwritten; read it again and resend. A name only update may skip it. A name, changes and blanks land in one write. The revision does not move. Answers with the recipe as get_recipe reads it. Nothing is baked."
    )]
    async fn update_recipe(&self, Parameters(args): Parameters<UpdateRecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let lists = args.changes.is_some() || args.blanks.is_some() || args.clear.is_some();
                if lists && args.expected.is_none() {
                    return Err(ToolError {
                        message: format!(
                            "Sending changes, blanks or clear needs expected: the version get_recipe answered with for {}, so a person's save since is never overwritten.",
                            args.recipe
                        ),
                        code: NEEDS_VERSION.to_string(),
                    });
                }
                let change = RecipeChange {
                    name: args.name,
                    layers: args
                        .changes
                        .map(|c| c.into_iter().map(layer_of).collect::<Result<Vec<_>, _>>())
                        .transpose()?,
                    blanks: args
                        .blanks
                        .map(|b| b.into_iter().map(blank_of).collect::<Result<Vec<_>, _>>())
                        .transpose()?,
                    expected: args.expected,
                    clear: args.clear.unwrap_or_default(),
                };
                let doc = commands::update(host.session(), &args.project, &args.recipe, change)?;
                host.recipe_changed(&args.project);
                Ok(with_next(recipe_view(&doc), after(&doc)))
            })
            .await;
        respond(found)
    }
}
