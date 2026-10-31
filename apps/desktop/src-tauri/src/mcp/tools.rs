//! The tools agents call. Read tools look at the workspace; act tools run
//! pipelines, accept models, render thumbnails, start, star and pick
//! concepts and send assets to 3D.
//!
//! The agent rules hold here by construction: Blender only ever runs
//! headless (thumbnails and pipeline steps), no tool opens its window, and
//! there is no screenshot or screen capture tool at all. Runs that would
//! ship wait for a person when "Ask before shipping" is on, and a send to
//! Tripo or Meshy always waits for a person before it spends credits.

pub mod animations;
pub mod avatar;
pub mod bake;
pub mod built;
pub mod codex_usage;
pub mod concept_jobs;
pub mod concepts;
pub mod credits;
pub mod game;
pub mod handoffs;
pub mod image_api;
pub mod image_round;
pub mod inbox;
pub mod inbox_triage;
pub mod library;
pub mod library_batch;
pub mod library_many;
pub mod materials;
pub mod motion;
pub mod packs;
pub mod page;
pub mod parts;
pub mod paths;
pub mod picks;
pub mod prepare;
pub mod projects;
pub mod refs;
pub mod renders;
pub mod retarget;
pub mod rig;
pub mod runs;
pub mod sending;
pub mod ship;
pub mod splitting;
pub mod starters;
pub mod style;
pub mod timeline;
pub mod trash;

mod instructions;

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
use instructions::INSTRUCTIONS;

/// The status a run reports while its Ship step waits for a person.
pub const WAITING: &str = "waiting for approval";

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
            tool_router: Self::router(),
        }
    }

    /// Every tool, one `#[tool_router]` per file in `tools/`.
    fn router() -> ToolRouter<Self> {
        Self::runs_router()
            + Self::projects_router()
            + Self::motion_router()
            + Self::inbox_router()
            + Self::inbox_triage_router()
            + Self::concepts_router()
            + Self::concept_jobs_router()
            + Self::codex_usage_router()
            + Self::image_api_router()
            + Self::style_router()
            + Self::picks_router()
            + Self::refs_router()
            + Self::sending_router()
            + Self::materials_router()
            + Self::avatar_router()
            + Self::prepare_router()
            + Self::parts_router()
            + Self::splitting_router()
            + Self::redo_router()
            + Self::credits_router()
            + Self::handoffs_router()
            + Self::built_router()
            + Self::bake_router()
            + Self::starters_router()
            + Self::page_router()
            + Self::game_router()
            + Self::ship_router()
            + Self::library_router()
            + Self::library_many_router()
            + Self::library_batch_router()
            + Self::packs_router()
            + Self::rig_router()
            + Self::renders_router()
            + Self::animations_router()
            + Self::retarget_router()
            + Self::trash_router()
    }

    /// The name of every tool the server lists, sorted. `tool_names.rs`
    /// writes them to the snapshot the handshake test and the skills check
    /// read.
    pub fn tool_names() -> Vec<String> {
        let mut names: Vec<String> = Self::router()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect();
        names.sort();
        names
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
