//! Running a concept job again: on its own after Codex rate limited it
//! (with a growing wait and a capped number of tries), or when someone
//! asks for it with Retry. A retry reruns the same prompt, refs and target
//! file under the same job, so it never makes a new direction.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{ConceptEvent, ConceptJob, ConceptJobState, Concepts, Shared};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::now_ms;
use crate::pipeline::runner::PipelineRunner;

/// How long a rate limited job waits before it runs again, and how often.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    /// The first wait; each later one doubles, up to [`Self::longest`].
    pub first: Duration,
    pub longest: Duration,
    /// Tries after the first run; then the job fails and waits for Retry.
    pub tries: u32,
}

impl Default for Backoff {
    /// 3, 6 and then 12 minutes.
    fn default() -> Self {
        Self {
            first: Duration::from_secs(180),
            longest: Duration::from_secs(900),
            tries: 3,
        }
    }
}

impl Backoff {
    /// The wait before retry `n` (1 for the first).
    fn wait(&self, n: u32) -> Duration {
        let factor = 2u32.saturating_pow(n.saturating_sub(1));
        self.first.saturating_mul(factor).min(self.longest)
    }
}

/// How often a waiting job checks whether it was cancelled or retried.
const TICK: Duration = Duration::from_millis(200);

/// Codex's own progress lines ("hook: SessionStart"), which say nothing
/// about the image or why a job failed.
pub(super) fn is_noise(line: &str) -> bool {
    line.trim_start().starts_with("hook:")
}

/// True when `text` says the request was rate limited (HTTP 429).
pub(super) fn says_rate_limited(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "429 too many requests",
        "status: 429",
        "status 429",
        "http 429",
        "too many requests",
        "rate limit",
        "rate_limit",
        "ratelimit",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

/// The plain message for a rate limited job.
pub(super) fn rate_limit_message(retries: u32, backoff: &Backoff) -> String {
    if retries < backoff.tries {
        format!(
            "Rate limited by Codex. Trying again on its own (try {} of {}).",
            retries + 2,
            backoff.tries + 1
        )
    } else if backoff.tries == 0 {
        "Rate limited by Codex. Try again in a few minutes.".into()
    } else {
        format!(
            "Rate limited by Codex, {} times in a row. Try again in a few minutes with Retry.",
            retries + 1
        )
    }
}

/// Turns a failed, rate limited job into one waiting for its next try, or
/// a failure with the plain message once the tries are used up. Returns
/// the wait before the next try, `None` once the tries are used up.
pub(super) fn rate_limited(job: &mut ConceptJob, backoff: &Backoff) -> Option<Duration> {
    job.message = Some(rate_limit_message(job.retries, backoff));
    if job.retries >= backoff.tries {
        job.retry_at_ms = None;
        return None;
    }
    job.retries += 1;
    let wait = backoff.wait(job.retries);
    job.state = ConceptJobState::RateLimited;
    job.retry_at_ms = Some(now_ms() + wait.as_secs_f64() * 1000.0);
    Some(wait)
}

/// Waits out `wait` on its own thread, then runs the job again, unless it
/// was cancelled, retried by hand or dropped in the meantime.
pub(super) fn schedule(shared: Arc<Shared>, id: String, generation: u64, wait: Duration) {
    let due = Instant::now() + wait;
    let spawned = std::thread::Builder::new()
        .name("concept-retry".into())
        .spawn(move || {
            let still_waiting = |shared: &Shared| {
                shared.jobs().iter().any(|e| {
                    e.job.id == id
                        && e.generation == generation
                        && e.job.state == ConceptJobState::RateLimited
                })
            };
            while Instant::now() < due {
                if !still_waiting(&shared) {
                    return;
                }
                std::thread::sleep(TICK.min(due.saturating_duration_since(Instant::now())));
            }
            if still_waiting(&shared)
                && let Some(runner) = shared.runner.get().and_then(|r| r.upgrade())
            {
                submit(&shared, &runner, &id);
            }
        });
    if let Err(err) = spawned {
        tracing::warn!(error = %err, "the retry of a rate limited concept job did not start");
    }
}

/// Queues the job's run (again): the same prompt, refs and file, a fresh
/// log. Returns the job as it is now, or `None` when it is gone.
pub(super) fn submit(shared: &Shared, runner: &PipelineRunner, id: &str) -> Option<ConceptJob> {
    let rerun = {
        let mut jobs = shared.jobs();
        let e = jobs.iter_mut().find(|e| e.job.id == id)?;
        e.generation += 1;
        e.session = None;
        e.last_error = None;
        e.started = None;
        e.last_emit = None;
        let job = &mut e.job;
        job.run_id = None;
        job.state = ConceptJobState::Queued;
        job.queued_at_ms = now_ms();
        job.started_at_ms = None;
        job.finished_at_ms = None;
        job.retry_at_ms = None;
        job.width = None;
        job.height = None;
        job.log_tail.clear();
        if job.retries == 0 {
            job.message = None;
        }
        e.rerun.clone()
    };
    let outcome = runner.submit_with(rerun.setup, rerun.options);
    let job = {
        let mut jobs = shared.jobs();
        let e = jobs.iter_mut().find(|e| e.job.id == id)?;
        match outcome {
            Ok(record) => {
                e.job.run_id.get_or_insert(record.id);
            }
            Err(err) => {
                e.job.state = ConceptJobState::Failed;
                e.job.finished_at_ms = Some(now_ms());
                e.job.message = Some(err.to_string());
            }
        }
        e.job.clone()
    };
    shared.emit(ConceptEvent::Job { job: job.clone() });
    Some(job)
}

impl Concepts {
    /// How rate limited jobs wait and how often they try again.
    pub fn set_backoff(&self, backoff: Backoff) {
        *self.shared.backoff() = backoff;
    }

    /// Runs a failed or cancelled job again with the same prompt, refs and
    /// target file, or a rate limited one now instead of at its next try.
    ///
    /// # Errors
    ///
    /// Fails for an unknown job, and for one that is queued, running or done.
    pub fn retry(&self, job_id: &str) -> CoreResult<ConceptJob> {
        {
            let mut jobs = self.shared.jobs();
            let e = jobs
                .iter_mut()
                .find(|e| e.job.id == job_id)
                .ok_or_else(|| CoreError::NotFound(format!("the concept job {job_id}")))?;
            match e.job.state {
                ConceptJobState::Failed
                | ConceptJobState::Cancelled
                | ConceptJobState::RateLimited => e.job.retries = 0,
                ConceptJobState::Done => {
                    return Err(CoreError::Invalid(format!(
                        "{} is done; a done image is not made again.",
                        e.job.file
                    )));
                }
                ConceptJobState::Queued | ConceptJobState::Running => {
                    return Err(CoreError::Invalid(format!(
                        "{} is still {}; only a failed or cancelled job runs again.",
                        e.job.file,
                        if e.job.state == ConceptJobState::Queued {
                            "queued"
                        } else {
                            "running"
                        }
                    )));
                }
            }
        }
        submit(&self.shared, &self.runner, job_id)
            .ok_or_else(|| CoreError::NotFound(format!("the concept job {job_id}")))
    }

    /// Cancels a job waiting out a rate limit; there is no run to stop.
    /// Returns the job when it was one.
    pub(super) fn cancel_waiting(&self, job_id: &str) -> Option<ConceptJob> {
        let job = {
            let mut jobs = self.shared.jobs();
            let e = jobs
                .iter_mut()
                .find(|e| e.job.id == job_id && e.job.state == ConceptJobState::RateLimited)?;
            e.job.state = ConceptJobState::Cancelled;
            e.job.finished_at_ms = Some(now_ms());
            e.job.retry_at_ms = None;
            e.job.message = Some("Cancelled while waiting out the rate limit.".into());
            e.job.clone()
        };
        self.shared.emit(ConceptEvent::Job { job: job.clone() });
        Some(job)
    }
}
