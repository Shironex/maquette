//! The asset registry on the Board: add an asset, move it between stages.

use maquette_core::{NewAsset, ProjectManifest, Stage, assets, project};
use tauri::State;

use super::CommandResult;
use super::blender;
use crate::session::Session;

#[tauri::command]
#[specta::specta]
pub fn asset_add(
    session: State<'_, Session>,
    project: String,
    asset: NewAsset,
) -> CommandResult<ProjectManifest> {
    add(&session, &project, asset)
}

pub(crate) fn add(
    session: &Session,
    project_id: &str,
    asset: NewAsset,
) -> CommandResult<ProjectManifest> {
    let (root, config) = session.workspace()?;
    Ok(project::modify(&root, &config.layout, project_id, |m| {
        assets::add(m, asset).map(|_| ())
    })?)
}

#[tauri::command]
#[specta::specta]
pub fn asset_move(
    session: State<'_, Session>,
    project: String,
    id: String,
    stage: Stage,
) -> CommandResult<ProjectManifest> {
    move_to(&session, &project, &id, stage)
}

pub(crate) fn move_to(
    session: &Session,
    project_id: &str,
    id: &str,
    stage: Stage,
) -> CommandResult<ProjectManifest> {
    let (root, config) = session.workspace()?;
    Ok(project::modify(&root, &config.layout, project_id, |m| {
        assets::move_to(m, id, stage).map(|_| ())
    })?)
}

#[tauri::command]
#[specta::specta]
pub fn asset_rename(
    session: State<'_, Session>,
    project: String,
    id: String,
    name: String,
) -> CommandResult<ProjectManifest> {
    rename(&session, &project, &id, &name)
}

pub(crate) fn rename(
    session: &Session,
    project_id: &str,
    id: &str,
    name: &str,
) -> CommandResult<ProjectManifest> {
    let (root, config) = session.workspace()?;
    Ok(project::modify(&root, &config.layout, project_id, |m| {
        assets::rename(m, id, name).map(|_| ())
    })?)
}

/// Opens the asset's latest file in Blender's window. Only ever from a
/// click on Open in Blender.
#[tauri::command]
#[specta::specta]
pub async fn asset_open_in_blender(
    session: State<'_, Session>,
    project: String,
    id: String,
) -> CommandResult<()> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, &project)?;
    let asset = assets::find(&manifest, &id)?;
    let latest = asset.latest().ok_or_else(|| {
        super::invalid("This asset has no file yet. Accept a model from the inbox first.")
    })?;
    let file = project::dir(&root, &config.layout, &project)?.join(&latest.file);
    blender::open_gui(&session, &file).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::{
        AppState, AssetKind, NewProject, ProjectTemplate, WorkspaceConfig, workspace,
    };

    #[test]
    fn add_and_move_round_trip_through_project_json() {
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
        let added = add(
            &session,
            &manifest.id,
            NewAsset {
                name: "Stone lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Concept,
            },
        )
        .expect("add");
        assert_eq!(added.assets[0].id, "stone-lantern");
        let moved = move_to(&session, &manifest.id, "stone-lantern", Stage::Tripo).expect("move");
        assert_eq!(moved.assets[0].stage, Stage::Tripo);
        assert_eq!(
            move_to(&session, &manifest.id, "nope", Stage::Web)
                .expect_err("missing")
                .code,
            "not-found"
        );
        let renamed =
            rename(&session, &manifest.id, "stone-lantern", "Mossy lantern").expect("rename");
        assert_eq!(renamed.assets[0].id, "stone-lantern");
        assert_eq!(renamed.assets[0].name, "Mossy lantern");
        let saved = project::load(&root, &config.layout, &manifest.id).expect("load");
        assert_eq!(saved.assets[0].name, "Mossy lantern");
        assert!(rename(&session, &manifest.id, "stone-lantern", " ").is_err());
    }
}
