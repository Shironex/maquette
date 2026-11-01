//! One run in progress: walking the planned steps in order, asking the
//! gate, logging and the run summary at the end.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::queue::ChildSlot;
use super::{GateDecision, PipelineEvent, RunOptions, RunSetup, RunSink};
use crate::pipeline::cache::FileHasher;
use crate::pipeline::graph::{self, Planned};
use crate::pipeline::history::{
    self, LogEntry, LogLevel, LogStream, RunRecord, RunState, StepState, now_ms,
};
use crate::pipeline::progress::Tracker;
use crate::pipeline::{StepDef, is_ship_step};

/// Progress events are sent at most this often, unless a step finishes.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);
/// Why a gated step was skipped without asking.
pub(super) const UNCHANGED_MESSAGE: &str =
    "Nothing changed since the last ship, so I did not ask again.";

/// One run in progress.
pub(super) struct Execution {
    pub(super) sink: Arc<dyn RunSink>,
    pub(super) record: RunRecord,
    pub(super) setup: RunSetup,
    pub(super) options: RunOptions,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) child: ChildSlot,
    pub(super) log: Vec<LogEntry>,
    /// The step record the running step reports into.
    pub(super) slot: usize,
    /// When progress was last sent, so a chatty tool does not flood the UI.
    pub(super) progress_sent: Option<Instant>,
    /// Why the run stopped early without a failure (a gate said no).
    pub(super) stop: Option<String>,
}

pub(super) enum StepOutcome {
    Done,
    Cached,
    Failed(String, Option<i32>),
    Cancelled,
}

impl Execution {
    fn emit_run(&self) {
        self.sink.event(PipelineEvent::Run {
            run: self.record.clone(),
        });
    }

    pub(super) fn log(
        &mut self,
        step: Option<&str>,
        stream: LogStream,
        level: LogLevel,
        text: String,
    ) {
        let entry = LogEntry {
            run_id: self.record.id.clone(),
            step: step.map(str::to_string),
            stream,
            level,
            text,
            at_ms: now_ms(),
        };
        self.sink.event(PipelineEvent::Log {
            entry: entry.clone(),
        });
        self.log.push(entry);
        if self.log.len() > history::MAX_LOG_LINES * 2 {
            self.log.drain(..history::MAX_LOG_LINES);
        }
    }

    pub(super) fn system(&mut self, step: Option<&str>, level: LogLevel, text: impl Into<String>) {
        self.log(step, LogStream::System, level, text.into());
    }

    pub(super) fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// Records how far the running step is and tells the UI, throttled.
    pub(super) fn report_progress(&mut self, tracker: &Tracker) {
        let Some(value) = tracker.value() else {
            return;
        };
        let rec = &mut self.record.steps[self.slot];
        if rec.progress == Some(value) {
            return;
        }
        rec.progress = Some(value);
        let due = self
            .progress_sent
            .is_none_or(|at| at.elapsed() >= PROGRESS_EVERY);
        if due || value >= 1.0 {
            self.progress_sent = Some(Instant::now());
            self.emit_run();
        }
    }

    pub(super) fn run(&mut self) {
        self.record.state = RunState::Running;
        self.record.started_at_ms = Some(now_ms());
        self.emit_run();
        let plan = graph::plan_run(
            &self.setup.def,
            self.setup.from.as_deref(),
            self.setup.force,
            self.setup.build_only,
        );
        let planned = match plan {
            Ok(p) => p,
            Err(err) => {
                self.finish(RunState::Failed, Some(err.to_string()));
                return;
            }
        };
        let runnable = planned.iter().filter(|(_, p)| *p != Planned::Skip).count();
        let header = format!(
            "Run {} · {runnable} of {} steps{}{}",
            self.record.number,
            planned.len(),
            self.setup
                .from
                .as_deref()
                .map(|f| format!(" · from {f}"))
                .unwrap_or_default(),
            if self.setup.force { " · forced" } else { "" }
        );
        let header = if self.setup.build_only {
            format!("{header} · build only, stops before Ship")
        } else {
            header
        };
        self.system(None, LogLevel::Info, header);
        let mut hasher = FileHasher::with_memo(&self.setup.cache_dir);
        let mut failure: Option<String> = None;
        for (slot, (index, planned)) in planned.iter().enumerate() {
            if *planned == Planned::Skip {
                continue;
            }
            let step = self.setup.def.steps[*index].clone();
            if failure.is_some() || self.cancelled() || self.stop.is_some() {
                let state = if failure.is_none() {
                    StepState::Cancelled
                } else {
                    StepState::Skipped
                };
                let rec = &mut self.record.steps[slot];
                rec.state = state;
                rec.message = Some(if state == StepState::Cancelled {
                    "Cancelled.".to_string()
                } else {
                    "Not run, an earlier step failed.".to_string()
                });
                continue;
            }
            if let Some(reason) = self.nowhere_to_ship(&step) {
                let rec = &mut self.record.steps[slot];
                rec.state = StepState::Skipped;
                rec.message = Some(reason.clone());
                self.record.not_shipped = Some(reason.clone());
                self.system(Some(&step.id), LogLevel::Warn, reason);
                self.emit_run();
                continue;
            }
            if self.gated(&step) && self.unchanged_since_last(&step, &mut hasher) {
                let rec = &mut self.record.steps[slot];
                rec.state = StepState::Cached;
                rec.progress = Some(1.0);
                rec.message = Some(UNCHANGED_MESSAGE.to_string());
                self.system(
                    Some(&step.id),
                    LogLevel::Ok,
                    format!("{} skipped: {UNCHANGED_MESSAGE}", step.name),
                );
                self.emit_run();
                continue;
            }
            if !self.pass_gate(slot, &step) {
                continue;
            }
            let started = Instant::now();
            {
                let rec = &mut self.record.steps[slot];
                rec.state = StepState::Running;
                rec.started_at_ms = Some(now_ms());
            }
            self.slot = slot;
            self.progress_sent = None;
            self.emit_run();
            let outcome = self.run_step(&step, *planned == Planned::Force, &mut hasher);
            let elapsed = started.elapsed();
            #[expect(clippy::cast_precision_loss, reason = "step durations are small")]
            let duration_ms = elapsed.as_millis() as f64;
            let rec = &mut self.record.steps[slot];
            rec.duration_ms = Some(duration_ms);
            rec.progress = matches!(outcome, StepOutcome::Done | StepOutcome::Cached)
                .then_some(1.0)
                .or(rec.progress);
            match outcome {
                StepOutcome::Done => rec.state = StepState::Done,
                StepOutcome::Cached => rec.state = StepState::Cached,
                StepOutcome::Cancelled => {
                    rec.state = StepState::Cancelled;
                    rec.message = Some("Cancelled.".to_string());
                }
                StepOutcome::Failed(message, code) => {
                    rec.state = StepState::Failed;
                    rec.exit_code = code;
                    rec.message = Some(message.clone());
                    failure = Some(format!("{} failed. {message}", step.name));
                }
            }
            let state = rec.state;
            let line = match state {
                StepState::Done => Some((
                    LogLevel::Ok,
                    format!("{} done in {}", step.name, seconds(elapsed)),
                )),
                StepState::Cached => Some((
                    LogLevel::Ok,
                    format!("{} inputs unchanged, cache hit", step.name),
                )),
                StepState::Failed => Some((
                    LogLevel::Error,
                    format!(
                        "{} failed after {}: {}",
                        step.name,
                        seconds(elapsed),
                        self.record.steps[slot].message.clone().unwrap_or_default()
                    ),
                )),
                _ => None,
            };
            if let Some((level, text)) = line {
                self.system(Some(&step.id), level, text);
            }
            self.emit_run();
        }
        hasher.flush();
        if self.cancelled() {
            self.finish(RunState::Cancelled, Some("Cancelled.".to_string()));
        } else if let Some(reason) = self.stop.clone() {
            self.finish(RunState::Cancelled, Some(reason));
        } else if let Some(message) = failure {
            self.finish(RunState::Failed, Some(message));
        } else {
            self.finish(RunState::Done, None);
        }
    }

    /// Why `step` has nowhere to write: a Ship step that takes the project's
    /// repo, in a project with none. The runner decides this, not the script,
    /// so a project on an older `ship.mjs` reads the same, and nobody is
    /// asked to approve a Ship that would copy nothing.
    fn nowhere_to_ship(&self, step: &StepDef) -> Option<String> {
        let takes_repo = step.args.iter().any(|a| a.contains("{siteRepo}"));
        let repo = self.setup.vars.get("siteRepo").map_or("", |r| r.trim());
        if !is_ship_step(step) || !takes_repo || !repo.is_empty() {
            return None;
        }
        let word = if step.args.iter().any(|a| a == "--game") {
            "game"
        } else {
            "site"
        };
        Some(format!(
            "No {word} repo is set for this project, so nothing was shipped. Pick one in Settings."
        ))
    }

    fn gated(&self, step: &StepDef) -> bool {
        self.options.gate.as_ref().is_some_and(|g| g.applies(step))
    }

    /// Asks the run's gate, if it has one for `step`, and waits for the
    /// answer. False when the step must not run; the run then stops.
    fn pass_gate(&mut self, slot: usize, step: &StepDef) -> bool {
        let Some(gate) = self.options.gate.clone().filter(|g| g.applies(step)) else {
            return true;
        };
        let waiting = gate.waiting_message(step);
        self.record.steps[slot].message = Some(waiting.clone());
        self.system(Some(&step.id), LogLevel::Warn, waiting);
        self.emit_run();
        match gate.wait(&self.record, step, &self.cancel) {
            GateDecision::Allow => {
                self.record.steps[slot].message = None;
                self.system(
                    Some(&step.id),
                    LogLevel::Info,
                    format!("{} approved", step.name),
                );
                true
            }
            GateDecision::Deny(reason) => {
                let rec = &mut self.record.steps[slot];
                rec.state = StepState::Cancelled;
                rec.message = Some(reason.clone());
                self.system(Some(&step.id), LogLevel::Error, reason.clone());
                self.stop = Some(reason);
                self.emit_run();
                false
            }
        }
    }

    fn finish(&mut self, state: RunState, message: Option<String>) {
        self.record.state = state;
        self.record.finished_at_ms = Some(now_ms());
        self.record.message = message;
        let total = self
            .record
            .started_at_ms
            .zip(self.record.finished_at_ms)
            .map(|(a, b)| (b - a).max(0.0))
            .unwrap_or(0.0);
        let cached = self
            .record
            .steps
            .iter()
            .filter(|s| s.state == StepState::Cached)
            .count();
        let summary = match state {
            RunState::Done => format!(
                "Run {} done in {:.1}s, {cached} cached{}",
                self.record.number,
                total / 1000.0,
                if self.record.not_shipped.is_some() {
                    ", nothing shipped"
                } else {
                    ""
                }
            ),
            RunState::Cancelled => format!("Run {} cancelled", self.record.number),
            _ => format!(
                "Run {} failed: {}",
                self.record.number,
                self.record.message.clone().unwrap_or_default()
            ),
        };
        let level = if self.record.not_shipped.is_some() && state == RunState::Done {
            LogLevel::Warn
        } else if state == RunState::Done {
            LogLevel::Ok
        } else {
            LogLevel::Error
        };
        self.system(None, level, summary);
        if let Err(err) = history::record_run(&self.setup.cache_dir, &self.record) {
            tracing::warn!(error = %err, "could not save the run record");
        }
        if let Err(err) = history::save_log(&self.setup.cache_dir, self.record.number, &self.log) {
            tracing::warn!(error = %err, "could not save the run log");
        }
        self.emit_run();
    }
}

fn seconds(d: Duration) -> String {
    format!("{:.1}s", d.as_secs_f64())
}
