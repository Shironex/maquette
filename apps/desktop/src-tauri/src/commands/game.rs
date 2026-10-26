//! Game projects: read a game repo, scan and import its models, change the
//! game settings and see what Ship would change in the game.

use std::path::Path;
use std::sync::Arc;

use maquette_core::game::{
    self, GameRepoInspection, GameScan, GameSettingsChange, ImportResult, ShipPlan,
};
use maquette_core::pipeline::runner::StepGate;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::thumbs::Thumbnails;
use maquette_core::toolchain::find_on_path;
use maquette_core::{CoreError, ProbeEnv, ProjectManifest, SystemRunner, project};
use tauri::State;

use super::thumbs::{self, Thumbs};
use super::{CommandResult, toolchain};
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

/// Imports the picked files, then asks for a preview of each new model, so
/// its Board card shows a render once Blender is done.
pub(crate) fn import(
    session: &Session,
    project_id: &str,
    files: &[String],
    service: &Thumbnails,
    tools: &Toolbox,
) -> CommandResult<ImportResult> {
    let (root, config) = session.workspace()?;
    let mut result = game::import_models(&root, &config.layout, project_id, files)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    maquette_core::pipeline::ensure(&dir, &result.project)?;
    if !result.imported.is_empty() {
        let only = result.imported.clone();
        match thumbs::render_missing(service, &root, &config, tools, project_id, Some(&only)) {
            Ok(asked) => result.project = asked.project,
            Err(err) => tracing::warn!(error = %err.message, "no previews for the imported models"),
        }
    }
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

/// How many files `git status` lists as changed in the game repo, when git
/// is installed. Reads only.
fn uncommitted(repo: &Path) -> Option<u32> {
    let env = ProbeEnv::from_system();
    let git = find_on_path(&["git"], &env)?;
    game::detect::uncommitted(repo, &git, &SystemRunner, &env.child_path(None))
}

pub(crate) fn ship_plan(session: &Session, project_id: &str) -> CommandResult<ShipPlan> {
    let (manifest, dir) = game_project(session, project_id)?;
    let mut plan = game::ship_plan(&dir, &manifest)?;
    plan.uncommitted = manifest
        .site_repo
        .as_deref()
        .and_then(|r| uncommitted(Path::new(r)));
    Ok(plan)
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
    service: State<'_, Thumbs>,
    project: String,
    files: Vec<String>,
) -> CommandResult<ImportResult> {
    let tools = toolchain::toolbox(&session).await?;
    import(&session, &project, &files, &service.0, &tools)
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
/// budget; a rigged model gets no Decimate. Returns the assets that changed.
#[tauri::command]
#[specta::specta]
pub fn game_add_budget_fixups(
    session: State<'_, Session>,
    project: String,
) -> CommandResult<Vec<String>> {
    add_budget_fixups(&session, &project)
}

/// What Ship would write into the game repo from the last Game export,
/// and how many files the game repo has not committed yet.
#[tauri::command]
#[specta::specta]
pub async fn game_ship_plan(
    session: State<'_, Session>,
    project: String,
) -> CommandResult<ShipPlan> {
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

    /// A Toolbox whose Blender writes the PNG it is asked for.
    #[cfg(unix)]
    fn fake_blender(dir: &Path) -> Toolbox {
        use maquette_core::ToolId;
        use maquette_core::pipeline::tools::ResolvedTool;
        use std::os::unix::fs::PermissionsExt;
        let fake = dir.join("blender");
        std::fs::write(
            &fake,
            "#!/bin/sh\nwhile [ \"$1\" != \"--out\" ]; do shift; done\nprintf '\\211PNG' > \"$2\"\n",
        )
        .expect("fake");
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let mut tools = Toolbox {
            path_env: std::env::var_os("PATH").unwrap_or_default(),
            os: Some(maquette_core::HostOs::current()),
            ..Toolbox::default()
        };
        tools.tools.insert(
            ToolId::Blender,
            ResolvedTool {
                program: fake,
                prefix: Vec::new(),
                version: "test".into(),
            },
        );
        tools
    }

    #[cfg(unix)]
    #[test]
    fn imported_models_get_a_preview_and_render_previews_skips_the_ones_that_have_one() {
        use std::sync::{Mutex, mpsc};
        let dir = tempfile::tempdir().expect("temp dir");
        let repo = dir.path().join("game");
        let props = repo.join("assets/models/props");
        std::fs::create_dir_all(&props).expect("models");
        std::fs::write(
            props.join("brazier.glb"),
            maquette_core::inbox::fixture::clean_prop(),
        )
        .expect("brazier");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let session = Session::with_state(dir.path().join("cfg"), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let game = project::create(
            &root,
            &config.layout,
            NewProject {
                name: "Crypt game".into(),
                template: ProjectTemplate::FloatingDiorama,
                target: ProjectTarget::Game,
                game: Some(NewGame {
                    format: GameFormat::Glb,
                    models_dir: "assets/models".into(),
                }),
                site_repo: Some(repo.display().to_string()),
                source_folder: None,
            },
        )
        .expect("project");
        let tools = fake_blender(dir.path());
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let service = Thumbnails::new(move |done| {
            let _ = tx.lock().expect("tx").send(done);
        });

        let result = import(
            &session,
            &game.id,
            &["props/brazier.glb".to_string()],
            &service,
            &tools,
        )
        .expect("import");
        assert_eq!(result.imported, ["brazier"]);
        let done = rx
            .recv_timeout(std::time::Duration::from_secs(20))
            .expect("the render finished");
        service.wait();
        assert_eq!(done.error, None);
        let target = done.targets.first().expect("the new version").clone();
        assert_eq!((target.asset.as_str(), target.version), ("brazier", 1));
        let applied =
            maquette_core::thumbs::apply(&root, &config, &target, &done.png.expect("png"))
                .expect("apply")
                .expect("changed");
        assert_eq!(
            applied.assets[0].thumbnail.as_deref(),
            Some("thumbs/brazier_v1.png")
        );

        let again = thumbs::render_missing(&service, &root, &config, &tools, &game.id, None)
            .expect("render previews");
        assert_eq!(again.asked, 0, "the brazier has its preview");
    }
}
