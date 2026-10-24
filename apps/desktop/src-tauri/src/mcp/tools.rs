//! The tools agents call. Read tools look at the workspace; act tools run
//! pipelines, accept models, render thumbnails, start concepts and send
//! assets to 3D.
//!
//! The agent rules hold here by construction: Blender only ever runs
//! headless (thumbnails and pipeline steps), no tool opens its window, and
//! there is no screenshot or screen capture tool at all. Runs that would
//! ship wait for a person when "Ask before shipping" is on, and a send to
//! Tripo or Meshy always waits for a person before it spends credits.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maquette_core::concepts::{Aspect, NewConcepts, NewObjectRefs};
use maquette_core::inbox::{self, AcceptRequest};
use maquette_core::pipeline::cache::project_cache_dir;
use maquette_core::pipeline::history::{self, LogEntry, RunRecord, RunState};
use maquette_core::pipeline::presets::{self, CompressPreset};
use maquette_core::pipeline::runner::{JobRunner as _, RunOptions, StepGate};
use maquette_core::providers::{
    BatchItem, BatchRequest, JobState, MeshyPreset, PresetVariant, Provider, ProviderId,
    ProviderJob, SendRequest, TripoPreset,
};
use maquette_core::thumbs::{self, Requested, ThumbTarget};
use maquette_core::{AssetKind, ErrorPayload, MeshoptLevel, TextureCodec, concepts, project};
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
use crate::commands::{lab, pipeline, providers};

/// The status a run reports while its Ship step waits for a person.
pub const WAITING: &str = "waiting for approval";

const INSTRUCTIONS: &str = "Maquette turns concept art into real-time 3D web scenes. \
Start with list_projects, then get_pipeline or list_assets for a project. \
run_pipeline runs a project's pipeline; when it reaches the Ship step it may answer \
\"waiting for approval\": a person approves or denies shipping in the Maquette app, \
so poll get_run instead of starting the run again. send_to_3d sends an asset's refs to \
Tripo or Meshy (it always waits for a person to approve the credits in the app; follow it \
with get_provider_job) or writes a by hand handoff folder; send_batch_to_3d sends several \
assets at once under one approval. make_object_refs turns the objects of one concept into \
one isolated image each. get_concepts reads the Lab. \
set_compression_presets changes the per model texture size, data map size, codec and \
meshopt level that get_pipeline shows (it spends nothing; locked rows refuse). \
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

/// The texture sizes the Pipeline view offers, in pixels.
const PRESET_SIZES: [u32; 4] = [512, 1024, 2048, 4096];
const PRESET_CODECS: &str = "etc1s, uastc";
const PRESET_MESHOPT: &str = "low, medium, high";

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
pub struct PresetEdit {
    /// The model as get_pipeline lists it in `models` (the GLB file stem, for example
    /// "clay_v1"), or the asset's name when it has one model.
    pub model: String,
    /// The longest texture edge in pixels: 512, 1024, 2048 or 4096.
    pub texture_size: Option<u32>,
    /// A smaller edge for the metal, roughness, occlusion and normal maps: 512, 1024,
    /// 2048 or 4096. null clears the split, so every map uses textureSize.
    #[serde(default, deserialize_with = "explicit")]
    pub data_texture_size: Option<Option<u32>>,
    /// The texture codec: "etc1s" or "uastc".
    pub codec: Option<String>,
    /// The meshopt level: "low", "medium" or "high".
    pub meshopt: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetPresetsArgs {
    /// The project id.
    pub project: String,
    /// The rows to change. Only the fields you give change; other models stay as they are.
    pub presets: Vec<PresetEdit>,
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRefsArgs {
    /// The project id.
    pub project: String,
    /// The concept to take the objects from, relative to the project folder, for example
    /// "concepts/workbench-hero/workbench-hero_01.png" or "refs/workbench.png".
    pub source: String,
    /// One short name per object (60 characters at most), for example ["desk lamp", "clay figure"]. At most 12. The name becomes the asset name; put long descriptions in notes.
    pub objects: Vec<String>,
    /// Optional extra instructions per object, keyed by the object's name, for example
    /// {"island": "painted stone and moss, no wireframe lines"}. They go into the image prompt
    /// only; the asset keeps the short object name. Up to 400 characters each.
    pub notes: Option<std::collections::HashMap<String, String>>,
    /// "prop" (default) or "character": what the objects become on the Board.
    pub kind: Option<VariantArg>,
    /// The aspect ratio, 1:1 by default.
    pub aspect: Option<AspectArg>,
    /// The new direction is called "<name> objects". By default the source's direction name,
    /// or its file name.
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetConceptsArgs {
    /// The project id.
    pub project: String,
    /// One direction, by id or name. Without it, every direction.
    pub direction: Option<String>,
}

/// Where a model gets made.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderArg {
    Tripo,
    Meshy,
    ByHand,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceArg {
    Tripo,
    Meshy,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum VariantArg {
    Prop,
    Character,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendTo3dArgs {
    /// The project id.
    pub project: String,
    /// The asset id, as list_assets returns it.
    pub asset: String,
    /// "tripo" or "meshy" spend credits and always wait for a person to approve
    /// them in the app; "by-hand" writes a handoff folder and spends nothing.
    pub provider: ProviderArg,
    /// By hand only: whose web app the settings card is for, "tripo" by default.
    pub service: Option<ServiceArg>,
    /// "prop" or "character". By default from the asset's kind.
    pub variant: Option<VariantArg>,
    /// Images relative to the project folder (for example "refs/lantern_front.png"),
    /// front first, at most 4. By default the asset's own refs.
    #[serde(default)]
    pub images: Vec<String>,
    /// Preset fields to change for this send only, named as in the workspace preset,
    /// for example {"faceLimit": 20000, "rig": false} for Tripo or
    /// {"aiModel": "meshy-6", "targetPolycount": 30000} for Meshy.
    pub overrides: Option<serde_json::Map<String, Value>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemArg {
    /// The asset id, as list_assets returns it.
    pub asset: String,
    /// "prop" or "character". By default from the asset's kind.
    pub variant: Option<VariantArg>,
    /// Images relative to the project folder, front first, at most 4. By default the
    /// asset's own refs.
    #[serde(default)]
    pub images: Vec<String>,
    /// Preset fields to change for this asset only, laid over the batch settings, for
    /// example {"faceLimit": 2000, "pbr": false}.
    pub overrides: Option<serde_json::Map<String, Value>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendBatchArgs {
    /// The project id.
    pub project: String,
    /// The same provider for every asset. "tripo" or "meshy" spend credits and wait for one
    /// approval in the app for the whole batch; "by-hand" writes one handoff folder per asset.
    pub provider: ProviderArg,
    /// By hand only: whose web app the settings cards are for, "tripo" by default.
    pub service: Option<ServiceArg>,
    /// One model per asset, at most 24.
    pub items: Vec<BatchItemArg>,
    /// Preset fields to change for this batch only, laid over both the prop and the
    /// character preset, for example {"faceLimit": 20000}. An item's own overrides
    /// go on top of these.
    pub overrides: Option<serde_json::Map<String, Value>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderJobArgs {
    /// The job id that send_to_3d returned, for example "stone-lantern-2".
    pub id: Option<String>,
    /// Without an id: every send in this project, newest last.
    pub project: Option<String>,
    /// With project: only this asset's sends.
    pub asset: Option<String>,
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

fn check_size(what: &str, size: u32) -> Result<u32, String> {
    if PRESET_SIZES.contains(&size) {
        Ok(size)
    } else {
        Err(format!(
            "{what} must be one of 512, 1024, 2048 or 4096, not {size}."
        ))
    }
}

fn parse_codec(word: &str) -> Result<TextureCodec, String> {
    serde_json::from_value(json!(word.to_lowercase()))
        .map_err(|_| format!("codec must be one of {PRESET_CODECS}, not {word:?}."))
}

fn parse_meshopt(word: &str) -> Result<MeshoptLevel, String> {
    serde_json::from_value(json!(word.to_lowercase()))
        .map_err(|_| format!("meshopt must be one of {PRESET_MESHOPT}, not {word:?}."))
}

/// The model an edit names: a stem, or an asset name with one model.
fn preset_target<'a>(models: &'a [pipeline::ModelFile], name: &str) -> Result<&'a str, String> {
    let wanted = name.to_lowercase();
    let stem = models.iter().find(|m| m.model.to_lowercase() == wanted);
    let named: Vec<_> = models
        .iter()
        .filter(|m| {
            m.asset_name
                .as_ref()
                .is_some_and(|n| n.to_lowercase() == wanted)
        })
        .collect();
    match (stem, named.as_slice()) {
        (Some(m), _) => Ok(&m.model),
        (None, [m]) => Ok(&m.model),
        (None, [_, ..]) => Err(format!(
            "{name:?} has several models ({}). Use the model name.",
            named
                .iter()
                .map(|m| m.model.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        (None, []) => Err(format!(
            "Unknown model {name:?}. Models: {}.",
            models
                .iter()
                .map(|m| m.model.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The stored table with the edits applied. A model without a row starts from
/// the workspace defaults, the way the Pipeline view's table does.
fn edited_presets(
    view: &pipeline::PipelineView,
    edits: &[PresetEdit],
) -> Result<Vec<CompressPreset>, String> {
    let mut table = view.def.presets.clone();
    let mut touched = Vec::new();
    for edit in edits {
        let model = preset_target(&view.models, &edit.model)
            .or_else(|err| {
                // A stored row can name a model that has no build yet.
                let stored = table
                    .iter()
                    .find(|p| p.model.eq_ignore_ascii_case(&edit.model));
                stored.map(|p| p.model.as_str()).ok_or(err)
            })?
            .to_string();
        if touched.contains(&model) {
            return Err(format!("{model} appears twice. Give each model one entry."));
        }
        let index = table
            .iter()
            .position(|p| p.model == model)
            .unwrap_or_else(|| {
                let label = view
                    .models
                    .iter()
                    .find(|m| m.model == model)
                    .and_then(|m| m.asset_name.clone())
                    .unwrap_or_else(|| presets::model_label(&model));
                table.push(CompressPreset {
                    label,
                    model: model.clone(),
                    texture_size: view.defaults.texture_size,
                    data_texture_size: None,
                    codec: view.defaults.codec,
                    meshopt: view.defaults.meshopt,
                    lock: None,
                });
                table.len() - 1
            });
        let row = &mut table[index];
        if let Some(reason) = &row.lock {
            return Err(format!("{} is locked: {reason}", row.label));
        }
        let name = |what: &str| format!("{}: {what}", row.label);
        if let Some(size) = edit.texture_size {
            row.texture_size = check_size(&name("textureSize"), size)?;
        }
        if let Some(data) = edit.data_texture_size {
            row.data_texture_size = data
                .map(|size| check_size(&name("dataTextureSize"), size))
                .transpose()?;
        }
        if let Some(word) = &edit.codec {
            row.codec = parse_codec(word).map_err(|e| name(&e))?;
        }
        if let Some(word) = &edit.meshopt {
            row.meshopt = parse_meshopt(word).map_err(|e| name(&e))?;
        }
        touched.push(model);
    }
    Ok(table)
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

/// A provider job as agents read it.
fn job_value(hub: &Hub, job: &ProviderJob) -> Value {
    let approval = hub.pending_for_job(&job.id);
    let status = if approval.is_some() {
        WAITING.to_string()
    } else {
        to_value(job.state).as_str().unwrap_or_default().to_string()
    };
    let mut out = json!({
        "status": status,
        "id": job.id,
        "project": job.project,
        "asset": job.asset,
        "assetName": job.asset_name,
        "provider": to_value(job.provider),
        "settings": job.summary(),
        "images": job.images,
        "phase": to_value(job.phase),
        "progress": job.progress,
        "credits": job.credits,
        "message": job.message,
        "inboxFile": job.inbox_file,
        "handoff": job.handoff,
        "version": job.version,
        "cancel": to_value(job.cancel),
    });
    if let Some(approval) = approval {
        out["approval"] = json!({
            "id": approval.id,
            "note": "A person approves or denies spending these credits in the Maquette app. Poll get_provider_job; do not send again.",
        });
    }
    out
}

/// Lays `overrides` over a preset, refusing names the preset does not have.
fn with_overrides<T: Serialize + serde::de::DeserializeOwned>(
    base: &T,
    overrides: Option<&serde_json::Map<String, Value>>,
) -> Result<T, String> {
    let Some(overrides) = overrides.filter(|o| !o.is_empty()) else {
        return serde_json::from_value(to_value(base)).map_err(|e| e.to_string());
    };
    let mut value = to_value(base);
    let Some(fields) = value.as_object_mut() else {
        return Err("The preset could not be read.".to_string());
    };
    for (key, field) in overrides {
        if !fields.contains_key(key) {
            let mut known: Vec<&str> = fields.keys().map(String::as_str).collect();
            known.sort_unstable();
            return Err(format!(
                "There is no preset setting {key:?}. The settings are {}.",
                known.join(", ")
            ));
        }
        fields.insert(key.clone(), field.clone());
    }
    serde_json::from_value(value).map_err(|e| format!("The overrides do not fit the preset. {e}"))
}

/// Builds the send an agent asked for, filling in what it left out.
fn send_request(host: &dyn Host, args: &SendTo3dArgs) -> Result<SendRequest, String> {
    let (_, config) = workspace(host)?;
    let options =
        providers::send_options(host.session(), host.studio(), &args.project, &args.asset)
            .map_err(|e| e.message)?;
    let variant = match args.variant {
        Some(VariantArg::Prop) => PresetVariant::Prop,
        Some(VariantArg::Character) => PresetVariant::Character,
        None => options.variant,
    };
    let images = if args.images.is_empty() {
        options.suggested.clone()
    } else {
        args.images.clone()
    };
    if images.is_empty() {
        return Err(format!(
            "{} has no refs of its own yet. Pass images, for example [\"refs/{}_front.png\"].",
            options.asset.name,
            options.asset.id.replace('-', "_")
        ));
    }
    let (provider, service) = match args.provider {
        ProviderArg::Tripo => (Provider::Tripo, ProviderId::Tripo),
        ProviderArg::Meshy => (Provider::Meshy, ProviderId::Meshy),
        ProviderArg::ByHand => (
            Provider::ByHand,
            match args.service {
                Some(ServiceArg::Meshy) => ProviderId::Meshy,
                _ => ProviderId::Tripo,
            },
        ),
    };
    let overrides = args.overrides.as_ref();
    let (tripo, meshy): (Option<TripoPreset>, Option<MeshyPreset>) = match service {
        ProviderId::Tripo => (
            Some(with_overrides(config.providers.tripo(variant), overrides)?),
            None,
        ),
        ProviderId::Meshy => (
            None,
            Some(with_overrides(config.providers.meshy(variant), overrides)?),
        ),
    };
    Ok(SendRequest {
        project: args.project.clone(),
        asset: options.asset.id,
        provider,
        service,
        variant,
        images,
        tripo,
        meshy,
    })
}

fn variant(arg: Option<VariantArg>, fallback: PresetVariant) -> PresetVariant {
    match arg {
        Some(VariantArg::Prop) => PresetVariant::Prop,
        Some(VariantArg::Character) => PresetVariant::Character,
        None => fallback,
    }
}

/// Builds the batch an agent asked for: each asset's own refs and kind
/// unless given, one preset per variant with the overrides laid over both,
/// and an asset's own overrides laid over its variant's preset.
fn batch_request(host: &dyn Host, args: &SendBatchArgs) -> Result<BatchRequest, String> {
    let (_, config) = workspace(host)?;
    let ids: Vec<String> = args.items.iter().map(|i| i.asset.clone()).collect();
    let options = providers::batch_options(host.session(), host.studio(), &args.project, &ids)
        .map_err(|e| e.message)?;
    let (provider, service) = match args.provider {
        ProviderArg::Tripo => (Provider::Tripo, ProviderId::Tripo),
        ProviderArg::Meshy => (Provider::Meshy, ProviderId::Meshy),
        ProviderArg::ByHand => (
            Provider::ByHand,
            match args.service {
                Some(ServiceArg::Meshy) => ProviderId::Meshy,
                _ => ProviderId::Tripo,
            },
        ),
    };
    let overrides = args.overrides.as_ref();
    let p = &config.providers;
    let tripo = service == ProviderId::Tripo;
    let tripo_for = |variant| with_overrides(p.tripo(variant), overrides);
    let meshy_for = |variant| with_overrides(p.meshy(variant), overrides);
    let mut items = Vec::new();
    for arg in &args.items {
        let Some(found) = options.items.iter().find(|o| o.asset.id == arg.asset) else {
            return Err(format!("{} is in the batch twice.", arg.asset));
        };
        let images = if arg.images.is_empty() {
            found.suggested.clone()
        } else {
            arg.images.clone()
        };
        if images.is_empty() {
            return Err(format!(
                "{} has no refs of its own yet. Pass its images, for example [\"refs/{}.png\"].",
                found.asset.name,
                found.asset.id.replace('-', "_")
            ));
        }
        let variant = variant(arg.variant, found.variant);
        let own = arg.overrides.as_ref().filter(|o| !o.is_empty());
        let name = &found.asset.name;
        let tag = |err: String| format!("{name}: {err}");
        items.push(BatchItem {
            asset: found.asset.id.clone(),
            variant,
            images,
            tripo: own
                .filter(|_| tripo)
                .map(|o| with_overrides(&tripo_for(variant)?, Some(o)))
                .transpose()
                .map_err(tag)?,
            meshy: own
                .filter(|_| !tripo)
                .map(|o| with_overrides(&meshy_for(variant)?, Some(o)))
                .transpose()
                .map_err(tag)?,
        });
    }
    Ok(BatchRequest {
        project: args.project.clone(),
        provider,
        service,
        items,
        tripo_prop: tripo.then(|| tripo_for(PresetVariant::Prop)).transpose()?,
        tripo_character: tripo
            .then(|| tripo_for(PresetVariant::Character))
            .transpose()?,
        meshy_prop: (!tripo)
            .then(|| meshy_for(PresetVariant::Prop))
            .transpose()?,
        meshy_character: (!tripo)
            .then(|| meshy_for(PresetVariant::Character))
            .transpose()?,
    })
}

/// The name an object refs direction starts with when the agent gives
/// none: the source's direction, else its file name.
fn source_name(project_dir: &Path, source: &str) -> String {
    let clean = source.replace('\\', "/");
    let mut parts = clean.split('/');
    if parts.next() == Some(concepts::CONCEPTS_DIR)
        && let Some(id) = parts.next()
        && let Ok(Some(direction)) = concepts::load_direction(project_dir, id)
    {
        return direction.name;
    }
    Path::new(&clean)
        .file_stem()
        .map(|s| s.to_string_lossy().replace(['_', '-'], " "))
        .unwrap_or_default()
}

/// Waits for the person's answer on its own thread, then releases or drops
/// every job it covers. Jobs cancelled from the app's queue withdraw the
/// approval once none is left waiting.
fn await_send_approval(host: Arc<dyn Host>, hub: Arc<Hub>, approval: String, jobs: Vec<String>) {
    let spawned = std::thread::Builder::new()
        .name("send-approval".into())
        .spawn(move || {
            let waiting = |id: &String| {
                host.studio()
                    .get(id)
                    .is_some_and(|j| j.state == JobState::Approval)
            };
            let gone = || !jobs.iter().any(waiting);
            let answer = hub.wait_until(&approval, gone);
            for job in jobs.iter().filter(|j| waiting(j)) {
                let result = match answer {
                    Some(true) => host.studio().approve(job).map(|_| ()),
                    Some(false) => host.studio().deny(job).map(|_| ()),
                    None => Ok(()),
                };
                if let Err(err) = result {
                    tracing::warn!(job, error = %err, "could not answer a send approval");
                }
            }
        });
    if let Err(err) = spawned {
        tracing::warn!(error = %err, "could not wait for a send approval");
    }
}

#[tool_router]
impl McpTools {
    #[tool(
        description = "List the projects in the open Maquette workspace: template, site repo, created, asset count, \
the picture the app shows for it, what runs in it now and which one is open in the app. Every other tool takes \
a project id; no tool changes which project the app has open.",
        annotations(read_only_hint = true)
    )]
    async fn list_projects(&self) -> Out {
        let found = self
            .blocking(|host, _| {
                let (root, config) = workspace(host)?;
                let rows = crate::commands::project::summaries(
                    host.session(),
                    host.runner(),
                    host.concepts(),
                    host.studio(),
                )
                .map_err(reason)?;
                // The app opens the remembered project, else the oldest one.
                let current = config
                    .current_project
                    .as_deref()
                    .filter(|id| rows.iter().any(|r| r.id == *id))
                    .or_else(|| rows.first().map(|r| r.id.as_str()))
                    .map(str::to_string);
                let projects: Vec<Value> = rows
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "name": p.name,
                            "template": to_value(p.template),
                            "siteRepo": p.site_repo,
                            "createdAt": p.created_at,
                            "assets": p.asset_count,
                            "thumb": p.thumb,
                            "activity": to_value(&p.activity),
                            "current": current.as_deref() == Some(p.id.as_str()),
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
        description = "Set compression presets, as the Pipeline view's Compression presets panel and Apply presets do. Each entry names a model (as get_pipeline lists it in `models`, or its asset name) and any of textureSize (512, 1024, 2048 or 4096), dataTextureSize (the same sizes for the metal, roughness, occlusion and normal maps, or null to use textureSize), codec (etc1s or uastc) and meshopt (low, medium or high). Only the given fields change. Locked rows are refused with their lock note, and an unknown model with the list of valid ones. It spends nothing; the presets apply the next time Compress runs. Returns the presets and the built sizes.",
        annotations(idempotent_hint = true)
    )]
    async fn set_compression_presets(&self, Parameters(args): Parameters<SetPresetsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = pipeline::view(host.session(), host.runner(), &args.project)
                    .map_err(|e| e.message)?;
                let table = edited_presets(&view, &args.presets)?;
                let saved = pipeline::save_presets(host.session(), host.runner(), &args.project, table)
                    .map_err(|e| e.message)?;
                host.pipeline_changed(&args.project);
                let built: f64 = saved
                    .models
                    .iter()
                    .map(|m| m.shipped_bytes.or(m.raw_bytes).unwrap_or(0.0))
                    .sum();
                Ok(json!({
                    "project": saved.project,
                    "presets": to_value(&saved.def.presets),
                    "models": to_value(&saved.models),
                    "builtBytes": built,
                    "note": "The presets apply the next time Compress runs. builtBytes is what the models weigh now (shipped, else raw).",
                }))
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
                let mut items = inbox::list(&root, &config).map_err(reason)?;
                host.studio().link_inbox(&mut items);
                let items: Vec<Value> = items
                    .iter()
                    .map(|i| {
                        json!({
                            "id": i.id,
                            "name": i.name,
                            "kind": to_value(i.kind),
                            "kindGuessed": i.kind_guessed,
                            "kindFromSend": i.kind_from_send,
                            "state": to_value(i.state),
                            "worst": to_value(i.worst),
                            "bytes": i.bytes,
                            "triangles": i.report.as_ref().map(|r| r.triangles),
                            "maxTexture": i.report.as_ref().map(|r| r.max_texture),
                            "error": i.error,
                            "sentFor": i.link.as_ref().map(|l| json!({
                                "job": l.job,
                                "project": l.project,
                                "asset": l.asset,
                                "assetName": l.asset_name,
                            })),
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
                // A model from a send goes to the asset it was sent for.
                let mut item = inbox::item(&root, &config, &args.id).map_err(reason)?;
                host.studio().link_inbox(std::slice::from_mut(&mut item));
                let linked = item
                    .link
                    .filter(|l| l.project == args.project)
                    .map(|l| l.asset);
                let request = AcceptRequest {
                    item: args.id,
                    project: args.project.clone(),
                    asset: args.asset.or(linked),
                    name: args.name,
                };
                let mut result = inbox::accept(&root, &config, &request).map_err(reason)?;
                host.studio().accepted(&request.item, result.version.version);
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

    #[tool(
        description = "Read a project's Concept lab: each direction's prompt, refs and images (starred, picked, and the ref file a picked image became), plus the concept jobs still queued or running.",
        annotations(read_only_hint = true)
    )]
    async fn get_concepts(&self, Parameters(args): Parameters<GetConceptsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let dir = project::dir(&root, &config.layout, &args.project).map_err(reason)?;
                project::load(&root, &config.layout, &args.project).map_err(reason)?;
                let wanted = args.direction.as_deref().map(str::trim);
                let directions: Vec<Value> = concepts::list_directions(&dir)
                    .into_iter()
                    .filter(|d| {
                        wanted.is_none_or(|w| d.id == w || d.name.trim().eq_ignore_ascii_case(w))
                    })
                    .map(|d| {
                        let images: Vec<Value> = d
                            .images
                            .iter()
                            .map(|i| {
                                json!({
                                    "file": i.file,
                                    "index": i.index,
                                    "width": i.width,
                                    "height": i.height,
                                    "starred": i.starred,
                                    "picked": i.picked,
                                    "refFile": i.ref_file,
                                    "title": i.title,
                                })
                            })
                            .collect();
                        json!({
                            "id": d.id,
                            "name": d.name,
                            "prompt": d.prompt,
                            "aspect": d.aspect.ratio(),
                            "refs": d.refs,
                            "source": d.source,
                            "kind": to_value(d.kind),
                            "images": images,
                        })
                    })
                    .collect();
                if let (Some(w), true) = (wanted, directions.is_empty()) {
                    return Err(format!("There is no direction {w:?} in {}.", args.project));
                }
                let jobs: Vec<Value> = host
                    .concepts()
                    .jobs(Some(&args.project))
                    .iter()
                    .filter(|j| !j.state.is_finished())
                    .map(|j| {
                        json!({
                            "id": j.id,
                            "direction": j.direction_name,
                            "file": j.file,
                            "state": to_value(j.state),
                        })
                    })
                    .collect();
                Ok(json!({
                    "project": args.project,
                    "directions": directions,
                    "running": jobs,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Send an asset's refs to 3D. \"tripo\" or \"meshy\" spend credits, so the send always waits for a person to approve it in the Maquette app and answers \"waiting for approval\"; follow it with get_provider_job. \"by-hand\" spends nothing: it writes a handoff folder with the images and a settings card, and the asset waits for a person to bring the GLB back. Settings start from the workspace preset (prop or character) and overrides change them for this send only."
    )]
    async fn send_to_3d(&self, Parameters(args): Parameters<SendTo3dArgs>) -> Out {
        let sent = self
            .blocking(move |host, hub| {
                let request = send_request(host, &args)?;
                let approval = request.provider != Provider::ByHand;
                let job = host
                    .studio()
                    .send(&request, approval, true)
                    .map_err(reason)?;
                let pending = (job.state == JobState::Approval).then(|| hub.request_send(&job));
                Ok((job, pending))
            })
            .await;
        let (job, pending) = match sent {
            Ok(sent) => sent,
            Err(message) => return failed(message),
        };
        if let Some(approval) = pending {
            await_send_approval(
                Arc::clone(&self.host),
                Arc::clone(&self.hub),
                approval,
                vec![job.id.clone()],
            );
        }
        let mut out = job_value(&self.hub, &job);
        if job.provider == Provider::ByHand {
            out["note"] = json!(format!(
                "Nothing is spent. A person makes the model in the {} web app with the settings card in the handoff folder, then drops the GLB on the asset's card in Maquette or saves it into the inbox as {}.glb.",
                job.service.label(),
                job.id
            ));
        }
        json_result(&out)
    }

    #[tool(
        description = "Send several assets to 3D at once, one model per asset, with the same provider and settings (each asset keeps its own refs and prop or character preset unless given). \"tripo\" or \"meshy\" spend credits, so the whole batch waits for one approval in the Maquette app that lists every asset; follow each job with get_provider_job. \"by-hand\" spends nothing and writes one handoff folder per asset. Every item is checked before any goes out."
    )]
    async fn send_batch_to_3d(&self, Parameters(args): Parameters<SendBatchArgs>) -> Out {
        let sent = self
            .blocking(move |host, hub| {
                let request = batch_request(host, &args)?;
                let approval = request.provider != Provider::ByHand;
                let sent = host
                    .studio()
                    .send_batch(&request, approval, true)
                    .map_err(reason)?;
                let waiting: Vec<ProviderJob> = sent
                    .jobs
                    .iter()
                    .filter(|j| j.state == JobState::Approval)
                    .cloned()
                    .collect();
                let pending = hub.request_batch(&waiting);
                let ids = waiting.into_iter().map(|j| j.id).collect::<Vec<_>>();
                Ok((sent, pending, ids))
            })
            .await;
        let (sent, pending, ids) = match sent {
            Ok(sent) => sent,
            Err(message) => return failed(message),
        };
        if let Some(approval) = pending {
            await_send_approval(Arc::clone(&self.host), Arc::clone(&self.hub), approval, ids);
        }
        let jobs: Vec<Value> = sent.jobs.iter().map(|j| job_value(&self.hub, j)).collect();
        let by_hand = sent
            .jobs
            .first()
            .is_some_and(|j| j.provider == Provider::ByHand);
        let status = if by_hand || sent.jobs.is_empty() {
            "started"
        } else {
            WAITING
        };
        let mut out = json!({
            "status": status,
            "batch": sent.batch,
            "jobs": jobs,
            "failed": sent.failed.iter().map(|f| json!({ "asset": f.asset, "message": f.message })).collect::<Vec<_>>(),
        });
        out["note"] = json!(if by_hand {
            "Nothing is spent. Each asset has its own handoff folder with its images and settings card; a person makes the models in the web app and brings each GLB back in Maquette."
        } else {
            "A person approves or denies the whole batch once in the Maquette app. Poll get_provider_job for each job; do not send again."
        });
        json_result(&out)
    }

    #[tool(
        description = "Make object refs from one concept: for each object named, Codex makes one image of only that object (centered, whole, front three-quarter view, plain light grey background, soft light, no text), with the concept attached as the reference. The images land in a new Lab direction \"<name> objects\", each titled by its object; read them with get_concepts. Each image spends Codex usage."
    )]
    async fn make_object_refs(&self, Parameters(args): Parameters<ObjectRefsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                let dir = project::dir(&root, &config.layout, &args.project).map_err(reason)?;
                let name = args
                    .name
                    .clone()
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| source_name(&dir, &args.source));
                let request = NewObjectRefs {
                    project: args.project,
                    source: args.source,
                    source_name: name,
                    objects: args.objects,
                    notes: args.notes.unwrap_or_default(),
                    aspect: args.aspect.map_or(Aspect::Square, Aspect::from),
                    kind: match args.kind {
                        Some(VariantArg::Character) => AssetKind::Character,
                        _ => AssetKind::Prop,
                    },
                };
                let tools = host.toolbox();
                let (direction, jobs) =
                    lab::make_objects(host.session(), host.concepts(), &tools, &request)
                        .map_err(|e| e.message)?;
                let jobs: Vec<Value> = jobs
                    .iter()
                    .map(|j| {
                        json!({ "id": j.id, "object": j.title, "file": j.file, "state": to_value(j.state) })
                    })
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

    #[tool(
        description = "Get a send to 3D: its status (\"waiting for approval\", queued, running, waiting for a by hand model, inbox, accepted, failed or cancelled), phase, progress, credits spent and message. With project instead of id, every send in the project.",
        annotations(read_only_hint = true)
    )]
    async fn get_provider_job(&self, Parameters(args): Parameters<ProviderJobArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                if let Some(id) = args.id.as_deref() {
                    let job = host
                        .studio()
                        .get(id.trim())
                        .ok_or_else(|| format!("There is no send {id}."))?;
                    return Ok(job_value(hub, &job));
                }
                let Some(project) = args.project.as_deref() else {
                    return Err("Pass the job id, or a project to list its sends.".to_string());
                };
                let (root, config) = workspace(host)?;
                project::load(&root, &config.layout, project).map_err(reason)?;
                let jobs: Vec<Value> = host
                    .studio()
                    .list(Some(project))
                    .iter()
                    .filter(|j| args.asset.as_deref().is_none_or(|a| j.asset == a))
                    .map(|j| job_value(hub, j))
                    .collect();
                Ok(json!({ "project": project, "jobs": jobs }))
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
