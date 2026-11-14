//! The job runner: one run at a time per project, the rest queued. Each
//! step's program is spawned directly with an args array (never through a
//! shell string), in its own process group on Unix or job object on
//! Windows (with no console window), so Cancel kills the whole tree. Output
//! lines stream to a [`RunSink`] as they arrive.

mod execution;
mod queue;
mod smaller;
mod spawn;
mod step;
mod tweaks;

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

pub use queue::{PipelineRunner, WeakRunner};

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
    /// Material tweaks by model file stem (lowercase), from
    /// [`crate::materials::by_model_stem`]. Compress applies them to a copy
    /// of the raw export before its texture passes.
    pub materials: BTreeMap<String, crate::materials::ModelMaterials>,
    pub from: Option<String>,
    pub force: bool,
    /// The format Compress encodes textures in, from `ship.json`.
    pub textures: crate::ship::TextureFormat,
    /// The WebP and AVIF quality, 1 to 100.
    pub quality: u8,
    /// Stops before Ship: the Ship step is left out of the run.
    pub build_only: bool,
}

impl RunSetup {
    /// The site repo Ship writes into, or None when it has none.
    fn site_repo(&self) -> Option<&str> {
        self.vars
            .get("siteRepo")
            .map(|repo| repo.trim())
            .filter(|repo| !repo.is_empty())
    }

    /// Why the website Ship step `step` would fail before it copies
    /// anything ([`crate::site::ship_blocker`]), checked before anyone is
    /// asked to approve it. None for any other step, a game's Ship
    /// (`--game`), and a Ship with no repo, which the runner leaves out.
    pub fn ship_blocker_at(&self, step: &StepDef) -> Option<String> {
        let takes_repo = step.args.iter().any(|a| a.contains("{siteRepo}"));
        let game = step.args.iter().any(|a| a == "--game");
        if !super::is_ship_step(step) || !takes_repo || game {
            return None;
        }
        crate::site::ship_blocker(&self.project_dir, std::path::Path::new(self.site_repo()?))
    }

    /// [`Self::ship_blocker_at`] for the Ship step this run reaches, or None
    /// when it reaches none (`build_only`, or a `from` after it).
    pub fn ship_blocker(&self) -> Option<String> {
        let planned =
            super::graph::plan_run(&self.def, self.from.as_deref(), self.force, self.build_only)
                .ok()?;
        planned
            .iter()
            .filter(|(_, p)| *p != super::graph::Planned::Skip)
            .find_map(|(index, _)| self.ship_blocker_at(&self.def.steps[*index]))
    }
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
    /// The repo Ship writes into for this run only, in place of the
    /// project's, kept on the run's record.
    pub repo_for_run: Option<String>,
}

impl std::fmt::Debug for RunOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOptions")
            .field("stdin", &self.stdin.as_ref().map(String::len))
            .field("timeout", &self.timeout)
            .field("gate", &self.gate.is_some())
            .field("repo_for_run", &self.repo_for_run)
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
