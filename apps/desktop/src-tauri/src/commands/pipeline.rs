//! The pipeline view: the graph, its runs and logs, the compression presets
//! and the run, cancel and run from here actions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use maquette_core::inbox::checks::texture_px;
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
    Ok(PipelineView {
        starters,
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
    Ok(RunSetup {
        project: project_id.to_string(),
        project_dir: ctx.project_dir,
        cache_dir: ctx.cache_dir,
        defaults: defaults(&ctx.config),
        def: ctx.def,
        tools,
        vars,
        shipped_names: maquette_core::assets::shipped_names(&ctx.manifest),
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
mod tests {
    use std::sync::Arc;

    use super::*;
    use maquette_core::pipeline::history::RunState;
    use maquette_core::pipeline::runner::NullSink;
    use maquette_core::{AppState, MeshoptLevel, NewProject, ProjectTemplate, workspace};

    fn open() -> (tempfile::TempDir, Session, String) {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let manifest = project::create(
            &root,
            &config.layout,
            NewProject {
                name: "Moon Island".into(),
                template: ProjectTemplate::FloatingDiorama,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        (dir, session, manifest.id)
    }

    #[test]
    fn exported_models_carry_the_name_of_their_asset() {
        use maquette_core::{Asset, AssetKind, AssetStatus, AssetVersion, Stage};
        let (_dir, session, id) = open();
        let runner = PipelineRunner::new(Arc::new(NullSink));
        let (root, config) = session.workspace().expect("ws");
        let dir = project::dir(&root, &config.layout, &id).expect("dir");
        std::fs::create_dir_all(dir.join("web/raw")).expect("raw");
        std::fs::write(dir.join("web/raw/clay_v1.glb"), b"glb").expect("export");
        std::fs::write(dir.join("web/raw/island.glb"), b"glb").expect("island");
        project::modify(&root, &config.layout, &id, |m| {
            m.assets.push(Asset {
                id: "clay".into(),
                name: "clay".into(),
                kind: AssetKind::Prop,
                stage: Stage::Blender,
                status: AssetStatus::Review,
                versions: vec![AssetVersion {
                    version: 1,
                    file: "models/clay_v1.glb".into(),
                    created_at: "2026-10-24T19:14:19.000Z".into(),
                    stats: None,
                }],
                thumbnail: None,
                created_at: "2026-10-24T19:00:00.000Z".into(),
                updated_at: "2026-10-24T19:14:19.000Z".into(),
            });
            Ok(())
        })
        .expect("asset");
        let view = view(&session, &runner, &id).expect("view");
        let clay = view
            .models
            .iter()
            .find(|m| m.model == "clay_v1")
            .expect("clay");
        assert_eq!(clay.asset_name.as_deref(), Some("clay"));
        assert_eq!(clay.version, Some(1));
        let island = view
            .models
            .iter()
            .find(|m| m.model == "island")
            .expect("island");
        assert_eq!(island.asset_name, None);
    }

    #[test]
    fn old_starter_scripts_show_and_update_from_the_view() {
        use maquette_core::pipeline::starter::StarterState;
        let (_dir, session, id) = open();
        let runner = PipelineRunner::new(Arc::new(NullSink));
        let first = view(&session, &runner, &id).expect("view");
        assert!(!first.starters.is_empty());
        assert!(
            first
                .starters
                .iter()
                .all(|s| s.state == StarterState::Current)
        );
        let (root, config) = session.workspace().expect("ws");
        let dir = project::dir(&root, &config.layout, &id).expect("dir");
        std::fs::write(dir.join("tools/bake_light.py"), "print('mine')\n").expect("edit");
        let edited = view(&session, &runner, &id).expect("view");
        let bake = |v: &PipelineView| {
            v.starters
                .iter()
                .find(|s| s.path == "tools/bake_light.py")
                .expect("bake")
                .clone()
        };
        assert_eq!(bake(&edited).state, StarterState::Edited);
        let all = vec!["tools/bake_light.py".to_string()];
        let kept = update_starters(&session, &runner, &id, &all, false).expect("update");
        assert_eq!(bake(&kept).state, StarterState::Edited);
        let replaced = update_starters(&session, &runner, &id, &all, true).expect("replace");
        assert_eq!(bake(&replaced).state, StarterState::Current);
        assert_eq!(
            std::fs::read_to_string(dir.join("tools/bake_light.py.bak")).expect("bak"),
            "print('mine')\n"
        );
    }

    #[test]
    fn the_view_writes_the_template_and_reports_no_runs() {
        let (_dir, session, id) = open();
        let runner = PipelineRunner::new(Arc::new(NullSink));
        let view = view(&session, &runner, &id).expect("view");
        assert_eq!(view.def.steps.len(), 6);
        assert!(view.issues.iter().all(|i| !i.is_error()));
        assert!(view.runs.is_empty());
        assert_eq!(view.defaults.texture_size, 2048);
        let (root, config) = session.workspace().expect("ws");
        let dir = project::dir(&root, &config.layout, &id).expect("dir");
        assert!(dir.join("pipeline.json").is_file());
    }

    #[test]
    fn locked_presets_refuse_edits() {
        use maquette_core::{Asset, AssetKind, AssetStatus, AssetVersion, Stage};
        let (_dir, session, id) = open();
        let runner = PipelineRunner::new(Arc::new(NullSink));
        // A scene without a character has no character row and no phantom model.
        let first = view(&session, &runner, &id).expect("view");
        assert!(first.def.presets.is_empty());
        assert!(first.models.iter().all(|m| m.model != "character"));
        let (root, config) = session.workspace().expect("ws");
        project::modify(&root, &config.layout, &id, |m| {
            m.assets.push(Asset {
                id: "keeper".into(),
                name: "Keeper".into(),
                kind: AssetKind::Character,
                stage: Stage::Blender,
                status: AssetStatus::Review,
                versions: vec![AssetVersion {
                    version: 1,
                    file: "models/keeper_v1.glb".into(),
                    created_at: String::new(),
                    stats: None,
                }],
                thumbnail: None,
                created_at: String::new(),
                updated_at: String::new(),
            });
            Ok(())
        })
        .expect("character");
        let mut rows = view(&session, &runner, &id).expect("view").def.presets;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].model, "keeper_v1");
        assert_eq!(rows[0].label, "Keeper");
        let locked = rows
            .iter()
            .position(|p| p.lock.is_some())
            .expect("character row");
        rows[locked].meshopt = MeshoptLevel::High;
        let err = save_presets(&session, &runner, &id, rows.clone()).expect_err("locked");
        assert_eq!(err.code, "invalid");
        rows[locked].meshopt = MeshoptLevel::Medium;
        rows.push(CompressPreset {
            model: "bench".into(),
            label: "Bench".into(),
            texture_size: 1024,
            data_texture_size: None,
            codec: maquette_core::TextureCodec::Uastc,
            meshopt: MeshoptLevel::High,
            lock: None,
        });
        let saved = save_presets(&session, &runner, &id, rows).expect("save");
        assert!(saved.def.presets.iter().any(|p| p.model == "bench"));
    }

    #[test]
    fn a_run_without_scripts_fails_with_a_clear_message() {
        let (_dir, session, id) = open();
        let runner = PipelineRunner::new(Arc::new(NullSink));
        let setup = setup(&session, &id, None, false, Toolbox::default()).expect("setup");
        let run = runner.run_blocking(setup).expect("run");
        assert_eq!(run.state, RunState::Failed);
        let view = view(&session, &runner, &id).expect("view");
        assert_eq!(view.runs.len(), 1);
        assert!(!view.log.is_empty());
    }
}
