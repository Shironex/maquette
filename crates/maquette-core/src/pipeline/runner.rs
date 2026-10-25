//! The job runner: one run at a time per project, the rest queued. Each
//! step's program is spawned directly with an args array (never through a
//! shell string), in its own process group on Unix or job object on
//! Windows (with no console window), so Cancel kills the whole tree. Output
//! lines stream to a [`RunSink`] as they arrive.

mod execution;
mod queue;
mod spawn;
mod step;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::history::{LogEntry, RunRecord};
use super::tools::Toolbox;
use super::{CompressDefaults, PipelineDef, StepDef};
use crate::error::CoreResult;

pub use queue::PipelineRunner;

const POLL: Duration = Duration::from_millis(40);

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
    /// Model file stem to the stem the compressed file ships under, from
    /// [`crate::assets::shipped_names`]. A model not listed ships as it is.
    pub shipped_names: BTreeMap<String, String>,
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

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

#[cfg(all(test, unix))]
mod tests;
