//! Lab jobs that outlive the app. Every queued, running, waiting and failed
//! job is kept in one JSON file with what its run needs (the codex call, or
//! the Image API request), written whenever a job joins, changes state or
//! leaves, never on a log line. On the next start the jobs come back:
//! one whose image already landed is done, the rest are failed with a
//! message that says so, and Retry runs them again with the same prompt and
//! file. Nothing runs again on its own, so an Image API job never spends
//! without a person or an agent asking.

#[cfg(all(test, unix))]
mod tests;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;

use serde::{Deserialize, Serialize};

use super::codex_run::CodexRun;
use super::settle::finish;
use super::{ConceptEvent, ConceptJob, ConceptJobState, Concepts, Entry, Run, Shared, api};
use crate::concepts::backend::{ApiPlan, ImageJobKind};
use crate::pipeline::history::{RunState, now_ms};
use crate::pipeline::tools::Toolbox;
use crate::providers::openai_images::{ImageCall, ImageQuality};
use crate::toolchain::HostOs;

/// The file's shape; a newer one is read as empty.
const VERSION: u32 = 1;

/// Where the jobs are kept and the states last written.
pub(super) struct Store {
    path: PathBuf,
    saved: HashMap<String, ConceptJobState>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    version: u32,
    jobs: Vec<Kept>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Kept {
    job: ConceptJob,
    project_dir: PathBuf,
    run: KeptRun,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "kebab-case")]
enum KeptRun {
    Codex {
        #[serde(flatten)]
        run: CodexRun,
        /// The PATH the run gets.
        path: String,
        os: Option<HostOs>,
    },
    Api(KeptApi),
}

/// An Image API job's request and plan.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeptApi {
    prompt: String,
    images: Vec<PathBuf>,
    size: String,
    quality: ImageQuality,
    transparent: bool,
    kind: ImageJobKind,
    monthly_cap_usd: u32,
    ledger_root: PathBuf,
    via_agent: bool,
    per_image_usd: Option<f64>,
}

/// A job worth keeping: one that has not finished, or one that failed and
/// waits for Retry.
fn kept(state: ConceptJobState) -> bool {
    !state.is_finished() || state == ConceptJobState::Failed
}

impl KeptRun {
    fn of(run: &Run) -> Self {
        match run {
            Run::Codex(rerun) => Self::Codex {
                run: CodexRun::of(rerun),
                path: rerun.setup.tools.path_env.to_string_lossy().into_owned(),
                os: rerun.setup.tools.os,
            },
            Run::Api(job) => Self::Api(KeptApi {
                prompt: job.call.prompt.clone(),
                images: job.call.images.clone(),
                size: job.call.size.clone(),
                quality: job.call.quality,
                transparent: job.call.transparent,
                kind: job.plan.kind,
                monthly_cap_usd: job.plan.monthly_cap_usd,
                ledger_root: job.plan.ledger_root.clone(),
                via_agent: job.plan.via_agent,
                per_image_usd: job.plan.per_image_usd,
            }),
        }
    }

    fn run(self, id: &str, project_dir: &Path) -> Run {
        match self {
            Self::Codex { run, path, os } => {
                let tools = Toolbox {
                    path_env: path.into(),
                    os,
                    ..Toolbox::default()
                };
                Run::Codex(Box::new(run.rerun(id, project_dir, tools)))
            }
            Self::Api(api) => Run::Api(api::ApiJob {
                call: ImageCall {
                    prompt: api.prompt,
                    images: api.images,
                    size: api.size,
                    quality: api.quality,
                    transparent: api.transparent,
                },
                plan: ApiPlan {
                    kind: api.kind,
                    quality: api.quality,
                    monthly_cap_usd: api.monthly_cap_usd,
                    ledger_root: api.ledger_root,
                    via_agent: api.via_agent,
                    hold: false,
                    per_image_usd: api.per_image_usd,
                },
            }),
        }
    }
}

fn store(shared: &Shared) -> MutexGuard<'_, Option<Store>> {
    shared.store.lock().unwrap_or_else(|p| p.into_inner())
}

/// Writes the kept jobs when `job` joined, changed state or left them.
pub(super) fn on_job(shared: &Shared, job: &ConceptJob) {
    let mut guard = store(shared);
    let Some(store) = guard.as_mut() else {
        return;
    };
    let now = kept(job.state).then_some(job.state);
    if store.saved.get(&job.id).copied() != now {
        write(shared, store);
    }
}

/// Writes the kept jobs now, for a change no job event tells of.
pub(super) fn save(shared: &Shared) {
    if let Some(store) = store(shared).as_mut() {
        write(shared, store);
    }
}

fn write(shared: &Shared, store: &mut Store) {
    let jobs: Vec<Kept> = shared
        .jobs()
        .iter()
        .filter(|e| kept(e.job.state))
        .map(|e| Kept {
            job: e.job.clone(),
            project_dir: e.project_dir.clone(),
            run: KeptRun::of(&e.run),
        })
        .collect();
    store.saved = jobs
        .iter()
        .map(|k| (k.job.id.clone(), k.job.state))
        .collect();
    let file = File {
        version: VERSION,
        jobs,
    };
    let written = serde_json::to_vec_pretty(&file)
        .map_err(std::io::Error::other)
        .and_then(|bytes| {
            if let Some(parent) = store.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            crate::fs::write_atomic(&store.path, &bytes)
        });
    if let Err(err) = written {
        tracing::warn!(error = %err, path = %store.path.display(), "the Lab jobs were not saved");
    }
}

fn read(path: &Path) -> Vec<Kept> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    match serde_json::from_slice::<File>(&bytes) {
        Ok(file) if file.version <= VERSION => file.jobs,
        Ok(_) => Vec::new(),
        Err(err) => {
            tracing::warn!(error = %err, path = %path.display(), "the saved Lab jobs could not be read");
            Vec::new()
        }
    }
}

/// What a job was doing when the app closed.
fn while_it(job: &ConceptJob) -> &'static str {
    match job.state {
        ConceptJobState::Running => "while it was running",
        ConceptJobState::RateLimited if job.usage_limit => {
            "while it waited for the Codex usage limit"
        }
        ConceptJobState::RateLimited => "while it waited out a rate limit",
        ConceptJobState::Approval => "while it waited for approval, so nothing was sent",
        _ => "while it was queued",
    }
}

/// A job the app closed on: done when its image landed, else failed with
/// a message that says what happened. Returns the direction an image joined.
fn interrupted(job: &mut ConceptJob, project_dir: &Path) -> Option<crate::concepts::Direction> {
    let what = while_it(job);
    let direction = finish(
        job,
        RunState::Done,
        None,
        None,
        project_dir,
        None,
        None,
        None,
    );
    if job.state == ConceptJobState::Done {
        job.message = Some("The image arrived before the app closed.".into());
    } else {
        job.state = ConceptJobState::Failed;
        job.message = Some(format!(
            "Interrupted by a restart: the app closed {what}. Retry runs it again with the same prompt and file."
        ));
    }
    job.run_id = None;
    job.retry_at_ms = None;
    job.usage_limit = false;
    job.finished_at_ms = Some(now_ms());
    direction
}

impl Concepts {
    /// Keeps the Lab's jobs in `path` from now on, and brings back the ones
    /// kept there: done when the image landed, otherwise failed so Retry
    /// (or `retry_concept_jobs`) runs them again. Returns them.
    pub fn keep_jobs_in(&self, path: PathBuf) -> Vec<ConceptJob> {
        let mut restored = Vec::new();
        let mut directions = Vec::new();
        {
            let mut jobs = self.shared.jobs();
            for Kept {
                mut job,
                project_dir,
                run,
            } in read(&path)
            {
                if jobs.iter().any(|e| e.job.id == job.id) {
                    continue;
                }
                if !job.state.is_finished() {
                    directions.extend(
                        interrupted(&mut job, &project_dir).map(|d| (job.project.clone(), d)),
                    );
                }
                restored.push(job.clone());
                jobs.push_back(Entry {
                    run: run.run(&job.id, &project_dir),
                    job,
                    generation: 0,
                    project_dir,
                    session: None,
                    last_error: None,
                    started: None,
                    last_emit: None,
                });
            }
        }
        *store(&self.shared) = Some(Store {
            path,
            saved: HashMap::new(),
        });
        save(&self.shared);
        for job in &restored {
            self.shared.emit(ConceptEvent::Job { job: job.clone() });
        }
        for (project, direction) in directions {
            self.shared
                .emit(ConceptEvent::Direction { project, direction });
        }
        restored
    }
}
