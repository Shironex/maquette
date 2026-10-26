//! The Motion view: every model's rig, clips and parts, what the clips
//! weigh inside the scene budget, and the object states and clip choices
//! kept in the project's `layout.json`.

use std::collections::BTreeMap;

use maquette_core::layout::ObjectStates;
use maquette_core::motion::{self, MotionView};
use maquette_core::prepare::preview::previews_dir;
use maquette_core::{pipeline, project};
use tauri::State;

use super::CommandResult;
use crate::session::Session;

#[tauri::command]
#[specta::specta]
pub fn motion_get(session: State<'_, Session>, project: String) -> CommandResult<MotionView> {
    get(&session, &project)
}

pub(crate) fn get(session: &Session, project_id: &str) -> CommandResult<MotionView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    let def = pipeline::ensure(&project_dir, &manifest)?;
    let previews = previews_dir(&root.join(&config.layout.cache));
    Ok(motion::view(
        &project_dir,
        &manifest,
        &def,
        &config.budgets,
        Some(&previews),
    )?)
}

/// Saves every asset's object states and the clips that stay off the site
/// (asset to clip names), replacing the ones the layout had. The models and
/// camera stops stay as they are.
#[tauri::command]
#[specta::specta]
pub fn motion_save(
    session: State<'_, Session>,
    project: String,
    states: BTreeMap<String, ObjectStates>,
    clips_off: BTreeMap<String, Vec<String>>,
) -> CommandResult<MotionView> {
    save(&session, &project, states, &clips_off)
}

pub(crate) fn save(
    session: &Session,
    project_id: &str,
    states: BTreeMap<String, ObjectStates>,
    clips_off: &BTreeMap<String, Vec<String>>,
) -> CommandResult<MotionView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    motion::save(&project_dir, &manifest, states, clips_off)?;
    get(session, project_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::layout::{self, ObjectState};
    use maquette_core::{
        AppState, AssetKind, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, assets,
        workspace,
    };

    #[test]
    fn the_view_reads_every_model_and_a_save_keeps_the_placements() {
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
                target: Default::default(),
                game: None,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
        std::fs::create_dir_all(project_dir.join("models")).expect("models");
        std::fs::write(project_dir.join("models/lamp_v1.glb"), b"glb").expect("glb");
        project::modify(&root, &config.layout, &manifest.id, |m| {
            let lamp = assets::add(
                m,
                NewAsset {
                    name: "Lamp".into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Blender,
                },
            )?;
            assets::add_version(m, &lamp.id, "models/lamp_v1.glb".into(), None).map(|_| ())
        })
        .expect("lamp");

        let view = get(&session, &manifest.id).expect("get");
        assert_eq!(view.models.len(), 1);
        let lamp = &view.models[0];
        assert!(lamp.clips.is_empty() && lamp.rig.is_none());
        assert!(lamp.error.is_some(), "the bytes are no model");
        assert!(view.clips_off_supported, "a new project has the starter");

        let mut placed = layout::SceneLayout::default();
        placed
            .models
            .insert("lamp".into(), layout::Placement::default());
        layout::save(&project_dir, &placed).expect("layout");
        let states = BTreeMap::from([(
            "lamp".to_string(),
            ObjectStates {
                states: vec![ObjectState {
                    id: "off".into(),
                    name: "Off".into(),
                    parts: BTreeMap::new(),
                }],
                ..ObjectStates::default()
            },
        )]);
        let off = BTreeMap::from([("lamp".to_string(), vec!["Flicker".to_string()])]);
        let saved = save(&session, &manifest.id, states, &off).expect("save");
        assert_eq!(saved.states["lamp"].states[0].name, "Off");
        assert_eq!(saved.clips_off["lamp"], ["Flicker"]);
        let back = layout::load(&project_dir).expect("load");
        assert!(back.models.contains_key("lamp"));
    }
}
