//! Recipe bakes as jobs: one per recipe of a project, run on a background
//! thread, its record (`bake.json`) and staged model kept in its folder
//! under the workspace cache. Every change is saved and told to the app;
//! Cancel kills Blender part way; a bake the app closed on reads as stopped
//! when it opens again.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Instant;

use super::job::{self, PROGRESS_FILE, Prepared};
use super::result::{mark_for, read_run, result_of};
use super::{
    RecipeBakeProgress, RecipeBakeRecord, RecipeBakeRequest, RecipeBakeStarter, RecipeBakeState,
    STAGED_FILE, WORK_DIR,
};
use crate::base::ports::{CANCELLED, ScriptRunner, Watch};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::pipeline::history::now_ms;
use crate::pipeline::tools::Toolbox;
use crate::recipes::baked::write_mark;
use crate::workspace::{WorkspaceConfig, is_single_segment};

const RECORD_FILE: &str = "bake.json";
/// Each run's own files go in a folder of its own, `run-<id>`, so a Blender
/// left over from an earlier session never writes into a new run's folder.
/// Earlier runs' folders are cleared when a bake starts again.
const RUN_PREFIX: &str = "run-";
const CLOSED: &str = "The app closed during this bake. Bake it again.";
/// Why a bake whose thread panicked stopped.
pub(super) const BROKE: &str = "The bake stopped on an error inside Maquette. Bake it again.";

type OnChange = Box<dyn Fn(&RecipeBakeRecord) + Send + Sync>;

struct Inner {
    runner: Arc<dyn ScriptRunner + Send + Sync>,
    on_change: OnChange,
    active: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// Recipes being added as a version now: no bake starts meanwhile.
    adding: Mutex<HashSet<String>>,
}

/// The service. Cheap to clone.
#[derive(Clone)]
pub struct RecipeBakes {
    inner: Arc<Inner>,
}

pub(super) fn key(project: &str, recipe: &str) -> String {
    format!("{project}/{recipe}")
}

/// The folder of the bake of `recipe` in `project`.
pub(super) fn folder(
    root: &Path,
    config: &WorkspaceConfig,
    project: &str,
    recipe: &str,
) -> CoreResult<PathBuf> {
    if !is_single_segment(project) || !is_single_segment(recipe) {
        return Err(CoreError::Invalid(format!(
            "{project}/{recipe} is not a recipe."
        )));
    }
    Ok(root
        .join(&config.layout.cache)
        .join(WORK_DIR)
        .join(project)
        .join(recipe))
}

/// The record of the bake of `recipe` in `project` as it is on disk, with
/// whether it is out of date now, or none when it was never baked.
///
/// # Errors
///
/// Fails when the record cannot be read.
pub fn record_of(
    root: &Path,
    config: &WorkspaceConfig,
    project: &str,
    recipe: &str,
) -> CoreResult<Option<RecipeBakeRecord>> {
    let file = folder(root, config, project, recipe)?.join(RECORD_FILE);
    if !file.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    let mut record: RecipeBakeRecord =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    record.stale = super::stale::stale_of(root, config, &record);
    record.review = record.result.as_ref().map(super::review_of);
    Ok(Some(record))
}

pub(super) fn save(dir: &Path, record: &RecipeBakeRecord) -> CoreResult<()> {
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    write_json(&dir.join(RECORD_FILE), record)
}

/// A fresh run folder name, never the same twice in a session or across one.
fn run_name() -> String {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    format!(
        "{RUN_PREFIX}{}-{}",
        now_ms() as u64,
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// Every earlier run folder in a bake's folder.
fn clear_runs(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(RUN_PREFIX) {
            clear(&entry.path());
        }
    }
}

/// A file or folder from an earlier bake, gone before a new one starts.
pub(super) fn clear(path: &Path) {
    let gone = if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    if let Err(err) = gone
        && err.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(error = %err, "could not clear an earlier recipe bake");
    }
}

/// One running bake: its record, kept and told about on every change.
struct Running {
    record: Mutex<RecipeBakeRecord>,
    dir: PathBuf,
    /// This run's own folder in `dir`.
    work: PathBuf,
    inner: Arc<Inner>,
}

impl Running {
    fn update(&self, change: impl FnOnce(&mut RecipeBakeRecord)) {
        let mut record = self.record.lock().unwrap_or_else(PoisonError::into_inner);
        change(&mut record);
        record.updated_ms = now_ms();
        if let Err(err) = save(&self.dir, &record) {
            tracing::warn!(error = %err, "could not save a recipe bake");
        }
        (self.inner.on_change)(&record);
    }

    /// Reads the progress the script writes and tells the app when it moved.
    fn tick(&self, work: &Path) {
        let Ok(text) = fs::read_to_string(work.join(PROGRESS_FILE)) else {
            return;
        };
        let Ok(progress) = serde_json::from_str::<RecipeBakeProgress>(&text) else {
            return;
        };
        let same = self
            .record
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .progress
            .as_ref()
            == Some(&progress);
        if !same {
            self.update(|r| r.progress = Some(progress));
        }
    }
}

impl RecipeBakes {
    pub fn new(
        runner: Arc<dyn ScriptRunner + Send + Sync>,
        on_change: impl Fn(&RecipeBakeRecord) + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                runner,
                on_change: Box::new(on_change),
                active: Mutex::new(HashMap::new()),
                adding: Mutex::new(HashSet::new()),
            }),
        }
    }

    /// Tells the app about a changed record.
    pub(super) fn tell(&self, record: &RecipeBakeRecord) {
        (self.inner.on_change)(record);
    }

    /// The recipes being added as a version now.
    pub(super) fn adding(&self) -> MutexGuard<'_, HashSet<String>> {
        self.inner
            .adding
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn active(&self) -> MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.inner
            .active
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The bake of `recipe` in `project`, or none when it was never baked.
    /// A bake the app closed on reads as failed, saying so. Whether it runs
    /// is asked before and after the record is read, and it counts as
    /// closed only when neither saw it run: a bake saves its end before it
    /// leaves the running ones, so one that ends during the read is never
    /// taken for one the app closed on, and no lock is held over the read.
    ///
    /// # Errors
    ///
    /// Fails when the record cannot be read.
    pub fn get(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        project: &str,
        recipe: &str,
    ) -> CoreResult<Option<RecipeBakeRecord>> {
        let name = key(project, recipe);
        let ran_before = self.active().contains_key(&name);
        let Some(mut found) = record_of(root, config, project, recipe)? else {
            return Ok(None);
        };
        if found.running() && !ran_before && !self.active().contains_key(&name) {
            found.state = RecipeBakeState::Failed;
            found.progress = None;
            found.error = Some(CLOSED.to_string());
        }
        Ok(Some(found))
    }

    /// Checks the bake and starts it; answers at once with its record. The
    /// project is not changed: the model goes to the bake's folder.
    ///
    /// # Errors
    ///
    /// As [`job::prepare`], and when this recipe is baking already.
    pub fn start(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        tools: Toolbox,
        request: &RecipeBakeRequest,
        starter: RecipeBakeStarter,
    ) -> CoreResult<RecipeBakeRecord> {
        let dir = folder(root, config, &request.project, &request.recipe)?;
        let prepared = job::prepare(root, config, request)?;
        let cancel = Arc::new(AtomicBool::new(false));
        let name = key(&request.project, &request.recipe);
        {
            let mut active = self.active();
            if active.contains_key(&name) {
                return Err(CoreError::Invalid(format!(
                    "{} is baking already. Wait for it, or cancel it first.",
                    prepared.doc.recipe.name
                )));
            }
            if self.adding().contains(&name) {
                return Err(CoreError::Invalid(format!(
                    "{} is being added as a version now. Bake it again once that is done.",
                    prepared.doc.recipe.name
                )));
            }
            active.insert(name.clone(), Arc::clone(&cancel));
        }
        clear_runs(&dir);
        clear(&dir.join(STAGED_FILE));
        let now = now_ms();
        let record = RecipeBakeRecord {
            project: request.project.clone(),
            recipe: request.recipe.clone(),
            recipe_name: prepared.doc.recipe.name.clone(),
            asset: request.asset.clone(),
            state: RecipeBakeState::Running,
            started_by: starter,
            progress: None,
            error: None,
            started_ms: now,
            updated_ms: now,
            finished_ms: None,
            result: None,
            stale: None,
            review: None,
            added_version: None,
        };
        let running = Arc::new(Running {
            record: Mutex::new(record.clone()),
            work: dir.join(run_name()),
            dir,
            inner: Arc::clone(&self.inner),
        });
        running.update(|_| {});
        let bakes = self.clone();
        let root = root.to_path_buf();
        let spawned = thread::Builder::new()
            .name("recipe-bake".to_string())
            .spawn(move || {
                let mut ending = Ending {
                    running: Arc::clone(&running),
                    bakes: bakes.clone(),
                    name,
                    outcome: None,
                };
                ending.outcome = Some(bakes.bake(&root, &running, &prepared, &tools, &cancel));
            });
        if let Err(err) = spawned {
            self.active()
                .remove(&key(&request.project, &request.recipe));
            return Err(CoreError::Unavailable(format!(
                "Could not start the bake. {err}"
            )));
        }
        Ok(record)
    }

    fn bake(
        &self,
        root: &Path,
        running: &Running,
        prepared: &Prepared,
        tools: &Toolbox,
        cancel: &AtomicBool,
    ) -> CoreResult<super::RecipeBakeResult> {
        let started = Instant::now();
        let work = running.work.clone();
        fs::create_dir_all(&work).map_err(|e| CoreError::io(&work, e))?;
        running.update(|r| {
            r.progress = Some(RecipeBakeProgress {
                label: "Copying the model and the base clip".to_string(),
                done: 0,
                total: 1,
            });
        });
        job::stage(root, prepared, &work)?;
        let mut tick = || running.tick(&work);
        let mut watch = Watch {
            cancel,
            tick: &mut tick,
        };
        job::run(self.inner.runner.as_ref(), tools, &work, &mut watch)?;
        let run = read_run(&work)?;
        let result = result_of(
            &run,
            prepared,
            &running.dir,
            started.elapsed().as_secs_f32(),
        );
        // Marked in the run's folder, then moved in whole: the staged model
        // is never there unmarked or half written.
        write_mark(&work.join(STAGED_FILE), &mark_for(&result))?;
        let staged = running.dir.join(STAGED_FILE);
        fs::rename(work.join(STAGED_FILE), &staged).map_err(|e| CoreError::io(&staged, e))?;
        // The copies of the model and the clip are not needed any more.
        clear(&work.join("in"));
        Ok(result)
    }

    /// Stops a running bake: Blender is killed and nothing is staged.
    ///
    /// # Errors
    ///
    /// Fails when that recipe is not baking.
    pub fn cancel(&self, project: &str, recipe: &str) -> CoreResult<()> {
        let active = self.active();
        let flag = active
            .get(&key(project, recipe))
            .ok_or_else(|| CoreError::Invalid("This recipe is not baking.".to_string()))?;
        flag.store(true, Ordering::SeqCst);
        Ok(())
    }
}

/// Ends a bake when its thread does, also when it panics: saved as
/// finished (failed, for a panic) before it leaves the running ones, so a
/// read in between never takes it for one the app closed on, and the
/// recipe can bake again.
struct Ending {
    running: Arc<Running>,
    bakes: RecipeBakes,
    name: String,
    outcome: Option<CoreResult<super::RecipeBakeResult>>,
}

impl Drop for Ending {
    fn drop(&mut self) {
        let outcome = self
            .outcome
            .take()
            .unwrap_or_else(|| Err(CoreError::Invalid(BROKE.to_string())));
        finish(&self.running, outcome);
        self.bakes.active().remove(&self.name);
    }
}

fn finish(running: &Running, outcome: CoreResult<super::RecipeBakeResult>) {
    running.update(|r| {
        r.progress = None;
        r.finished_ms = Some(now_ms());
        match outcome {
            Ok(result) => {
                r.state = RecipeBakeState::Done;
                r.review = Some(super::review_of(&result));
                r.result = Some(result);
            }
            Err(err) if err.to_string() == CANCELLED => r.state = RecipeBakeState::Cancelled,
            Err(err) => {
                r.state = RecipeBakeState::Failed;
                r.error = Some(err.to_string());
            }
        }
    });
}
