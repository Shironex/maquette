//! Run next: queued jobs move to the front of their line, so a project's
//! urgent images do not wait behind every other project's. A Codex job
//! moves in the runner's lane queue, an Image API job in its own line.
//! Jobs already running, waiting out a limit or for approval stay put.

#[cfg(all(test, unix))]
mod tests;

use super::{ConceptJob, ConceptJobState, Concepts, LANE, Run, api};

impl Concepts {
    /// Moves the queued jobs among `ids` to the front of the line, in the
    /// order they were queued. Returns the jobs that moved.
    pub fn run_next(&self, ids: &[String]) -> Vec<ConceptJob> {
        let (runs, api_ids, moved) = {
            let jobs = self.shared.jobs();
            let mut runs = Vec::new();
            let mut api_ids = Vec::new();
            let mut moved = Vec::new();
            for e in jobs
                .iter()
                .filter(|e| e.job.state == ConceptJobState::Queued && ids.contains(&e.job.id))
            {
                match (&e.run, &e.job.run_id) {
                    (Run::Codex(_), Some(run)) => runs.push(run.clone()),
                    (Run::Codex(_), None) => continue,
                    (Run::Api(_), _) => api_ids.push(e.job.id.clone()),
                }
                moved.push(e.job.clone());
            }
            (runs, api_ids, moved)
        };
        self.runner.run_next(LANE, &runs);
        api::run_next(&self.shared, &api_ids);
        moved
    }

    /// [`Concepts::run_next`] for every queued job of `project`.
    pub fn run_next_project(&self, project: &str) -> Vec<ConceptJob> {
        let ids: Vec<String> = self
            .shared
            .jobs()
            .iter()
            .filter(|e| e.job.project == project && e.job.state == ConceptJobState::Queued)
            .map(|e| e.job.id.clone())
            .collect();
        self.run_next(&ids)
    }
}
