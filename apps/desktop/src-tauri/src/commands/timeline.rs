//! The Timeline view: the project's camera stops as the page scrolls, with
//! the stop and scroll triggers of its object states and the explode tracks,
//! all kept in the project's `layout.json`.

use std::collections::BTreeMap;

use maquette_core::layout::{StateTrigger, Timeline};
use maquette_core::project;
use maquette_core::timeline::{self, TimelineView};
use tauri::State;

use super::CommandResult;
use crate::session::Session;

#[tauri::command(async)]
#[specta::specta]
pub fn timeline_get(session: State<'_, Session>, project: String) -> CommandResult<TimelineView> {
    get(&session, &project)
}

pub(crate) fn get(session: &Session, project_id: &str) -> CommandResult<TimelineView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    Ok(timeline::view(&project_dir, &manifest)?)
}

/// Saves the stop and scroll triggers of each asset in `triggers` and the
/// whole timeline. The states, their transitions and the click and hover
/// triggers stay as the Motion view saved them; the models and camera stops
/// stay as they are.
#[tauri::command]
#[specta::specta]
pub fn timeline_save(
    session: State<'_, Session>,
    project: String,
    triggers: BTreeMap<String, Vec<StateTrigger>>,
    timeline: Timeline,
) -> CommandResult<TimelineView> {
    save(&session, &project, &triggers, timeline)
}

pub(crate) fn save(
    session: &Session,
    project_id: &str,
    triggers: &BTreeMap<String, Vec<StateTrigger>>,
    next: Timeline,
) -> CommandResult<TimelineView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    Ok(timeline::save(&project_dir, &manifest, triggers, next)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::layout::{self, ExplodeKey, ExplodeTrack, LayoutStop, SceneLayout};
    use maquette_core::{
        AppState, AssetKind, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, assets,
        workspace,
    };

    #[test]
    fn the_view_reads_the_stops_and_a_save_keeps_the_placements() {
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
        placed
            .models
            .insert("lamp".into(), layout::Placement::default());
        layout::save(&project_dir, &placed).expect("layout");

        let view = get(&session, &manifest.id).expect("get");
        assert_eq!(view.stops[0].id, "porch");
        assert_eq!(view.models[0].asset, "lamp");
        assert!(view.models[0].parts.is_empty(), "the bytes are no model");

        let mut next = Timeline::default();
        next.explode.insert(
            "lamp".into(),
            ExplodeTrack {
                keys: vec![ExplodeKey {
                    stop: "porch".into(),
                    amount: 1.0,
                }],
                ..ExplodeTrack::default()
            },
        );
        let saved = save(&session, &manifest.id, &BTreeMap::new(), next).expect("save");
        assert_eq!(saved.timeline.explode["lamp"].keys.len(), 1);
        let back = layout::load(&project_dir).expect("load");
        assert!(back.models.contains_key("lamp"));
        assert_eq!(back.timeline, saved.timeline);
    }
}
