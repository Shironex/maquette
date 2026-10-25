//! The pipeline view: the graph, its runs and logs, the compression presets
//! and the run, cancel and run from here actions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use maquette_core::inbox::checks::texture_px;
use maquette_core::layout::{self, FirstView};
use maquette_core::materials;
use maquette_core::pipeline::cache::{self, project_cache_dir};
use maquette_core::pipeline::history::{LogEntry, RunRecord};
use maquette_core::pipeline::runner::{JobRunner, PipelineRunner, RunSetup};
use maquette_core::pipeline::starter::{self, StarterStatus};
use maquette_core::pipeline::tools::Toolbox;
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
    let shipped_names = maquette_core::assets::shipped_names(&ctx.manifest);
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
    Ok(PipelineView {
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

pub(crate) fn setup(
    session: &Session,
    project_id: &str,
    from: Option<String>,
    force: bool,
    tools: Toolbox,
) -> CommandResult<RunSetup> {
    let ctx = context(session, project_id)?;
    let mut vars = BTreeMap::new();
    vars.insert("workspace".to_string(), ctx.root.display().to_string());
    vars.insert(
        "siteRepo".to_string(),
        ctx.manifest.site_repo.clone().unwrap_or_default(),
    );
    let tweaks = materials::by_model_stem(&ctx.manifest, &materials::load(&ctx.project_dir)?);
    Ok(RunSetup {
        project: project_id.to_string(),
        project_dir: ctx.project_dir,
        cache_dir: ctx.cache_dir,
        defaults: defaults(&ctx.config),
        def: ctx.def,
        tools,
        vars,
        shipped_names: maquette_core::assets::shipped_names(&ctx.manifest),
        materials: tweaks,
        from,
        force,
    })
}

#[tauri::command]
#[specta::specta]
pub fn pipeline_get(
    session: State<'_, Session>,
    runner: State<'_, Pipelines>,
    project: String,
) -> CommandResult<PipelineView> {
    view(&session, &runner.0, &project)
}

/// Queues a run. `from` runs that step (always) and what depends on it;
/// `force` ignores the cache.
#[tauri::command]
#[specta::specta]
pub async fn pipeline_run(
    session: State<'_, Session>,
    runner: State<'_, Pipelines>,
    project: String,
    from: Option<String>,
    force: bool,
) -> CommandResult<RunRecord> {
    let tools = super::toolchain::toolbox(&session).await?;
    let setup = setup(&session, &project, from, force, tools)?;
    Ok(runner.0.submit(setup)?)
}

#[tauri::command]
#[specta::specta]
pub fn pipeline_cancel(runner: State<'_, Pipelines>, run_id: String) -> CommandResult<()> {
    Ok(runner.0.cancel(&run_id)?)
}

#[tauri::command]
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
    let mut ctx = context(session, project_id)?;
    ctx.def.presets = presets::apply(&ctx.def.presets, incoming)?;
    pipeline::save(&ctx.project_dir, &ctx.def)?;
    view(session, runner, project_id)
}

/// Saves the preset table. Locked rows must come back unchanged.
#[tauri::command]
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

#[tauri::command]
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

#[cfg(test)]
mod tests;
