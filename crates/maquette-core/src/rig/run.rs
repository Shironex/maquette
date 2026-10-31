//! One running pose test or fix: the record it changes, its scripts and its
//! progress.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use super::service::Inner;
use super::{
    RigLogLine, RigProgress, RigRecord, RigRoute, RigRun, RigStage, RigState, creature, humanoid,
    record, scripts,
};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::now_ms;
use crate::pipeline::tools::Toolbox;

const CANCELLED: &str = "The rig doctor run was cancelled.";

/// One running pose test or fix.
pub(super) struct Run {
    record: Arc<Mutex<RigRecord>>,
    inner: Arc<Inner>,
    tools: Toolbox,
    cancel: Arc<AtomicBool>,
    pub dir: PathBuf,
    scripts: PathBuf,
}

impl Run {
    pub(super) fn new(
        case: RigRecord,
        inner: Arc<Inner>,
        tools: Toolbox,
        cancel: Arc<AtomicBool>,
        dir: PathBuf,
    ) -> Self {
        Self {
            record: Arc::new(Mutex::new(case)),
            inner,
            tools,
            cancel,
            scripts: dir.join("scripts"),
            dir,
        }
    }

    pub(super) fn go(&self, what: RigRun) -> CoreResult<()> {
        fs::create_dir_all(&self.dir).map_err(|e| CoreError::io(&self.dir, e))?;
        scripts::install(&self.dir)?;
        let case = self.snapshot();
        let creature = case.route == Some(RigRoute::Creature);
        match (what, creature) {
            (RigRun::PoseTest, false) => humanoid::test(self),
            (RigRun::PoseTest, true) => creature::test(self),
            (_, false) => humanoid::fix(self),
            (_, true) => creature::fix(self),
        }
    }

    pub(super) fn finish(&self, outcome: CoreResult<()>) {
        self.update(|case| {
            case.progress = None;
            for stage in case.stages.iter_mut().filter(|s| s.seconds.is_none()) {
                stage.seconds = Some(0.0);
            }
            if let Err(error) = &outcome {
                let text = error.to_string();
                if text.contains(CANCELLED) {
                    case.state = RigState::Cancelled;
                } else {
                    case.state = RigState::Failed;
                    case.error = Some(text);
                }
            }
        });
    }

    pub(super) fn snapshot(&self) -> RigRecord {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, RigRecord> {
        self.record
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Changes the record, saves it and tells the app.
    pub(super) fn update(&self, change: impl FnOnce(&mut RigRecord)) {
        let mut case = self.lock();
        change(&mut case);
        case.updated_ms = now_ms();
        if let Err(error) = record::save(&case) {
            tracing::warn!(error = %error, "could not save a rig doctor case");
        }
        (self.inner.on_change)(&case);
    }

    /// The working copy of the model: `in.glb`, made once per case.
    pub(super) fn working_copy(&self) -> CoreResult<PathBuf> {
        let input = self.snapshot().input;
        let copy = self.dir.join("in.glb");
        fs::copy(&input, &copy).map_err(|e| CoreError::io(&copy, e))?;
        Ok(copy)
    }

    /// Runs one bundled script of `route` headless.
    pub(super) fn script(&self, route: &str, name: &str, args: &[String]) -> CoreResult<()> {
        if self.cancel.load(Ordering::SeqCst) {
            return Err(CoreError::Invalid(CANCELLED.to_string()));
        }
        let files = if route == "humanoid" {
            scripts::HUMANOID
        } else {
            scripts::CREATURE
        };
        let text = scripts::text(files, name)?;
        self.inner.runner.run_step(
            &self.tools,
            &self.scripts.join(route),
            name,
            (name, text),
            args,
        )
    }

    /// A step of the Headless Blender list around `work`; its answer is the
    /// detail the list shows when it is done.
    pub(super) fn stage(
        &self,
        name: &str,
        work: impl FnOnce() -> CoreResult<String>,
    ) -> CoreResult<()> {
        self.update(|case| {
            case.stages.push(RigStage {
                name: name.to_string(),
                detail: String::new(),
                seconds: None,
            });
        });
        let started = Instant::now();
        let detail = work()?;
        let seconds = started.elapsed().as_secs_f64();
        self.update(|case| {
            if let Some(stage) = case.stages.iter_mut().rev().find(|s| s.name == name) {
                stage.detail.clone_from(&detail);
                stage.seconds = Some(seconds);
            }
        });
        Ok(())
    }

    /// Adds a line to the log under the progress bar.
    pub(super) fn log(&self, script: &str, text: String) {
        let at = crate::project::now_rfc3339();
        let at = at.get(11..19).unwrap_or_default().to_string();
        self.update(|case| {
            case.log.push(RigLogLine {
                at,
                script: script.to_string(),
                text,
            });
        });
    }

    /// Runs a render script while counting the PNGs it writes in `folder`.
    pub(super) fn watched(
        &self,
        (label, total): (&str, u32),
        (folder, prefix): (&Path, &str),
        work: impl FnOnce() -> CoreResult<()>,
    ) -> CoreResult<()> {
        let done = Arc::new(AtomicBool::new(false));
        let watcher = {
            let (done, folder, prefix) =
                (Arc::clone(&done), folder.to_path_buf(), prefix.to_string());
            let (record, inner, label) = (
                Arc::clone(&self.record),
                Arc::clone(&self.inner),
                label.to_string(),
            );
            thread::spawn(move || {
                let mut last = u32::MAX;
                while !done.load(Ordering::SeqCst) {
                    let count = fs::read_dir(&folder).map_or(0, |dir| {
                        dir.flatten()
                            .filter(|e| {
                                let name = e.file_name();
                                let name = name.to_string_lossy();
                                name.ends_with(".png") && name.starts_with(&prefix)
                            })
                            .count()
                    });
                    let count = u32::try_from(count).unwrap_or(u32::MAX).min(total);
                    if count != last {
                        last = count;
                        let mut case = record
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        case.progress = Some(RigProgress {
                            label: label.clone(),
                            done: count,
                            total,
                        });
                        (inner.on_change)(&case);
                    }
                    thread::sleep(Duration::from_millis(400));
                }
            })
        };
        let outcome = work();
        done.store(true, Ordering::SeqCst);
        let _ = watcher.join();
        self.update(|case| case.progress = None);
        outcome
    }
}
