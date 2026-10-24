//! The job runner: one run at a time per project, the rest queued. Each
//! step's program is spawned directly with an args array (never through a
//! shell string), in its own process group on Unix or job object on
//! Windows (with no console window), so Cancel kills the whole tree. Output
//! lines stream to a [`RunSink`] as they arrive.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use process_wrap::std::{ChildWrapper, CommandWrap};
use serde::{Deserialize, Serialize};

use super::cache::{self, CacheRecord, FileHasher, KeyParts};
use super::graph::{self, BLENDER_HEADLESS_MESSAGE, Planned, blender_headless};
use super::history::{
    self, LogEntry, LogLevel, LogStream, RunRecord, RunState, StepRecord, StepState, now_ms,
};
use super::presets::{compress_passes, resolve};
use super::progress::Tracker;
use super::tools::{ResolvedTool, Toolbox};
use super::{CompressDefaults, PipelineDef, StepDef, StepKind};
use crate::error::{CoreError, CoreResult};
use crate::toolchain::ToolId;

const POLL: Duration = Duration::from_millis(40);
/// How long to keep reading pipes after the process exits.
const DRAIN_GRACE: Duration = Duration::from_secs(2);
const MAX_LINE_CHARS: usize = 4000;
/// Progress events are sent at most this often, unless a step finishes.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);
/// Why a gated step was skipped without asking.
const UNCHANGED_MESSAGE: &str = "Nothing changed since the last ship, so I did not ask again.";

/// A step's resolved tool and calls, and its cache key.
struct Prepared {
    tool: ResolvedTool,
    invocations: Vec<Invocation>,
    key: String,
}

/// What the runner reports while it works. The UI replaces its copy of the
/// run on every `run` event and appends `log` lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PipelineEvent {
    Run { run: RunRecord },
    Log { entry: LogEntry },
}

pub trait RunSink: Send + Sync {
    fn event(&self, event: PipelineEvent);
}

/// A sink that drops everything, for callers that only need the result.
pub struct NullSink;

impl RunSink for NullSink {
    fn event(&self, _event: PipelineEvent) {}
}

/// Everything a run needs, resolved by the caller when it submits.
#[derive(Debug, Clone)]
pub struct RunSetup {
    pub project: String,
    pub project_dir: PathBuf,
    /// `<workspace cache>/pipeline/<project>`.
    pub cache_dir: PathBuf,
    pub def: PipelineDef,
    pub tools: Toolbox,
    pub defaults: CompressDefaults,
    /// Placeholder values besides `{script}`, `{root}` and `{project}`, for
    /// example `workspace` and `siteRepo`.
    pub vars: BTreeMap<String, String>,
    pub from: Option<String>,
    pub force: bool,
}

/// Extra controls for one run, for jobs that are not project pipelines
/// (the Concept lab's Codex jobs) and for runs an agent started.
#[derive(Clone, Default)]
pub struct RunOptions {
    /// Written to each step's standard input, which is then closed. Without
    /// it the input is empty.
    pub stdin: Option<String>,
    /// A step still running after this long is stopped and fails.
    pub timeout: Option<Duration>,
    /// Asked before each step whether it may run (see [`StepGate`]).
    pub gate: Option<Arc<dyn StepGate>>,
}

impl std::fmt::Debug for RunOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOptions")
            .field("stdin", &self.stdin.as_ref().map(String::len))
            .field("timeout", &self.timeout)
            .field("gate", &self.gate.is_some())
            .finish()
    }
}

/// What a [`StepGate`] decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Allow,
    /// The step and the rest of the run are cancelled with this reason.
    Deny(String),
}

/// Holds a run before a step until someone decides, for example "ask
/// before shipping" for runs an agent starts. `wait` blocks the run's
/// worker thread; it should return [`GateDecision::Deny`] once `cancelled`
/// turns true.
pub trait StepGate: Send + Sync {
    /// True when `step` needs a decision before it runs.
    fn applies(&self, step: &StepDef) -> bool;
    /// The line the step shows while it waits.
    fn waiting_message(&self, step: &StepDef) -> String;
    fn wait(&self, run: &RunRecord, step: &StepDef, cancelled: &AtomicBool) -> GateDecision;
}

/// Runs pipelines. [`PipelineRunner`] is the real one.
pub trait JobRunner: Send + Sync {
    /// Queues a run and returns its record (queued, or running when the
    /// project was idle).
    fn submit(&self, setup: RunSetup) -> CoreResult<RunRecord>;
    /// Cancels a queued or running run.
    fn cancel(&self, run_id: &str) -> CoreResult<()>;
}

type ChildSlot = Arc<Mutex<Option<Box<dyn ChildWrapper>>>>;

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

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
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
        let project = setup.project.clone();
        let cache_dir = setup.cache_dir.clone();
        let queued = self.submit(setup)?;
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

/// One run in progress.
struct Execution {
    sink: Arc<dyn RunSink>,
    record: RunRecord,
    setup: RunSetup,
    options: RunOptions,
    cancel: Arc<AtomicBool>,
    child: ChildSlot,
    log: Vec<LogEntry>,
    /// The step record the running step reports into.
    slot: usize,
    /// When progress was last sent, so a chatty tool does not flood the UI.
    progress_sent: Option<Instant>,
    /// Why the run stopped early without a failure (a gate said no).
    stop: Option<String>,
}

enum StepOutcome {
    Done,
    Cached,
    Failed(String, Option<i32>),
    Cancelled,
}

impl Execution {
    fn emit_run(&self) {
        self.sink.event(PipelineEvent::Run {
            run: self.record.clone(),
        });
    }

    fn log(&mut self, step: Option<&str>, stream: LogStream, level: LogLevel, text: String) {
        let entry = LogEntry {
            run_id: self.record.id.clone(),
            step: step.map(str::to_string),
            stream,
            level,
            text,
            at_ms: now_ms(),
        };
        self.sink.event(PipelineEvent::Log {
            entry: entry.clone(),
        });
        self.log.push(entry);
        if self.log.len() > history::MAX_LOG_LINES * 2 {
            self.log.drain(..history::MAX_LOG_LINES);
        }
    }

    fn system(&mut self, step: Option<&str>, level: LogLevel, text: impl Into<String>) {
        self.log(step, LogStream::System, level, text.into());
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// Records how far the running step is and tells the UI, throttled.
    fn report_progress(&mut self, tracker: &Tracker) {
        let Some(value) = tracker.value() else {
            return;
        };
        let rec = &mut self.record.steps[self.slot];
        if rec.progress == Some(value) {
            return;
        }
        rec.progress = Some(value);
        let due = self
            .progress_sent
            .is_none_or(|at| at.elapsed() >= PROGRESS_EVERY);
        if due || value >= 1.0 {
            self.progress_sent = Some(Instant::now());
            self.emit_run();
        }
    }

    fn run(&mut self) {
        self.record.state = RunState::Running;
        self.record.started_at_ms = Some(now_ms());
        self.emit_run();
        let plan = graph::plan(
            &self.setup.def,
            self.setup.from.as_deref(),
            self.setup.force,
        );
        let planned = match plan {
            Ok(p) => p,
            Err(err) => {
                self.finish(RunState::Failed, Some(err.to_string()));
                return;
            }
        };
        let runnable = planned.iter().filter(|(_, p)| *p != Planned::Skip).count();
        let header = format!(
            "Run {} · {runnable} of {} steps{}{}",
            self.record.number,
            planned.len(),
            self.setup
                .from
                .as_deref()
                .map(|f| format!(" · from {f}"))
                .unwrap_or_default(),
            if self.setup.force { " · forced" } else { "" }
        );
        self.system(None, LogLevel::Info, header);
        let mut hasher = FileHasher::with_memo(&self.setup.cache_dir);
        let mut failure: Option<String> = None;
        for (slot, (index, planned)) in planned.iter().enumerate() {
            if *planned == Planned::Skip {
                continue;
            }
            let step = self.setup.def.steps[*index].clone();
            if failure.is_some() || self.cancelled() || self.stop.is_some() {
                let state = if failure.is_none() {
                    StepState::Cancelled
                } else {
                    StepState::Skipped
                };
                let rec = &mut self.record.steps[slot];
                rec.state = state;
                rec.message = Some(if state == StepState::Cancelled {
                    "Cancelled.".to_string()
                } else {
                    "Not run, an earlier step failed.".to_string()
                });
                continue;
            }
            if self.gated(&step) && self.unchanged_since_last(&step, &mut hasher) {
                let rec = &mut self.record.steps[slot];
                rec.state = StepState::Cached;
                rec.progress = Some(1.0);
                rec.message = Some(UNCHANGED_MESSAGE.to_string());
                self.system(
                    Some(&step.id),
                    LogLevel::Ok,
                    format!("{} skipped: {UNCHANGED_MESSAGE}", step.name),
                );
                self.emit_run();
                continue;
            }
            if !self.pass_gate(slot, &step) {
                continue;
            }
            let started = Instant::now();
            {
                let rec = &mut self.record.steps[slot];
                rec.state = StepState::Running;
                rec.started_at_ms = Some(now_ms());
            }
            self.slot = slot;
            self.progress_sent = None;
            self.emit_run();
            let outcome = self.run_step(&step, *planned == Planned::Force, &mut hasher);
            let elapsed = started.elapsed();
            #[expect(clippy::cast_precision_loss, reason = "step durations are small")]
            let duration_ms = elapsed.as_millis() as f64;
            let rec = &mut self.record.steps[slot];
            rec.duration_ms = Some(duration_ms);
            rec.progress = matches!(outcome, StepOutcome::Done | StepOutcome::Cached)
                .then_some(1.0)
                .or(rec.progress);
            match outcome {
                StepOutcome::Done => rec.state = StepState::Done,
                StepOutcome::Cached => rec.state = StepState::Cached,
                StepOutcome::Cancelled => {
                    rec.state = StepState::Cancelled;
                    rec.message = Some("Cancelled.".to_string());
                }
                StepOutcome::Failed(message, code) => {
                    rec.state = StepState::Failed;
                    rec.exit_code = code;
                    rec.message = Some(message.clone());
                    failure = Some(format!("{} failed. {message}", step.name));
                }
            }
            let state = rec.state;
            let line = match state {
                StepState::Done => Some((
                    LogLevel::Ok,
                    format!("{} done in {}", step.name, seconds(elapsed)),
                )),
                StepState::Cached => Some((
                    LogLevel::Ok,
                    format!("{} inputs unchanged, cache hit", step.name),
                )),
                StepState::Failed => Some((
                    LogLevel::Error,
                    format!(
                        "{} failed after {}: {}",
                        step.name,
                        seconds(elapsed),
                        self.record.steps[slot].message.clone().unwrap_or_default()
                    ),
                )),
                _ => None,
            };
            if let Some((level, text)) = line {
                self.system(Some(&step.id), level, text);
            }
            self.emit_run();
        }
        hasher.flush();
        if self.cancelled() {
            self.finish(RunState::Cancelled, Some("Cancelled.".to_string()));
        } else if let Some(reason) = self.stop.clone() {
            self.finish(RunState::Cancelled, Some(reason));
        } else if let Some(message) = failure {
            self.finish(RunState::Failed, Some(message));
        } else {
            self.finish(RunState::Done, None);
        }
    }

    fn gated(&self, step: &StepDef) -> bool {
        self.options.gate.as_ref().is_some_and(|g| g.applies(step))
    }

    /// Asks the run's gate, if it has one for `step`, and waits for the
    /// answer. False when the step must not run; the run then stops.
    fn pass_gate(&mut self, slot: usize, step: &StepDef) -> bool {
        let Some(gate) = self.options.gate.clone().filter(|g| g.applies(step)) else {
            return true;
        };
        let waiting = gate.waiting_message(step);
        self.record.steps[slot].message = Some(waiting.clone());
        self.system(Some(&step.id), LogLevel::Warn, waiting);
        self.emit_run();
        match gate.wait(&self.record, step, &self.cancel) {
            GateDecision::Allow => {
                self.record.steps[slot].message = None;
                self.system(
                    Some(&step.id),
                    LogLevel::Info,
                    format!("{} approved", step.name),
                );
                true
            }
            GateDecision::Deny(reason) => {
                let rec = &mut self.record.steps[slot];
                rec.state = StepState::Cancelled;
                rec.message = Some(reason.clone());
                self.system(Some(&step.id), LogLevel::Error, reason.clone());
                self.stop = Some(reason);
                self.emit_run();
                false
            }
        }
    }

    fn finish(&mut self, state: RunState, message: Option<String>) {
        self.record.state = state;
        self.record.finished_at_ms = Some(now_ms());
        self.record.message = message;
        let total = self
            .record
            .started_at_ms
            .zip(self.record.finished_at_ms)
            .map(|(a, b)| (b - a).max(0.0))
            .unwrap_or(0.0);
        let cached = self
            .record
            .steps
            .iter()
            .filter(|s| s.state == StepState::Cached)
            .count();
        let summary = match state {
            RunState::Done => format!(
                "Run {} done in {:.1}s, {cached} cached",
                self.record.number,
                total / 1000.0
            ),
            RunState::Cancelled => format!("Run {} cancelled", self.record.number),
            _ => format!(
                "Run {} failed: {}",
                self.record.number,
                self.record.message.clone().unwrap_or_default()
            ),
        };
        let level = if state == RunState::Done {
            LogLevel::Ok
        } else {
            LogLevel::Error
        };
        self.system(None, level, summary);
        if let Err(err) = history::record_run(&self.setup.cache_dir, &self.record) {
            tracing::warn!(error = %err, "could not save the run record");
        }
        if let Err(err) = history::save_log(&self.setup.cache_dir, self.record.number, &self.log) {
            tracing::warn!(error = %err, "could not save the run log");
        }
        self.emit_run();
    }

    fn root(&self) -> PathBuf {
        self.setup.def.root_dir(&self.setup.project_dir)
    }

    fn fill(&self, arg: &str, step: &StepDef, root: &Path) -> String {
        let mut out = arg.to_string();
        if let Some(script) = &step.script {
            out = out.replace("{script}", script);
        }
        out = out.replace("{root}", &root.display().to_string());
        out = out.replace("{project}", &self.setup.project_dir.display().to_string());
        for (name, value) in &self.setup.vars {
            out = out.replace(&format!("{{{name}}}"), value);
        }
        out
    }

    /// Resolves the tool and the calls of `step` and its cache key.
    fn prepare(&self, step: &StepDef, hasher: &mut FileHasher) -> Result<Prepared, StepOutcome> {
        let root = self.root();
        let tool = self
            .setup
            .tools
            .resolve(step.kind, step.command.as_deref(), &root)
            .map_err(|err| StepOutcome::Failed(err.to_string(), None))?;
        if let Some(script) = &step.script
            && !root.join(script).is_file()
        {
            return Err(StepOutcome::Failed(
                format!("The script {script} is missing."),
                None,
            ));
        }
        let invocations = self
            .invocations(step, &root)
            .map_err(|message| StepOutcome::Failed(message, None))?;
        if step.kind == StepKind::Blender
            && invocations.iter().any(|(_, args)| !blender_headless(args))
        {
            // The definition was validated, but the filled args are checked
            // again right before spawning: Blender never starts with a window.
            return Err(StepOutcome::Failed(
                BLENDER_HEADLESS_MESSAGE.to_string(),
                None,
            ));
        }
        let all_args: Vec<String> = invocations
            .iter()
            .flat_map(|(_, args)| tool.prefix.iter().chain(args.iter()).cloned())
            .collect();
        let extra = self.cache_extra(step);
        let key = cache::key(
            &KeyParts {
                root: &root,
                step,
                tool: &tool,
                args: &all_args,
                extra,
            },
            hasher,
        )
        .map_err(|err| StepOutcome::Failed(err.to_string(), None))?;
        Ok(Prepared {
            tool,
            invocations,
            key,
        })
    }

    /// The record of the last finished run of a step without outputs, such
    /// as Ship. Only used to skip a gate when nothing changed since.
    fn last_id(step: &StepDef) -> String {
        format!("{}.last", step.id)
    }

    /// True when a gated step without outputs would do exactly what its
    /// last finished run did, so asking again is pointless.
    fn unchanged_since_last(&self, step: &StepDef, hasher: &mut FileHasher) -> bool {
        if !step.outputs.is_empty() || self.setup.force {
            return false;
        }
        let Ok(prepared) = self.prepare(step, hasher) else {
            return false;
        };
        cache::load_record(&self.setup.cache_dir, &Self::last_id(step))
            .is_some_and(|record| record.key == prepared.key)
    }

    fn run_step(&mut self, step: &StepDef, force: bool, hasher: &mut FileHasher) -> StepOutcome {
        let root = self.root();
        let Prepared {
            tool,
            invocations,
            key,
        } = match self.prepare(step, hasher) {
            Ok(prepared) => prepared,
            Err(outcome) => return outcome,
        };
        if !force
            && let Some(record) = cache::load_record(&self.setup.cache_dir, &step.id)
            && cache::is_fresh(&record, &key, &root, &self.outputs_of(step))
        {
            return StepOutcome::Cached;
        }
        cache::forget(&self.setup.cache_dir, &step.id);
        let started = Instant::now();
        if let Some(spec) = &step.compress {
            let dir = root.join(&spec.to);
            if let Err(err) = fs::create_dir_all(&dir) {
                return StepOutcome::Failed(format!("Could not create {}. {err}", spec.to), None);
            }
            if !self.setup.tools.has(ToolId::Toktx) {
                self.system(
                    Some(&step.id),
                    LogLevel::Warn,
                    "toktx is missing, so textures stay PNG or JPEG. Install KTX-Software for KTX2.",
                );
            }
        }
        let mut tracker = Tracker::new(invocations.len());
        for (index, (label, args)) in invocations.iter().enumerate() {
            if let Some(label) = label {
                self.system(Some(&step.id), LogLevel::Info, label.clone());
            }
            tracker.start_call(index);
            self.report_progress(&tracker);
            let mut full = tool.prefix.clone();
            full.extend(args.iter().cloned());
            match self.spawn(step, &tool, &full, &root, &mut tracker) {
                Ok(0) => {}
                Ok(code) => {
                    return StepOutcome::Failed(
                        format!("{} exited with code {code}.", program_name(&tool)),
                        Some(code),
                    );
                }
                Err(SpawnError::Cancelled) => return StepOutcome::Cancelled,
                Err(SpawnError::Start(message)) => return StepOutcome::Failed(message, None),
                Err(SpawnError::TimedOut(limit)) => {
                    return StepOutcome::Failed(
                        format!(
                            "It ran longer than {} and was stopped.",
                            human_duration(limit)
                        ),
                        None,
                    );
                }
            }
            if self.cancelled() {
                return StepOutcome::Cancelled;
            }
        }
        let outputs = self.outputs_of(step);
        if let Some((pattern, _)) = cache::expand(&root, &outputs)
            .into_iter()
            .find(|(_, files)| files.is_empty())
        {
            return StepOutcome::Failed(
                format!("It finished but wrote nothing to {pattern}."),
                Some(0),
            );
        }
        let record = CacheRecord {
            key,
            outputs: cache::stamp_outputs(&root, &outputs),
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        };
        let id = if outputs.is_empty() {
            Self::last_id(step)
        } else {
            step.id.clone()
        };
        if let Err(err) = cache::save_record(&self.setup.cache_dir, &id, &record) {
            tracing::warn!(error = %err, "could not save the cache record");
        }
        StepOutcome::Done
    }

    fn outputs_of(&self, step: &StepDef) -> Vec<String> {
        step.outputs.clone()
    }

    /// The presets and toktx availability, for the compress step's key.
    fn cache_extra(&self, step: &StepDef) -> Option<String> {
        step.compress.as_ref()?;
        let presets = serde_json::to_string(&self.setup.def.presets).unwrap_or_default();
        Some(format!(
            "presets {presets} defaults {} {:?} {:?} toktx {}",
            self.setup.defaults.texture_size,
            self.setup.defaults.codec,
            self.setup.defaults.meshopt,
            self.setup.tools.has(ToolId::Toktx)
        ))
    }

    /// The args lists to run, each with an optional log label. Most steps
    /// have one; the compress step has a chain per model.
    fn invocations(&self, step: &StepDef, root: &Path) -> Result<Vec<Invocation>, String> {
        let Some(spec) = &step.compress else {
            return Ok(vec![(
                None,
                step.args.iter().map(|a| self.fill(a, step, root)).collect(),
            )]);
        };
        let models = cache::expand(root, &[format!("{}/*.glb", spec.from)])
            .into_iter()
            .flat_map(|(_, files)| files)
            .collect::<Vec<_>>();
        if models.is_empty() {
            return Err(format!(
                "There are no GLB files in {} to compress.",
                spec.from
            ));
        }
        let tmp = self.setup.cache_dir.join("tmp");
        fs::create_dir_all(&tmp).map_err(|e| format!("Could not create the temp folder. {e}"))?;
        let toktx = self.setup.tools.has(ToolId::Toktx);
        let mut out = Vec::new();
        for model in models {
            let stem = model
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let preset = resolve(&self.setup.def.presets, &stem, self.setup.defaults);
            let output = root.join(&spec.to).join(format!("{stem}.glb"));
            let tmp_file = tmp.join(format!("{stem}.tex.glb"));
            for pass in compress_passes(&model, &output, &tmp_file, &preset, toktx) {
                out.push((Some(pass.label), pass.args));
            }
        }
        Ok(out)
    }

    fn spawn(
        &mut self,
        step: &StepDef,
        tool: &ResolvedTool,
        args: &[String],
        root: &Path,
        tracker: &mut Tracker,
    ) -> Result<i32, SpawnError> {
        let path_env: OsString = {
            let mut dirs: Vec<PathBuf> = tool
                .program
                .parent()
                .map(Path::to_path_buf)
                .into_iter()
                .collect();
            dirs.extend(std::env::split_paths(&self.setup.tools.path_env));
            std::env::join_paths(dirs).unwrap_or_else(|_| self.setup.tools.path_env.clone())
        };
        let stdin_text = self.options.stdin.clone();
        let timeout = self.options.timeout;
        let mut command = CommandWrap::with_new(&tool.program, |cmd| {
            cmd.args(args)
                .current_dir(root)
                .env("PATH", &path_env)
                .env("PYTHONUNBUFFERED", "1")
                .stdin(if stdin_text.is_some() {
                    Stdio::piped()
                } else {
                    Stdio::null()
                })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        });
        #[cfg(unix)]
        {
            command.wrap(process_wrap::std::ProcessGroup::leader());
        }
        #[cfg(windows)]
        {
            use windows::Win32::System::Threading::CREATE_NO_WINDOW;
            command.wrap(process_wrap::std::CreationFlags(CREATE_NO_WINDOW));
            command.wrap(process_wrap::std::JobObject);
        }
        let mut child = command.spawn().map_err(|e| {
            SpawnError::Start(format!("Could not start {}. {e}", tool.program.display()))
        })?;
        // The input goes in on its own thread, so a program that prints a
        // lot before it reads never blocks on a full pipe.
        if let (Some(text), Some(mut input)) = (stdin_text, child.stdin().take()) {
            thread::spawn(move || {
                use std::io::Write as _;
                let _ = input.write_all(text.as_bytes());
            });
        }
        let started = Instant::now();
        let mut timed_out = false;
        let (tx, rx) = mpsc::channel::<(LogStream, String)>();
        let readers = [
            child
                .stdout()
                .take()
                .map(|s| read_lines(s, LogStream::Stdout, tx.clone())),
            child
                .stderr()
                .take()
                .map(|s| read_lines(s, LogStream::Stderr, tx.clone())),
        ];
        drop(tx);
        *lock(&self.child) = Some(child);
        let step_id = step.id.clone();
        let mut exit: Option<i32> = None;
        let mut exited_at: Option<Instant> = None;
        loop {
            match rx.recv_timeout(POLL) {
                Ok((stream, text)) => {
                    let level = history::classify(&text);
                    let moved = tracker.line(&text);
                    self.log(Some(&step_id), stream, level, text);
                    if moved {
                        self.report_progress(tracker);
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) if exit.is_some() => break,
                Err(RecvTimeoutError::Disconnected | RecvTimeoutError::Timeout) => {}
            }
            if exit.is_none() {
                let mut slot = lock(&self.child);
                if let Some(child) = slot.as_mut() {
                    if self.cancel.load(Ordering::SeqCst) {
                        let _ = child.start_kill();
                    }
                    if !timed_out && timeout.is_some_and(|limit| started.elapsed() > limit) {
                        timed_out = true;
                        let _ = child.start_kill();
                    }
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            exit = Some(status.code().unwrap_or(-1));
                            exited_at = Some(Instant::now());
                        }
                        Ok(None) => {}
                        Err(err) => {
                            let _ = child.start_kill();
                            drop(slot);
                            *lock(&self.child) = None;
                            return Err(SpawnError::Start(format!(
                                "Lost track of the process. {err}"
                            )));
                        }
                    }
                }
            }
            if exited_at.is_some_and(|t| t.elapsed() > DRAIN_GRACE) {
                break;
            }
        }
        *lock(&self.child) = None;
        for reader in readers.into_iter().flatten() {
            if reader.is_finished() {
                let _ = reader.join();
            }
        }
        if self.cancel.load(Ordering::SeqCst) {
            return Err(SpawnError::Cancelled);
        }
        if let Some(limit) = timeout.filter(|_| timed_out) {
            return Err(SpawnError::TimedOut(limit));
        }
        Ok(exit.unwrap_or(-1))
    }
}

/// One program call: an optional log label and its args.
type Invocation = (Option<String>, Vec<String>);

enum SpawnError {
    Start(String),
    Cancelled,
    TimedOut(Duration),
}

/// "0.4s", "45s", "2m", "1m 30s".
fn human_duration(d: Duration) -> String {
    let secs = d.as_secs();
    match (secs / 60, secs % 60) {
        (0, _) if d.subsec_millis() > 0 => format!("{:.1}s", d.as_secs_f64()),
        (0, s) => format!("{s}s"),
        (m, 0) => format!("{m}m"),
        (m, s) => format!("{m}m {s}s"),
    }
}

fn program_name(tool: &ResolvedTool) -> String {
    tool.program
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| tool.program.display().to_string())
}

fn seconds(d: Duration) -> String {
    format!("{:.1}s", d.as_secs_f64())
}

/// Reads `stream` line by line (lossy UTF-8, `\r` progress lines split too)
/// and sends each to `tx`.
fn read_lines<R: Read + Send + 'static>(
    stream: R,
    kind: LogStream,
    tx: mpsc::Sender<(LogStream, String)>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let text = String::from_utf8_lossy(&buf);
                    for part in text.split('\r') {
                        let line = part.trim_end_matches(['\n', '\r']);
                        if line.trim().is_empty() {
                            continue;
                        }
                        let line: String = line.chars().take(MAX_LINE_CHARS).collect();
                        if tx.send((kind, line)).is_err() {
                            return;
                        }
                    }
                }
            }
        }
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::pipeline::template::character_preset;
    use crate::pipeline::{CompressSpec, PIPELINE_VERSION};
    use crate::toolchain::HostOs;
    use crate::workspace::{MeshoptLevel, TextureCodec};

    /// Collects every event, for assertions.
    #[derive(Default)]
    struct Recorder {
        events: Mutex<Vec<PipelineEvent>>,
    }

    impl RunSink for Recorder {
        fn event(&self, event: PipelineEvent) {
            lock(&self.events).push(event);
        }
    }

    impl Recorder {
        fn lines(&self) -> Vec<String> {
            lock(&self.events)
                .iter()
                .filter_map(|e| match e {
                    PipelineEvent::Log { entry } => Some(entry.text.clone()),
                    PipelineEvent::Run { .. } => None,
                })
                .collect()
        }

        fn wait_for_line(&self, needle: &str, timeout: Duration) -> bool {
            let start = Instant::now();
            while start.elapsed() < timeout {
                if self.lines().iter().any(|l| l.contains(needle)) {
                    return true;
                }
                thread::sleep(Duration::from_millis(20));
            }
            false
        }
    }

    fn shell_step(id: &str, script: &str, outputs: &[&str], deps: &[&str]) -> StepDef {
        StepDef {
            id: id.into(),
            name: format!("Step {id}"),
            kind: StepKind::Shell,
            command: Some("sh".into()),
            script: Some(script.into()),
            args: vec!["{script}".into(), "{root}".into()],
            inputs: vec!["in/*.txt".into()],
            outputs: outputs.iter().map(|o| o.to_string()).collect(),
            depends_on: deps.iter().map(|d| d.to_string()).collect(),
            compress: None,
        }
    }

    fn setup(dir: &Path, steps: Vec<StepDef>) -> RunSetup {
        let project_dir = dir.join("project");
        fs::create_dir_all(project_dir.join("in")).expect("mkdir");
        fs::create_dir_all(project_dir.join("tools")).expect("mkdir");
        fs::write(project_dir.join("in/a.txt"), b"input").expect("write");
        RunSetup {
            project: "moon".into(),
            project_dir,
            cache_dir: dir.join("cache"),
            def: PipelineDef {
                version: PIPELINE_VERSION,
                root: None,
                steps,
                presets: Vec::new(),
            },
            tools: Toolbox {
                path_env: OsString::from("/bin:/usr/bin"),
                os: Some(HostOs::current()),
                ..Toolbox::default()
            },
            defaults: CompressDefaults {
                texture_size: 2048,
                codec: TextureCodec::Etc1s,
                meshopt: MeshoptLevel::High,
            },
            vars: BTreeMap::new(),
            from: None,
            force: false,
        }
    }

    fn script(setup: &RunSetup, name: &str, body: &str) {
        fs::write(setup.project_dir.join("tools").join(name), body).expect("script");
    }

    fn states(run: &RunRecord) -> Vec<StepState> {
        run.steps.iter().map(|s| s.state).collect()
    }

    #[test]
    fn runs_streams_and_then_caches() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = setup(
            dir.path(),
            vec![
                shell_step("build", "tools/build.sh", &["out/build.txt"], &[]),
                shell_step("export", "tools/export.sh", &["out/export.txt"], &["build"]),
            ],
        );
        script(
            &s,
            "build.sh",
            "echo building in \"$1\"\necho oops >&2\nmkdir -p out\ncat in/a.txt > out/build.txt\n",
        );
        script(
            &s,
            "export.sh",
            "echo exporting\nmkdir -p out\necho glb > out/export.txt\n",
        );
        let sink = Arc::new(Recorder::default());
        let runner = PipelineRunner::new(sink.clone());

        let first = runner.run_blocking(s.clone()).expect("first run");
        assert_eq!(first.state, RunState::Done, "{:?}", first.message);
        assert_eq!(states(&first), vec![StepState::Done, StepState::Done]);
        assert_eq!(first.number, 1);
        let lines = sink.lines();
        assert!(
            lines.iter().any(|l| l.starts_with("building in /")),
            "{lines:?}"
        );
        assert!(lines.iter().any(|l| l == "oops"));
        assert!(lines.iter().any(|l| l == "exporting"));

        let second = runner.run_blocking(s.clone()).expect("second run");
        assert_eq!(second.number, 2);
        assert_eq!(states(&second), vec![StepState::Cached, StepState::Cached]);

        // A changed input reruns the step that reads it; the next one sees
        // the same input bytes it had before only if the output is unchanged.
        fs::write(s.project_dir.join("in/a.txt"), b"changed").expect("write");
        let third = runner.run_blocking(s.clone()).expect("third run");
        assert_eq!(states(&third), vec![StepState::Done, StepState::Done]);

        let mut forced = s.clone();
        forced.force = true;
        let fourth = runner.run_blocking(forced).expect("forced");
        assert_eq!(states(&fourth), vec![StepState::Done, StepState::Done]);

        let mut from = s.clone();
        from.from = Some("export".into());
        let fifth = runner.run_blocking(from).expect("from here");
        assert_eq!(states(&fifth), vec![StepState::Skipped, StepState::Done]);

        let runs = history::load_runs(&s.cache_dir);
        assert_eq!(runs.len(), 5);
        assert_eq!(runs[0].number, 5);
        assert!(!history::load_log(&s.cache_dir, 5).is_empty());
    }

    #[test]
    fn progress_lines_reach_the_step_record() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = setup(
            dir.path(),
            vec![shell_step("bake", "tools/bake.sh", &["out/x.txt"], &[])],
        );
        script(
            &s,
            "bake.sh",
            "echo '[bake] progress 1/4 rock'\nsleep 0.3\necho '[bake] progress 3/4 tree'\nsleep 0.3\nmkdir -p out\necho x > out/x.txt\n",
        );
        let sink = Arc::new(Recorder::default());
        let run = PipelineRunner::new(sink.clone())
            .run_blocking(s)
            .expect("run");
        assert_eq!(run.state, RunState::Done);
        assert_eq!(run.steps[0].progress, Some(1.0));
        let seen: Vec<f64> = lock(&sink.events)
            .iter()
            .filter_map(|e| match e {
                PipelineEvent::Run { run } if run.steps[0].state == StepState::Running => {
                    run.steps[0].progress
                }
                _ => None,
            })
            .collect();
        assert!(seen.contains(&0.25), "{seen:?}");
        assert!(seen.contains(&0.75), "{seen:?}");
    }

    #[test]
    fn exit_codes_fail_the_run_and_skip_the_rest() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = setup(
            dir.path(),
            vec![
                shell_step("build", "tools/build.sh", &["out/build.txt"], &[]),
                shell_step("export", "tools/export.sh", &["out/export.txt"], &["build"]),
            ],
        );
        script(
            &s,
            "build.sh",
            "echo 'Error: camera poster not found'\nexit 3\n",
        );
        script(&s, "export.sh", "echo never\n");
        let sink = Arc::new(Recorder::default());
        let runner = PipelineRunner::new(sink.clone());
        let run = runner.run_blocking(s).expect("run");
        assert_eq!(run.state, RunState::Failed);
        assert_eq!(states(&run), vec![StepState::Failed, StepState::Skipped]);
        assert_eq!(run.steps[0].exit_code, Some(3));
        assert!(
            run.message
                .as_deref()
                .unwrap_or_default()
                .contains("exited with code 3")
        );
        let errors: Vec<LogEntry> = lock(&sink.events)
            .iter()
            .filter_map(|e| match e {
                PipelineEvent::Log { entry } if entry.level == LogLevel::Error => {
                    Some(entry.clone())
                }
                _ => None,
            })
            .collect();
        assert!(errors.iter().any(|e| e.text.contains("camera poster")));
        assert!(!sink.lines().iter().any(|l| l == "never"));
    }

    #[test]
    fn a_step_that_writes_nothing_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = setup(
            dir.path(),
            vec![shell_step("build", "tools/build.sh", &["out/*.glb"], &[])],
        );
        script(&s, "build.sh", "echo done\n");
        let run = PipelineRunner::new(Arc::new(NullSink))
            .run_blocking(s)
            .expect("run");
        assert_eq!(run.state, RunState::Failed);
        assert!(
            run.steps[0]
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("wrote nothing to out/*.glb")
        );
    }

    #[test]
    fn missing_script_and_missing_tool_fail_cleanly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut blender = shell_step("bake", "tools/bake.py", &["out/x"], &[]);
        blender.kind = StepKind::Blender;
        blender.command = None;
        blender.args = vec![
            "-b".into(),
            "x.blend".into(),
            "--python".into(),
            "{script}".into(),
        ];
        let s = setup(dir.path(), vec![blender]);
        let run = PipelineRunner::new(Arc::new(NullSink))
            .run_blocking(s.clone())
            .expect("run");
        assert_eq!(run.state, RunState::Failed);
        assert!(
            run.steps[0]
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("Blender is not installed")
        );

        let s = setup(
            dir.path(),
            vec![shell_step("build", "tools/nope.sh", &["out/x"], &[])],
        );
        let run = PipelineRunner::new(Arc::new(NullSink))
            .run_blocking(s)
            .expect("run");
        assert!(
            run.steps[0]
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("tools/nope.sh is missing")
        );
    }

    #[test]
    fn blender_without_background_is_refused_before_spawning() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut step = shell_step("build", "tools/build.py", &["out/x"], &[]);
        step.kind = StepKind::Blender;
        step.command = None;
        step.args = vec!["scene.blend".into(), "--python".into(), "{script}".into()];
        let s = setup(dir.path(), vec![step]);
        let err = PipelineRunner::new(Arc::new(NullSink))
            .submit(s)
            .expect_err("refused");
        assert!(err.to_string().contains("always run headless"));
    }

    #[test]
    fn cancel_kills_the_process_tree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = setup(
            dir.path(),
            vec![shell_step("build", "tools/build.sh", &["out/x"], &[])],
        );
        // A grandchild that would write a marker after a second, and a long
        // sleep in the step itself.
        script(
            &s,
            "build.sh",
            "(sleep 1; echo late > \"$1/marker\") &\necho started\nsleep 30\n",
        );
        let sink = Arc::new(Recorder::default());
        let runner = PipelineRunner::new(sink.clone());
        let started = Instant::now();
        let run = runner.submit(s.clone()).expect("submit");
        assert!(sink.wait_for_line("started", Duration::from_secs(10)));
        // A second run queues behind the first and is cancelled while queued.
        let queued = runner.submit(s.clone()).expect("queue");
        assert_eq!(queued.state, RunState::Queued);
        runner.cancel(&queued.id).expect("cancel queued");
        runner.cancel(&run.id).expect("cancel");
        loop {
            if runner.active("moon").0.is_none() {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "cancel did not stop the run"
            );
            thread::sleep(Duration::from_millis(20));
        }
        assert!(started.elapsed() < Duration::from_secs(10));
        thread::sleep(Duration::from_millis(1500));
        assert!(
            !s.project_dir.join("marker").exists(),
            "the grandchild survived"
        );
        let runs = history::load_runs(&s.cache_dir);
        let first = runs.iter().find(|r| r.id == run.id).expect("first");
        assert_eq!(first.state, RunState::Cancelled);
        assert_eq!(first.steps[0].state, StepState::Cancelled);
        let second = runs.iter().find(|r| r.id == queued.id).expect("second");
        assert_eq!(second.state, RunState::Cancelled);
        assert_eq!(
            runner.cancel("nope").expect_err("unknown").code(),
            "not-found"
        );
    }

    #[test]
    fn runs_queue_one_at_a_time_per_project() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = setup(
            dir.path(),
            vec![shell_step("build", "tools/build.sh", &["out/x.txt"], &[])],
        );
        script(
            &s,
            "build.sh",
            "mkdir -p out\necho start >> out/order.log\nsleep 0.3\necho end >> out/order.log\necho x > out/x.txt\n",
        );
        let runner = PipelineRunner::new(Arc::new(NullSink));
        let mut forced = s.clone();
        forced.force = true;
        let a = runner.submit(forced.clone()).expect("a");
        let b = runner.submit(forced.clone()).expect("b");
        assert_eq!(b.state, RunState::Queued);
        assert_eq!(runner.active("moon").1.len(), 1);
        while runner.active("moon").0.is_some() {
            thread::sleep(Duration::from_millis(20));
        }
        let log = fs::read_to_string(s.project_dir.join("out/order.log")).expect("log");
        assert_eq!(log, "start\nend\nstart\nend\n");
        let runs = history::load_runs(&s.cache_dir);
        assert_eq!(runs.len(), 2);
        assert!(runs.iter().all(|r| r.state == RunState::Done));
        assert_eq!((a.number, b.number), (1, 2));
    }

    #[test]
    fn a_lane_runs_up_to_its_limit_at_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = setup(
            dir.path(),
            vec![shell_step("job", "tools/job.sh", &[], &[])],
        );
        // Each job notes when it starts and ends, sleeping in between.
        script(
            &s,
            "job.sh",
            "echo \"start $(date +%s%N)\" >> order.log\nsleep 0.6\necho \"end $(date +%s%N)\" >> order.log\n",
        );
        let runner = PipelineRunner::new(Arc::new(NullSink));
        runner.set_limit("moon", 2);
        let started = Instant::now();
        let mut forced = s.clone();
        forced.force = true;
        let runs: Vec<RunRecord> = (0..3)
            .map(|_| runner.submit(forced.clone()).expect("submit"))
            .collect();
        assert_eq!(runs[2].state, RunState::Queued);
        thread::sleep(Duration::from_millis(200));
        assert_eq!(runner.running("moon").len(), 2);
        while !runner.running("moon").is_empty() || !runner.active("moon").1.is_empty() {
            thread::sleep(Duration::from_millis(20));
        }
        // Two at a time: about two rounds of 0.6 s, not three.
        let took = started.elapsed();
        assert!(took < Duration::from_millis(1700), "{took:?}");
        let log = fs::read_to_string(s.project_dir.join("order.log")).expect("log");
        assert_eq!(log.lines().filter(|l| l.starts_with("start")).count(), 3);
        let done = history::load_runs(&s.cache_dir);
        assert!(done.iter().all(|r| r.state == RunState::Done), "{done:?}");
    }

    #[test]
    fn stdin_reaches_the_program_and_timeouts_stop_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = setup(
            dir.path(),
            vec![shell_step(
                "read",
                "tools/read.sh",
                &["out/prompt.txt"],
                &[],
            )],
        );
        script(
            &s,
            "read.sh",
            "mkdir -p out\ncat > out/prompt.txt\necho \"got $(wc -l < out/prompt.txt) lines\"\n",
        );
        let runner = PipelineRunner::new(Arc::new(NullSink));
        let options = RunOptions {
            stdin: Some("Generate an image.\nA small island.\n".into()),
            ..RunOptions::default()
        };
        let run = runner.submit_with(s.clone(), options).expect("submit");
        while runner.active("moon").0.is_some() {
            thread::sleep(Duration::from_millis(20));
        }
        let done = history::load_runs(&s.cache_dir);
        assert_eq!(done[0].id, run.id);
        assert_eq!(done[0].state, RunState::Done, "{:?}", done[0].message);
        assert_eq!(
            fs::read_to_string(s.project_dir.join("out/prompt.txt")).expect("prompt"),
            "Generate an image.\nA small island.\n"
        );

        script(&s, "read.sh", "echo waiting\nsleep 30\n");
        let mut forced = s.clone();
        forced.force = true;
        let options = RunOptions {
            timeout: Some(Duration::from_millis(400)),
            ..RunOptions::default()
        };
        let started = Instant::now();
        runner.submit_with(forced, options).expect("submit");
        while runner.active("moon").0.is_some() {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(started.elapsed() < Duration::from_secs(10));
        let runs = history::load_runs(&s.cache_dir);
        assert_eq!(runs[0].state, RunState::Failed);
        assert!(
            runs[0].steps[0]
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("ran longer than 0.4s"),
            "{:?}",
            runs[0].steps[0].message
        );
    }

    /// A gate on the "ship" step that answers from a channel.
    struct Approval {
        answer: Mutex<mpsc::Receiver<GateDecision>>,
        asked: AtomicBool,
    }

    impl StepGate for Approval {
        fn applies(&self, step: &StepDef) -> bool {
            step.id == "ship"
        }
        fn waiting_message(&self, _step: &StepDef) -> String {
            "Waiting for approval".into()
        }
        fn wait(&self, _run: &RunRecord, _step: &StepDef, cancelled: &AtomicBool) -> GateDecision {
            self.asked.store(true, Ordering::SeqCst);
            loop {
                if cancelled.load(Ordering::SeqCst) {
                    return GateDecision::Deny("Cancelled.".into());
                }
                if let Ok(answer) = lock(&self.answer).recv_timeout(Duration::from_millis(20)) {
                    return answer;
                }
            }
        }
    }

    #[test]
    fn a_gate_holds_the_step_until_it_is_approved_or_denied() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = setup(
            dir.path(),
            vec![
                shell_step("build", "tools/build.sh", &["out/build.txt"], &[]),
                shell_step("ship", "tools/ship.sh", &[], &["build"]),
            ],
        );
        script(&s, "build.sh", "mkdir -p out\necho b > out/build.txt\n");
        for (round, (answer, state)) in [
            (GateDecision::Allow, RunState::Done),
            (
                GateDecision::Deny("Shipping was denied in Maquette.".into()),
                RunState::Cancelled,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            // A changed script, so the second round has something new to ship.
            script(
                &s,
                "ship.sh",
                &format!("echo shipped >> shipped.log\n# round {round}\n"),
            );
            let (tx, rx) = mpsc::channel();
            let gate = Arc::new(Approval {
                answer: Mutex::new(rx),
                asked: AtomicBool::new(false),
            });
            let sink = Arc::new(Recorder::default());
            let runner = PipelineRunner::new(sink.clone());
            let options = RunOptions {
                gate: Some(gate.clone()),
                ..RunOptions::default()
            };
            runner.submit_with(s.clone(), options).expect("submit");
            assert!(sink.wait_for_line("Waiting for approval", Duration::from_secs(10)));
            assert!(gate.asked.load(Ordering::SeqCst));
            tx.send(answer).expect("answer");
            while runner.active("moon").0.is_some() {
                thread::sleep(Duration::from_millis(20));
            }
            let run = history::load_runs(&s.cache_dir).remove(0);
            assert_eq!(run.state, state, "{:?}", run.message);
            if state == RunState::Cancelled {
                assert_eq!(run.steps[1].state, StepState::Cancelled);
                assert_eq!(
                    run.message.as_deref(),
                    Some("Shipping was denied in Maquette.")
                );
            }
        }
        // Shipped once: after the approval, never after the denial.
        let shipped = fs::read_to_string(s.project_dir.join("shipped.log")).expect("log");
        assert_eq!(shipped, "shipped\n");
    }

    #[test]
    fn a_gated_step_with_nothing_new_is_skipped_without_asking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = setup(
            dir.path(),
            vec![
                shell_step("build", "tools/build.sh", &["out/build.txt"], &[]),
                shell_step("ship", "tools/ship.sh", &[], &["build"]),
            ],
        );
        script(&s, "build.sh", "mkdir -p out\necho b > out/build.txt\n");
        script(&s, "ship.sh", "echo shipped >> shipped.log\n");
        let run_once = |answer: Option<GateDecision>| {
            let (tx, rx) = mpsc::channel();
            let gate = Arc::new(Approval {
                answer: Mutex::new(rx),
                asked: AtomicBool::new(false),
            });
            if let Some(answer) = answer {
                tx.send(answer).expect("answer");
            }
            let runner = PipelineRunner::new(Arc::new(Recorder::default()));
            let options = RunOptions {
                gate: Some(gate.clone()),
                ..RunOptions::default()
            };
            runner.submit_with(s.clone(), options).expect("submit");
            while runner.active("moon").0.is_some() {
                thread::sleep(Duration::from_millis(20));
            }
            (
                gate.asked.load(Ordering::SeqCst),
                history::load_runs(&s.cache_dir).remove(0),
            )
        };
        let (asked, run) = run_once(Some(GateDecision::Allow));
        assert!(asked);
        assert_eq!(run.steps[1].state, StepState::Done);

        // Nothing changed: no question, and Ship counts as cached.
        let (asked, run) = run_once(None);
        assert!(!asked, "the gate was asked although nothing changed");
        assert_eq!(run.state, RunState::Done);
        assert_eq!(run.steps[1].state, StepState::Cached);
        assert_eq!(run.steps[1].message.as_deref(), Some(UNCHANGED_MESSAGE));

        // A changed input means a new ship, so it asks again.
        fs::create_dir_all(s.project_dir.join("in")).expect("in");
        fs::write(s.project_dir.join("in/model.txt"), "v2").expect("input");
        let (asked, _) = run_once(Some(GateDecision::Allow));
        assert!(asked);
        let shipped = fs::read_to_string(s.project_dir.join("shipped.log")).expect("log");
        assert_eq!(shipped, "shipped\nshipped\n");
    }

    #[test]
    fn compress_expands_presets_into_gltf_transform_calls() {
        let dir = tempfile::tempdir().expect("tempdir");
        let step = StepDef {
            id: "compress".into(),
            name: "Compress".into(),
            kind: StepKind::GltfTransform,
            command: None,
            script: None,
            args: Vec::new(),
            inputs: vec!["web/raw/*.glb".into()],
            outputs: vec!["web/dist/*.glb".into()],
            depends_on: Vec::new(),
            compress: Some(CompressSpec {
                from: "web/raw".into(),
                to: "web/dist".into(),
            }),
        };
        let mut s = setup(dir.path(), vec![step]);
        s.def.presets = vec![character_preset()];
        let raw = s.project_dir.join("web/raw");
        fs::create_dir_all(&raw).expect("mkdir");
        fs::write(raw.join("character.glb"), b"glb").expect("write");
        fs::write(raw.join("bench.glb"), b"glb").expect("write");
        // A fake gltf-transform: logs its args and copies input to output.
        let fake = dir.path().join("gltf-transform");
        fs::write(&fake, "#!/bin/sh\necho \"args: $*\"\ncp \"$2\" \"$3\"\n").expect("fake");
        std::os::unix::fs::PermissionsExt::set_mode(
            &mut fs::metadata(&fake).expect("meta").permissions(),
            0o755,
        );
        let mut perms = fs::metadata(&fake).expect("meta").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        fs::set_permissions(&fake, perms).expect("chmod");
        s.tools.tools.insert(
            ToolId::GltfTransform,
            ResolvedTool {
                program: fake,
                prefix: Vec::new(),
                version: "4.5.1".into(),
            },
        );
        let sink = Arc::new(Recorder::default());
        let run = PipelineRunner::new(sink.clone())
            .run_blocking(s.clone())
            .expect("run");
        assert_eq!(
            run.state,
            RunState::Done,
            "{:?} {:?}",
            run.message,
            sink.lines()
        );
        let lines = sink.lines();
        let calls: Vec<&String> = lines.iter().filter(|l| l.starts_with("args: ")).collect();
        assert_eq!(calls.len(), 2, "{lines:?}");
        assert!(
            calls[0].contains("meshopt")
                && calls[0].contains("bench.glb")
                && calls[0].ends_with("--level high")
        );
        assert!(calls[1].contains("character.glb") && calls[1].ends_with("--level medium"));
        assert!(lines.iter().any(|l| l.contains("toktx is missing")));
        assert!(s.project_dir.join("web/dist/character.glb").is_file());
    }
}
