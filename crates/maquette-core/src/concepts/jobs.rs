//! Concept jobs: one `codex exec` per image, through the shared job runner
//! on its own lane with a configurable number of jobs at once.
//!
//! A job counts as done when the image appears at its path, whatever Codex
//! printed. When Codex finishes without copying the image out of its own
//! folder, the job looks in `<codex home>/generated_images/<session id>/`
//! (the session id is in Codex's log header) for an image made during the
//! job and copies that one. A job that runs past the timeout is stopped.

mod settle;
mod sink;
#[cfg(all(test, unix))]
mod tests;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

use super::objects::{self, NewObjectRefs};
use super::{Direction, NewConcepts, codex_args, prepare, prompt};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::now_ms;
use crate::pipeline::runner::{JobRunner, PipelineRunner, RunOptions, RunSetup};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec};

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
    Queued,
    Running,
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
}

/// Where the jobs of a start run, shared by concepts and object refs.
pub struct Lane<'a> {
    pub project_dir: &'a Path,
    pub cache_dir: &'a Path,
    pub tools: &'a Toolbox,
    pub timeout: Duration,
}

/// One planned image: where it goes, what Codex is told, its title.
struct Planned {
    index: u32,
    file: String,
    prompt: String,
    title: Option<String>,
}

struct Entry {
    job: ConceptJob,
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
}

impl Shared {
    fn jobs(&self) -> MutexGuard<'_, VecDeque<Entry>> {
        self.jobs.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn emit(&self, event: ConceptEvent) {
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
        });
        let runner = PipelineRunner::new(Arc::new(Sink(Arc::clone(&shared))));
        let concepts = Self { shared, runner };
        concepts.set_concurrency(concurrency);
        concepts
    }

    /// How many images generate at once. Raising it starts queued jobs.
    pub fn set_concurrency(&self, n: u32) {
        self.runner.set_limit(LANE, n.max(1) as usize);
    }

    /// Queues one Codex job per image and returns the direction and jobs.
    ///
    /// # Errors
    ///
    /// Fails when Codex CLI is missing or the direction cannot be written.
    pub fn start(&self, start: Start<'_>) -> CoreResult<(Direction, Vec<ConceptJob>)> {
        codex_tool(start.tools)?;
        let request = &start.request;
        let (direction, files) = prepare(start.project_dir, request)?;
        let planned = files
            .into_iter()
            .enumerate()
            .map(|(n, (index, file))| {
                #[expect(clippy::cast_possible_truncation, reason = "a batch is at most 12")]
                let text = prompt(request, n as u32 + 1, &start.project_dir.join(&file));
                Planned {
                    index,
                    file,
                    prompt: text,
                    title: None,
                }
            })
            .collect();
        let lane = Lane {
            project_dir: start.project_dir,
            cache_dir: start.cache_dir,
            tools: start.tools,
            timeout: start.timeout,
        };
        let jobs = self.queue(&lane, &request.project, &direction, &request.refs, planned)?;
        Ok((direction, jobs))
    }

    /// Queues one Codex job per object, each with its own prompt and the
    /// source concept attached. `request` is already checked with
    /// [`objects::clean`].
    ///
    /// # Errors
    ///
    /// Fails when Codex CLI is missing or the direction cannot be written.
    pub fn start_objects(
        &self,
        lane: &Lane<'_>,
        request: &NewObjectRefs,
    ) -> CoreResult<(Direction, Vec<ConceptJob>)> {
        codex_tool(lane.tools)?;
        let (direction, files) = objects::prepare(lane.project_dir, request)?;
        let planned = files
            .into_iter()
            .map(|p| Planned {
                index: p.index,
                prompt: objects::object_prompt(
                    &p.object,
                    &p.notes,
                    request.kind,
                    &request.source_name,
                    request.aspect,
                    &lane.project_dir.join(&p.file),
                ),
                file: p.file,
                title: Some(p.object),
            })
            .collect();
        let refs = [request.source.clone()];
        let jobs = self.queue(lane, &request.project, &direction, &refs, planned)?;
        Ok((direction, jobs))
    }

    fn queue(
        &self,
        lane: &Lane<'_>,
        project: &str,
        direction: &Direction,
        refs: &[String],
        planned: Vec<Planned>,
    ) -> CoreResult<Vec<ConceptJob>> {
        let codex = codex_tool(lane.tools)?;
        let args = codex_args(lane.project_dir, refs);
        let stamp = now_ms() as u64;
        let mut queued = Vec::new();
        for Planned {
            index,
            file,
            prompt: text,
            title,
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
            };
            self.shared.jobs().push_back(Entry {
                job: job.clone(),
                project_dir: lane.project_dir.to_path_buf(),
                session: None,
                last_error: None,
                started: None,
                last_emit: None,
            });
            let setup = RunSetup {
                project: LANE.to_string(),
                project_dir: lane.project_dir.to_path_buf(),
                cache_dir: lane.cache_dir.to_path_buf(),
                def: PipelineDef {
                    version: PIPELINE_VERSION,
                    root: None,
                    steps: vec![StepDef {
                        id: id.clone(),
                        name: format!("{} #{index}", direction.name),
                        kind: StepKind::Shell,
                        command: Some(codex.program.display().to_string()),
                        script: None,
                        args: args.clone(),
                        inputs: Vec::new(),
                        outputs: Vec::new(),
                        depends_on: Vec::new(),
                        compress: None,
                    }],
                    presets: Vec::new(),
                },
                tools: lane.tools.clone(),
                defaults: CompressDefaults {
                    texture_size: 2048,
                    codec: TextureCodec::Etc1s,
                    meshopt: MeshoptLevel::Medium,
                },
                vars: Default::default(),
                shipped_names: Default::default(),
                from: None,
                force: true,
            };
            let options = RunOptions {
                stdin: Some(text),
                timeout: Some(lane.timeout),
                gate: None,
            };
            match self.runner.submit_with(setup, options) {
                Ok(record) => {
                    let mut jobs = self.shared.jobs();
                    if let Some(e) = jobs.iter_mut().find(|e| e.job.id == id) {
                        e.job.run_id.get_or_insert(record.id);
                        queued.push(e.job.clone());
                    }
                }
                Err(err) => {
                    let failed = {
                        let mut jobs = self.shared.jobs();
                        jobs.iter_mut().find(|e| e.job.id == id).map(|e| {
                            e.job.state = ConceptJobState::Failed;
                            e.job.finished_at_ms = Some(now_ms());
                            e.job.message = Some(err.to_string());
                            e.job.clone()
                        })
                    };
                    if let Some(job) = failed {
                        self.shared.emit(ConceptEvent::Job { job: job.clone() });
                        queued.push(job);
                    }
                }
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
            Some((Some(run_id), _)) => self.runner.cancel(&run_id),
            _ => Err(CoreError::NotFound(format!("the concept job {job_id}"))),
        }
    }

    /// Stops every unfinished job of `project`, returning how many.
    pub fn cancel_all(&self, project: &str) -> usize {
        let runs: Vec<String> = self
            .shared
            .jobs()
            .iter()
            .filter(|e| e.job.project == project && !e.job.state.is_finished())
            .filter_map(|e| e.job.run_id.clone())
            .collect();
        runs.iter()
            .filter(|r| self.runner.cancel(r).is_ok())
            .count()
    }

    /// Drops finished jobs of `project` from the queue panel.
    pub fn clear_finished(&self, project: &str) {
        self.shared
            .jobs()
            .retain(|e| e.job.project != project || !e.job.state.is_finished());
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
