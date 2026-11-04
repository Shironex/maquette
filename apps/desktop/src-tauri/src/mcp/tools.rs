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
pub mod groups;
pub mod handoffs;
mod hints;
pub mod image_api;
pub mod image_round;
pub mod inbox;
pub mod inbox_triage;
pub mod library;
pub mod library_batch;
pub mod library_many;
mod list_changed;
pub mod materials;
pub mod motion;
pub mod packs;
pub mod page;
pub mod parts;
pub mod paths;
pub mod picks;
pub mod plan_camera;
pub mod plan_edits;
pub mod plan_extras;
pub mod plans;
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
pub mod start;
pub mod starters;
pub mod style;
pub mod timeline;
pub mod trash;

mod errors;
mod instructions;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use maquette_core::pipeline::cache::project_cache_dir;
use maquette_core::project;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, SubscriptionFilter,
};
use rmcp::service::{NotificationContext, RequestContext, SubscriptionContext};
use rmcp::{ErrorData, RoleServer, ServerHandler, schemars, tool_handler};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::host::Host;
use super::hub::{CallOutcome, Hub};
pub use errors::ToolError;
use instructions::INSTRUCTIONS;

/// The status a run reports while its Ship step waits for a person.
pub const WAITING: &str = "waiting for approval";

type Out = Result<CallToolResult, ErrorData>;

fn json_result(value: &Value) -> Out {
    let mut value = value.clone();
    hints::add_waiting_fields(&mut value);
    let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

fn failed(err: impl Into<ToolError>) -> Out {
    let body = errors::body(&err.into());
    Ok(CallToolResult::error(vec![ContentBlock::text(
        body.to_string(),
    )]))
}

/// A tool's answer: its JSON, or the reason it could not do the job.
fn respond(found: Result<Value, ToolError>) -> Out {
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

/// Starts every description in `router` with its group, so an agent can
/// tell the groups apart in a long list.
fn tagged(group: &str, mut router: ToolRouter<McpTools>) -> ToolRouter<McpTools> {
    for route in router.map.values_mut() {
        let text = route.attr.description.as_deref().unwrap_or_default();
        route.attr.description = Some(format!("[{group}] {text}").into());
    }
    router
}

/// One MCP session's view of the app.
#[derive(Clone)]
pub struct McpTools {
    host: Arc<dyn Host>,
    hub: Arc<Hub>,
    tool_router: ToolRouter<Self>,
}

impl McpTools {
    /// Every tool stays routed: the list leaves out the groups a person
    /// turned off, and a call to one of their tools is refused, both read
    /// from the setting as it is now.
    pub fn new(host: Arc<dyn Host>, hub: Arc<Hub>) -> Self {
        Self {
            host,
            hub,
            tool_router: Self::router(),
        }
    }

    /// Every tool's name and description, as agents read them.
    pub(crate) fn descriptions() -> Vec<(String, String)> {
        Self::router()
            .list_all()
            .into_iter()
            .map(|t| {
                let description = t.description.as_deref().unwrap_or_default().to_string();
                (t.name.to_string(), description)
            })
            .collect()
    }

    /// Every tool, one `#[tool_router]` per file in `tools/`.
    fn router() -> ToolRouter<Self> {
        let mut all = ToolRouter::new();
        all.merge(tagged("start", Self::start_router()));
        all.merge(tagged(
            "project",
            Self::projects_router() + Self::trash_router(),
        ));
        all.merge(tagged(
            "lab",
            Self::concepts_router()
                + Self::concept_jobs_router()
                + Self::codex_usage_router()
                + Self::image_api_router()
                + Self::style_router()
                + Self::picks_router(),
        ));
        all.merge(tagged("refs", Self::refs_router()));
        all.merge(tagged(
            "plan",
            Self::plans_router()
                + Self::plan_edits_router()
                + Self::plan_extras_router()
                + Self::plan_camera_router(),
        ));
        all.merge(tagged(
            "send",
            Self::sending_router()
                + Self::redo_router()
                + Self::splitting_router()
                + Self::credits_router()
                + Self::handoffs_router(),
        ));
        all.merge(tagged(
            "inbox",
            Self::inbox_router() + Self::inbox_triage_router(),
        ));
        all.merge(tagged(
            "prepare",
            Self::materials_router() + Self::prepare_router() + Self::parts_router(),
        ));
        all.merge(tagged("built", Self::built_router()));
        all.merge(tagged(
            "pipeline",
            Self::runs_router()
                + Self::bake_router()
                + Self::starters_router()
                + Self::page_router()
                + Self::ship_router()
                + Self::motion_router(),
        ));
        all.merge(tagged("game", Self::game_router()));
        all.merge(tagged(
            "library",
            Self::library_router() + Self::library_many_router() + Self::library_batch_router(),
        ));
        all.merge(tagged(
            "animation",
            Self::animations_router() + Self::retarget_router(),
        ));
        all.merge(tagged(
            "export",
            Self::packs_router() + Self::renders_router(),
        ));
        all.merge(tagged(
            "avatar-rig",
            Self::avatar_router() + Self::rig_router(),
        ));
        all
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
        job: impl FnOnce(&dyn Host, &Hub) -> Result<T, ToolError> + Send + 'static,
    ) -> Result<T, ToolError> {
        let host = Arc::clone(&self.host);
        let hub = Arc::clone(&self.hub);
        tokio::task::spawn_blocking(move || job(host.as_ref(), hub.as_ref()))
            .await
            .map_err(|e| ToolError::from(format!("The task stopped unexpectedly. {e}")))?
    }
}

fn workspace(host: &dyn Host) -> Result<(PathBuf, maquette_core::WorkspaceConfig), ToolError> {
    Ok(host.session().workspace()?)
}

fn cache_dir(host: &dyn Host, project_id: &str) -> Result<PathBuf, ToolError> {
    let (root, config) = workspace(host)?;
    project::load(&root, &config.layout, project_id)?;
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

/// What a result tells the activity list.
struct Logged {
    outcome: CallOutcome,
    message: Option<String>,
    /// The approval a waiting result raised.
    approval: Option<String>,
}

/// The approval id a waiting result names, on its own or on its first job.
fn approval_of(value: &Value) -> Option<String> {
    ["/approval/id", "/jobs/0/approval/id"]
        .iter()
        .find_map(|at| value.pointer(at).and_then(Value::as_str))
        .map(str::to_string)
}

/// The status word, error text and approval of a result.
fn outcome(result: &Result<CallToolResponse, ErrorData>) -> Logged {
    let done = |outcome, message| Logged {
        outcome,
        message,
        approval: None,
    };
    match result {
        Err(err) => done(CallOutcome::Error, Some(err.message.to_string())),
        Ok(CallToolResponse::Complete(answer)) => {
            let text = answer.content.iter().find_map(|c| match c {
                ContentBlock::Text(t) => Some(t.text.clone()),
                _ => None,
            });
            if answer.is_error == Some(true) {
                return done(CallOutcome::Error, text.map(errors::message_of));
            }
            let value = text
                .as_deref()
                .and_then(|t| serde_json::from_str::<Value>(t).ok());
            let status = value
                .as_ref()
                .and_then(|v| v.get("status").and_then(Value::as_str).map(str::to_string));
            match status {
                Some(s) if s == WAITING => Logged {
                    outcome: CallOutcome::Waiting,
                    message: Some(s),
                    approval: value.as_ref().and_then(approval_of),
                },
                other => done(CallOutcome::Ok, other),
            }
        }
        Ok(_) => done(CallOutcome::Ok, None),
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
        .with_server_info(
            Implementation::new("maquette", env!("CARGO_PKG_VERSION")).with_title("Maquette"),
        )
        .with_instructions(INSTRUCTIONS)
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        list_changed::forward_to_session(&context, self.hub.watch_tools());
    }

    fn accepted_subscription_filter(
        &self,
        requested: &SubscriptionFilter,
    ) -> Option<SubscriptionFilter> {
        list_changed::accepted(requested)
    }

    async fn listen(&self, subscription: SubscriptionContext) -> Result<(), ErrorData> {
        list_changed::listen(subscription, self.hub.watch_tools()).await;
        Ok(())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let off = groups::off_now(self.host.as_ref());
        Ok(groups::list_result(
            groups::listed(self.tool_router.list_all(), &off),
            &context,
        ))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let tool = request.name.to_string();
        let detail = describe(request.arguments.as_ref());
        let started = Instant::now();
        let result = match groups::off_group_of(&tool, self.host.as_ref()) {
            // A list from before the person turned the group off: nothing runs.
            Some(group) => failed(groups::refusal(&tool, group)).map(CallToolResponse::from),
            None => {
                self.tool_router
                    .call(ToolCallContext::new(self, request, context))
                    .await
            }
        };
        let result = groups::scrub_off_tools(result, &groups::off_now(self.host.as_ref()));
        let logged = outcome(&result);
        #[expect(clippy::cast_precision_loss, reason = "call durations are small")]
        let ms = started.elapsed().as_millis() as f64;
        self.hub.log_waiting_call(
            &tool,
            detail,
            logged.outcome,
            logged.message,
            ms,
            logged.approval,
        );
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The longest a tool's description may grow. Detail goes on the arguments.
    const MAX_DESCRIPTION: usize = 1250;

    #[test]
    fn every_tool_has_a_group_and_a_description_of_reasonable_length() {
        for (name, description) in McpTools::descriptions() {
            assert!(description.starts_with('['), "{name} has no group");
            let (tag, rest) = description.split_once("] ").expect("group");
            assert!(!tag.is_empty() && !rest.is_empty(), "{name}");
            assert!(
                description.len() <= MAX_DESCRIPTION,
                "{name} has {} characters",
                description.len()
            );
        }
    }
}
