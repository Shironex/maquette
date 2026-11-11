//! How a website project ships: the style, texture format and per asset
//! choices in `ship.json`, the site's folders in its `maquette.site.json`,
//! the dry run plan of the next Ship and the size report.

use std::path::Path;

use maquette_core::app_state::RepoSetBy;
use maquette_core::fs::real_folder;
use maquette_core::pipeline::{self, runner::PipelineRunner};
use maquette_core::ship::{self, ShipSettings, ShipSettingsChange, SiteShipPlan, SizeReport};
use maquette_core::site::{self, SiteTarget};
use maquette_core::{ProjectTarget, project};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::site::gate::{folders_set_by_agent, mark_folders};
use super::{CommandResult, invalid};
use crate::session::Session;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShipView {
    pub project: String,
    pub settings: ShipSettings,
    pub plan: SiteShipPlan,
    pub report: SizeReport,
}

/// A save from the Ship view or an agent: settings, and the site's folders
/// when they change.
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShipSave {
    #[serde(default)]
    pub settings: ShipSettingsChange,
    /// Written into the site's `maquette.site.json`.
    #[serde(default)]
    pub target: Option<SiteTarget>,
}

pub(crate) fn view(session: &Session, project_id: &str) -> CommandResult<ShipView> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    if manifest.target == ProjectTarget::Game {
        return Err(invalid(
            "A game project ships through Game export. Use get_ship_plan or the Ship view.",
        ));
    }
    let dir = project::dir(&root, &config.layout, project_id)?;
    let def = pipeline::ensure(&dir, &manifest)?;
    let mut plan = ship::plan(&dir, &manifest, &def)?;
    if let Some(repo) = &plan.target.repo {
        plan.target.repo = Some(session.display(Path::new(repo)));
    }
    plan.target.set_by_agent = manifest
        .site_repo
        .as_deref()
        .is_some_and(|repo| folders_set_by_agent(session, Path::new(repo)));
    Ok(ShipView {
        project: manifest.id.clone(),
        settings: ship::load_or_default(&dir),
        report: ship::size_report(&dir, &manifest, &def),
        plan,
    })
}

/// Checks a save against `repo`, writing nothing, so a save cannot stop
/// half way and leave a file behind. The folders cannot change while any
/// run, of any project, ships into the same repo ([`refuse_while_shipping`]).
pub(crate) fn check_save(
    session: &Session,
    runner: &PipelineRunner,
    project_id: &str,
    change: &ShipSave,
    repo: Option<&str>,
) -> CommandResult<()> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    if manifest.target == ProjectTarget::Game {
        return Err(invalid("A game project ships through Game export."));
    }
    let dir = project::dir(&root, &config.layout, project_id)?;
    for asset in change.settings.assets.keys() {
        if !manifest.assets.iter().any(|a| &a.id == asset) {
            return Err(invalid(&format!(
                "{asset} is not an asset of {}.",
                manifest.name
            )));
        }
    }
    ship::check_change(&dir, change.settings.clone())?;
    if let Some(target) = &change.target {
        let repo = repo.ok_or_else(|| {
            invalid("This project has no site repo yet. Pick one in Settings > Project.")
        })?;
        site::check_target(Path::new(repo), target)?;
        refuse_while_shipping(runner, Path::new(repo))?;
    }
    Ok(())
}

/// Refuses a folder change while a run going or queued, in any project,
/// ships into the same real folder: Ship reads the folders from the site's
/// `maquette.site.json` when it gets there, so a change would move a run a
/// card already named, or one that never asks, somewhere else.
fn refuse_while_shipping(runner: &PipelineRunner, repo: &Path) -> CommandResult<()> {
    let real = real_folder(repo);
    let busy = runner.any_run_with("siteRepo", |to| {
        !to.is_empty() && real_folder(Path::new(to)) == real
    });
    if busy {
        return Err(invalid(&format!(
            "A run is going that ships into {}. Wait for it or cancel it, then change the site's folders.",
            repo.display()
        )));
    }
    Ok(())
}

/// Refuses an agent's data file that names a file already in the site other
/// than the site's own data file: Ship replaces the data file whole, and the
/// only file it is known to write there is that one. A person can still pick
/// such a file in the Ship view.
pub(crate) fn refuse_foreign_data(repo: &Path, target: &SiteTarget) -> CommandResult<()> {
    let clean = |p: &str| p.trim().trim_matches('/').replace('\\', "/");
    let Some(data) = target.data.as_deref().map(clean).filter(|d| !d.is_empty()) else {
        return Ok(());
    };
    let current = site::load_config(repo).ok().and_then(|c| c.data);
    if current.as_deref().map(clean).as_deref() == Some(data.as_str()) {
        return Ok(());
    }
    if repo.join(&data).symlink_metadata().is_ok() {
        return Err(invalid(&format!(
            "{data} is already in the site and is not its data file, so Ship would replace it. \
             Name a new file, or a person picks this one in the Ship view."
        )));
    }
    Ok(())
}

/// Saves `change`. Folders `by` an agent mark their repo until a person saves
/// them: an agent's run into that repo, from any project, waits for a person
/// before Ship (`pipeline::agent_run`).
pub(crate) fn save(
    session: &Session,
    runner: &PipelineRunner,
    project_id: &str,
    change: ShipSave,
    by: RepoSetBy,
) -> CommandResult<ShipView> {
    let (root, config) = session.workspace()?;
    // The turn a run's gate reads the folders' mark in.
    let _turn = session.repo_turn();
    let manifest = project::load(&root, &config.layout, project_id)?;
    let repo = manifest.site_repo.as_deref();
    check_save(session, runner, project_id, &change, repo)?;
    let dir = project::dir(&root, &config.layout, project_id)?;
    if let (Some(target), Some(repo)) = (&change.target, repo) {
        // An agent's mark goes down before its folders do, so a failure in
        // between asks; a person's comes off once theirs are written.
        // The mark is the repo's: every project on it reads these folders.
        if by == RepoSetBy::Agent {
            refuse_foreign_data(Path::new(repo), target)?;
            mark_folders(session, Path::new(repo), by)?;
        }
        site::save_target(Path::new(repo), target)?;
        if by == RepoSetBy::Person {
            mark_folders(session, Path::new(repo), by)?;
        }
    }
    ship::update(&dir, change.settings)?;
    view(session, project_id)
}

/// The ship settings, the plan of the next Ship and the size report.
#[tauri::command(async)]
#[specta::specta]
pub fn ship_get(session: State<'_, Session>, project: String) -> CommandResult<ShipView> {
    view(&session, &project)
}

/// Saves the ship settings and, with a target, the site's folders: a
/// person's, so the folders lose an agent's mark.
#[tauri::command(async)]
#[specta::specta]
pub fn ship_save(
    session: State<'_, Session>,
    pipelines: State<'_, super::pipeline::Pipelines>,
    project: String,
    change: ShipSave,
) -> CommandResult<ShipView> {
    save(&session, &pipelines.0, &project, change, RepoSetBy::Person)
}
