//! A project's canvas files: listing, making, reading, saving, renaming and
//! removing them. Writes in this process go one at a time, and each one
//! reads the file on disk under the same lock before writing, so a canvas
//! removed meanwhile (here or by a sync from another computer) is not
//! written back, and a rename changes only the name of what is there now.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;

use super::log::{self, PlanLogEntry};
use super::summary::ChangeMark;
use super::{
    MAX_FILE_BYTES, PLAN_SUFFIX, PlanFile, PlanPreset, change, check_name, dir, images, load_file,
    normalized, path, trimmed, validate,
};
use crate::error::{CoreError, CoreResult};
use crate::project::{latin_slug, now_rfc3339, windows_safe};

/// One canvas as the view gets it: its id, its file relative to the
/// project, the canvas, and where each of its pictures is on this computer.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanDoc {
    pub id: String,
    /// `plans/<id>.plan.json`.
    pub file: String,
    pub plan: PlanFile,
    /// A picture's stored path to its absolute path here.
    pub images: BTreeMap<String, String>,
}

/// A line in the list of a project's canvases.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub id: String,
    pub file: String,
    /// The canvas name, or the file's id when it could not be read.
    pub name: String,
    pub preset: Option<PlanPreset>,
    pub nodes: u32,
    pub edges: u32,
    pub images: u32,
    pub links: u32,
    /// Asset nodes, each linking an asset of the project.
    pub assets: u32,
    /// RFC 3339; empty when the file could not be read.
    pub edited: String,
    /// Why the file could not be read, for a canvas that only lists.
    pub problem: Option<String>,
}

static WRITES: Mutex<()> = Mutex::new(());

/// The lock every canvas write in this process holds.
pub(super) fn writing() -> MutexGuard<'static, ()> {
    WRITES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Writes a canvas file as `write_json` would, refused when the text would
/// be over [`MAX_FILE_BYTES`], since such a file would not open again.
pub(super) fn write_plan(file: &Path, plan: &PlanFile) -> CoreResult<()> {
    let mut text = serde_json::to_string_pretty(plan).map_err(|e| CoreError::json(file, e))?;
    text.push('\n');
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        return Err(CoreError::Invalid(format!(
            "The canvas would be over {} MB, too big to save. Move some notes to another canvas.",
            MAX_FILE_BYTES / (1024 * 1024)
        )));
    }
    maquette_base::fs::write_atomic(file, text.as_bytes()).map_err(|e| CoreError::io(file, e))
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn file_of(id: &str) -> String {
    format!("{}/{id}{PLAN_SUFFIX}", super::PLANS_DIR)
}

pub(super) fn doc(project_dir: &Path, id: &str, plan: PlanFile) -> PlanDoc {
    PlanDoc {
        id: id.to_string(),
        file: file_of(id),
        images: images::paths_of(project_dir, &plan),
        plan,
    }
}

fn summary(id: &str, plan: &PlanFile) -> PlanSummary {
    let of = |kind| count(plan.nodes.iter().filter(|n| n.kind == kind).count());
    PlanSummary {
        id: id.to_string(),
        file: file_of(id),
        name: plan.name.clone(),
        preset: Some(plan.preset),
        nodes: count(plan.nodes.len()),
        edges: count(plan.edges.len()),
        images: of(super::NodeKind::Image),
        links: of(super::NodeKind::Link),
        assets: of(super::NodeKind::Asset),
        edited: plan.edited.clone(),
        problem: None,
    }
}

/// Every canvas in the project, sorted by id. A file that cannot be read
/// still lists, with its problem, so it can be removed.
///
/// # Errors
///
/// Fails only when the `plans` folder exists but cannot be read.
pub fn list(project_dir: &Path) -> CoreResult<Vec<PlanSummary>> {
    let folder = dir(project_dir);
    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&folder, err)),
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(id) = name.strip_suffix(PLAN_SUFFIX) else {
            continue;
        };
        if !super::is_plan_id(id) || !entry.path().is_file() {
            continue;
        }
        found.push(match load_file(&entry.path()) {
            Ok(plan) => summary(id, &plan),
            Err(err) => PlanSummary {
                id: id.to_string(),
                file: file_of(id),
                name: id.to_string(),
                preset: None,
                nodes: 0,
                edges: 0,
                images: 0,
                links: 0,
                assets: 0,
                edited: String::new(),
                problem: Some(err.to_string()),
            },
        });
    }
    found.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(found)
}

/// The canvas `id`.
///
/// # Errors
///
/// Fails when it is missing or [`load_file`] refuses it.
pub fn get(project_dir: &Path, id: &str) -> CoreResult<PlanDoc> {
    let plan = load_file(&path(project_dir, id)?)?;
    Ok(doc(project_dir, id, plan))
}

/// A free id for a canvas called `name`: its slug, then `-2`, `-3` and on.
fn free_id(project_dir: &Path, name: &str) -> CoreResult<String> {
    let base = latin_slug(name).map_or_else(|| "canvas".to_string(), windows_safe);
    for n in 1..10_000 {
        let id = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        if !path(project_dir, &id)?.exists() {
            return Ok(id);
        }
    }
    Err(CoreError::Invalid(format!(
        "There are too many canvases called {name}."
    )))
}

/// Makes an empty canvas called `name` and writes its file.
///
/// # Errors
///
/// Fails on a name [`check_name`] refuses or when the file cannot be written.
pub fn create(project_dir: &Path, name: &str, preset: PlanPreset) -> CoreResult<PlanDoc> {
    create_by(project_dir, name, preset, None)
}

/// [`create`], noting who made it (an agent's name) in the change log.
///
/// # Errors
///
/// As [`create`].
pub fn create_by(
    project_dir: &Path,
    name: &str,
    preset: PlanPreset,
    by: Option<&str>,
) -> CoreResult<PlanDoc> {
    check_name(name)?;
    let _lock = writing();
    let id = free_id(project_dir, trimmed(name))?;
    let folder = dir(project_dir);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    let plan = PlanFile::new(name, preset);
    write_plan(&path(project_dir, &id)?, &plan)?;
    let line = plain_entry(
        by,
        format!("Made the canvas {}", plan.name),
        ChangeMark::Create,
    );
    change::note(project_dir, &id, &line);
    Ok(doc(project_dir, &id, plan))
}

/// A log line that changed no node: a canvas made or renamed.
fn plain_entry(by: Option<&str>, summary: String, mark: ChangeMark) -> PlanLogEntry {
    PlanLogEntry {
        id: log::new_id("c"),
        at: now_rfc3339(),
        by: by.map(str::to_string),
        run: None,
        summary,
        mark,
        ops: Vec::new(),
        undoes: Vec::new(),
        trimmed: false,
        keeps: Vec::new(),
    }
}

/// Writes `plan` over the canvas `id`, normalised and then validated.
/// `edited` moves only when something changed, so a canvas of an older
/// version opened and sent back unchanged keeps its file as it was; the name, `created` and the preset stay as the file
/// has them (only [`rename`] changes the name), so a save sent before a
/// rename never puts the old name back.
///
/// # Errors
///
/// Fails when the canvas is gone, the plan does not validate or the file
/// cannot be written.
pub fn save(project_dir: &Path, id: &str, plan: &PlanFile) -> CoreResult<PlanDoc> {
    validate(&normalized(plan))?;
    let _lock = writing();
    let current = load_file(&path(project_dir, id)?)?;
    let writer = change::Writer::PERSON;
    Ok(change::commit(project_dir, id, current, plan, &writer, None)?.0)
}

/// Renames the canvas `id`. The file and its id stay, so links to it from
/// elsewhere keep working.
///
/// # Errors
///
/// As [`save`], and on a name [`check_name`] refuses.
pub fn rename(project_dir: &Path, id: &str, name: &str) -> CoreResult<PlanSummary> {
    check_name(name)?;
    let _lock = writing();
    let file = path(project_dir, id)?;
    let mut plan = load_file(&file)?;
    let name = trimmed(name);
    if plan.name != name {
        plan.name = name.to_string();
        plan.edited = now_rfc3339();
        write_plan(&file, &plan)?;
        let line = plain_entry(None, format!("Renamed to {name}"), ChangeMark::Rename);
        change::note(project_dir, id, &line);
    }
    Ok(summary(id, &plan))
}

/// Moves the canvas file into `trash_dir` (the workspace's trash for this
/// project) as `<id>-<time>.plan.json`, so it can still be got back by
/// hand. Pictures copied into `plans/images` stay: another canvas may show
/// them.
///
/// # Errors
///
/// Fails when the canvas is missing or the file cannot be moved.
pub fn remove(project_dir: &Path, trash_dir: &Path, id: &str) -> CoreResult<String> {
    let _lock = writing();
    let file = path(project_dir, id)?;
    if !file.is_file() {
        return Err(CoreError::NotFound(format!("the canvas {id}")));
    }
    let to_dir = trash_dir.join(super::PLANS_DIR);
    fs::create_dir_all(&to_dir).map_err(|e| CoreError::io(&to_dir, e))?;
    let stamp: String = now_rfc3339().chars().filter(char::is_ascii_digit).collect();
    let to = to_dir.join(format!("{id}-{stamp}{PLAN_SUFFIX}"));
    // The log first: one left behind would pass to a new canvas with this id.
    let moved_log = log::move_beside(project_dir, id, &to)?;
    if let Err(err) = move_file(&file, &to) {
        let Some(log_to) = moved_log else {
            return Err(err);
        };
        let back = log::path(project_dir, id)?;
        if let Err(stuck) = maquette_base::fs::rename_retrying(&log_to, &back) {
            return Err(CoreError::Invalid(format!(
                "The canvas could not move to the trash ({err}), and its change log, already there, could not come back ({stuck}). Move {} back to {} by hand to keep its history.",
                log_to.display(),
                back.display()
            )));
        }
        return Err(err);
    }
    Ok(to.display().to_string())
}

/// A rename, or a copy and delete when the trash is on another volume.
fn move_file(from: &Path, to: &Path) -> CoreResult<()> {
    if maquette_base::fs::rename_retrying(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to).map_err(|e| CoreError::io(to, e))?;
    fs::remove_file(from).map_err(|e| CoreError::io(from, e))
}
