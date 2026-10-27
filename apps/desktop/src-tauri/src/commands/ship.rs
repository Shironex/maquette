//! How a website project ships: the style, texture format and per asset
//! choices in `ship.json`, the site's folders in its `maquette.site.json`,
//! the dry run plan of the next Ship and the size report.

use std::path::Path;

use maquette_core::pipeline;
use maquette_core::ship::{self, ShipSettings, ShipSettingsChange, SiteShipPlan, SizeReport};
use maquette_core::site::{self, SiteTarget};
use maquette_core::{ProjectTarget, project};
use serde::{Deserialize, Serialize};
use tauri::State;

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
    Ok(ShipView {
        project: manifest.id.clone(),
        settings: ship::load_or_default(&dir),
        report: ship::size_report(&dir, &manifest, &def),
        plan,
    })
}

pub(crate) fn save(
    session: &Session,
    project_id: &str,
    change: ShipSave,
) -> CommandResult<ShipView> {
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
    if let Some(target) = &change.target {
        let repo = manifest.site_repo.as_deref().ok_or_else(|| {
            invalid("This project has no site repo yet. Pick one in Settings > Project.")
        })?;
        site::save_target(Path::new(repo), target)?;
    }
    ship::update(&dir, change.settings)?;
    view(session, project_id)
}

/// The ship settings, the plan of the next Ship and the size report.
#[tauri::command]
#[specta::specta]
pub fn ship_get(session: State<'_, Session>, project: String) -> CommandResult<ShipView> {
    view(&session, &project)
}

/// Saves the ship settings and, with a target, the site's folders.
#[tauri::command]
#[specta::specta]
pub fn ship_save(
    session: State<'_, Session>,
    project: String,
    change: ShipSave,
) -> CommandResult<ShipView> {
    save(&session, &project, change)
}
