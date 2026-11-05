//! Recipe bakes as jobs: one per recipe of a project, run on a background
//! thread, its record (`bake.json`) and staged model kept in its folder
//! under the workspace cache. Every change is saved and told to the app;
//! Cancel kills Blender part way; a bake the app closed on reads as stopped
//! when it opens again.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Instant;

use super::job::{self, PROGRESS_FILE, Prepared};
use super::mark::{RecipeBakeMark, write_mark};
use super::result::{read_run, result_of};
use super::{
    RecipeBakeProgress, RecipeBakeRecord, RecipeBakeRequest, RecipeBakeStarter, RecipeBakeState,
    STAGED_FILE, WORK_DIR,
};
use crate::base::ports::{CANCELLED, ScriptRunner, Watch};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::pipeline::history::now_ms;
use crate::pipeline::tools::Toolbox;
use crate::workspace::{WorkspaceConfig, is_single_segment};

const RECORD_FILE: &str = "bake.json";
/// The run's own files, cleared when a bake starts again.
const RUN_DIR: &str = "run";
const CLOSED: &str = "The app closed during this bake. Bake it again.";

type OnChange = Box<dyn Fn(&RecipeBakeRecord) + Send + Sync>;

struct Inner {
    runner: Arc<dyn ScriptRunner + Send + Sync>,
    on_change: OnChange,
    active: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

/// The service. Cheap to clone.
#[derive(Clone)]
pub struct RecipeBakes {
    inner: Arc<Inner>,
}

fn key(project: &str, recipe: &str) -> String {
    format!("{project}/{recipe}")
}

/// The folder of the bake of `recipe` in `project`.
fn folder(
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

/// The record of the bake of `recipe` in `project` as it is on disk, or
/// none when it was never baked.
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
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| CoreError::json(&file, e))
}

fn save(dir: &Path, record: &RecipeBakeRecord) -> CoreResult<()> {
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    write_json(&dir.join(RECORD_FILE), record)
}

/// A file or folder from an earlier bake, gone before a new one starts.
fn clear(path: &Path) {
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
            }),
        }
    }

    fn active(&self) -> MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.inner
            .active
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The bake of `recipe` in `project`, or none when it was never baked.
    /// A bake the app closed on reads as failed, saying so.
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
        let Some(mut found) = record_of(root, config, project, recipe)? else {
            return Ok(None);
        };
        if found.running() && !self.active().contains_key(&key(project, recipe)) {
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
            active.insert(name.clone(), Arc::clone(&cancel));
        }
        clear(&dir.join(RUN_DIR));
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
        };
        let running = Arc::new(Running {
            record: Mutex::new(record.clone()),
            dir,
            inner: Arc::clone(&self.inner),
        });
        running.update(|_| {});
        let bakes = self.clone();
        let root = root.to_path_buf();
        let spawned = thread::Builder::new()
            .name("recipe-bake".to_string())
            .spawn(move || {
                let outcome = bakes.bake(&root, &running, &prepared, &tools, &cancel);
                // Saved as finished before it leaves the running ones, so a
                // read in between never takes it for one the app closed on.
                finish(&running, outcome);
                bakes.active().remove(&name);
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
        let work = running.dir.join(RUN_DIR);
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
        let staged = running.dir.join(STAGED_FILE);
        fs::rename(work.join(STAGED_FILE), &staged).map_err(|e| CoreError::io(&staged, e))?;
        // The copies of the model and the clip are not needed any more.
        clear(&work.join("in"));
        let result = result_of(
            &run,
            prepared,
            &running.dir,
            started.elapsed().as_secs_f32(),
        );
        write_mark(&staged, &RecipeBakeMark::of(&result))?;
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

fn finish(running: &Running, outcome: CoreResult<super::RecipeBakeResult>) {
    running.update(|r| {
        r.progress = None;
        r.finished_ms = Some(now_ms());
        match outcome {
            Ok(result) => {
                r.state = RecipeBakeState::Done;
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
