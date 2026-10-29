//! An agent's round on the paid Image API waits for a person: its jobs are
//! queued in [`ConceptJobState::Approval`] and nothing is sent until the
//! shell releases them (approved in the app) or denies them. A Codex round
//! and a round the person starts in the app never wait here.

use super::retry::{self, submit};
use super::{ConceptEvent, ConceptJob, ConceptJobState, Concepts, Run};
use crate::pipeline::history::now_ms;

impl Concepts {
    /// The jobs among `ids` still waiting for approval.
    pub fn awaiting_approval(&self, ids: &[String]) -> Vec<String> {
        self.shared
            .jobs()
            .iter()
            .filter(|e| e.job.state == ConceptJobState::Approval && ids.contains(&e.job.id))
            .map(|e| e.job.id.clone())
            .collect()
    }

    /// What the jobs among `ids` will likely cost on the Image API.
    pub fn estimate_usd(&self, ids: &[String]) -> f64 {
        self.shared
            .jobs()
            .iter()
            .filter(|e| ids.contains(&e.job.id))
            .filter_map(|e| match &e.run {
                Run::Api(api) => Some(api.estimate()),
                Run::Codex(_) => None,
            })
            .sum()
    }

    /// Approved: the jobs among `ids` still waiting go to the queue.
    /// Returns them as they are now.
    pub fn release(&self, ids: &[String]) -> Vec<ConceptJob> {
        self.awaiting_approval(ids)
            .iter()
            .filter_map(|id| submit(&self.shared, &self.runner, id, false))
            .collect()
    }

    /// Denied: the jobs among `ids` still waiting are cancelled, nothing
    /// sent. Returns them.
    pub fn deny(&self, ids: &[String]) -> Vec<ConceptJob> {
        let denied: Vec<ConceptJob> = {
            let mut jobs = self.shared.jobs();
            jobs.iter_mut()
                .filter(|e| e.job.state == ConceptJobState::Approval && ids.contains(&e.job.id))
                .map(|e| {
                    retry::reset(e, ConceptJobState::Cancelled);
                    e.job.finished_at_ms = Some(now_ms());
                    e.job.message =
                        Some("Denied in Maquette. Nothing was sent or spent.".to_string());
                    e.job.clone()
                })
                .collect()
        };
        for job in &denied {
            self.shared.emit(ConceptEvent::Job { job: job.clone() });
        }
        denied
    }
}
