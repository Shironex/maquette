//! Tools that read, edit and run a project's pipeline.

use std::sync::Arc;
use std::time::{Duration, Instant};

use maquette_core::pipeline::history::{self, LogEntry, RunRecord, RunState};
use maquette_core::pipeline::presets::{self, CompressPreset};
use maquette_core::pipeline::runner::{JobRunner as _, RunOptions, StepGate};
use maquette_core::{MeshoptLevel, TextureCodec};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::pipeline;
use crate::mcp::gate::ShipGate;
use crate::mcp::host::Host;
use crate::mcp::hub::Hub;

use super::{
    McpTools, Out, ProjectArgs, WAITING, cache_dir, explicit, failed, json_result, reason, respond,
    to_value, workspace,
};

/// The texture sizes the Pipeline view offers, in pixels.
const PRESET_SIZES: [u32; 4] = [512, 1024, 2048, 4096];

const PRESET_CODECS: &str = "etc1s, uastc";

const PRESET_MESHOPT: &str = "low, medium, high";

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
                    map_codecs: None,
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

#[tool_router(router = runs_router, vis = "pub(super)")]
impl McpTools {
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
        description = "Run a project's pipeline, optionally from one step and optionally ignoring the cache. A full run (no from) first rebuilds the built assets whose script or helpers changed, and is refused when one of them fails to build. Waits up to waitSeconds, then answers with the run's status. If the run reaches the Ship step while \"Ask before shipping\" is on, or in a game project (whose Ship always asks), it answers \"waiting for approval\" until a person approves or denies it in the app; follow it with get_run."
    )]
    async fn run_pipeline(&self, Parameters(args): Parameters<RunPipelineArgs>) -> Out {
        let wait = Duration::from_secs(u64::from(args.wait_seconds.unwrap_or(20).min(120)));
        let gate_hub = Arc::clone(&self.hub);
        let started = self
            .blocking(move |host, _| {
                let (_, config) = workspace(host)?;
                let tools = host.toolbox();
                if args.from.is_none() {
                    super::built::rebuild_stale(host, &args.project, &tools)?;
                }
                let setup =
                    pipeline::setup(host.session(), &args.project, args.from, args.force, tools)
                        .map_err(|e| e.message)?;
                let gate =
                    crate::commands::game::ship_gate(host.session(), &args.project, &gate_hub)
                        .or_else(|| {
                            config.agents.rules.ask_before_shipping.then(|| {
                                Arc::new(ShipGate::for_agent(gate_hub)) as Arc<dyn StepGate>
                            })
                        });
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
}
