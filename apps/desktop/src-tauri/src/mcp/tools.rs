//! The tools agents call. Read tools look at the workspace; act tools run
//! pipelines, accept models, render thumbnails and start concepts.
//!
//! The agent rules hold here by construction: Blender only ever runs
//! headless (thumbnails and pipeline steps), no tool opens its window, and
//! there is no screenshot or screen capture tool at all. Runs that would
//! ship wait for a person when "Ask before shipping" is on.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maquette_core::concepts::{Aspect, NewConcepts};
use maquette_core::inbox::{self, AcceptRequest};
use maquette_core::pipeline::cache::project_cache_dir;
use maquette_core::pipeline::history::{self, LogEntry, RunRecord, RunState};
use maquette_core::pipeline::runner::{JobRunner as _, RunOptions, StepGate};
use maquette_core::thumbs::{self, Requested, ThumbTarget};
use maquette_core::{ErrorPayload, project};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, schemars, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::gate::ShipGate;
use super::host::Host;
use super::hub::{CallOutcome, Hub};
use crate::commands::{lab, pipeline};

/// The status a run reports while its Ship step waits for a person.
pub const WAITING: &str = "waiting for approval";

const INSTRUCTIONS: &str = "Maquette turns concept art into real-time 3D web scenes. \
Start with list_projects, then get_pipeline or list_assets for a project. \
run_pipeline runs a project's pipeline; when it reaches the Ship step it may answer \
\"waiting for approval\": a person approves or denies shipping in the Maquette app, \
so poll get_run instead of starting the run again. Blender always runs headless here; \
there is no tool that opens Blender's window or captures the screen.";

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

/// A run state as agents read it.
fn state_word(state: RunState) -> &'static str {
    match state {
        RunState::Queued => "queued",
        RunState::Running => "running",
        RunState::Done => "done",
        RunState::Failed => "failed",
        RunState::Cancelled => "cancelled",
    }
}

fn log_line(entry: &LogEntry) -> String {
    match &entry.step {
        Some(step) => format!("[{step}] {}", entry.text),
        None => entry.text.clone(),
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectArgs {
    /// The project id, as list_projects returns it.
    pub project: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAssetsArgs {
    /// The project id, as list_projects returns it.
    pub project: String,
    /// Only assets at this Board stage, for example "refs", "model" or "shipped".
    pub stage: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetRunArgs {
    /// The project id.
    pub project: String,
    /// The run number ("Run 12" is 12). Without it and without runId, the newest run.
    pub number: Option<u32>,
    /// The run id that run_pipeline returned, for example "moon-island-run-12".
    pub run_id: Option<String>,
    /// How many log lines to return from the end, 40 by default and 400 at most.
    pub log_lines: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunPipelineArgs {
    /// The project id.
    pub project: String,
    /// Start at this step id: it runs even when cached, and so does everything after it.
    pub from: Option<String>,
    /// Ignore the cache and run every step.
    #[serde(default)]
    pub force: bool,
    /// Seconds to wait for the run before answering, 20 by default and 120 at most.
    /// The run carries on either way; follow it with get_run.
    pub wait_seconds: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunIdArgs {
    /// The run id that run_pipeline or get_run returned.
    pub run_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InboxItemArgs {
    /// The inbox item id (its file name), as list_inbox returns it.
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AcceptArgs {
    /// The inbox item id (its file name).
    pub id: String,
    /// The project that gets the model.
    pub project: String,
    /// An existing asset id to add a version to. Without it, the asset named
    /// `name` (or after the file) is found or created.
    pub asset: Option<String>,
    /// The asset name, when a new asset is made.
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailArgs {
    /// A .glb or .gltf inside the workspace, absolute or relative to the workspace folder.
    pub file: String,
    /// Seconds to wait for the render, 45 by default and 120 at most.
    pub wait_seconds: Option<u32>,
}

/// The aspect ratio of the concept images.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
pub enum AspectArg {
    #[serde(rename = "16:9")]
    Wide,
    #[serde(rename = "3:2")]
    Landscape,
    #[serde(rename = "1:1")]
    Square,
    #[serde(rename = "3:4")]
    Portrait,
    #[serde(rename = "9:16")]
    Tall,
}

impl From<AspectArg> for Aspect {
    fn from(value: AspectArg) -> Self {
        match value {
            AspectArg::Wide => Aspect::Wide,
            AspectArg::Landscape => Aspect::Landscape,
            AspectArg::Square => Aspect::Square,
            AspectArg::Portrait => Aspect::Portrait,
            AspectArg::Tall => Aspect::Tall,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConceptsArgs {
    /// The project id.
    pub project: String,
    /// The direction name, for example "Moon island porch". Reusing a name adds to it.
    pub direction: String,
    /// What the images should show.
    pub prompt: String,
    /// Reference images: paths relative to the project folder (for example
    /// "refs/island.png") or absolute paths inside it. At most 8.
    #[serde(default)]
    pub refs: Vec<String>,
    /// How many images, 1 to 12. 4 by default.
    pub count: Option<u32>,
    /// The aspect ratio, 16:9 by default.
    pub aspect: Option<AspectArg>,
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
            tool_router: Self::tool_router(),
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

/// The run an agent asked for: live from the runner, or from history.
fn find_run(host: &dyn Host, hub: &Hub, args: &GetRunArgs) -> Result<RunRecord, String> {
    let cache = cache_dir(host, &args.project)?;
    let saved = history::load_runs(&cache);
    let wanted = args
        .run_id
        .clone()
        .or_else(|| args.number.map(|n| format!("{}-run-{n}", args.project)));
    match wanted {
        Some(id) => hub
            .live_run(&id)
            .or_else(|| saved.into_iter().find(|r| r.id == id))
            .ok_or_else(|| format!("There is no run {id} in {}.", args.project)),
        None => {
            let (active, queued) = host.runner().active(&args.project);
            let live = active
                .or_else(|| queued.last().map(|r| r.id.clone()))
                .and_then(|id| hub.live_run(&id));
            live.or_else(|| saved.into_iter().next())
                .ok_or_else(|| format!("{} has not run yet.", args.project))
        }
    }
}

fn run_summary(hub: &Hub, run: &RunRecord) -> Value {
    let approval = hub.pending_for(&run.id);
    let status = if approval.is_some() {
        WAITING
    } else {
        state_word(run.state)
    };
    let steps: Vec<Value> = run
        .steps
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "name": s.name,
                "state": to_value(s.state),
                "durationMs": s.duration_ms,
                "message": s.message,
            })
        })
        .collect();
    let mut out = json!({
        "status": status,
        "runId": run.id,
        "number": run.number,
        "project": run.project,
        "from": run.from,
        "force": run.force,
        "message": run.message,
        "steps": steps,
    });
    if let Some(approval) = approval {
        out["approval"] = json!({
            "id": approval.id,
            "step": approval.step,
            "note": "A person approves or denies this in the Maquette app. Poll get_run; do not start the run again.",
        });
    }
    out
}

fn resolve_model(root: &Path, file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file.trim());
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    let inside = path
        .canonicalize()
        .ok()
        .zip(root.canonicalize().ok())
        .is_some_and(|(p, r)| p.starts_with(r));
    if !inside {
        return Err(format!(
            "{} is not a file inside the workspace.",
            path.display()
        ));
    }
    if !thumbs::is_renderable(&path) {
        return Err(format!("{} is not a .glb or .gltf model.", path.display()));
    }
    Ok(path)
}

#[tool_router]
impl McpTools {
    #[tool(
        description = "List the projects in the open Maquette workspace, with their template, site repo and asset count.",
        annotations(read_only_hint = true)
    )]
    async fn list_projects(&self) -> Out {
        let found = self
            .blocking(|host, _| {
                let (root, config) = workspace(host)?;
                let list = project::list(&root, &config.layout).map_err(reason)?;
                let projects: Vec<Value> = list
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "name": p.name,
                            "template": to_value(p.template),
                            "siteRepo": p.site_repo,
                            "assets": p.assets.len(),
                            "current": config.current_project.as_deref() == Some(p.id.as_str()),
                        })
                    })
                    .collect();
                Ok(json!({
                    "workspace": host.session().display(&root),
                    "projects": projects,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Get one project's manifest: name, template, site repo, source folder and its Board assets with every version.",
        annotations(read_only_hint = true)
    )]
    async fn get_project(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let manifest =
                    project::load(&root, &config.layout, &args.project).map_err(reason)?;
                let dir = project::dir(&root, &config.layout, &args.project).map_err(reason)?;
                Ok(json!({
                    "folder": host.session().display(&dir),
                    "project": to_value(&manifest),
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "List a project's Board assets: kind, stage, status, version count and the latest version's file and stats.",
        annotations(read_only_hint = true)
    )]
    async fn list_assets(&self, Parameters(args): Parameters<ListAssetsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let manifest =
                    project::load(&root, &config.layout, &args.project).map_err(reason)?;
                let stage = args.stage.as_deref().map(str::to_ascii_lowercase);
                let assets: Vec<Value> = manifest
                    .assets
                    .iter()
                    .filter(|a| {
                        stage
                            .as_deref()
                            .is_none_or(|s| to_value(a.stage).as_str() == Some(s))
                    })
                    .map(|a| {
                        json!({
                            "id": a.id,
                            "name": a.name,
                            "kind": to_value(a.kind),
                            "stage": to_value(a.stage),
                            "status": to_value(a.status),
                            "versions": a.versions.len(),
                            "latest": a.latest().map(to_value),
                            "thumbnail": a.thumbnail,
                        })
                    })
                    .collect();
                Ok(json!({ "project": manifest.id, "assets": assets }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Get a project's pipeline: its steps and dependencies, validation issues, compression presets (with lock notes), model sizes, the run in progress and the latest runs.",
        annotations(read_only_hint = true)
    )]
    async fn get_pipeline(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = pipeline::view(host.session(), host.runner(), &args.project)
                    .map_err(|e| e.message)?;
                let mut value = to_value(&view);
                if let Some(map) = value.as_object_mut() {
                    // The log has its own tool, and five runs are plenty here.
                    map.remove("log");
                    if let Some(Value::Array(runs)) = map.get_mut("runs") {
                        runs.truncate(5);
                    }
                }
                Ok(value)
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Get one run's status and the end of its log. Status is queued, running, done, failed, cancelled or \"waiting for approval\" (its Ship step waits for a person in the app).",
        annotations(read_only_hint = true)
    )]
    async fn get_run(&self, Parameters(args): Parameters<GetRunArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                let run = find_run(host, hub, &args)?;
                let lines = args.log_lines.unwrap_or(40).clamp(1, 400) as usize;
                let log = match hub.live_log(&run.id) {
                    Some(log) => log,
                    None => history::load_log(&cache_dir(host, &args.project)?, run.number),
                };
                let tail: Vec<String> = log
                    .iter()
                    .skip(log.len().saturating_sub(lines))
                    .map(log_line)
                    .collect();
                let mut out = run_summary(hub, &run);
                out["log"] = json!(tail);
                Ok(out)
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "List the models waiting in the workspace inbox, with their kind, size, triangle count and worst check.",
        annotations(read_only_hint = true)
    )]
    async fn list_inbox(&self) -> Out {
        let found = self
            .blocking(|host, _| {
                let (root, config) = workspace(host)?;
                let items = inbox::list(&root, &config).map_err(reason)?;
                let items: Vec<Value> = items
                    .iter()
                    .map(|i| {
                        json!({
                            "id": i.id,
                            "name": i.name,
                            "kind": to_value(i.kind),
                            "kindGuessed": i.kind_guessed,
                            "state": to_value(i.state),
                            "worst": to_value(i.worst),
                            "bytes": i.bytes,
                            "triangles": i.report.as_ref().map(|r| r.triangles),
                            "maxTexture": i.report.as_ref().map(|r| r.max_texture),
                            "error": i.error,
                        })
                    })
                    .collect();
                Ok(json!({
                    "inbox": host.session().display(&inbox::inbox_dir(&root, &config)),
                    "items": items,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Get the checks for one inbox model (triangles, textures, rig, scale, materials) and its analysis report.",
        annotations(read_only_hint = true)
    )]
    async fn get_inbox_checks(&self, Parameters(args): Parameters<InboxItemArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let item = inbox::item(&root, &config, &args.id).map_err(reason)?;
                Ok(json!({
                    "id": item.id,
                    "name": item.name,
                    "kind": to_value(item.kind),
                    "worst": to_value(item.worst),
                    "checks": to_value(&item.checks),
                    "report": to_value(&item.report),
                    "error": item.error,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Run a project's pipeline, optionally from one step and optionally ignoring the cache. Waits up to waitSeconds, then answers with the run's status. If the run reaches the Ship step while \"Ask before shipping\" is on, it answers \"waiting for approval\" until a person approves or denies it in the app; follow it with get_run."
    )]
    async fn run_pipeline(&self, Parameters(args): Parameters<RunPipelineArgs>) -> Out {
        let wait = Duration::from_secs(u64::from(args.wait_seconds.unwrap_or(20).min(120)));
        let gate_hub = Arc::clone(&self.hub);
        let started = self
            .blocking(move |host, _| {
                let (_, config) = workspace(host)?;
                let tools = host.toolbox();
                let setup =
                    pipeline::setup(host.session(), &args.project, args.from, args.force, tools)
                        .map_err(|e| e.message)?;
                let gate = config
                    .agents
                    .rules
                    .ask_before_shipping
                    .then(|| Arc::new(ShipGate(gate_hub)) as Arc<dyn StepGate>);
                host.runner()
                    .submit_with(
                        setup,
                        RunOptions {
                            gate,
                            ..RunOptions::default()
                        },
                    )
                    .map_err(reason)
            })
            .await;
        let record = match started {
            Ok(record) => record,
            Err(message) => return failed(message),
        };
        let deadline = Instant::now() + wait;
        let mut latest = record;
        loop {
            if let Some(run) = self.hub.live_run(&latest.id) {
                latest = run;
            }
            let waiting = self.hub.pending_for(&latest.id).is_some();
            if waiting || latest.state.is_finished() || Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let mut out = run_summary(&self.hub, &latest);
        if !latest.state.is_finished() && self.hub.pending_for(&latest.id).is_none() {
            out["note"] = json!("The run carries on. Follow it with get_run.");
        }
        json_result(&out)
    }

    #[tool(
        description = "Cancel a queued or running pipeline run. A run waiting for approval is cancelled too."
    )]
    async fn cancel_run(&self, Parameters(args): Parameters<RunIdArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                host.runner().cancel(&args.run_id).map_err(reason)?;
                Ok(json!({ "status": "cancelled", "runId": args.run_id }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Accept a model from the inbox into a project: it moves into the project as a new asset version on the Board, and its thumbnail is rendered headless."
    )]
    async fn accept_inbox_model(&self, Parameters(args): Parameters<AcceptArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let request = AcceptRequest {
                    item: args.id,
                    project: args.project.clone(),
                    asset: args.asset,
                    name: args.name,
                };
                let mut result = inbox::accept(&root, &config, &request).map_err(reason)?;
                let target = ThumbTarget {
                    project: args.project,
                    asset: result.asset.id.clone(),
                    version: result.version.version,
                };
                let tools = host.toolbox();
                let file = PathBuf::from(&result.file);
                if let Some(project) = crate::commands::thumbs::for_version(
                    host.thumbs(),
                    &root,
                    &config,
                    &tools,
                    &file,
                    target,
                ) {
                    result.project = project;
                }
                host.project_changed(&result.project);
                Ok(json!({
                    "asset": { "id": result.asset.id, "name": result.asset.name, "stage": to_value(result.asset.stage) },
                    "version": result.version.version,
                    "file": result.file,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Render a studio thumbnail of a model inside the workspace with headless Blender and return the PNG's path. Renders are cached by file content."
    )]
    async fn render_thumbnail(&self, Parameters(args): Parameters<ThumbnailArgs>) -> Out {
        let wait = Duration::from_secs(u64::from(args.wait_seconds.unwrap_or(45).min(120)));
        let requested = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let file = resolve_model(&root, &args.file)?;
                let dir = thumbs::thumbs_dir(&root, &config);
                let tools = host.toolbox();
                let found = host
                    .thumbs()
                    .request(&dir, &file, None, &tools)
                    .map_err(reason)?;
                let png = thumbs::png_for(&dir, &file).map_err(reason)?;
                Ok((found, png))
            })
            .await;
        let (found, png) = match requested {
            Ok(found) => found,
            Err(message) => return failed(message),
        };
        match found {
            Requested::Ready(path) => json_result(&json!({ "status": "ready", "png": path })),
            Requested::Fallback(why) => failed(format!("No Blender render for this model. {why}")),
            Requested::Rendering => {
                let deadline = Instant::now() + wait;
                while Instant::now() < deadline {
                    if png.is_file() {
                        return json_result(&json!({ "status": "ready", "png": png }));
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                json_result(&json!({
                    "status": "rendering",
                    "png": png,
                    "note": "Blender is still rendering. The PNG appears at this path when it is done.",
                }))
            }
        }
    }

    #[tool(
        description = "Start concept image jobs in a project's Concept lab. Each image runs through the Codex CLI and takes about a minute; the images land in concepts/<direction>/ inside the project and show up in the app's Lab."
    )]
    async fn start_concepts(&self, Parameters(args): Parameters<ConceptsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let request = NewConcepts {
                    project: args.project,
                    direction: args.direction,
                    prompt: args.prompt,
                    refs: args.refs,
                    aspect: args.aspect.map(Aspect::from).unwrap_or_default(),
                    count: args.count.unwrap_or(4),
                };
                let tools = host.toolbox();
                let (direction, jobs) =
                    lab::generate(host.session(), host.concepts(), &tools, &request)
                        .map_err(|e| e.message)?;
                let jobs: Vec<Value> = jobs
                    .iter()
                    .map(|j| json!({ "id": j.id, "file": j.file, "state": to_value(j.state) }))
                    .collect();
                Ok(json!({
                    "status": "started",
                    "direction": { "id": direction.id, "name": direction.name },
                    "jobs": jobs,
                    "note": "Each file appears at its path, relative to the project folder, once Codex saves it.",
                }))
            })
            .await;
        respond(found)
    }
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
