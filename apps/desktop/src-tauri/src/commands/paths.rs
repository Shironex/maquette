//! The Paths editor: the curves through the scene and their energize
//! tracks, kept in the project's `layout.json` next to the models and stops.
//! The editor reads them with the rest of the layout (`layout_get`).

use std::collections::BTreeMap;

use maquette_core::layout::{self, LayoutView, PathTrack, ScenePath};
use maquette_core::project;
use tauri::State;

use super::CommandResult;
use crate::session::Session;

/// Saves every path and their energize tracks, replacing the ones the file
/// has; a track for a path that is gone goes too. The models, stops, states
/// and explode tracks stay as they are. Returns the layout view.
#[tauri::command]
#[specta::specta]
pub fn paths_save(
    session: State<'_, Session>,
    project: String,
    paths: Vec<ScenePath>,
    tracks: BTreeMap<String, PathTrack>,
) -> CommandResult<LayoutView> {
    save(&session, &project, paths, tracks)
}

pub(crate) fn save(
    session: &Session,
    project_id: &str,
    paths: Vec<ScenePath>,
    tracks: BTreeMap<String, PathTrack>,
) -> CommandResult<LayoutView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    let current = layout::load(&project_dir)?;
    let known = layout::known_assets(&manifest);
    let next = layout::paths::with_paths(&current, &known, paths, tracks)?;
    layout::save(&project_dir, &next)?;
    super::layout::get(session, project_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::layout::{Easing, LayoutStop, PathCurve, PathKey, Placement, SceneLayout};
    use maquette_core::{
        AppState, AssetKind, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, assets,
        workspace,
    };

    fn cord(end: Option<&str>) -> ScenePath {
        ScenePath {
            id: "cord".into(),
            name: "Cord".into(),
            curve: PathCurve::Polyline,
            points: vec![[0.0, 0.0, 0.0], [0.4, 0.0, 0.0]],
            start: None,
            end: end.map(str::to_string),
            width: 0.01,
            color: "#3a3936".into(),
            glow: "#ffe08a".into(),
        }
    }

    #[test]
    fn a_save_keeps_the_layout_and_the_layout_editor_keeps_the_paths() {
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
        let mut placed = SceneLayout {
            stops: vec![LayoutStop {
                id: "porch".into(),
                name: "Porch".into(),
                position: [0.0, 1.0, 2.0],
                target: [0.0; 3],
                fov: 35.0,
                models: None,
            }],
            ..SceneLayout::default()
        };
        placed.models.insert("lamp".into(), Placement::default());
        layout::save(&project_dir, &placed).expect("layout");

        let refused = save(
            &session,
            &manifest.id,
            vec![cord(Some("ghost"))],
            BTreeMap::new(),
        );
        assert!(refused.is_err(), "ghost has no model");

        let track = PathTrack {
            easing: Easing::Linear,
            keys: vec![PathKey {
                stop: "porch".into(),
                amount: 1.0,
            }],
        };
        let tracks = BTreeMap::from([("cord".to_string(), track)]);
        let view = save(&session, &manifest.id, vec![cord(Some("lamp"))], tracks).expect("save");
        assert_eq!(view.layout.paths[0].id, "cord");
        assert_eq!(view.layout.timeline.paths["cord"].keys.len(), 1);
        assert!(view.layout.models.contains_key("lamp"));

        // The Layout editor moves the lamp: the cord's end goes with it and the
        // paths stay, though the editor's own layout has none.
        let mut moved = view.layout.clone();
        moved.paths.clear();
        moved.models.insert(
            "lamp".into(),
            Placement {
                position: [1.0, 0.0, 0.0],
                ..Placement::default()
            },
        );
        let after = super::super::layout::save(&session, &manifest.id, &moved).expect("layout");
        assert_eq!(after.layout.paths[0].points[1], [1.4, 0.0, 0.0]);
        assert_eq!(after.layout.timeline.paths["cord"].keys.len(), 1);
    }
}
