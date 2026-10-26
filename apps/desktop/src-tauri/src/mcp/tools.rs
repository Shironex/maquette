//! The tools agents call. Read tools look at the workspace; act tools run
//! pipelines, accept models, render thumbnails, start, star and pick
//! concepts and send assets to 3D.
//!
//! The agent rules hold here by construction: Blender only ever runs
//! headless (thumbnails and pipeline steps), no tool opens its window, and
//! there is no screenshot or screen capture tool at all. Runs that would
//! ship wait for a person when "Ask before shipping" is on, and a send to
//! Tripo or Meshy always waits for a person before it spends credits.

pub mod bake;
pub mod built;
pub mod concept_jobs;
pub mod concepts;
pub mod credits;
pub mod handoffs;
pub mod inbox;
pub mod materials;
pub mod picks;
pub mod prepare;
pub mod projects;
pub mod runs;
pub mod sending;
pub mod starters;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use maquette_core::pipeline::cache::project_cache_dir;
use maquette_core::{ErrorPayload, project};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, schemars, tool_handler};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::host::Host;
use super::hub::{CallOutcome, Hub};

/// The status a run reports while its Ship step waits for a person.
pub const WAITING: &str = "waiting for approval";

const INSTRUCTIONS: &str = "Maquette turns concept art into real-time 3D web scenes. \
Start with list_projects, then get_pipeline or list_assets for a project. \
run_pipeline runs a project's pipeline; when it reaches the Ship step it may answer \
\"waiting for approval\": a person approves or denies shipping in the Maquette app, \
so poll get_run instead of starting the run again. send_to_3d sends an asset's refs to \
Tripo or Meshy (it always waits for a person to approve the credits in the app; follow it \
with get_provider_job) or writes a by hand handoff folder; send_batch_to_3d sends several \
assets at once under one approval. redo_inbox_model sends an inbox model's asset again \
prefilled from its send (redo_inbox_models for several), under the same approval. get_credits \
reads the provider balances and the local ledger of paid sends (API spend and By hand \
subscription spend apart). list_handoffs lists the By hand sends waiting for a model made on \
the provider's site, the export file name each waits for and what the Downloads watcher \
matched; it reads only. make_object_refs turns the objects of one concept into \
one isolated image each. get_concepts reads the Lab, with its failed jobs; a job Codex rate \
limits runs again on its own a few times, and retry_concept_jobs runs failed ones again (same \
prompt, refs and file, no new direction). pick_concepts puts concepts on the Board \
as Send picks to Refs does: each image goes into refs/ and becomes or joins an asset at Refs \
(named after its object for object refs, else after its direction), ready for send_to_3d; \
picking an image again returns its asset. star_concepts stars or unstars them. Both spend \
nothing and the app updates live. \
set_compression_presets changes the per model texture size, data map size, codec and \
meshopt level that get_pipeline shows (it spends nothing; locked rows refuse). \
get_material_tweaks reads an asset's material names, tweaks and per map codecs; \
set_material_tweaks changes its tint, roughness, metalness and emissive strength (for every \
material or one by name) and the ETC1S or UASTC codec of its color, normal and other maps; \
Compress applies them to the shipped model on the next run. \
get_real_size reads an asset's real size and fix-ups with what its model measures; set_real_size \
sets the size its model is scaled to (height, width, depth or longest, cm or m); add_fixups and \
remove_fixups change its fix-ups (decimate, shrink textures, merge, apply transforms, remove loose \
parts, origin to bottom centre), which the Build scene step applies headless on a copy of every \
version; list_fixups lists them and preview_fixups shows their numbers before and after. \
set_pad_edges sets whether an asset's texture edges around its UV islands are padded before the \
textures shrink (models accepted from the Inbox pad by default). \
get_bake_settings reads the Bake light step's lightmap size, samples and denoiser and the \
assets that skip the bake; set_bake_settings changes them and set_skip_bake sets assets to skip \
the bake (they keep their own material lighting and ship no lightmap; models accepted from the \
Inbox skip it by default). get_starters compares a project's starter scripts in tools/ \
with the ones this Maquette ships, and update_starters brings the outdated ones up to date, as \
the Pipeline view's Update banner does, for the project named in the call. \
get_layout reads where each model stands in the scene, the camera stops and the first view \
budget; set_layout changes them (metres, y up, degrees) and the app updates live; the next \
run_pipeline rebuilds the scene and its posters from it. copy_asset copies an asset with its refs and \
versions to another project. \
add_built_asset adds a built asset: a model a Blender script in the project makes. Its script \
and helpers are copied into the project's built/ folder and it builds headless into the asset's \
next version, with no credits and no approval; rebuild_built_asset builds one again (or all \
whose files changed) and list_built_assets reads their state and last build. A full \
run_pipeline rebuilds the changed ones first. \
Blender always runs headless here; there is no tool that opens Blender's window or \
captures the screen.";

type Out = Result<CallToolResult, ErrorData>;

fn reason(err: impl Into<ErrorPayload>) -> String {
    err.into().message
}

fn json_result(value: &Value) -> Out {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

fn failed(message: impl Into<String>) -> Out {
    Ok(CallToolResult::error(vec![ContentBlock::text(
        message.into(),
    )]))
}

/// A tool's answer: its JSON, or the reason it could not do the job.
fn respond(found: Result<Value, String>) -> Out {
    found.map_or_else(failed, |v| json_result(&v))
}

fn to_value(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// [`to_value`] through text, so `f32` fields read as written (0.2, not
/// 0.20000000298023224).
fn to_short_value(value: impl Serialize) -> Value {
    serde_json::to_string(&value)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}

/// Tells a missing field from an explicit null.
fn explicit<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectArgs {
    /// The project id, as list_projects returns it.
    pub project: String,
}

/// One MCP session's view of the app.
#[derive(Clone)]
pub struct McpTools {
    host: Arc<dyn Host>,
    hub: Arc<Hub>,
    tool_router: ToolRouter<Self>,
}

impl McpTools {
    pub fn new(host: Arc<dyn Host>, hub: Arc<Hub>) -> Self {
        Self {
            host,
            hub,
            tool_router: Self::runs_router()
                + Self::projects_router()
                + Self::inbox_router()
                + Self::concepts_router()
                + Self::concept_jobs_router()
                + Self::picks_router()
                + Self::sending_router()
                + Self::materials_router()
                + Self::prepare_router()
                + Self::redo_router()
                + Self::credits_router()
                + Self::handoffs_router()
                + Self::built_router()
                + Self::bake_router()
                + Self::starters_router(),
        }
    }

    /// Runs `job` on a blocking thread, since the services touch the disk
    /// and may wait on locks.
    async fn blocking<T: Send + 'static>(
        &self,
        job: impl FnOnce(&dyn Host, &Hub) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        let host = Arc::clone(&self.host);
        let hub = Arc::clone(&self.hub);
        tokio::task::spawn_blocking(move || job(host.as_ref(), hub.as_ref()))
            .await
            .map_err(|e| format!("The task stopped unexpectedly. {e}"))?
    }
}

fn workspace(host: &dyn Host) -> Result<(PathBuf, maquette_core::WorkspaceConfig), String> {
    host.session().workspace().map_err(reason)
}

fn cache_dir(host: &dyn Host, project_id: &str) -> Result<PathBuf, String> {
    let (root, config) = workspace(host)?;
    project::load(&root, &config.layout, project_id).map_err(reason)?;
    Ok(project_cache_dir(
        &root.join(&config.layout.cache),
        project_id,
    ))
}

/// A few words about the arguments, for the activity list.
fn describe(args: Option<&serde_json::Map<String, Value>>) -> String {
    let Some(args) = args else {
        return String::new();
    };
    let mut parts = Vec::new();
    for (key, value) in args {
        let text = match value {
            Value::String(s) => s.clone(),
            Value::Bool(true) => key.clone(),
            Value::Bool(false) | Value::Null => continue,
            Value::Array(items) => format!("{} {key}", items.len()),
            other => other.to_string(),
        };
        parts.push(
            if matches!(value, Value::Bool(_) | Value::Array(_)) || key == "project" || key == "id"
            {
                text
            } else {
                format!("{key} {text}")
            },
        );
    }
    let mut line = parts.join(", ");
    if line.chars().count() > 90 {
        line = line.chars().take(89).collect::<String>() + "\u{2026}";
    }
    line
}

/// The status word and error text of a result, for the activity list.
fn outcome(result: &Result<CallToolResponse, ErrorData>) -> (CallOutcome, Option<String>) {
    match result {
        Err(err) => (CallOutcome::Error, Some(err.message.to_string())),
        Ok(CallToolResponse::Complete(done)) => {
            let text = done.content.iter().find_map(|c| match c {
                ContentBlock::Text(t) => Some(t.text.clone()),
                _ => None,
            });
            if done.is_error == Some(true) {
                return (CallOutcome::Error, text);
            }
            let status = text
                .as_deref()
                .and_then(|t| serde_json::from_str::<Value>(t).ok())
                .and_then(|v| v.get("status").and_then(Value::as_str).map(str::to_string));
            match status {
                Some(s) if s == WAITING => (CallOutcome::Waiting, Some(s)),
                other => (CallOutcome::Ok, other),
            }
        }
        Ok(_) => (CallOutcome::Ok, None),
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("maquette", env!("CARGO_PKG_VERSION")).with_title("Maquette"),
            )
            .with_instructions(INSTRUCTIONS)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let tool = request.name.to_string();
        let detail = describe(request.arguments.as_ref());
        let started = Instant::now();
        let result = self
            .tool_router
            .call(ToolCallContext::new(self, request, context))
            .await;
        let (kind, message) = outcome(&result);
        #[expect(clippy::cast_precision_loss, reason = "call durations are small")]
        let ms = started.elapsed().as_millis() as f64;
        self.hub.log_call(&tool, detail, kind, message, ms);
        result
    }
}
