//! The pipeline view: the graph, its runs and logs, the compression presets
//! and the run, cancel and run from here actions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use maquette_core::inbox::checks::texture_px;
use maquette_core::layout::{self, FirstView};
use maquette_core::pipeline::cache::{self, project_cache_dir};
use maquette_core::pipeline::history::{LogEntry, RunRecord};
use maquette_core::pipeline::runner::{JobRunner, PipelineRunner};
use maquette_core::pipeline::starter::{self, StarterStatus};
use maquette_core::pipeline::{
    self, CompressDefaults, CompressPreset, PipelineDef, PipelineIssue, history, presets,
};
use maquette_core::{ProjectManifest, WorkspaceConfig, project};
use serde::Serialize;
use tauri::State;

use super::CommandResult;
use crate::session::Session;

/// The runner the app shares between commands.
pub struct Pipelines(pub PipelineRunner);

/// A file a step wrote.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub step: String,
    /// Relative to the pipeline root.
    pub path: String,
    pub bytes: f64,
}

/// A model the compress step will see, with its sizes before and after.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelFile {
    /// The GLB file stem, for example `clay_v1`.
    pub model: String,
    /// The asset it was exported from, for example `clay`, when one was.
    pub asset_name: Option<String>,
    /// That asset's version.
    pub version: Option<u32>,
    pub raw_bytes: Option<f64>,
    pub shipped_bytes: Option<f64>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompressDefaultsView {
    pub texture_size: u32,
    pub codec: maquette_core::TextureCodec,
    pub meshopt: maquette_core::MeshoptLevel,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PipelineView {
    pub project: String,
    pub root: String,
    pub def: PipelineDef,
    pub issues: Vec<PipelineIssue>,
    /// Newest first.
    pub runs: Vec<RunRecord>,
    /// The run in progress, if any.
    pub active: Option<String>,
    pub queued: Vec<RunRecord>,
    /// The log of the newest run.
    pub log: Vec<LogEntry>,
    pub artifacts: Vec<Artifact>,
    pub models: Vec<ModelFile>,
    pub defaults: CompressDefaultsView,
    /// The template's starter scripts against the ones this build ships.
    pub starters: Vec<StarterStatus>,
    /// What the first camera stop loads, against its budget.
    pub first_view: FirstView,
    /// The Bake light step's size, samples and denoiser, when the pipeline
    /// runs the starter bake.
    pub bake: Option<pipeline::bake::BakeView>,
}

fn defaults(config: &WorkspaceConfig) -> CompressDefaults {
    CompressDefaults {
        texture_size: texture_px(config.compression.texture_size),
        codec: config.compression.texture_codec,
        meshopt: config.compression.meshopt,
    }
}

fn file_bytes(path: &Path) -> Option<f64> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "model sizes stay far below 2^53"
    )]
    std::fs::metadata(path).ok().map(|m| m.len() as f64)
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

struct Context {
    root: PathBuf,
    config: WorkspaceConfig,
    manifest: ProjectManifest,
    project_dir: PathBuf,
    cache_dir: PathBuf,
    def: PipelineDef,
}

fn context(session: &Session, project_id: &str) -> CommandResult<Context> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    let def = pipeline::ensure(&project_dir, &manifest)?;
    let cache_dir = project_cache_dir(&root.join(&config.layout.cache), project_id);
    Ok(Context {
        root,
        config,
        manifest,
        project_dir,
        cache_dir,
        def,
    })
}

/// The file stem Export GLB gives a model file: lowercase snake case.
fn export_stem(stem: &str) -> String {
    let snake: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = snake.trim_matches('_');
    let mut out = String::new();
    for part in trimmed.split('_').filter(|p| !p.is_empty()) {
        if !out.is_empty() {
            out.push('_');
        }
        out.push_str(part);
    }
    if out.is_empty() {
        "model".to_string()
    } else {
        out
    }
}

pub(crate) fn view(
    session: &Session,
    runner: &PipelineRunner,
    project_id: &str,
) -> CommandResult<PipelineView> {
    let ctx = context(session, project_id)?;
    let run_root = ctx.def.root_dir(&ctx.project_dir);
    let runs = history::load_runs(&ctx.cache_dir);
    let log = runs
        .first()
        .map(|r| history::load_log(&ctx.cache_dir, r.number))
        .unwrap_or_default();
    let mut artifacts = Vec::new();
    for step in &ctx.def.steps {
        for (_, files) in cache::expand(&run_root, &step.outputs) {
            for file in files.into_iter().filter(|f| f.is_file()).take(24) {
                artifacts.push(Artifact {
                    step: step.id.clone(),
                    path: rel(&run_root, &file),
                    bytes: file_bytes(&file).unwrap_or(0.0),
                });
            }
        }
    }
    let ship = maquette_core::ship::load_or_default(&ctx.project_dir);
    let shipped_names = maquette_core::assets::shipped_names(&ctx.manifest, &ship);
    let mut models: BTreeMap<String, ModelFile> = BTreeMap::new();
    for spec in ctx.def.steps.iter().filter_map(|s| s.compress.as_ref()) {
        for (_, files) in cache::expand(&run_root, &[format!("{}/*.glb", spec.from)]) {
            for file in files {
                let Some(stem) = file.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
                    continue;
                };
                // A renamed asset ships under its new name once the step ran;
                // until then the old file is what is built.
                let named = shipped_names.get(&stem.to_lowercase());
                let shipped = named
                    .map(|name| run_root.join(&spec.to).join(format!("{name}.glb")))
                    .filter(|path| path.is_file())
                    .unwrap_or_else(|| run_root.join(&spec.to).join(format!("{stem}.glb")));
                models.insert(
                    stem.clone(),
                    ModelFile {
                        model: stem,
                        asset_name: None,
                        version: None,
                        raw_bytes: file_bytes(&file),
                        shipped_bytes: file_bytes(&shipped),
                    },
                );
            }
        }
    }
    for preset in &ctx.def.presets {
        models
            .entry(preset.model.clone())
            .or_insert_with(|| ModelFile {
                model: preset.model.clone(),
                asset_name: None,
                version: None,
                raw_bytes: None,
                shipped_bytes: None,
            });
    }
    // Before the first export, each asset's newest model is listed under the
    // stem its export will have, so its presets can be set already.
    for asset in &ctx.manifest.assets {
        let Some(stem) = asset.latest().and_then(|v| {
            let path = Path::new(&v.file);
            let glb = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("glb") || e.eq_ignore_ascii_case("gltf"));
            glb.then(|| path.file_stem().map(|s| export_stem(&s.to_string_lossy())))
                .flatten()
        }) else {
            continue;
        };
        if !models.contains_key(&stem) {
            models.insert(
                stem.clone(),
                ModelFile {
                    model: stem.clone(),
                    asset_name: None,
                    version: None,
                    raw_bytes: None,
                    shipped_bytes: None,
                },
            );
        }
    }
    // An export of an asset version is labelled by the asset ("clay").
    for model in models.values_mut() {
        if let Some((asset, version)) =
            maquette_core::assets::for_model_stem(&ctx.manifest, &model.model)
        {
            model.asset_name = Some(asset.name.clone());
            model.version = Some(version);
        }
    }
    let (active, queued) = runner.active(project_id);
    let d = defaults(&ctx.config);
    let starters = starter::status(&ctx.project_dir, ctx.manifest.template);
    let previews =
        maquette_core::prepare::preview::previews_dir(&ctx.root.join(&ctx.config.layout.cache));
    let first_view = layout::first_view_of(
        &ctx.project_dir,
        &ctx.manifest,
        &ctx.def,
        ctx.config.budgets.first_view_mb,
        Some(&previews),
    );
    let bake = pipeline::bake::view(&ctx.project_dir, ctx.manifest.template, &ctx.def);
    Ok(PipelineView {
        bake,
        starters,
        first_view,
        project: ctx.manifest.id.clone(),
        root: session.display(&run_root),
        issues: pipeline::validate(&ctx.def),
        def: ctx.def,
        runs,
        active,
        queued,
        log,
        artifacts,
        models: models.into_values().collect(),
        defaults: CompressDefaultsView {
            texture_size: d.texture_size,
            codec: d.codec,
            meshopt: d.meshopt,
        },
    })
}

#[tauri::command(async)]
#[specta::specta]
pub fn pipeline_get(
    session: State<'_, Session>,
    runner: State<'_, Pipelines>,
    project: String,
) -> CommandResult<PipelineView> {
    view(&session, &runner.0, &project)
}

/// Queues a run. `from` runs that step (always) and what depends on it;
/// `force` ignores the cache; `buildOnly` leaves the Ship step out. A full
/// run (no `from`) first rebuilds the built assets that are new or stale,
/// and is refused when one fails.
#[tauri::command]
#[specta::specta]
pub async fn pipeline_run(
    app: tauri::AppHandle,
    session: State<'_, Session>,
    runner: State<'_, Pipelines>,
    thumbs: State<'_, super::thumbs::Thumbs>,
    agents: State<'_, crate::mcp::Agents>,
    project: String,
    ask: RunAsk,
) -> CommandResult<RunRecord> {
    let tools = super::toolchain::toolbox(&session).await?;
    // A full run first rebuilds the built assets whose scripts changed.
    if ask.from.is_none() {
        let (project, tools, thumbs) = (project.clone(), tools.clone(), thumbs.0.clone());
        tauri::async_runtime::spawn_blocking(move || {
            use tauri::Manager as _;
            let session = app.state::<Session>();
            let env = super::built::BuildEnv {
                session: session.inner(),
                thumbs: &thumbs,
                tools: &tools,
                notify: &super::built::Windows(app.clone()),
            };
            super::built::before_run(&env, &project)
        })
        .await
        .map_err(super::join_failed)??;
    }
    let setup = setup(&session, &project, ask, tools)?;
    // A game's Ship writes into the game repo, so it waits for approval.
    let gate = super::game::ship_gate(&session, &project, &agents.hub);
    Ok(runner.0.submit_with(
        setup,
        maquette_core::pipeline::runner::RunOptions {
            gate,
            ..Default::default()
        },
    )?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn pipeline_cancel(runner: State<'_, Pipelines>, run_id: String) -> CommandResult<()> {
    Ok(runner.0.cancel(&run_id)?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn pipeline_log(
    session: State<'_, Session>,
    project: String,
    number: u32,
) -> CommandResult<Vec<LogEntry>> {
    let ctx = context(&session, &project)?;
    Ok(history::load_log(&ctx.cache_dir, number))
}

pub(crate) fn save_presets(
    session: &Session,
    runner: &PipelineRunner,
    project_id: &str,
    incoming: Vec<CompressPreset>,
) -> CommandResult<PipelineView> {
    let ctx = context(session, project_id)?;
    pipeline::update(&ctx.project_dir, &ctx.manifest, |def| {
        def.presets = presets::apply(&def.presets, incoming)?;
        Ok(())
    })?;
    view(session, runner, project_id)
}

/// Saves the preset table. Locked rows must come back unchanged.
#[tauri::command(async)]
#[specta::specta]
pub fn pipeline_save_presets(
    session: State<'_, Session>,
    runner: State<'_, Pipelines>,
    project: String,
    presets: Vec<CompressPreset>,
) -> CommandResult<PipelineView> {
    save_presets(&session, &runner.0, &project, presets)
}

/// Updates the starter scripts at `scripts`: untouched older copies and
/// missing ones always, edited ones only with `replace` (the old file is kept
/// as `<name>.bak`). Refused while the project's pipeline runs.
pub(crate) fn update_starters(
    session: &Session,
    runner: &PipelineRunner,
    project_id: &str,
    scripts: &[String],
    replace: bool,
) -> CommandResult<PipelineView> {
    if runner.active(project_id).0.is_some() {
        return Err(super::invalid(
            "The pipeline is running. Update the scripts once the run is done.",
        ));
    }
    let ctx = context(session, project_id)?;
    starter::update(&ctx.project_dir, ctx.manifest.template, scripts, replace)?;
    view(session, runner, project_id)
}

#[tauri::command(async)]
#[specta::specta]
pub fn pipeline_update_starters(
    session: State<'_, Session>,
    runner: State<'_, Pipelines>,
    project: String,
    scripts: Vec<String>,
    replace: bool,
) -> CommandResult<PipelineView> {
    update_starters(&session, &runner.0, &project, &scripts, replace)
}

pub(crate) mod bake;
pub(crate) use bake::save_bake;
mod run;
pub use run::RunAsk;
pub(crate) use run::setup;

#[cfg(test)]
mod tests;
