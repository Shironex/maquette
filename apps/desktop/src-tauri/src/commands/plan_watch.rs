//! Watching a project's `plans/` folder while a canvas of it is open, so a
//! canvas file or change log changed from outside (another computer through
//! Syncthing, an agent, another window) reloads in the view. One watcher per
//! folder, shared by every window; the few newest stay, older ones stop.
//! The view itself tells its own writes apart (the file then holds what it
//! sent), so they never reload in a loop.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use maquette_core::inbox::watch::{self, InboxWatcher};
use maquette_core::plans::{self, PLAN_SUFFIX, log::LOG_SUFFIX};
use tauri::{AppHandle, State};
use tauri_specta::Event as _;

use super::CommandResult;
use crate::events::PlanChanged;
use crate::session::Session;

/// How long a canvas file must stay quiet before it reloads. A sync tool
/// writes in a burst; the app's own atomic write is one rename.
const QUIET: Duration = Duration::from_millis(250);
/// The most folders watched at once.
const MAX_FOLDERS: usize = 4;

/// The watched `plans/` folders, newest last.
#[derive(Default)]
pub struct PlanWatch(Mutex<Vec<(PathBuf, InboxWatcher)>>);

/// The canvas id a changed path in `plans/` belongs to, when it is a
/// canvas file or a change log.
pub fn canvas_of(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let id = name
        .strip_suffix(PLAN_SUFFIX)
        .or_else(|| name.strip_suffix(LOG_SUFFIX))?;
    plans::is_plan_id(id).then(|| id.to_string())
}

/// Watches the project's `plans/` folder, if it is not watched yet, and
/// sends `PlanChanged` for each canvas whose file or log changes. Answers
/// false when the project has no `plans/` folder yet (no canvas): the
/// view asks again once it has one, so watching never makes the folder.
#[tauri::command(async)]
#[specta::specta]
pub fn plan_watch(
    app: AppHandle,
    session: State<'_, Session>,
    watching: State<'_, PlanWatch>,
    project: String,
) -> CommandResult<bool> {
    let (root, config) = session.workspace()?;
    maquette_core::project::load(&root, &config.layout, &project)?;
    let dir = plans::dir(&maquette_core::project::dir(
        &root,
        &config.layout,
        &project,
    )?);
    if !dir.is_dir() {
        return Ok(false);
    }
    let mut folders = watching
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(at) = folders.iter().position(|(d, _)| *d == dir) {
        let kept = folders.remove(at);
        folders.push(kept);
        return Ok(true);
    }
    let watcher = watch::watch_where(
        &dir,
        QUIET,
        "plans",
        |p| canvas_of(p).is_some(),
        move |paths| {
            // A canvas file and its log changing together reload once.
            let ids: BTreeSet<String> = paths.iter().filter_map(|p| canvas_of(p)).collect();
            for id in ids {
                let event = PlanChanged {
                    project: project.clone(),
                    id,
                };
                if let Err(err) = event.emit(&app) {
                    tracing::warn!(error = %err, "could not emit a canvas event");
                }
            }
        },
    )?;
    folders.push((dir, watcher));
    if folders.len() > MAX_FOLDERS {
        folders.remove(0);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_canvas_files_and_logs_count() {
        let id = |p: &str| canvas_of(Path::new(p));
        assert_eq!(
            id("/p/plans/hero-ideas.plan.json").as_deref(),
            Some("hero-ideas")
        );
        assert_eq!(id("/p/plans/hero.log.jsonl").as_deref(), Some("hero"));
        assert_eq!(id("/p/plans/.hero.plan.json.tmp"), None);
        assert_eq!(id("/p/plans/images/a.png"), None);
        assert_eq!(id("/p/plans/Bad Name.plan.json"), None);
    }
}
