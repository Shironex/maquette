//! The Inspector: a project's models with their numbers, the comparisons
//! and the camera stops (see `maquette_core::inspect`).

use std::path::Path;

use maquette_core::inbox::checks::texture_px;
use maquette_core::inspect::{self, InspectView, Sources};
use maquette_core::pipeline::history::RunState;
use maquette_core::pipeline::{self, CompressDefaults, export_one};
use maquette_core::{CoreError, WorkspaceConfig, project};
use serde::Serialize;
use tauri::State;

use super::CommandResult;
use super::pipeline::Pipelines;
use crate::session::Session;

fn view(root: &Path, config: &WorkspaceConfig, project_id: &str) -> CommandResult<InspectView> {
    let manifest = project::load(root, &config.layout, project_id)?;
    let project_dir = project::dir(root, &config.layout, project_id)?;
    let def = pipeline::ensure(&project_dir, &manifest)?;
    Ok(inspect::inspect(&Sources {
        project_dir: &project_dir,
        manifest: &manifest,
        def: &def,
        defaults: CompressDefaults {
            texture_size: texture_px(config.compression.texture_size),
            codec: config.compression.texture_codec,
            meshopt: config.compression.meshopt,
        },
        limits: &config.budgets,
    }))
}

/// Reads every model of the project, off the main thread: a large raw
/// export takes a moment to measure.
#[tauri::command]
#[specta::specta]
pub async fn inspect_project(
    session: State<'_, Session>,
    project: String,
) -> CommandResult<InspectView> {
    let (root, config) = session.workspace()?;
    tauri::async_runtime::spawn_blocking(move || view(&root, &config, &project))
        .await
        .map_err(super::join_failed)?
}

/// What Export GLB made.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportedModel {
    pub model: String,
    /// The raw file it replaced, relative to the project.
    pub file: String,
    pub run_number: u32,
}

/// Export GLB for one model: the pipeline's export step runs on its own
/// (as a run of its own, so it shows in the Pipeline log) into a scratch
/// folder, and only this model's raw GLB is replaced. Compress picks it up
/// on the next run.
#[tauri::command]
#[specta::specta]
pub async fn inspect_export_glb(
    session: State<'_, Session>,
    runner: State<'_, Pipelines>,
    project: String,
    model: String,
) -> CommandResult<ExportedModel> {
    let model = model.trim().to_string();
    if model.is_empty() || model.contains(['/', '\\']) || model.starts_with('.') {
        return Err(CoreError::Invalid(format!("{model:?} is not a model name.")).into());
    }
    let tools = super::toolchain::toolbox(&session).await?;
    let mut setup = super::pipeline::setup(&session, &project, None, true, tools)?;
    let out_dir = setup.cache_dir.join(export_one::STEP_ID).join(&model);
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).map_err(|e| CoreError::io(&out_dir, e))?;
    }
    std::fs::create_dir_all(&out_dir).map_err(|e| CoreError::io(&out_dir, e))?;
    let full = setup.def.clone();
    setup.def = export_one::def(&full, &setup.project_dir, &model, &out_dir)?;
    let project_dir = setup.project_dir.clone();
    let runner = runner.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let run = runner.run_blocking(setup)?;
        if run.state != RunState::Done {
            return Err(CoreError::Invalid(run.message.unwrap_or_else(|| {
                format!("Run {} did not finish. See the Pipeline log.", run.number)
            }))
            .into());
        }
        let kept = export_one::keep(&full, &project_dir, &model, &out_dir)?;
        let _ = std::fs::remove_dir_all(&out_dir);
        Ok(ExportedModel {
            file: kept
                .strip_prefix(&project_dir)
                .unwrap_or(&kept)
                .to_string_lossy()
                .replace('\\', "/"),
            model,
            run_number: run.number,
        })
    })
    .await
    .map_err(super::join_failed)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::{NewProject, ProjectTemplate, workspace};

    #[test]
    fn inspects_a_new_project_without_models() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let manifest = project::create(
            &root,
            &config.layout,
            NewProject {
                name: "Moon Island".into(),
                template: ProjectTemplate::FloatingDiorama,
                target: Default::default(),
                game: None,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        let found = view(&root, &config, &manifest.id).expect("view");
        assert_eq!(found.project, "moon-island");
        assert!(found.models.is_empty());
        assert!(found.comparisons.is_empty());
        assert!((found.limits.scene_mb - 3.0).abs() < f64::EPSILON);
        assert_eq!(
            view(&root, &config, "missing").expect_err("missing").code,
            "not-found"
        );
    }
}
