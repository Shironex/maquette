//! Recipe bakes for agents: bake_recipe starts one headless Blender run of a
//! recipe onto a rigged model, get_recipe_bake reads how it stands with its
//! review, and add_recipe_bake asks a person to add the staged result as the
//! model's next version. Only the person's Add in the app adds it. Nothing
//! here spends credits.

use maquette_core::animations::recipes::bake::{
    OUT_OF_DATE, RecipeBakeRecord, RecipeBakeRequest, RecipeBakeStarter, RecipeBakeState,
    RecipeBakes, not_bakeable, open_blanks, ready_to_add,
};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::hints::{step, with_next};
use super::recipes::RecipeArgs;
use super::{Host, McpTools, Out, ToolError, WAITING, respond, to_value, workspace};
use crate::commands::{recipe_bakes as commands, recipes};
use crate::mcp::hub::Hub;

/// The code of a bake refused for a recipe with open blanks.
pub(super) const OPEN_BLANKS: &str = "open-blanks";
/// The code of a bake refused for a recipe using format 2 features (pose
/// keys or grips) that this Maquette cannot bake yet.
pub(super) const NOT_BAKEABLE: &str = "not-bakeable-yet";
/// The code of an add refused for a bake that is out of date.
pub(super) const STALE_BAKE: &str = "stale-bake";

/// Where a person adds a bake an agent asked to add.
pub(super) const ADD_IN_APP: &str = "The approval prompt in the Maquette app: Review opens the bake in the Motion view's Recipes tab, where Add as vN adds it. Also listed under Settings > Agents. No tool can approve.";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BakeRecipeArgs {
    /// The project id.
    pub project: String,
    /// The recipe id, as list_recipes returns it.
    pub recipe: String,
    /// The rigged model to bake onto: an asset id of the project, as list_assets returns it.
    pub asset: String,
    /// The `version` get_recipe answered with. Given, a recipe that changed
    /// since is refused (moved), so the bake is the recipe you read.
    pub expected: Option<String>,
}

fn bakes(host: &dyn Host) -> Result<&RecipeBakes, String> {
    host.recipe_bakes()
        .ok_or_else(|| "Recipe bakes are not available in this session.".to_string())
}

/// A bake as agents read it: the staged file's path is left out, since only
/// a person's review adds it.
fn shown(record: &RecipeBakeRecord) -> Value {
    let mut value = to_value(record);
    if let Some(result) = value.get_mut("result").and_then(Value::as_object_mut) {
        result.remove("file");
    }
    value
}

/// What to do next with a bake, by how it stands.
fn after(record: &RecipeBakeRecord) -> Vec<Value> {
    if record.state == RecipeBakeState::Added {
        return vec![step(
            "list_assets",
            &format!(
                "A person added this bake as v{} of the model.",
                record.added_version.unwrap_or_default()
            ),
        )];
    }
    if record.running() {
        return vec![step(
            "get_recipe_bake",
            "Follow the bake: it runs in headless Blender, seconds to minutes.",
        )];
    }
    if let (Some(_), Some(why)) = (&record.result, &record.stale) {
        return vec![step(
            "bake_recipe",
            &format!("This bake is out of date: {why}"),
        )];
    }
    if record.result.is_some() {
        return vec![step(
            "add_recipe_bake",
            "The bake is staged with its review. Ask a person to add it as the model's next version; they review it in the app.",
        )];
    }
    vec![step("bake_recipe", "Fix what stopped it and bake again.")]
}

/// A bake as get_recipe_bake answers it: waiting, with its approval, while
/// a person has an add of it to answer.
fn read_answer(hub: &Hub, record: &RecipeBakeRecord) -> Value {
    let mut value = json!({ "bake": shown(record) });
    if let Some(waiting) = hub.recipe_bake_for(&record.project, &record.recipe) {
        value["status"] = json!(WAITING);
        value["approval"] = json!(waiting.id);
        value["whereToApprove"] = json!(ADD_IN_APP);
        return with_next(
            value,
            vec![step(
                "get_recipe_bake",
                "A person has the add to answer. Once added, the bake reads as added with its version.",
            )],
        );
    }
    let next = after(record);
    with_next(value, next)
}

/// Why a bake cannot be added, with `stale-bake` for one out of date.
fn refused_add(err: maquette_core::CoreError) -> ToolError {
    let mut out = ToolError::from(err);
    if out.message.starts_with(OUT_OF_DATE) {
        out.code = STALE_BAKE.to_string();
    }
    out
}

impl McpTools {
    /// Every recipe tool: making and changing recipes, baking them, and the
    /// recipe library.
    pub(super) fn recipe_routers() -> ToolRouter<Self> {
        Self::recipes_router() + Self::recipe_bake_router() + Self::recipe_library_router()
    }
}

#[tool_router(router = recipe_bake_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Bake an animation recipe onto a rigged model of its project, in headless Blender, as the Motion view's Apply does: its clips go on through the retarget and play as the preview plays them (trims and time edits, blends lined up at each seam, then mirror), then the recipe's changes are solved over it exactly (reach by IK, look-at clamped to 80 degrees, nudge and wave eased) and pins last (each hand or foot held on its spot by IK, a foot kept flat), into one new clip named after the recipe. It answers at once; follow it with get_recipe_bake. The model is staged in the workspace cache, not added to the project: add_recipe_bake asks a person to review it and add it as the next version in the app. A recipe with open blanks is refused (open-blanks) naming them. Pose keys and grips are refused (not-bakeable-yet). A bake counts as Mixamo's when any of its clips is: its model carries a mark, recorded on the asset when a marked file comes in as a version, so export packs leave its clips out and save_model_clips skips its clip, and add_model_file refuses the staged file (staged-bake). Every clip an earlier bake of this recipe made is replaced, whatever its name; another same-named clip stays and the new one takes _2. One bake per recipe runs at a time and replaces the staged one. Spends no credits."
    )]
    async fn bake_recipe(&self, Parameters(args): Parameters<BakeRecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let doc = recipes::get(host.session(), &args.project, &args.recipe)?;
                if let Some(message) = not_bakeable(&doc) {
                    return Err(ToolError {
                        message,
                        code: NOT_BAKEABLE.to_string(),
                    });
                }
                let open = open_blanks(&doc);
                if !open.is_empty() {
                    return Err(ToolError {
                        message: format!(
                            "{} has open blanks, so it cannot be baked: {}. Fill each with a prop's asset id or a point first.",
                            doc.recipe.name,
                            open.join(", ")
                        ),
                        code: OPEN_BLANKS.to_string(),
                    });
                }
                let request = RecipeBakeRequest {
                    project: args.project,
                    recipe: args.recipe,
                    asset: args.asset,
                    expected: args.expected,
                };
                let record = commands::start(
                    host.session(),
                    bakes(host)?,
                    host.toolbox(),
                    &request,
                    RecipeBakeStarter::Agent,
                )?;
                let next = after(&record);
                Ok(with_next(shown(&record), next))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Read a recipe's last bake, or bake null when it was never baked: its state (running, done, failed or cancelled), who started it, progress while it runs, why it stopped, and when done the result: the recipe version and clip hashes it was baked from (base, then joinedClips), mixamo, the new clip's name and frames, the facing read from the rig, the planted feet's worst slide in cm and the rig's worst stretch past a healthy skin in mm (each with the base clip's own), each reach's gap to its target in cm at its end and the first frame its elbow or knee flips, each pin's frames and its worst drift from its spot in cm while it holds whole (with how far the bone moves there without the pin), and the timings; review holds each number against its limit as pass, warn or not-measured (hand or foot gap 2 cm, elbow or knee no flip, pin drift 1 cm, foot slide 1.5 cm over the base clip's own, rig stretch 5 mm over the base clip's) with how many warn, never stopping an add; clipNote says whether the clip replaces this recipe's own older ones or took a _2 name; stale says why a finished bake is out of date (the recipe changed, a clip it used, named, has other keys, or the model has a newer version); bake it again then. While a person has an add_recipe_bake of it to answer, status is waiting for approval with the approval id; once added, state is added with addedVersion. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn get_recipe_bake(&self, Parameters(args): Parameters<RecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                let record = commands::get(
                    host.session(),
                    host.recipe_bakes(),
                    &args.project,
                    &args.recipe,
                )?;
                Ok(match record {
                    Some(record) => read_answer(hub, &record),
                    None => with_next(
                        json!({ "bake": Value::Null }),
                        vec![step("bake_recipe", "Bake the recipe onto a rigged model.")],
                    ),
                })
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Ask a person to add a recipe's finished bake as its model's next version. Nothing is added by this tool: it answers status waiting for approval with the approval id, and the person reviews the bake in the Motion view's Recipes tab (the baked clip on the staged model next to the base clip, and each number against its limit) and adds it there. Poll get_recipe_bake: it reads as waiting while the person has it, then added with addedVersion. Adding records on the new version which recipe and recipe version made it, and Mixamo's for a Mixamo base, and clears the staged model. A bake that is out of date (the recipe changed, or the model has another newest version) is refused (stale-bake): bake it again. A bake still running, failed or added already is refused too. Asking twice returns the same approval. Spends no credits."
    )]
    async fn add_recipe_bake(&self, Parameters(args): Parameters<RecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                let (root, config) = workspace(host)?;
                let (record, _) =
                    ready_to_add(&root, &config, &args.project, &args.recipe).map_err(refused_add)?;
                let approval = hub.request_recipe_bake(&record);
                let model = record
                    .result
                    .as_ref()
                    .map_or_else(|| record.asset.clone(), |r| format!("{} v{}", r.asset_name, r.from_version + 1));
                let answer = json!({
                    "status": WAITING,
                    "approval": approval,
                    "whereToApprove": ADD_IN_APP,
                    "message": format!("Adding this bake as {model} waits for a person to review it in Maquette's Motion view and add it there. Poll get_recipe_bake; nothing is added until then."),
                    "bake": shown(&record),
                });
                Ok(with_next(
                    answer,
                    vec![step("get_recipe_bake", "Follow the add: it reads as added with its version once the person adds it.")],
                ))
            })
            .await;
        respond(found)
    }
}
