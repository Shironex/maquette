//! Material tweaks: per asset base color, roughness, metalness and emissive
//! changes, kept in the project's `materials.json` and applied by the
//! Compress step to the shipped models.

use maquette_core::materials::{self, SceneMaterials};
use maquette_core::project;
use tauri::State;

use super::CommandResult;
use crate::session::Session;

#[tauri::command]
#[specta::specta]
pub fn materials_get(
    session: State<'_, Session>,
    project: String,
) -> CommandResult<SceneMaterials> {
    get(&session, &project)
}

pub(crate) fn get(session: &Session, project_id: &str) -> CommandResult<SceneMaterials> {
    let (root, config) = session.workspace()?;
    project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    Ok(materials::load(&project_dir)?)
}

/// Saves every asset's tweaks and returns what was written (empty entries
/// dropped). They apply the next time Compress runs.
#[tauri::command]
#[specta::specta]
pub fn materials_save(
    session: State<'_, Session>,
    project: String,
    materials: SceneMaterials,
) -> CommandResult<SceneMaterials> {
    save(&session, &project, &materials)
}

pub(crate) fn save(
    session: &Session,
    project_id: &str,
    next: &SceneMaterials,
) -> CommandResult<SceneMaterials> {
    let (root, config) = session.workspace()?;
    project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    Ok(materials::save(&project_dir, next)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::materials::{
        FactorMode, MaterialTweak, MaterialsPatch, ModelMaterials, RoughnessTweak,
    };
    use maquette_core::{AppState, NewProject, ProjectTemplate, WorkspaceConfig, workspace};

    #[test]
    fn tweaks_save_load_and_patch_per_project() {
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
                name: "Workbench".into(),
                template: ProjectTemplate::FloatingDiorama,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        assert!(
            get(&session, &manifest.id)
                .expect("empty")
                .models
                .is_empty()
        );

        let mut next = SceneMaterials::default();
        next.models.insert(
            "laptop".into(),
            ModelMaterials {
                all: MaterialTweak {
                    roughness: Some(RoughnessTweak {
                        mode: FactorMode::Override,
                        value: 0.62,
                    }),
                    ..MaterialTweak::default()
                },
                ..ModelMaterials::default()
            },
        );
        let saved = save(&session, &manifest.id, &next).expect("save");
        assert_eq!(get(&session, &manifest.id).expect("get"), saved);

        let change: MaterialsPatch =
            serde_json::from_value(serde_json::json!({ "all": { "metalness": 0.25 } }))
                .expect("patch");
        let next = materials::apply_patch(&saved, "laptop", &change, &[]).expect("patch");
        let patched = save(&session, &manifest.id, &next).expect("save patch");
        let laptop = &patched.models["laptop"].all;
        assert_eq!(laptop.metalness, Some(0.25));
        assert_eq!(laptop.roughness.map(|r| r.value), Some(0.62));

        let mut bad = patched.clone();
        bad.models.get_mut("laptop").expect("laptop").all.tint = Some("green".into());
        assert_eq!(
            save(&session, &manifest.id, &bad).expect_err("bad").code,
            "invalid"
        );
        assert!(get(&session, "nope").is_err());
    }
}
