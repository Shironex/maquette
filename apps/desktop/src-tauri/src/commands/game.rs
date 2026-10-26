//! Game projects: read a game repo, scan and import its models, change the
//! game settings and see what Ship would change in the game.

use std::path::Path;
use std::sync::Arc;

use maquette_core::game::{
    self, GameRepoInspection, GameScan, GameSettingsChange, ImportResult, ShipPlan,
};
use maquette_core::pipeline::runner::StepGate;
use maquette_core::{CoreError, ProjectManifest, project};
use tauri::State;

use super::CommandResult;
use crate::mcp::gate::{ShipGate, ShipReason};
use crate::mcp::hub::Hub;
use crate::session::Session;

/// The project and its folder, refused for a website.
fn game_project(
    session: &Session,
    project_id: &str,
) -> CommandResult<(ProjectManifest, std::path::PathBuf)> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    game::settings_of(&manifest)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    Ok((manifest, dir))
}

/// The game repo a game project ships to.
fn game_repo(manifest: &ProjectManifest) -> CommandResult<&Path> {
    manifest.site_repo.as_deref().map(Path::new).ok_or_else(|| {
        CoreError::Invalid(format!(
            "{} has no game repo. Pick one in Settings.",
            manifest.name
        ))
        .payload()
    })
}

/// What a game run needs before it starts: the game index Build models and
/// Game export read. Nothing for a website.
pub(crate) fn before_run(project_dir: &Path, manifest: &ProjectManifest) -> CommandResult<()> {
    if game::settings_of(manifest).is_ok() {
        game::index::write(project_dir, manifest)?;
    }
    Ok(())
}

/// The gate a game project's Ship always passes: it writes into the game
/// repo, so a person approves it in the app, whoever started the run. None
/// for a website.
pub(crate) fn ship_gate(
    session: &Session,
    project_id: &str,
    hub: &Arc<Hub>,
) -> Option<Arc<dyn StepGate>> {
    game_project(session, project_id).ok()?;
    Some(Arc::new(ShipGate {
        hub: Arc::clone(hub),
        reason: ShipReason::Game,
    }))
}

pub(crate) fn scan(session: &Session, project_id: &str) -> CommandResult<GameScan> {
    let (manifest, _) = game_project(session, project_id)?;
    let settings = game::settings_of(&manifest)?;
    Ok(game::scan_models(game_repo(&manifest)?, settings)?)
}

pub(crate) fn import(
    session: &Session,
    project_id: &str,
    files: &[String],
) -> CommandResult<ImportResult> {
    let (root, config) = session.workspace()?;
    let result = game::import_models(&root, &config.layout, project_id, files)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    maquette_core::pipeline::ensure(&dir, &result.project)?;
    Ok(result)
}

pub(crate) fn save_settings(
    session: &Session,
    project_id: &str,
    change: GameSettingsChange,
) -> CommandResult<ProjectManifest> {
    let (root, config) = session.workspace()?;
    Ok(game::save_settings(
        &root,
        &config.layout,
        project_id,
        change,
    )?)
}

pub(crate) fn add_budget_fixups(session: &Session, project_id: &str) -> CommandResult<Vec<String>> {
    let (manifest, dir) = game_project(session, project_id)?;
    Ok(game::add_budget_fixups(&dir, &manifest)?)
}

pub(crate) fn ship_plan(session: &Session, project_id: &str) -> CommandResult<ShipPlan> {
    let (manifest, dir) = game_project(session, project_id)?;
    Ok(game::ship_plan(&dir, &manifest)?)
}

/// What New project shows for a game repo: CMake, the model folder, the
/// format and the loader.
#[tauri::command]
#[specta::specta]
pub fn game_inspect_repo(session: State<'_, Session>, path: String) -> GameRepoInspection {
    game::inspect_game_repo(&session.expand(&path))
}

/// Every model in the game's model folder, with its numbers and budget.
#[tauri::command]
#[specta::specta]
pub async fn game_scan(session: State<'_, Session>, project: String) -> CommandResult<GameScan> {
    scan(&session, &project)
}

/// Imports the picked model files (relative to the model folder), each as
/// an asset whose v1 is the repo file.
#[tauri::command]
#[specta::specta]
pub async fn game_import(
    session: State<'_, Session>,
    project: String,
    files: Vec<String>,
) -> CommandResult<ImportResult> {
    import(&session, &project, &files)
}

#[tauri::command]
#[specta::specta]
pub fn game_save_settings(
    session: State<'_, Session>,
    project: String,
    change: GameSettingsChange,
) -> CommandResult<ProjectManifest> {
    save_settings(&session, &project, change)
}

/// Adds Decimate and Shrink textures to every model over its folder's
/// budget. Returns the assets that changed.
#[tauri::command]
#[specta::specta]
pub fn game_add_budget_fixups(
    session: State<'_, Session>,
    project: String,
) -> CommandResult<Vec<String>> {
    add_budget_fixups(&session, &project)
}

/// What Ship would write into the game repo from the last Game export.
#[tauri::command]
#[specta::specta]
pub fn game_ship_plan(session: State<'_, Session>, project: String) -> CommandResult<ShipPlan> {
    ship_plan(&session, &project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::game::{GameFormat, NewGame, ProjectTarget};
    use maquette_core::{AppState, NewProject, ProjectTemplate, WorkspaceConfig, workspace};

    #[test]
    fn a_game_ship_always_waits_and_a_website_ship_does_not() {
        let dir = tempfile::tempdir().expect("temp dir");
        let repo = dir.path().join("game");
        std::fs::create_dir_all(repo.join("assets/models/props")).expect("models");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let make = |name: &str, target, game| {
            project::create(
                &root,
                &config.layout,
                NewProject {
                    name: name.into(),
                    template: ProjectTemplate::FloatingDiorama,
                    target,
                    game,
                    site_repo: Some(repo.display().to_string()),
                    source_folder: None,
                },
            )
            .expect("project")
        };
        let game = make(
            "Crypt game",
            ProjectTarget::Game,
            Some(NewGame {
                format: GameFormat::Glb,
                models_dir: "assets/models".into(),
            }),
        );
        let site = make("Moon Island", ProjectTarget::Website, None);
        let hub = Arc::new(Hub::new(|| {}));

        let gate = ship_gate(&session, &game.id, &hub).expect("a game gates Ship");
        let dir = project::dir(&root, &config.layout, &game.id).expect("dir");
        let def = maquette_core::pipeline::ensure(&dir, &game).expect("pipeline");
        let ship = def
            .steps
            .iter()
            .find(|s| s.id == "ship")
            .expect("ship step");
        assert!(gate.applies(ship));
        assert!(gate.waiting_message(ship).contains("game repo"));
        assert!(ship_gate(&session, &site.id, &hub).is_none());
        assert!(
            scan(&session, &site.id).is_err(),
            "a website has no game scan"
        );
    }
}
