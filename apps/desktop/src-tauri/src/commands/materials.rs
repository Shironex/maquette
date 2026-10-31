//! Material tweaks: per asset base color, roughness, metalness and emissive
//! changes, kept in the project's `materials.json` and applied by the
//! Compress step to the shipped models.

use std::collections::BTreeMap;
use std::path::PathBuf;

use maquette_core::materials::{self, ModelMaterials, SceneMaterials};
use maquette_core::{CoreResult, project};
use tauri::State;

use super::CommandResult;
use crate::session::Session;

#[tauri::command(async)]
#[specta::specta]
pub fn materials_get(
    session: State<'_, Session>,
    project: String,
) -> CommandResult<SceneMaterials> {
    get(&session, &project)
}

pub(crate) fn get(session: &Session, project_id: &str) -> CommandResult<SceneMaterials> {
    Ok(materials::load(&project_dir(session, project_id)?)?)
}

/// The folder of a project that exists in the open workspace.
fn project_dir(session: &Session, project_id: &str) -> CommandResult<PathBuf> {
    let (root, config) = session.workspace()?;
    project::load(&root, &config.layout, project_id)?;
    Ok(project::dir(&root, &config.layout, project_id)?)
}

/// Saves the looks of the assets the panel edited (an empty look clears
/// the asset's tweaks) into `materials.json` as it is on disk, so every other
/// asset keeps its tweaks. Returns the whole file as written. They apply the
/// next time Compress runs.
#[tauri::command]
#[specta::specta]
pub fn materials_save(
    session: State<'_, Session>,
    project: String,
    looks: BTreeMap<String, ModelMaterials>,
) -> CommandResult<SceneMaterials> {
    save(&session, &project, &looks)
}

pub(crate) fn save(
    session: &Session,
    project_id: &str,
    looks: &BTreeMap<String, ModelMaterials>,
) -> CommandResult<SceneMaterials> {
    Ok(materials::save_looks(
        &project_dir(session, project_id)?,
        looks,
    )?)
}

/// Applies `change` to the project's `materials.json` as it is on disk
/// ([`materials::update`]) and returns what was written.
pub(crate) fn update(
    session: &Session,
    project_id: &str,
    change: impl FnOnce(&SceneMaterials) -> CoreResult<SceneMaterials>,
) -> CommandResult<SceneMaterials> {
    Ok(materials::update(
        &project_dir(session, project_id)?,
        change,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::materials::{FactorMode, MaterialTweak, MaterialsPatch, RoughnessTweak};
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
                target: Default::default(),
                game: None,
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

        let laptop = ModelMaterials {
            all: MaterialTweak {
                roughness: Some(RoughnessTweak {
                    mode: FactorMode::Override,
                    value: 0.62,
                }),
                ..MaterialTweak::default()
            },
            ..ModelMaterials::default()
        };
        let looks = BTreeMap::from([("laptop".to_string(), laptop)]);
        let saved = save(&session, &manifest.id, &looks).expect("save");
        assert_eq!(get(&session, &manifest.id).expect("get"), saved);

        let change: MaterialsPatch =
            serde_json::from_value(serde_json::json!({ "all": { "metalness": 0.25 } }))
                .expect("patch");
        let patched = update(&session, &manifest.id, |current| {
            materials::apply_patch(current, "laptop", &change, &[])
        })
        .expect("save patch");
        let laptop = &patched.models["laptop"].all;
        assert_eq!(laptop.metalness, Some(0.25));
        assert_eq!(laptop.roughness.map(|r| r.value), Some(0.62));

        // Saving another asset's look keeps the laptop's.
        let mut bench = patched.models["laptop"].clone();
        bench.all.tint = Some("#6f7446".into());
        let both = save(
            &session,
            &manifest.id,
            &BTreeMap::from([("bench".to_string(), bench)]),
        )
        .expect("bench");
        assert_eq!(both.models["laptop"], patched.models["laptop"]);

        let mut bad = patched.models["laptop"].clone();
        bad.all.tint = Some("green".into());
        assert_eq!(
            save(
                &session,
                &manifest.id,
                &BTreeMap::from([("laptop".to_string(), bad)])
            )
            .expect_err("bad")
            .code,
            "invalid"
        );
        assert!(get(&session, "nope").is_err());
    }
}
