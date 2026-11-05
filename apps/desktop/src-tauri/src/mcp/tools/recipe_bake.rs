//! Recipe bakes for agents: bake_recipe starts one headless Blender run of a
//! recipe onto a rigged model, get_recipe_bake reads how it stands. The
//! result is staged in the workspace cache for a person to review; no tool
//! adds it to the model as a version. Nothing here spends credits.

use maquette_core::animations::recipes::bake::{
    RecipeBakeRecord, RecipeBakeRequest, RecipeBakeStarter, RecipeBakes, open_blanks,
};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::hints::{step, with_next};
use super::recipes::RecipeArgs;
use super::{Host, McpTools, Out, ToolError, respond, to_value};
use crate::commands::{recipe_bakes as commands, recipes};

/// The code of a bake refused for a recipe with open blanks.
pub(super) const OPEN_BLANKS: &str = "open-blanks";

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
            "get_recipe",
            "The bake is staged. A person reviews it and adds it as the model's next version in the app; no tool adds it.",
        )];
    }
    vec![step("bake_recipe", "Fix what stopped it and bake again.")]
}

impl McpTools {
    /// Every recipe tool: making and changing recipes, and baking them.
    pub(super) fn recipe_routers() -> ToolRouter<Self> {
        Self::recipes_router() + Self::recipe_bake_router()
    }
}

#[tool_router(router = recipe_bake_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Bake an animation recipe onto a rigged model of its project, in headless Blender, as the Motion view's Apply does: the base clip goes on through the retarget, then the recipe's changes are solved over it exactly (reach by IK to its target, look-at clamped to 80 degrees, nudge and wave eased in and out), into one new clip named after the recipe. It answers at once; follow it with get_recipe_bake. The model is staged in the workspace cache, never added to the project: a person reviews it and adds it as the next version in the app. A recipe with open blanks is refused (open-blanks) naming them. A bake of a Mixamo base counts as Mixamo's: its model carries a mark, recorded on the asset when a marked file comes in as a version (inbox accept, a build, a library entry or a game import), so export packs leave its clips out and save_model_clips skips its clip, and add_model_file refuses the staged file (staged-bake). One bake per recipe runs at a time; a new one replaces the staged one before it. Spends no credits."
    )]
    async fn bake_recipe(&self, Parameters(args): Parameters<BakeRecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let doc = recipes::get(host.session(), &args.project, &args.recipe)?;
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
        description = "Read a recipe's last bake, or bake null when it was never baked: its state (running, done, failed or cancelled), who started it, progress while it runs, why it stopped, and when done the result: the recipe version and base clip hash it was baked from, mixamo, the new clip's name and frames, the facing read from the rig, the planted feet's worst slide in cm (and the base clip's own), the rig's worst stretch past a healthy skin in mm (and the base clip's), each reach's gap to its target in cm at its end (reached within 2 cm) and the first frame its elbow or knee flips, and the timings; stale says why a finished bake is out of date (the recipe changed since, or the model has another newest version); bake it again then. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn get_recipe_bake(&self, Parameters(args): Parameters<RecipeArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let record = commands::get(
                    host.session(),
                    host.recipe_bakes(),
                    &args.project,
                    &args.recipe,
                )?;
                Ok(match record {
                    Some(record) => {
                        let next = after(&record);
                        with_next(json!({ "bake": shown(&record) }), next)
                    }
                    None => with_next(
                        json!({ "bake": Value::Null }),
                        vec![step("bake_recipe", "Bake the recipe onto a rigged model.")],
                    ),
                })
            })
            .await;
        respond(found)
    }
}
