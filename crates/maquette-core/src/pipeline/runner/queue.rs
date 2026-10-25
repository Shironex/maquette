//! Scheduling: one lane per project with its queue and the runs going
//! now, and the worker threads that take runs off it.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use process_wrap::std::ChildWrapper;

use super::execution::Execution;
use super::{JobRunner, POLL, PipelineEvent, RunOptions, RunSetup, RunSink, lock};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::PipelineDef;
use crate::pipeline::graph::{self, Planned};
use crate::pipeline::history::{self, RunRecord, RunState, StepRecord, StepState, now_ms};

pub(super) type ChildSlot = Arc<Mutex<Option<Box<dyn ChildWrapper>>>>;

struct Active {
    run_id: String,
    cancel: Arc<AtomicBool>,
    child: ChildSlot,
}

type Job = (RunRecord, RunSetup, RunOptions);

struct Lane {
    /// The runs going now, at most `limit`.
    active: Vec<Active>,
    queue: VecDeque<Job>,
    next_number: Option<u32>,
    limit: usize,
}

impl Default for Lane {
    fn default() -> Self {
        Self {
            active: Vec::new(),
            queue: VecDeque::new(),
            next_number: None,
            limit: 1,
        }
    }
}

impl Lane {
    /// Moves queued runs into free slots and returns them to start.
    fn take_startable(&mut self) -> Vec<(Job, Handles)> {
        let mut out = Vec::new();
        while self.active.len() < self.limit {
            let Some(job) = self.queue.pop_front() else {
                break;
            };
            let active = Active::new(&job.0.id);
            let handles = active.handles();
            self.active.push(active);
            out.push((job, handles));
        }
        out
    }
}

struct Inner {
    sink: Arc<dyn RunSink>,
    lanes: Mutex<HashMap<String, Lane>>,
}

impl Inner {
    fn lanes(&self) -> MutexGuard<'_, HashMap<String, Lane>> {
        self.lanes.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[derive(Clone)]
pub struct PipelineRunner {
    inner: Arc<Inner>,
}

impl PipelineRunner {
    pub fn new(sink: Arc<dyn RunSink>) -> Self {
        Self {
            inner: Arc::new(Inner {
                sink,
                lanes: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// The running and queued runs of `project`. With more than one run at
    /// a time ([`Self::set_limit`]) the first one that started is named.
    pub fn active(&self, project: &str) -> (Option<String>, Vec<RunRecord>) {
        let lanes = self.inner.lanes();
        match lanes.get(project) {
            Some(lane) => (
                lane.active.first().map(|a| a.run_id.clone()),
                lane.queue.iter().map(|(r, _, _)| r.clone()).collect(),
            ),
            None => (None, Vec::new()),
        }
    }

    /// Every run going now on `project`'s lane.
    pub fn running(&self, project: &str) -> Vec<String> {
        self.inner
            .lanes()
            .get(project)
            .map(|lane| lane.active.iter().map(|a| a.run_id.clone()).collect())
            .unwrap_or_default()
    }

    /// How many runs `lane` may run at once (1 for pipelines, so a
    /// project's steps never race). Raising it starts queued runs.
    pub fn set_limit(&self, lane: &str, limit: usize) {
        let start = {
            let mut lanes = self.inner.lanes();
            let entry = lanes.entry(lane.to_string()).or_default();
            entry.limit = limit.max(1);
            entry.take_startable()
        };
        for (job, handles) in start {
            if let Err(err) = self.start(lane, job, handles) {
                tracing::warn!(error = %err, "a queued run did not start");
            }
        }
    }

    /// Queues a run with extra controls.
    ///
    /// # Errors
    ///
    /// Fails when the pipeline does not plan (a cycle, an unknown step) or a
    /// worker thread cannot start.
    pub fn submit_with(&self, setup: RunSetup, options: RunOptions) -> CoreResult<RunRecord> {
        let plan = graph::plan(&setup.def, setup.from.as_deref(), setup.force)?;
        let mut lanes = self.inner.lanes();
        let lane = lanes.entry(setup.project.clone()).or_default();
        let number = match lane.next_number {
            Some(n) => n,
            None => {
                history::load_runs(&setup.cache_dir)
                    .iter()
                    .map(|r| r.number)
                    .max()
                    .unwrap_or(0)
                    + 1
            }
        };
        lane.next_number = Some(number + 1);
        let record = RunRecord {
            id: format!("{}-run-{number}", setup.project),
            number,
            project: setup.project.clone(),
            from: setup.from.clone(),
            force: setup.force,
            state: RunState::Queued,
            queued_at_ms: now_ms(),
            started_at_ms: None,
            finished_at_ms: None,
            steps: initial_steps(&setup.def, &plan),
            message: None,
        };
        let project = setup.project.clone();
        lane.queue.push_back((record.clone(), setup, options));
        // A lane with a free slot starts right away; a busy one queues.
        let start = lane.take_startable();
        drop(lanes);
        self.inner.sink.event(PipelineEvent::Run {
            run: record.clone(),
        });
        for (job, handles) in start {
            let id = job.0.id.clone();
            if let Err(err) = self.start(&project, job, handles)
                && id == record.id
            {
                return Err(err);
            }
        }
        Ok(record)
    }

    fn start(&self, project: &str, job: Job, handles: Handles) -> CoreResult<()> {
        let inner = Arc::clone(&self.inner);
        let lane = project.to_string();
        let run_id = job.0.id.clone();
        let spawned = thread::Builder::new()
            .name(format!("pipeline-{project}"))
            .spawn(move || worker(&inner, &lane, job, handles));
        if let Err(e) = spawned {
            if let Some(lane) = self.inner.lanes().get_mut(project) {
                lane.active.retain(|a| a.run_id != run_id);
            }
            return Err(CoreError::Unavailable(format!(
                "Could not start the runner. {e}"
            )));
        }
        Ok(())
    }

    /// Submits and blocks until the run finishes. For the CLI and tests.
    pub fn run_blocking(&self, setup: RunSetup) -> CoreResult<RunRecord> {
        self.run_blocking_with(setup, RunOptions::default())
    }

    /// [`Self::run_blocking`] with extra controls, such as a timeout.
    pub fn run_blocking_with(&self, setup: RunSetup, options: RunOptions) -> CoreResult<RunRecord> {
        let project = setup.project.clone();
        let cache_dir = setup.cache_dir.clone();
        let queued = self.submit_with(setup, options)?;
        loop {
            thread::sleep(POLL);
            let busy = {
                let lanes = self.inner.lanes();
                lanes
                    .get(&project)
                    .is_some_and(|l| !l.active.is_empty() || !l.queue.is_empty())
            };
            if !busy {
                break;
            }
        }
        history::load_runs(&cache_dir)
            .into_iter()
            .find(|r| r.id == queued.id)
            .ok_or_else(|| CoreError::NotFound(format!("the run {}", queued.id)))
    }
}

fn initial_steps(def: &PipelineDef, plan: &[(usize, Planned)]) -> Vec<StepRecord> {
    plan.iter()
        .map(|&(i, planned)| {
            let step = &def.steps[i];
            StepRecord {
                id: step.id.clone(),
                name: step.name.clone(),
                state: if planned == Planned::Skip {
                    StepState::Skipped
                } else {
                    StepState::Waiting
                },
                started_at_ms: None,
                duration_ms: None,
                exit_code: None,
                message: (planned == Planned::Skip).then(|| "Not part of this run.".to_string()),
                progress: None,
            }
        })
        .collect()
}

impl JobRunner for PipelineRunner {
    fn submit(&self, setup: RunSetup) -> CoreResult<RunRecord> {
        self.submit_with(setup, RunOptions::default())
    }

    fn cancel(&self, run_id: &str) -> CoreResult<()> {
        let mut lanes = self.inner.lanes();
        for lane in lanes.values_mut() {
            if let Some(active) = lane.active.iter().find(|a| a.run_id == run_id) {
                active.cancel.store(true, Ordering::SeqCst);
                if let Some(child) = lock(&active.child).as_mut() {
                    let _ = child.start_kill();
                }
                return Ok(());
            }
            if let Some(pos) = lane.queue.iter().position(|(r, _, _)| r.id == run_id) {
                if let Some((mut record, setup, _)) = lane.queue.remove(pos) {
                    drop(lanes);
                    record.state = RunState::Cancelled;
                    record.finished_at_ms = Some(now_ms());
                    record.message = Some("Cancelled before it started.".to_string());
                    for step in &mut record.steps {
                        if step.state == StepState::Waiting {
                            step.state = StepState::Cancelled;
                        }
                    }
                    let _ = history::record_run(&setup.cache_dir, &record);
                    self.inner.sink.event(PipelineEvent::Run { run: record });
                }
                return Ok(());
            }
        }
        Err(CoreError::NotFound(format!("the run {run_id}")))
    }
}

type Handles = (Arc<AtomicBool>, ChildSlot);

impl Active {
    fn new(run_id: &str) -> Self {
        Self {
            run_id: run_id.to_string(),
            cancel: Arc::new(AtomicBool::new(false)),
            child: Arc::new(Mutex::new(None)),
        }
    }

    fn handles(&self) -> Handles {
        (Arc::clone(&self.cancel), Arc::clone(&self.child))
    }
}

/// Runs `job`, then whatever queued behind it while its slot is free, and
/// gives the slot back when the queue is empty.
fn worker(inner: &Arc<Inner>, project: &str, job: Job, handles: Handles) {
    let mut next = Some((job, handles));
    while let Some(((record, setup, options), (cancel, child))) = next.take() {
        let run_id = record.id.clone();
        let mut exec = Execution {
            sink: Arc::clone(&inner.sink),
            record,
            setup,
            options,
            cancel,
            child,
            log: Vec::new(),
            slot: 0,
            progress_sent: None,
            stop: None,
        };
        exec.run();
        let mut lanes = inner.lanes();
        let Some(lane) = lanes.get_mut(project) else {
            return;
        };
        lane.active.retain(|a| a.run_id != run_id);
        next = lane.take_startable().into_iter().next();
    }
}
