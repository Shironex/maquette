//! The asset registry on the Board: add an asset, move it between stages.

use maquette_core::assets::copy::{self, CopiedAsset};
use maquette_core::providers::ProviderJobs;
use maquette_core::{NewAsset, ProjectManifest, Stage, assets, project};
use tauri::State;

use super::CommandResult;
use super::blender;
use super::providers::Studio;
use crate::session::Session;

#[tauri::command(async)]
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

#[tauri::command(async)]
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

#[tauri::command(async)]
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

/// Copies an asset with its refs, versions, thumbnails and finished sends
/// to another project. The asset stays where it is.
#[tauri::command(async)]
#[specta::specta]
pub fn asset_copy_to_project(
    session: State<'_, Session>,
    studio: State<'_, Studio>,
    project: String,
    id: String,
    to: String,
) -> CommandResult<CopiedAsset> {
    copy_to(&session, &studio.0, &project, &id, &to)
}

pub(crate) fn copy_to(
    session: &Session,
    jobs: &ProviderJobs,
    project_id: &str,
    id: &str,
    to: &str,
) -> CommandResult<CopiedAsset> {
    let (root, config) = session.workspace()?;
    let mut sent: Vec<String> = jobs
        .list(Some(project_id))
        .into_iter()
        .filter(|j| j.asset == id)
        .flat_map(|j| j.images)
        .collect();
    sent.dedup();
    let copied = copy::copy_to_project(&root, &config.layout, project_id, id, to, &sent)?;
    jobs.copy_records(project_id, id, to, &copied.asset, &copied.refs);
    Ok(copied)
}

/// Opens one of the asset's versions (the latest without `version`) in
/// Blender's window. Only ever from a click on Open in Blender.
#[tauri::command]
#[specta::specta]
pub async fn asset_open_in_blender(
    session: State<'_, Session>,
    project: String,
    id: String,
    version: Option<u32>,
) -> CommandResult<()> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, &project)?;
    let asset = assets::find(&manifest, &id)?;
    let chosen = match version {
        Some(v) => asset.versions.iter().find(|x| x.version == v),
        None => asset.latest(),
    };
    let chosen = chosen.ok_or_else(|| match version {
        Some(v) => super::invalid(&format!("{} has no v{v}.", asset.name)),
        None => super::invalid("This asset has no file yet. Accept a model from the inbox first."),
    })?;
    let file = project::dir(&root, &config.layout, &project)?.join(&chosen.file);
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
                target: Default::default(),
                game: None,
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
