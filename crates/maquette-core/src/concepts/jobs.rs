//! Concept jobs: one `codex exec` per image, through the shared job runner
//! on its own lane with a configurable number of jobs at once.
//!
//! A job counts as done when the image appears at its path, whatever Codex
//! printed. When Codex finishes without copying the image out of its own
//! folder, the job looks in `<codex home>/generated_images/<session id>/`
//! (the session id is in Codex's log header) for an image made during the
//! job and copies that one. A job that runs past the timeout is stopped.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

use super::{Direction, NewConcepts, add_image, codex_args, prepare, prompt};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::{LogLevel, LogStream, RunRecord, RunState, now_ms};
use crate::pipeline::runner::{
    JobRunner, PipelineEvent, PipelineRunner, RunOptions, RunSetup, RunSink,
};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec};

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

struct Sink(Arc<Shared>);

impl RunSink for Sink {
    fn event(&self, event: PipelineEvent) {
        match event {
            PipelineEvent::Run { run } => self.run(&run),
            PipelineEvent::Log { entry } => {
                let text = entry.text.trim_end().to_string();
                let update = {
                    let mut jobs = self.0.jobs();
                    let Some(e) = jobs
                        .iter_mut()
                        .find(|e| e.job.run_id.as_deref() == Some(entry.run_id.as_str()))
                    else {
                        return;
                    };
                    if e.session.is_none() {
                        e.session = session_id(&text);
                    }
                    // The runner's own lines ("Run 3 failed") stay in the
                    // run log; the tail is what Codex printed.
                    if entry.stream == LogStream::Stderr
                        && !text.is_empty()
                        && (entry.level == LogLevel::Error || e.last_error.is_none())
                    {
                        e.last_error = Some(text.clone());
                    }
                    if entry.stream != LogStream::System && !text.is_empty() {
                        e.job.log_tail.push(text);
                        let over = e.job.log_tail.len().saturating_sub(LOG_TAIL);
                        e.job.log_tail.drain(..over);
                    }
                    let due = e.last_emit.is_none_or(|t| t.elapsed() >= EMIT_EVERY);
                    if due {
                        e.last_emit = Some(Instant::now());
                    }
                    due.then(|| e.job.clone())
                };
                if let Some(job) = update {
                    self.0.emit(ConceptEvent::Job { job });
                }
            }
        }
    }
}

impl Sink {
    fn run(&self, run: &RunRecord) {
        let Some(step) = run.steps.first() else {
            return;
        };
        let snapshot = {
            let mut jobs = self.0.jobs();
            let Some(e) = jobs.iter_mut().find(|e| e.job.id == step.id) else {
                return;
            };
            if e.job.state.is_finished() {
                return;
            }
            e.job.run_id = Some(run.id.clone());
            match run.state {
                RunState::Queued => e.job.state = ConceptJobState::Queued,
                RunState::Running => {
                    if e.job.state != ConceptJobState::Running {
                        e.job.state = ConceptJobState::Running;
                        e.job.started_at_ms = run.started_at_ms.or(Some(now_ms()));
                        e.started = Some(SystemTime::now());
                    }
                }
                _ => {}
            }
            (
                e.job.clone(),
                e.project_dir.clone(),
                e.session.clone(),
                e.last_error.clone(),
                e.started,
            )
        };
        let (mut job, project_dir, session, last_error, started) = snapshot;
        if !run.state.is_finished() {
            self.0.emit(ConceptEvent::Job { job });
            return;
        }
        let reason = step.message.clone().or_else(|| run.message.clone());
        let direction = finish(
            &mut job,
            run.state,
            reason,
            last_error,
            &project_dir,
            self.0.codex_home.as_deref(),
            session.as_deref(),
            started,
        );
        job.finished_at_ms = Some(now_ms());
        {
            let mut jobs = self.0.jobs();
            if let Some(e) = jobs.iter_mut().find(|e| e.job.id == job.id) {
                e.job = job.clone();
            }
            prune(&mut jobs);
        }
        self.0.emit(ConceptEvent::Job { job: job.clone() });
        if let Some(direction) = direction {
            self.0.emit(ConceptEvent::Direction {
                project: job.project,
                direction,
            });
        }
    }
}

fn prune(jobs: &mut VecDeque<Entry>) {
    let finished = jobs.iter().filter(|e| e.job.state.is_finished()).count();
    let mut extra = finished.saturating_sub(KEEP_FINISHED);
    jobs.retain(|e| {
        if extra > 0 && e.job.state.is_finished() {
            extra -= 1;
            false
        } else {
            true
        }
    });
}

/// `session id: 0199a213-...` from Codex's header.
fn session_id(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let at = lower.find("session id:")?;
    let id = line[at + "session id:".len()..].trim();
    let ok = !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    ok.then(|| id.to_string())
}

fn is_image(path: &Path) -> bool {
    fs::read(path)
        .ok()
        .and_then(|b| crate::inbox::images::dimensions(&b))
        .is_some()
}

/// The newest image Codex made in `session` since `since`.
fn generated_image(codex_home: &Path, session: &str, since: Option<SystemTime>) -> Option<PathBuf> {
    let dir = codex_home.join("generated_images").join(session);
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if !matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg" | "webp")) {
            continue;
        }
        let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        if since.is_some_and(|s| modified + Duration::from_secs(2) < s) {
            continue;
        }
        if best.as_ref().is_none_or(|(t, _)| modified > *t) {
            best = Some((modified, path));
        }
    }
    best.map(|(_, p)| p)
}

/// Settles a finished job: done when the image is there (copied out of
/// Codex's folder when it was left there), failed with a reason otherwise.
/// Returns the direction when an image was added.
fn finish(
    job: &mut ConceptJob,
    state: RunState,
    reason: Option<String>,
    last_error: Option<String>,
    project_dir: &Path,
    codex_home: Option<&Path>,
    session: Option<&str>,
    started: Option<SystemTime>,
) -> Option<Direction> {
    let target = project_dir.join(&job.file);
    if state == RunState::Cancelled {
        job.state = ConceptJobState::Cancelled;
        job.message = Some(reason.unwrap_or_else(|| "Cancelled.".into()));
        return None;
    }
    let mut note = None;
    if !is_image(&target)
        && let (Some(home), Some(session)) = (codex_home, session)
        && let Some(found) = generated_image(home, session, started)
    {
        if let Some(parent) = target.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::copy(&found, &target).is_ok() {
            note = Some(format!(
                "Codex left the image in its own folder; copied {}.",
                found.display()
            ));
        }
    }
    if is_image(&target) {
        match add_image(project_dir, &job.direction, job.index, &job.file) {
            Ok(direction) => {
                job.state = ConceptJobState::Done;
                if let Some(image) = direction.images.iter().find(|i| i.file == job.file) {
                    job.width = image.width;
                    job.height = image.height;
                }
                job.message = note.or_else(|| {
                    (state == RunState::Failed).then(|| {
                        format!(
                            "The image arrived, but Codex then stopped with an error: {}",
                            reason.clone().unwrap_or_default()
                        )
                    })
                });
                return Some(direction);
            }
            Err(err) => {
                job.state = ConceptJobState::Failed;
                job.message = Some(format!("The image arrived but was not recorded. {err}"));
                return None;
            }
        }
    }
    job.state = ConceptJobState::Failed;
    let last = last_error.or_else(|| {
        job.log_tail
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .cloned()
    });
    job.message = Some(match (state, reason) {
        (RunState::Done, _) => format!(
            "Codex finished without saving the image to {}. Its log shows what it did.",
            job.file
        ),
        (_, Some(reason)) => match last {
            Some(last) if !reason.contains(&last) => format!("{reason} Last line: {last}"),
            _ => reason,
        },
        (_, None) => "Codex stopped before the image arrived.".into(),
    });
    None
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
        let codex = start.tools.tools.get(&ToolId::Codex).ok_or_else(|| {
            CoreError::Unavailable(
                "Codex CLI is not installed or did not pass the tool check. Install it, run codex login, then check the tools in Settings.".into(),
            )
        })?;
        let request = &start.request;
        let (direction, files) = prepare(start.project_dir, request)?;
        let args = codex_args(start.project_dir, &request.refs);
        let stamp = now_ms() as u64;
        let mut queued = Vec::new();
        for (n, (index, file)) in files.into_iter().enumerate() {
            let id = format!("{}-{index:02}-{stamp:x}", direction.id);
            let job = ConceptJob {
                id: id.clone(),
                run_id: None,
                project: request.project.clone(),
                direction: direction.id.clone(),
                direction_name: direction.name.clone(),
                index,
                file: file.clone(),
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
                project_dir: start.project_dir.to_path_buf(),
                session: None,
                last_error: None,
                started: None,
                last_emit: None,
            });
            let out = start.project_dir.join(&file);
            #[expect(clippy::cast_possible_truncation, reason = "a batch is at most 12")]
            let text = prompt(request, n as u32 + 1, &out);
            let setup = RunSetup {
                project: LANE.to_string(),
                project_dir: start.project_dir.to_path_buf(),
                cache_dir: start.cache_dir.to_path_buf(),
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
                tools: start.tools.clone(),
                defaults: CompressDefaults {
                    texture_size: 2048,
                    codec: TextureCodec::Etc1s,
                    meshopt: MeshoptLevel::Medium,
                },
                vars: Default::default(),
                from: None,
                force: true,
            };
            let options = RunOptions {
                stdin: Some(text),
                timeout: Some(start.timeout),
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
        Ok((direction, queued))
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::concepts::{Aspect, load_direction};
    use crate::pipeline::tools::ResolvedTool;
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;

    /// A fake `codex` that reads the prompt from stdin, checks the command
    /// line, and acts out `mode`.
    fn fake_codex(dir: &Path, placeholder: &Path, home: &Path, mode: &str) -> PathBuf {
        let script = format!(
            r#"#!/bin/sh
prompt=$(cat)
out=$(printf '%s\n' "$prompt" | grep -E '^/.*\.png$' | tail -n 1)
case "$prompt" in "Generate an image."*) ;; *) echo "bad prompt start" >&2; exit 9;; esac
[ "$1" = "exec" ] || {{ echo "not exec" >&2; exit 9; }}
for last; do :; done
[ "$last" = "-" ] || {{ echo "prompt not from stdin" >&2; exit 9; }}
echo "OpenAI Codex (fake)"
echo "session id: 01fake-$$"
case "{mode}" in
  ok) mkdir -p "$(dirname "$out")"; cp "{png}" "$out"; echo "saved $out";;
  leave) mkdir -p "{home}/generated_images/01fake-$$"; cp "{png}" "{home}/generated_images/01fake-$$/exec-1.png"; echo "image ready";;
  nothing) echo "I could not make that.";;
  fail) echo "error: not signed in" >&2; exit 2;;
  slow) sleep 30;;
esac
"#,
            mode = mode,
            png = placeholder.display(),
            home = home.display()
        );
        let path = dir.join(format!("codex-{mode}"));
        fs::write(&path, script).expect("script");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        path
    }

    fn toolbox(codex: &Path) -> Toolbox {
        Toolbox {
            tools: BTreeMap::from([(
                ToolId::Codex,
                ResolvedTool {
                    program: codex.to_path_buf(),
                    prefix: Vec::new(),
                    version: "0.0.0-fake".into(),
                },
            )]),
            path_env: std::env::var_os("PATH").unwrap_or_default(),
            os: Some(crate::toolchain::HostOs::current()),
        }
    }

    fn request(direction: &str, count: u32) -> NewConcepts {
        NewConcepts {
            project: "moon".into(),
            direction: direction.into(),
            prompt: "A cosy porch at dusk.".into(),
            refs: Vec::new(),
            aspect: Aspect::Wide,
            count,
        }
    }

    struct Rig {
        _dir: tempfile::TempDir,
        root: PathBuf,
        project: PathBuf,
        cache: PathBuf,
        png: PathBuf,
        home: PathBuf,
    }

    fn rig() -> Rig {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        let project = root.join("moon");
        let cache = root.join("cache/concepts");
        let home = root.join("codex-home");
        fs::create_dir_all(&project).expect("project");
        let png = root.join("placeholder.png");
        fs::write(
            &png,
            crate::inbox::images::write_png(8, 4, &[120; 8 * 4 * 3]),
        )
        .expect("png");
        Rig {
            _dir: dir,
            root,
            project,
            cache,
            png,
            home,
        }
    }

    fn events() -> (
        Arc<Mutex<Vec<ConceptEvent>>>,
        impl Fn(ConceptEvent) + Send + Sync,
    ) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        (log, move |e| sink.lock().expect("events").push(e))
    }

    #[test]
    fn images_arrive_by_file_and_land_in_the_direction() {
        let r = rig();
        let codex = fake_codex(&r.root, &r.png, &r.home, "ok");
        let (log, on_event) = events();
        let concepts = Concepts::new(Some(r.home.clone()), 2, on_event);
        let (direction, jobs) = concepts
            .start(Start {
                project_dir: &r.project,
                cache_dir: &r.cache,
                tools: &toolbox(&codex),
                request: request("Porch at dusk", 3),
                timeout: Duration::from_secs(20),
            })
            .expect("start");
        assert_eq!(jobs.len(), 3);
        assert_eq!(direction.id, "porch-at-dusk");
        concepts.wait();
        let done = concepts.jobs(Some("moon"));
        assert!(
            done.iter().all(|j| j.state == ConceptJobState::Done),
            "{done:#?}"
        );
        assert_eq!((done[0].width, done[0].height), (Some(8), Some(4)));
        assert!(done[0].log_tail.iter().any(|l| l.starts_with("saved ")));
        let stored = load_direction(&r.project, "porch-at-dusk")
            .expect("load")
            .expect("direction");
        assert_eq!(stored.images.len(), 3);
        assert!(
            r.project
                .join("concepts/porch-at-dusk/porch-at-dusk_03.png")
                .is_file()
        );
        let events = log.lock().expect("events");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ConceptEvent::Direction { .. }))
        );
        assert!(events.iter().any(
            |e| matches!(e, ConceptEvent::Job { job } if job.state == ConceptJobState::Running)
        ));
    }

    #[test]
    fn an_image_left_in_codex_home_is_copied_out() {
        let r = rig();
        let codex = fake_codex(&r.root, &r.png, &r.home, "leave");
        let concepts = Concepts::new(Some(r.home.clone()), 1, |_| {});
        concepts
            .start(Start {
                project_dir: &r.project,
                cache_dir: &r.cache,
                tools: &toolbox(&codex),
                request: request("Lantern", 1),
                timeout: Duration::from_secs(20),
            })
            .expect("start");
        concepts.wait();
        let job = &concepts.jobs(None)[0];
        assert_eq!(job.state, ConceptJobState::Done, "{job:#?}");
        assert!(job.message.as_deref().is_some_and(|m| m.contains("copied")));
        assert!(r.project.join("concepts/lantern/lantern_01.png").is_file());
    }

    #[test]
    fn failures_and_timeouts_say_what_happened() {
        let r = rig();
        let concepts = Concepts::new(Some(r.home.clone()), 3, |_| {});
        for (mode, direction, timeout) in [
            ("nothing", "Empty", 20),
            ("fail", "Broken", 20),
            ("slow", "Slow", 1),
        ] {
            let codex = fake_codex(&r.root, &r.png, &r.home, mode);
            concepts
                .start(Start {
                    project_dir: &r.project,
                    cache_dir: &r.cache,
                    tools: &toolbox(&codex),
                    request: request(direction, 1),
                    timeout: Duration::from_secs(timeout),
                })
                .expect("start");
        }
        concepts.wait();
        let jobs = concepts.jobs(Some("moon"));
        let message = |d: &str| {
            let job = jobs.iter().find(|j| j.direction == d).expect("job");
            assert_eq!(job.state, ConceptJobState::Failed, "{job:#?}");
            job.message.clone().unwrap_or_default()
        };
        assert!(message("empty").contains("without saving the image"));
        let broken = message("broken");
        assert!(broken.contains("exited with code 2"), "{broken}");
        assert!(broken.contains("not signed in"), "{broken}");
        assert!(message("slow").contains("ran longer than"));
    }

    #[test]
    fn queued_jobs_can_be_cancelled() {
        let r = rig();
        let codex = fake_codex(&r.root, &r.png, &r.home, "slow");
        let concepts = Concepts::new(Some(r.home.clone()), 1, |_| {});
        let (_, jobs) = concepts
            .start(Start {
                project_dir: &r.project,
                cache_dir: &r.cache,
                tools: &toolbox(&codex),
                request: request("Slow", 2),
                timeout: Duration::from_secs(60),
            })
            .expect("start");
        concepts.cancel(&jobs[1].id).expect("cancel queued");
        assert_eq!(concepts.cancel_all("moon"), 1);
        concepts.wait();
        assert!(
            concepts
                .jobs(None)
                .iter()
                .all(|j| j.state == ConceptJobState::Cancelled)
        );
        concepts.clear_finished("moon");
        assert!(concepts.jobs(None).is_empty());
        // Without Codex nothing starts.
        let err = concepts
            .start(Start {
                project_dir: &r.project,
                cache_dir: &r.cache,
                tools: &Toolbox::default(),
                request: request("Slow", 1),
                timeout: Duration::from_secs(60),
            })
            .expect_err("no codex");
        assert!(err.to_string().contains("Codex CLI"));
    }

    #[test]
    fn reads_the_session_id_from_the_header() {
        assert_eq!(
            session_id("session id: 01a16a3c-fd0b-7983-bfe5-5ec6b314d77e").as_deref(),
            Some("01a16a3c-fd0b-7983-bfe5-5ec6b314d77e")
        );
        assert_eq!(session_id("Session ID: ../etc"), None);
        assert_eq!(session_id("workdir: /tmp"), None);
    }
}
