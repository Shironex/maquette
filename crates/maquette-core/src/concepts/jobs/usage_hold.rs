//! The jobs Codex's usage limit holds, moved by what a usage read says:
//! to the reset time the read names (it beats the time read from Codex's
//! message and the backoff guess), or straight back into the queue once
//! the read says the limit has lifted.

use std::sync::Arc;
use std::time::Duration;

use super::retry::{schedule, submit, usage_lifted, wait_for_usage};
use super::{ConceptEvent, ConceptJob, ConceptJobState, Concepts, Entry};
use crate::pipeline::history::now_ms;

/// Added to the reset time, so the rerun lands after the limit lifts.
const MARGIN_MS: f64 = 60_000.0;
/// A held job whose time is this close already keeps it.
const SAME_MS: f64 = 1_000.0;

fn held(e: &Entry) -> bool {
    e.job.state == ConceptJobState::RateLimited && e.job.usage_limit
}

impl Concepts {
    /// The jobs waiting for Codex's usage limit to lift.
    pub fn usage_held(&self) -> Vec<ConceptJob> {
        self.shared
            .jobs()
            .iter()
            .filter(|e| held(e))
            .map(|e| e.job.clone())
            .collect()
    }

    /// Moves every job the usage limit holds to `resets_at_ms` (plus a
    /// minute), the reset time a usage read named, and holds the queue
    /// until then. A time already past moves nothing. Returns the jobs
    /// that moved.
    pub fn retime_usage_limit(&self, resets_at_ms: f64) -> Vec<ConceptJob> {
        let until = resets_at_ms + MARGIN_MS;
        let now = now_ms();
        if until <= now {
            return Vec::new();
        }
        let moved: Vec<(ConceptJob, u64)> = {
            let mut jobs = self.shared.jobs();
            jobs.iter_mut()
                .filter(|e| held(e))
                .filter(|e| {
                    e.job
                        .retry_at_ms
                        .is_none_or(|at| (at - until).abs() > SAME_MS)
                })
                .map(|e| {
                    e.generation += 1;
                    wait_for_usage(&mut e.job, until);
                    (e.job.clone(), e.generation)
                })
                .collect()
        };
        if moved.is_empty() {
            return Vec::new();
        }
        *self.shared.usage_until() = Some(until);
        let wait = Duration::from_secs_f64(((until - now) / 1000.0).max(0.0));
        moved
            .into_iter()
            .map(|(job, generation)| {
                schedule(Arc::clone(&self.shared), job.id.clone(), generation, wait);
                self.shared.emit(ConceptEvent::Job { job: job.clone() });
                job
            })
            .collect()
    }

    /// The usage limit has lifted: the queue no longer waits for it and
    /// every job it held runs again now, with the same prompt, refs and
    /// file. Returns those jobs as they are now.
    pub fn lift_usage_limit(&self) -> Vec<ConceptJob> {
        usage_lifted(&self.shared);
        let ids: Vec<String> = self
            .shared
            .jobs()
            .iter()
            .filter(|e| held(e))
            .map(|e| e.job.id.clone())
            .collect();
        ids.iter()
            .filter_map(|id| submit(&self.shared, &self.runner, id, true))
            .collect()
    }
}
