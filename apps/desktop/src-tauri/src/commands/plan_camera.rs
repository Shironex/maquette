//! The camera plan of a website project: opening it (made from
//! `layout.json` the first time), reviewing its main line against
//! `layout.json`, applying it, undoing an Apply and rebasing it on a
//! `layout.json` that changed underneath. Only these person commands apply
//! a plan; the MCP server has no route to it.

use std::path::PathBuf;

use maquette_core::plan_camera::{self, CameraView};
use maquette_core::plans::camera::{self, CAMERA_PLAN_ID, CameraReview, RebaseMode, RebaseReport};
use maquette_core::{ProjectTarget, project};
use tauri::{AppHandle, State};
use tauri_specta::Event as _;

use super::CommandResult;
use crate::events::{LayoutChanged, PlanChanged};
use crate::session::Session;

fn website_dir(session: &Session, project_id: &str) -> CommandResult<PathBuf> {
    let (root, config) = session.workspace()?;
    let manifest = project::load(&root, &config.layout, project_id)?;
    if manifest.target != ProjectTarget::Website {
        return Err(super::invalid("Only a website project has a camera plan."));
    }
    Ok(project::dir(&root, &config.layout, project_id)?)
}

/// Tells the open views that `layout.json` and the camera plan changed.
fn changed(app: &AppHandle, project: &str) {
    let layout = LayoutChanged {
        project: project.to_string(),
    };
    let plan = PlanChanged {
        project: project.to_string(),
        id: CAMERA_PLAN_ID.to_string(),
    };
    if layout.emit(app).is_err() || plan.emit(app).is_err() {
        tracing::warn!("could not tell the views about an Apply");
    }
}

/// The project's camera plan as the canvas shows it.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_camera(session: State<'_, Session>, project: String) -> CommandResult<CameraView> {
    Ok(plan_camera::view(&website_dir(&session, &project)?)?)
}

/// What Apply would write: the exact diff of the main line against
/// `layout.json` now, and the keys Apply takes back.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_camera_review(
    session: State<'_, Session>,
    project: String,
) -> CommandResult<CameraReview> {
    Ok(camera::review(
        &website_dir(&session, &project)?,
        CAMERA_PLAN_ID,
    )?)
}

/// Writes the reviewed main line to `layout.json`, as the person. Refused
/// when `layout.json` or the plan changed since the review was shown.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_camera_apply(
    app: AppHandle,
    session: State<'_, Session>,
    project: String,
    layout_key: String,
    plan_key: String,
) -> CommandResult<CameraView> {
    let dir = website_dir(&session, &project)?;
    camera::apply(&dir, CAMERA_PLAN_ID, &layout_key, &plan_key)?;
    changed(&app, &project);
    Ok(plan_camera::view(&dir)?)
}

/// Undoes the Apply logged as `change`: puts the old `layout.json` back,
/// refused when `layout.json` changed since that Apply.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_camera_undo_apply(
    app: AppHandle,
    session: State<'_, Session>,
    project: String,
    change: String,
) -> CommandResult<CameraView> {
    let dir = website_dir(&session, &project)?;
    camera::undo_apply(&dir, CAMERA_PLAN_ID, &change)?;
    changed(&app, &project);
    Ok(plan_camera::view(&dir)?)
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CameraRebased {
    pub view: CameraView,
    pub report: RebaseReport,
}

/// Brings the plan up to `layout.json` as it is now (or starts its main
/// line over from it). With `dry` nothing is written: the report says what
/// a rebase would take, keep and could not settle.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_camera_rebase(
    session: State<'_, Session>,
    project: String,
    mode: RebaseMode,
    dry: bool,
) -> CommandResult<CameraRebased> {
    let dir = website_dir(&session, &project)?;
    let (_, report) = camera::rebase(&dir, CAMERA_PLAN_ID, mode, dry)?;
    Ok(CameraRebased {
        view: plan_camera::view(&dir)?,
        report,
    })
}
