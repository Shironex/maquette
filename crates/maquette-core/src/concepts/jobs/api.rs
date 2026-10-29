//! Concept jobs on OpenAI's Image API: their own small queue (as many at
//! once as the Lab's concurrency), one request per job on a thread of its
//! own, and the same settling as a Codex job: the image lands at the job's
//! file and joins its direction. Every image made is written to the
//! credits ledger in dollars, and a job that would take the month past the
//! cap fails before it is sent. A 429 waits as `Retry-After` asks; one that
//! keeps coming waits with the job [`Backoff`](super::Backoff) like a
//! Codex rate limit.

#[cfg(all(test, unix))]
mod tests;

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::retry;
use super::settle::finish;
use super::sink::prune;
use super::{ConceptEvent, ConceptJob, ConceptJobState, Run, Shared};
use crate::concepts::backend::ApiPlan;
use crate::concepts::codex::save_instructions;
use crate::concepts::resolve_ref;
use crate::pipeline::history::{RunState, now_ms};
use crate::providers::http::{Api, ApiError, StopFlag, Timing};
use crate::providers::ledger::{self, ImageSpend, month_to_date};
use crate::providers::openai_images::{self, ImageCall, Made, SERVICE};
use crate::providers::secrets::{OPENAI_ACCOUNT, SecretStore};

/// Where the Image API is and the store its key is in, set by the shell.
pub struct ImageApiClient {
    pub secrets: Arc<dyn SecretStore>,
    pub base: String,
    pub timing: Timing,
}

/// One job's request, kept so a retry sends the very same one.
#[derive(Clone)]
pub(super) struct ApiJob {
    pub(super) call: ImageCall,
    pub(super) plan: ApiPlan,
}

impl ApiJob {
    /// The request for the image at `file`: Codex's prompt without its
    /// save instructions, and the same attached images in the same order.
    pub(super) fn new(
        plan: &ApiPlan,
        project_dir: &Path,
        attached: &[String],
        file: &str,
        prompt: &str,
        size: &str,
        transparent: bool,
    ) -> Self {
        let out = project_dir.join(file);
        let prompt = prompt
            .strip_suffix(&save_instructions(&out))
            .unwrap_or(prompt)
            .trim_end()
            .to_string();
        Self {
            call: ImageCall {
                prompt,
                images: attached
                    .iter()
                    .map(|r| resolve_ref(project_dir, r))
                    .collect(),
                size: size.to_string(),
                quality: plan.quality,
                transparent,
            },
            plan: plan.clone(),
        }
    }

    pub(super) fn estimate(&self) -> f64 {
        self.plan.estimate_usd(1, self.call.images.len())
    }
}

/// The jobs waiting for a slot and the ones running, with their stop flags.
#[derive(Default)]
pub(super) struct ApiLane {
    waiting: VecDeque<String>,
    running: HashMap<String, Arc<StopFlag>>,
    limit: usize,
}

pub(super) fn set_limit(shared: &Arc<Shared>, limit: usize) {
    shared.api().limit = limit.max(1);
    pump(shared);
}

/// Puts a queued job in line and starts it when a slot is free.
pub(super) fn enqueue(shared: &Arc<Shared>, id: &str) {
    {
        let mut lane = shared.api();
        if !lane.waiting.iter().any(|w| w == id) && !lane.running.contains_key(id) {
            lane.waiting.push_back(id.to_string());
        }
    }
    pump(shared);
}

/// Moves the waiting jobs among `ids` to the front of the line, in the
/// order they were waiting. Returns how many moved.
pub(super) fn run_next(shared: &Shared, ids: &[String]) -> usize {
    let mut lane = shared.api();
    let (mut first, rest): (VecDeque<String>, VecDeque<String>) =
        lane.waiting.drain(..).partition(|id| ids.contains(id));
    let moved = first.len();
    first.extend(rest);
    lane.waiting = first;
    moved
}

fn pump(shared: &Arc<Shared>) {
    loop {
        let next = {
            let mut lane = shared.api();
            if lane.running.len() >= lane.limit.max(1) {
                return;
            }
            let Some(id) = lane.waiting.pop_front() else {
                return;
            };
            let stop = Arc::new(StopFlag::default());
            lane.running.insert(id.clone(), Arc::clone(&stop));
            (id, stop)
        };
        let (id, stop) = next;
        let worker = Arc::clone(shared);
        let thread_id = id.clone();
        let spawned = std::thread::Builder::new()
            .name("concept-image-api".into())
            .spawn(move || {
                run(&worker, &thread_id, &stop);
                worker.api().running.remove(&thread_id);
                pump(&worker);
            });
        if let Err(err) = spawned {
            shared.api().running.remove(&id);
            tracing::warn!(error = %err, "an Image API job did not start");
            return;
        }
    }
}

/// Stops an Image API job: a waiting one is cancelled now, a running one
/// when its request returns. False when the job is not one of them.
/// A running one settles as cancelled at once; its request may still be
/// charged, so its ledger line is written when it returns.
pub(super) fn cancel(shared: &Arc<Shared>, id: &str) -> bool {
    let running = {
        let mut lane = shared.api();
        if let Some(stop) = lane.running.get(id) {
            stop.stop();
            true
        } else {
            let before = lane.waiting.len();
            lane.waiting.retain(|w| w != id);
            if lane.waiting.len() == before {
                return false;
            }
            false
        }
    };
    let job = {
        let mut jobs = shared.jobs();
        let Some(e) = jobs.iter_mut().find(|e| e.job.id == id) else {
            return true;
        };
        e.generation += 1;
        e.job.state = ConceptJobState::Cancelled;
        e.job.finished_at_ms = Some(now_ms());
        e.job.message = Some(if running {
            "Cancelled. The request was already sent, so OpenAI may still charge it.".into()
        } else {
            "Cancelled.".into()
        });
        e.job.clone()
    };
    shared.emit(ConceptEvent::Job { job });
    true
}

/// The job's request and folder, marked running; `None` when it was
/// cancelled or dropped in the meantime.
fn begin(shared: &Shared, id: &str) -> Option<(ApiJob, PathBuf, ConceptJob, u64)> {
    let mut jobs = shared.jobs();
    let e = jobs.iter_mut().find(|e| e.job.id == id)?;
    let Run::Api(api) = &e.run else { return None };
    if e.job.state != ConceptJobState::Queued {
        return None;
    }
    e.job.state = ConceptJobState::Running;
    e.job.started_at_ms = Some(now_ms());
    e.job.run_id = None;
    Some((
        api.clone(),
        e.project_dir.clone(),
        e.job.clone(),
        e.generation,
    ))
}

/// Fails the job when its image would take the month past the cap.
fn check_cap(job: &ApiJob) -> Result<(), String> {
    let spent = ledger::read(&job.plan.ledger_root)
        .map(|l| month_to_date(&l.images, now_ms()))
        .map_err(|e| format!("Could not read the credits ledger, so nothing was sent. {e}"))?;
    let cap = f64::from(job.plan.monthly_cap_usd);
    let estimate = job.estimate();
    if spent + estimate > cap {
        return Err(format!(
            "Not sent: this image (about ${estimate:.2}) would pass the monthly OpenAI image cap, ${spent:.2} of ${cap:.0} spent this month. Raise the cap in Settings > Image generation, or make it with Codex."
        ));
    }
    Ok(())
}

/// Sends the request once the key and the cap allow it.
fn send(shared: &Shared, job: &ApiJob, stop: &StopFlag) -> Result<Made, ApiError> {
    let client = shared
        .image_api
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
        .ok_or_else(|| ApiError::Malformed("the Image API is not set up in this app.".into()))?;
    let key = client
        .secrets
        .get_account(OPENAI_ACCOUNT)
        .ok()
        .flatten()
        .ok_or(ApiError::Auth)?;
    let api = Api::new(SERVICE, &client.base, &key, client.timing)?;
    openai_images::make(&api, stop, &job.call)
}

/// Writes the image. Err holds the failure message.
fn save(made: &Made, entry: &ConceptJob, project_dir: &Path) -> Result<(), String> {
    let target = project_dir.join(&entry.file);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&target, &made.bytes)
        .map_err(|e| format!("The image arrived but could not be saved. {e}"))
}

#[expect(clippy::cast_precision_loss, reason = "token counts stay small")]
fn tokens(n: u64) -> f64 {
    n as f64
}

/// Writes what the image cost to the credits ledger.
fn record(made: &Made, job: &ApiJob, entry: &ConceptJob) {
    let mut line = ImageSpend::new(now_ms(), openai_images::MODEL, job.call.quality, made.usd);
    line.project.clone_from(&entry.project);
    line.job.clone_from(&entry.id);
    line.file.clone_from(&entry.file);
    line.estimated = made.estimated;
    if let Some(usage) = made.usage {
        line.input_tokens = Some(tokens(usage.input_tokens));
        line.output_tokens = Some(tokens(usage.output_tokens));
    }
    line.via_agent = job.plan.via_agent;
    if let Err(err) = ledger::append(&job.plan.ledger_root, &line) {
        tracing::warn!(error = %err, "an Image API spend was not written to the ledger");
    }
}

/// True while the job is still the run that started.
fn current(shared: &Shared, id: &str, generation: u64) -> bool {
    shared
        .jobs()
        .iter()
        .any(|e| e.job.id == id && e.generation == generation)
}

/// Runs one job to its end and settles it like a Codex run.
fn run(shared: &Arc<Shared>, id: &str, stop: &StopFlag) {
    let Some((job, project_dir, mut entry, generation)) = begin(shared, id) else {
        return;
    };
    shared.emit(ConceptEvent::Job { job: entry.clone() });
    let mut refused = false;
    let outcome = check_cap(&job).and_then(|()| {
        send(shared, &job, stop).map_err(|e| match e {
            ApiError::Stopped => String::new(),
            ApiError::RateLimited => "rate-limited".to_string(),
            e => {
                refused = openai_images::refusal(&e).is_some();
                openai_images::error_message(&e)
            }
        })
    });
    if let Ok(made) = &outcome {
        record(made, &job, &entry);
    }
    if !current(shared, id, generation) {
        // Cancelled while the request ran: the spend is recorded, the
        // image is not kept.
        return;
    }
    let mut wait = None;
    let mut direction = None;
    match outcome {
        Ok(made) => match save(&made, &entry, &project_dir) {
            Ok(()) => {
                direction = finish(
                    &mut entry,
                    RunState::Done,
                    None,
                    None,
                    &project_dir,
                    None,
                    None,
                    None,
                );
                if entry.state == ConceptJobState::Done && entry.message.is_none() {
                    let about = if made.estimated { "about " } else { "" };
                    entry.message = Some(format!(
                        "Made with OpenAI Images ({}), {about}${:.3}.",
                        job.call.quality.as_str(),
                        made.usd
                    ));
                }
            }
            Err(message) => fail(&mut entry, message),
        },
        Err(message) if message.is_empty() => {
            entry.state = ConceptJobState::Cancelled;
            entry.message = Some("Cancelled.".into());
        }
        Err(message) if message == "rate-limited" => {
            entry.state = ConceptJobState::Failed;
            wait = retry::rate_limited(&mut entry, &shared.backoff());
            entry.message = entry.message.map(|m| m.replace("Codex", SERVICE));
        }
        Err(message) => {
            fail(&mut entry, message);
            entry.refused = refused;
        }
    }
    entry.finished_at_ms = Some(now_ms());
    let current = {
        let mut jobs = shared.jobs();
        let current = jobs
            .iter_mut()
            .find(|e| e.job.id == id && e.generation == generation)
            .map(|e| e.job = entry.clone())
            .is_some();
        prune(&mut jobs);
        current
    };
    if !current {
        return;
    }
    if let Some(wait) = wait {
        retry::schedule(Arc::clone(shared), entry.id.clone(), generation, wait);
    }
    shared.emit(ConceptEvent::Job { job: entry.clone() });
    if let Some(direction) = direction {
        shared.emit(ConceptEvent::Direction {
            project: entry.project,
            direction,
        });
    }
}

fn fail(entry: &mut ConceptJob, message: String) {
    entry.state = ConceptJobState::Failed;
    entry.message = Some(message);
}

impl super::Concepts {
    /// Sets where the Image API is and where its key is kept.
    pub fn set_image_api(&self, client: ImageApiClient) {
        *self
            .shared
            .image_api
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(Arc::new(client));
    }

    /// What the Image API jobs still queued or running will likely cost.
    pub fn api_pending_usd(&self) -> f64 {
        self.shared
            .jobs()
            .iter()
            .filter(|e| !e.job.state.is_finished())
            .filter_map(|e| match &e.run {
                Run::Api(api) => Some(api.estimate()),
                Run::Codex(_) => None,
            })
            .sum()
    }

    /// Refuses a batch of `images` images, `attached` inputs each, that
    /// would take the month past the cap, counting the jobs still pending.
    ///
    /// # Errors
    ///
    /// [`crate::CoreError::Invalid`] with the numbers when it would.
    pub fn check_api_cap(
        &self,
        plan: &ApiPlan,
        images: usize,
        attached: usize,
    ) -> crate::CoreResult<()> {
        let spent = month_to_date(&ledger::read(&plan.ledger_root)?.images, now_ms());
        let pending = self.api_pending_usd();
        let more = plan.estimate_usd(images, attached);
        let cap = f64::from(plan.monthly_cap_usd);
        if spent + pending + more > cap {
            return Err(crate::CoreError::Invalid(format!(
                "This round of {images} would pass the monthly OpenAI image cap: ${spent:.2} spent this month, ${pending:.2} still queued and about ${more:.2} more against a cap of ${cap:.0}. Raise the cap in Settings > Image generation, make fewer images or use Codex."
            )));
        }
        Ok(())
    }
}
