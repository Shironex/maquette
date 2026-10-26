//! The scene layout: where each model stands, the camera stops and the
//! desk, kept in the project's `layout.json`.

use maquette_core::layout::{self, LayoutPatch, LayoutView, SceneLayout};
use maquette_core::prepare::preview::previews_dir;
use maquette_core::{pipeline, project};
use tauri::State;

use super::CommandResult;
use crate::session::Session;

#[tauri::command]
#[specta::specta]
pub fn layout_get(session: State<'_, Session>, project: String) -> CommandResult<LayoutView> {
    get(&session, &project)
}

pub(crate) fn get(session: &Session, project_id: &str) -> CommandResult<LayoutView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    let def = pipeline::ensure(&project_dir, &manifest)?;
    let previews = previews_dir(&root.join(&config.layout.cache));
    Ok(layout::view(
        &project_dir,
        &manifest,
        &def,
        config.budgets.first_view_mb,
        Some(&previews),
    )?)
}

/// Saves the whole layout and returns the view of it. The states, clip
/// choices and timeline stay as the file has them: they belong to the
/// Motion view ([`crate::commands::motion`]) and the Timeline view
/// ([`crate::commands::timeline`]). Triggers and explode keys that wait for
/// a stop this layout removes go with it.
#[tauri::command]
#[specta::specta]
pub fn layout_save(
    session: State<'_, Session>,
    project: String,
    layout: SceneLayout,
) -> CommandResult<LayoutView> {
    save(&session, &project, &layout)
}

pub(crate) fn save(
    session: &Session,
    project_id: &str,
    next: &SceneLayout,
) -> CommandResult<LayoutView> {
    let (root, config) = session.workspace()?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    let current = layout::load(&project_dir).unwrap_or_default();
    let mut next = SceneLayout {
        states: current.states,
        clips: current.clips,
        timeline: current.timeline,
        ..next.clone()
    };
    layout::prune_stops(&mut next);
    layout::save(&project_dir, &next)?;
    get(session, project_id)
}

/// Applies a partial change, for agents: only what `patch` names changes.
pub(crate) fn patch(
    session: &Session,
    project_id: &str,
    patch: &LayoutPatch,
) -> CommandResult<LayoutView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    let project_dir = project::dir(&root, &config.layout, project_id)?;
    let current = layout::load(&project_dir)?;
    let next = layout::apply(&current, patch, &layout::known_assets(&manifest))?;
    let changed = next
        .states
        .iter()
        .filter(|(asset, _)| patch.states.contains_key(*asset))
        .map(|(asset, set)| (asset.clone(), set.clone()))
        .collect();
    maquette_core::motion::check_parts(&project_dir, &manifest, &changed)?;
    layout::save(&project_dir, &next)?;
    get(session, project_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::layout::Placement;
    use maquette_core::{
        AppState, AssetKind, NewAsset, NewProject, ProjectTemplate, Stage, WorkspaceConfig, assets,
        workspace,
    };

    #[test]
    fn a_saved_layout_comes_back_and_a_patch_changes_only_what_it_names() {
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

        let empty = get(&session, &manifest.id).expect("get");
        assert!(!empty.saved);
        assert_eq!(empty.models.len(), 1);
        assert!((empty.first_view.target_mb - 3.0).abs() < f64::EPSILON);

        let mut next = SceneLayout::default();
        next.models.insert(
            "lamp".into(),
            Placement {
                position: [0.5, 0.0, 0.0],
                ..Placement::default()
            },
        );
        let saved = save(&session, &manifest.id, &next).expect("save");
        assert!(saved.saved);
        assert_eq!(saved.layout.models["lamp"].position, [0.5, 0.0, 0.0]);

        let change: LayoutPatch =
            serde_json::from_value(serde_json::json!({ "models": { "lamp": { "scale": 2.0 } } }))
                .expect("patch");
        let patched = patch(&session, &manifest.id, &change).expect("patch");
        assert_eq!(patched.layout.models["lamp"].position, [0.5, 0.0, 0.0]);
        assert!((patched.layout.models["lamp"].scale - 2.0).abs() < f32::EPSILON);

        let unknown: LayoutPatch =
            serde_json::from_value(serde_json::json!({ "models": { "nope": {} } })).expect("patch");
        assert_eq!(
            patch(&session, &manifest.id, &unknown)
                .expect_err("unknown")
                .code,
            "invalid"
        );

        // The Layout editor's save keeps the states the Motion view wrote.
        let with_states: LayoutPatch = serde_json::from_value(serde_json::json!({
            "states": { "lamp": { "states": [{ "id": "off", "name": "Off", "parts": {} }],
                "transitions": [], "triggers": [] } },
            "clips": { "lamp": { "Spin": { "ship": false } } }
        }))
        .expect("patch");
        patch(&session, &manifest.id, &with_states).expect("states");
        let kept = save(&session, &manifest.id, &SceneLayout::default()).expect("save");
        assert_eq!(kept.layout.states["lamp"].states[0].name, "Off");
        assert!(!kept.layout.clips["lamp"]["Spin"].ship);
        assert!(kept.layout.models.is_empty());
    }
}
