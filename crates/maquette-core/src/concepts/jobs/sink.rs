//! The runner sink: turns pipeline runs and log lines into concept job
//! updates, and settles a job when its run ends. Codex's "hook: ..." lines
//! are left out of the tail and the error, and a run Codex rate limited
//! waits for its next try.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use super::retry::{self, says_rate_limited};
use super::settle::finish;
use super::{
    ConceptEvent, ConceptJob, ConceptJobState, EMIT_EVERY, Entry, KEEP_FINISHED, LOG_TAIL, Shared,
};
use crate::pipeline::history::{LogLevel, LogStream, RunRecord, RunState, now_ms};
use crate::pipeline::runner::{PipelineEvent, RunSink};

pub(super) struct Sink(pub(super) Arc<Shared>);

impl RunSink for Sink {
    fn event(&self, event: PipelineEvent) {
        match event {
            PipelineEvent::Run { run } => self.run(&run),
            PipelineEvent::Log { entry } => {
                let text = entry.text.trim_end().to_string();
                if retry::is_noise(&text) {
                    return;
                }
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
        let limited = run.state == RunState::Failed
            && rate_limited_run(&job, last_error.as_deref(), reason.as_deref());
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
        let wait = (limited && job.state == ConceptJobState::Failed)
            .then(|| retry::rate_limited(&mut job, &self.0.backoff()))
            .flatten();
        let generation = {
            let mut jobs = self.0.jobs();
            let generation = jobs.iter_mut().find(|e| e.job.id == job.id).map(|e| {
                e.job = job.clone();
                e.generation
            });
            prune(&mut jobs);
            generation
        };
        if let (Some(wait), Some(generation)) = (wait, generation) {
            retry::schedule(Arc::clone(&self.0), job.id.clone(), generation, wait);
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

/// True when the run's error, its reason or the last lines Codex printed
/// say it was rate limited.
fn rate_limited_run(job: &ConceptJob, last_error: Option<&str>, reason: Option<&str>) -> bool {
    last_error.is_some_and(says_rate_limited)
        || reason.is_some_and(says_rate_limited)
        || job
            .log_tail
            .iter()
            .rev()
            .take(8)
            .any(|l| says_rate_limited(l))
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
pub(super) fn session_id(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let at = lower.find("session id:")?;
    let id = line[at + "session id:".len()..].trim();
    let ok = !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    ok.then(|| id.to_string())
}
