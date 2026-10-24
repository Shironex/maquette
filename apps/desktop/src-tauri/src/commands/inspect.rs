//! The Inspector: a project's models with their numbers, the comparisons
//! and the camera stops (see `maquette_core::inspect`).

use std::path::Path;

use maquette_core::inbox::checks::texture_px;
use maquette_core::inspect::{self, InspectView, Sources};
use maquette_core::pipeline::{self, CompressDefaults};
use maquette_core::{WorkspaceConfig, project};
use tauri::State;

use super::CommandResult;
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
