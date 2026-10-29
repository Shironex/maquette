//! Concept jobs: one `codex exec` per image, through the shared job runner
//! on its own lane with a configurable number of jobs at once.
//!
//! A job counts as done when the image appears at its path, whatever Codex
//! printed. When Codex finishes without copying the image out of its own
//! folder, the job looks in `<codex home>/generated_images/<session id>/`
//! (the session id is in Codex's log header) for an image made during the
//! job and copies that one. A job that runs past the timeout is stopped.
//! A job Codex rate limits waits and runs again on its own a few times
//! ([`Backoff`]). When Codex says its usage limit is hit, the job and every
//! queued one wait until the time Codex names and then run on their own.
//! Any failed job can run again with [`Concepts::retry`].
//!
//! A batch can instead run on OpenAI's Image API ([`ApiPlan`]): the same
//! prompt and attached images go to the API, the image lands at the same
//! file and the job settles the same way, only without Codex ([`api`]).

mod api;
mod approval;
mod codex_run;
mod persist;
mod priority;
mod retry;
mod settle;
mod sink;
mod start;
#[cfg(all(test, unix))]
mod tests;
mod usage;
mod usage_hold;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

use super::backend::{ApiPlan, ImageBackend};
use super::style::StyleBoard;
use super::{Direction, NewConcepts, attached, codex_args};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::now_ms;
use crate::pipeline::runner::{JobRunner, PipelineRunner, RunOptions, RunSetup, WeakRunner};
use crate::pipeline::tools::Toolbox;
use crate::toolchain::ToolId;

pub use api::ImageApiClient;
use codex_run::CodexRun;
pub use retry::Backoff;
use sink::Sink;

/// The runner lane every concept job shares.
pub const LANE: &str = "concepts";
const LOG_TAIL: usize = 60;
/// Live log lines refresh a job at most this often.
const EMIT_EVERY: Duration = Duration::from_millis(700);
/// Finished jobs kept for the queue panel.
const KEEP_FINISHED: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ConceptJobState {
    /// An agent asked for it on the paid Image API: it waits for a person
    /// to approve the round in the app, and nothing is sent until then.
    Approval,
    Queued,
    Running,
    /// Codex rate limited it, or its usage limit is hit (`usage_limit`);
    /// it runs again on its own at `retry_at_ms`.
    RateLimited,
    Done,
    Failed,
    Cancelled,
}

impl ConceptJobState {
    pub fn is_finished(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConceptJob {
    pub id: String,
    pub run_id: Option<String>,
    pub project: String,
    /// The direction id (its folder name).
    pub direction: String,
    pub direction_name: String,
    pub index: u32,
    /// Where the image goes, relative to the project folder.
    pub file: String,
    /// Object refs: the object this image shows.
    pub title: Option<String>,
    pub state: ConceptJobState,
    pub queued_at_ms: f64,
    pub started_at_ms: Option<f64>,
    pub finished_at_ms: Option<f64>,
    /// Why it failed or was stopped, or a note on how the image arrived.
    pub message: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// The last lines Codex printed.
    pub log_tail: Vec<String>,
    /// Rate limited: when it runs again, ms since the epoch.
    pub retry_at_ms: Option<f64>,
    /// How often it ran again on its own after a rate limit.
    pub retries: u32,
    /// Waiting: the wait is Codex's usage limit, not a rate limit.
    pub usage_limit: bool,
    /// The style board revision the job carries, `None` when it carries
    /// none. A retry runs with the same board.
    pub style_revision: Option<u32>,
    /// Codex, or OpenAI's Image API. A retry runs on the same one.
    #[serde(default)]
    pub backend: ImageBackend,
    /// Failed because OpenAI refused the image on safety grounds; the
    /// message says why. Nothing was charged, and the same prompt is
    /// likely refused again.
    #[serde(default)]
    pub refused: bool,
}

/// What the service reports to the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ConceptEvent {
    Job {
        job: ConceptJob,
    },
    /// A direction changed (a new image arrived).
    Direction {
        project: String,
        direction: Direction,
    },
    /// The project's style board changed (an agent set it).
    Style {
        project: String,
        style: StyleBoard,
    },
}

/// One start request, resolved by the caller.
pub struct Start<'a> {
    pub project_dir: &'a Path,
    /// `<workspace cache>/concepts`, for the run history and logs.
    pub cache_dir: &'a Path,
    pub tools: &'a Toolbox,
    /// Already checked with [`super::clean_request`].
    pub request: NewConcepts,
    pub timeout: Duration,
    /// Runs the batch on OpenAI's Image API; `None` runs it on Codex.
    pub api: Option<ApiPlan>,
}

/// Where the jobs of a start run, shared by concepts and object refs.
pub struct Lane<'a> {
    pub project_dir: &'a Path,
    pub cache_dir: &'a Path,
    pub tools: &'a Toolbox,
    pub timeout: Duration,
    /// Runs the batch on OpenAI's Image API; `None` runs it on Codex.
    pub api: Option<ApiPlan>,
}

/// One planned image: where it goes, what Codex is told, its title, and
/// the size and ground the Image API is asked for.
struct Planned {
    index: u32,
    file: String,
    prompt: String,
    title: Option<String>,
    /// `1536x1024`, or `auto` to follow the attached image.
    size: &'static str,
    transparent: bool,
}

/// What a job's run needs, kept so a retry runs the very same thing.
#[derive(Clone)]
struct Rerun {
    setup: RunSetup,
    options: RunOptions,
}

/// How a job runs: a `codex exec` run, or one Image API request.
#[derive(Clone)]
enum Run {
    Codex(Box<Rerun>),
    Api(api::ApiJob),
}

struct Entry {
    job: ConceptJob,
    run: Run,
    /// Counts the job's runs, so a stale retry timer knows to stand down.
    generation: u64,
    project_dir: PathBuf,
    session: Option<String>,
    /// The last error Codex printed, for the failure message.
    last_error: Option<String>,
    started: Option<SystemTime>,
    last_emit: Option<Instant>,
}

type Callback = Box<dyn Fn(ConceptEvent) + Send + Sync>;

struct Shared {
    jobs: Mutex<VecDeque<Entry>>,
    on_event: Callback,
    codex_home: Option<PathBuf>,
    /// The runner, for retries started from the sink.
    runner: OnceLock<WeakRunner>,
    backoff: Mutex<Backoff>,
    /// Codex's usage limit holds until then (ms since the epoch); jobs
    /// queued meanwhile wait for it instead of running.
    usage_until: Mutex<Option<f64>>,
    /// The Image API jobs waiting and running.
    api: Mutex<api::ApiLane>,
    /// The Image API's key store and address, once the shell set them.
    image_api: Mutex<Option<Arc<ImageApiClient>>>,
    /// Where the jobs are kept across restarts, once the shell set it.
    store: Mutex<Option<persist::Store>>,
}

impl Shared {
    fn jobs(&self) -> MutexGuard<'_, VecDeque<Entry>> {
        self.jobs.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn backoff(&self) -> MutexGuard<'_, Backoff> {
        self.backoff.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn usage_until(&self) -> MutexGuard<'_, Option<f64>> {
        self.usage_until.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn api(&self) -> MutexGuard<'_, api::ApiLane> {
        self.api.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Tells the UI, and keeps the jobs when one changed state. Never
    /// called while the jobs are locked.
    fn emit(&self, event: ConceptEvent) {
        if let ConceptEvent::Job { job } = &event {
            persist::on_job(self, job);
        }
        (self.on_event)(event);
    }
}

fn codex_tool(tools: &Toolbox) -> CoreResult<&crate::pipeline::tools::ResolvedTool> {
    tools.tools.get(&ToolId::Codex).ok_or_else(|| {
        CoreError::Unavailable(
            "Codex CLI is not installed or did not pass the tool check. Install it, run codex login, then check the tools in Settings.".into(),
        )
    })
}

/// The Concept lab's job service. Cheap to clone.
#[derive(Clone)]
pub struct Concepts {
    shared: Arc<Shared>,
    runner: PipelineRunner,
}

impl Concepts {
    /// `codex_home` is `$CODEX_HOME` or `~/.codex`, where Codex keeps the
    /// images it generates.
    pub fn new(
        codex_home: Option<PathBuf>,
        concurrency: u32,
        on_event: impl Fn(ConceptEvent) + Send + Sync + 'static,
    ) -> Self {
        let shared = Arc::new(Shared {
            jobs: Mutex::new(VecDeque::new()),
            on_event: Box::new(on_event),
            codex_home,
            runner: OnceLock::new(),
            backoff: Mutex::new(Backoff::default()),
            usage_until: Mutex::new(None),
            api: Mutex::new(api::ApiLane::default()),
            image_api: Mutex::new(None),
            store: Mutex::new(None),
        });
        let runner = PipelineRunner::new(Arc::new(Sink(Arc::clone(&shared))));
        let _ = shared.runner.set(runner.downgrade());
        let concepts = Self { shared, runner };
        concepts.set_concurrency(concurrency);
        concepts
    }

    /// How many images generate at once. Raising it starts queued jobs.
    pub fn set_concurrency(&self, n: u32) {
        self.runner.set_limit(LANE, n.max(1) as usize);
        api::set_limit(&self.shared, n.max(1) as usize);
    }

    fn queue(
        &self,
        lane: &Lane<'_>,
        project: &str,
        direction: &Direction,
        refs: &[String],
        board: Option<&StyleBoard>,
        planned: Vec<Planned>,
    ) -> CoreResult<Vec<ConceptJob>> {
        let files = attached(refs, board);
        let codex = match lane.api {
            None => Some(codex_tool(lane.tools)?.program.display().to_string()),
            Some(_) => None,
        };
        let args = codex_args(lane.project_dir, &files);
        let stamp = now_ms() as u64;
        let mut queued = Vec::new();
        for Planned {
            index,
            file,
            prompt: text,
            title,
            size,
            transparent,
        } in planned
        {
            let id = format!("{}-{index:02}-{stamp:x}", direction.id);
            let job = ConceptJob {
                id: id.clone(),
                run_id: None,
                project: project.to_string(),
                direction: direction.id.clone(),
                direction_name: direction.name.clone(),
                index,
                file: file.clone(),
                title,
                state: ConceptJobState::Queued,
                queued_at_ms: now_ms(),
                started_at_ms: None,
                finished_at_ms: None,
                message: None,
                width: None,
                height: None,
                log_tail: Vec::new(),
                retry_at_ms: None,
                retries: 0,
                usage_limit: false,
                style_revision: board.map(|b| b.revision),
                backend: if lane.api.is_some() {
                    ImageBackend::OpenaiImages
                } else {
                    ImageBackend::Codex
                },
                refused: false,
            };
            let run = match (&lane.api, &codex) {
                (Some(plan), _) => Run::Api(api::ApiJob::new(
                    plan,
                    lane.project_dir,
                    &files,
                    &file,
                    &text,
                    size,
                    transparent,
                )),
                (None, codex) => Run::Codex(Box::new(
                    CodexRun {
                        cache_dir: lane.cache_dir.to_path_buf(),
                        program: codex.clone(),
                        args: args.clone(),
                        step: format!("{} #{index}", direction.name),
                        prompt: text,
                        timeout_ms: u64::try_from(lane.timeout.as_millis()).unwrap_or(u64::MAX),
                    }
                    .rerun(&id, lane.project_dir, lane.tools.clone()),
                )),
            };
            let hold = lane.api.as_ref().is_some_and(|p| p.hold);
            let held = hold.then(|| {
                let mut job = job.clone();
                job.state = ConceptJobState::Approval;
                job
            });
            self.shared.jobs().push_back(Entry {
                job: held.clone().unwrap_or(job),
                run,
                generation: 0,
                project_dir: lane.project_dir.to_path_buf(),
                session: None,
                last_error: None,
                started: None,
                last_emit: None,
            });
            match held {
                Some(job) => {
                    self.shared.emit(ConceptEvent::Job { job: job.clone() });
                    queued.push(job);
                }
                None => queued.extend(retry::submit(&self.shared, &self.runner, &id, false)),
            }
        }
        Ok(queued)
    }

    /// Jobs of `project` (every project when `None`), oldest first.
    pub fn jobs(&self, project: Option<&str>) -> Vec<ConceptJob> {
        self.shared
            .jobs()
            .iter()
            .filter(|e| project.is_none_or(|p| e.job.project == p))
            .map(|e| e.job.clone())
            .collect()
    }

    /// Stops a queued or running job.
    pub fn cancel(&self, job_id: &str) -> CoreResult<()> {
        let run = self
            .shared
            .jobs()
            .iter()
            .find(|e| e.job.id == job_id)
            .map(|e| (e.job.run_id.clone(), e.job.state));
        match run {
            Some((_, state)) if state.is_finished() => Ok(()),
            Some((_, ConceptJobState::RateLimited | ConceptJobState::Approval)) => {
                self.cancel_waiting(job_id);
                Ok(())
            }
            Some(_) if api::cancel(&self.shared, job_id) => Ok(()),
            Some((Some(run_id), _)) => self.runner.cancel(&run_id),
            _ => Err(CoreError::NotFound(format!("the concept job {job_id}"))),
        }
    }

    /// Stops every unfinished job of `project`, returning how many.
    pub fn cancel_all(&self, project: &str) -> usize {
        let (waiting, runs): (Vec<_>, Vec<_>) = self
            .shared
            .jobs()
            .iter()
            .filter(|e| e.job.project == project && !e.job.state.is_finished())
            .map(|e| (e.job.id.clone(), e.job.run_id.clone(), e.job.state))
            .partition(|(_, _, state)| {
                matches!(
                    state,
                    ConceptJobState::RateLimited | ConceptJobState::Approval
                )
            });
        let waited = waiting
            .iter()
            .filter(|(id, _, _)| self.cancel_waiting(id).is_some())
            .count();
        let stopped = runs
            .iter()
            .filter(|(id, run, _)| {
                api::cancel(&self.shared, id)
                    || run.as_ref().is_some_and(|r| self.runner.cancel(r).is_ok())
            })
            .count();
        waited + stopped
    }

    /// Drops finished jobs of `project` from the queue panel.
    pub fn clear_finished(&self, project: &str) {
        self.shared
            .jobs()
            .retain(|e| e.job.project != project || !e.job.state.is_finished());
        persist::save(&self.shared);
    }

    /// Blocks until no job is queued or running. For tests and the CLI.
    pub fn wait(&self) {
        while self
            .shared
            .jobs()
            .iter()
            .any(|e| !e.job.state.is_finished())
        {
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}
