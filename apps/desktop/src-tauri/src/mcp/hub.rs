//! What the MCP server shares with the app: the activity list, the runs an
//! agent can ask about, and the shipping approvals waiting for a person.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

use maquette_core::pipeline::StepDef;
use maquette_core::pipeline::history::{LogEntry, RunRecord, now_ms};
use maquette_core::pipeline::runner::PipelineEvent;
use serde::{Deserialize, Serialize};

/// How many calls the activity list keeps.
pub const MAX_CALLS: usize = 200;
/// How many log lines a live run keeps for `get_run`.
const MAX_LIVE_LOG: usize = 400;
/// Finished runs kept in memory before the oldest are dropped. History on
/// disk still has them.
const MAX_LIVE_RUNS: usize = 32;

/// How an MCP call went, for the activity list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum CallOutcome {
    Ok,
    /// The call returned while a run waits for a shipping approval.
    Waiting,
    /// The tool reported an error.
    Error,
    /// The request had no valid token, or asked for something not served.
    Refused,
}

/// One line of the activity list in Settings > Agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct McpCall {
    pub id: u32,
    pub at_ms: f64,
    pub tool: String,
    /// The arguments in a few words, for example `moon-island, from compress`.
    pub detail: String,
    pub outcome: CallOutcome,
    pub message: Option<String>,
    pub duration_ms: f64,
}

/// A run an agent started that stopped before its Ship step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub id: String,
    pub run_id: String,
    pub project: String,
    pub run_number: u32,
    pub step: String,
    pub requested_at_ms: f64,
}

#[derive(Default)]
struct Inner {
    calls: VecDeque<McpCall>,
    next_call: u32,
    approvals: Vec<Approval>,
    decisions: HashMap<String, bool>,
    next_approval: u32,
    runs: HashMap<String, RunRecord>,
    logs: HashMap<String, VecDeque<LogEntry>>,
}

/// Shared between the server's tools, the run gate and the app.
pub struct Hub {
    inner: Mutex<Inner>,
    decided: Condvar,
    on_change: Box<dyn Fn() + Send + Sync>,
}

impl Hub {
    /// `on_change` runs (outside the lock) after every change the app shows.
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            decided: Condvar::new(),
            on_change: Box::new(on_change),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Newest first.
    pub fn calls(&self) -> Vec<McpCall> {
        self.lock().calls.iter().cloned().collect()
    }

    pub fn approvals(&self) -> Vec<Approval> {
        self.lock().approvals.clone()
    }

    pub fn log_call(
        &self,
        tool: &str,
        detail: String,
        outcome: CallOutcome,
        message: Option<String>,
        duration_ms: f64,
    ) {
        {
            let mut inner = self.lock();
            inner.next_call += 1;
            let call = McpCall {
                id: inner.next_call,
                at_ms: now_ms(),
                tool: tool.to_string(),
                detail,
                outcome,
                message,
                duration_ms,
            };
            inner.calls.push_front(call);
            inner.calls.truncate(MAX_CALLS);
        }
        (self.on_change)();
    }

    pub fn clear_calls(&self) {
        self.lock().calls.clear();
        (self.on_change)();
    }

    /// Keeps the latest record and log of every run, so an agent can follow
    /// a run that has not reached the history file yet.
    pub fn observe(&self, event: &PipelineEvent) {
        let mut inner = self.lock();
        match event {
            PipelineEvent::Run { run } => {
                inner.runs.insert(run.id.clone(), run.clone());
                if inner.runs.len() > MAX_LIVE_RUNS {
                    let mut finished: Vec<(String, f64)> = inner
                        .runs
                        .values()
                        .filter(|r| r.state.is_finished())
                        .map(|r| (r.id.clone(), r.finished_at_ms.unwrap_or(0.0)))
                        .collect();
                    finished.sort_by(|a, b| a.1.total_cmp(&b.1));
                    let extra = inner.runs.len() - MAX_LIVE_RUNS;
                    for (id, _) in finished.into_iter().take(extra) {
                        inner.runs.remove(&id);
                        inner.logs.remove(&id);
                    }
                }
            }
            PipelineEvent::Log { entry } => {
                let log = inner.logs.entry(entry.run_id.clone()).or_default();
                log.push_back(entry.clone());
                if log.len() > MAX_LIVE_LOG {
                    log.pop_front();
                }
            }
        }
    }

    pub fn live_run(&self, run_id: &str) -> Option<RunRecord> {
        self.lock().runs.get(run_id).cloned()
    }

    pub fn live_log(&self, run_id: &str) -> Option<Vec<LogEntry>> {
        self.lock()
            .logs
            .get(run_id)
            .map(|l| l.iter().cloned().collect())
    }

    /// The approval a run waits for, if any.
    pub fn pending_for(&self, run_id: &str) -> Option<Approval> {
        self.lock()
            .approvals
            .iter()
            .find(|a| a.run_id == run_id)
            .cloned()
    }

    /// Records that `run` waits before `step` and returns the approval's id.
    pub fn request_approval(&self, run: &RunRecord, step: &StepDef) -> String {
        let id = {
            let mut inner = self.lock();
            inner.next_approval += 1;
            let id = format!("approval-{}", inner.next_approval);
            inner.approvals.push(Approval {
                id: id.clone(),
                run_id: run.id.clone(),
                project: run.project.clone(),
                run_number: run.number,
                step: step.name.clone(),
                requested_at_ms: now_ms(),
            });
            id
        };
        (self.on_change)();
        id
    }

    /// Approves or denies. False when the approval is gone (decided already,
    /// or its run was cancelled).
    pub fn decide(&self, approval_id: &str, approve: bool) -> bool {
        let found = {
            let mut inner = self.lock();
            let before = inner.approvals.len();
            inner.approvals.retain(|a| a.id != approval_id);
            let found = inner.approvals.len() != before;
            if found {
                inner.decisions.insert(approval_id.to_string(), approve);
            }
            found
        };
        if found {
            self.decided.notify_all();
            (self.on_change)();
        }
        found
    }

    /// Blocks until the approval is decided. `None` when the run was
    /// cancelled first; the approval is then withdrawn.
    pub fn wait_decision(&self, approval_id: &str, cancelled: &AtomicBool) -> Option<bool> {
        let mut inner = self.lock();
        loop {
            if let Some(answer) = inner.decisions.remove(approval_id) {
                return Some(answer);
            }
            if cancelled.load(Ordering::SeqCst) {
                inner.approvals.retain(|a| a.id != approval_id);
                drop(inner);
                (self.on_change)();
                return None;
            }
            inner = self
                .decided
                .wait_timeout(inner, Duration::from_millis(100))
                .map(|(guard, _)| guard)
                .unwrap_or_else(|p| p.into_inner().0);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    use super::*;
    use maquette_core::pipeline::history::RunState;

    fn run(id: &str) -> RunRecord {
        RunRecord {
            id: id.into(),
            number: 3,
            project: "moon".into(),
            from: None,
            force: false,
            state: RunState::Running,
            queued_at_ms: 0.0,
            started_at_ms: None,
            finished_at_ms: None,
            steps: Vec::new(),
            message: None,
        }
    }

    fn ship() -> StepDef {
        StepDef {
            id: "ship".into(),
            name: "Ship".into(),
            kind: maquette_core::pipeline::StepKind::Shell,
            command: Some("true".into()),
            script: None,
            args: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            depends_on: Vec::new(),
            compress: None,
        }
    }

    #[test]
    fn the_activity_list_keeps_the_newest_calls() {
        let changes = Arc::new(AtomicUsize::new(0));
        let counter = changes.clone();
        let hub = Hub::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        for i in 0..(MAX_CALLS + 5) {
            hub.log_call("list_projects", format!("{i}"), CallOutcome::Ok, None, 1.0);
        }
        let calls = hub.calls();
        assert_eq!(calls.len(), MAX_CALLS);
        assert_eq!(calls[0].detail, format!("{}", MAX_CALLS + 4));
        assert_eq!(changes.load(Ordering::SeqCst), MAX_CALLS + 5);
    }

    #[test]
    fn an_approval_waits_for_a_decision_or_a_cancel() {
        let hub = Arc::new(Hub::new(|| {}));
        let id = hub.request_approval(&run("moon-run-3"), &ship());
        assert_eq!(hub.pending_for("moon-run-3").expect("pending").id, id);
        let waiter = {
            let hub = hub.clone();
            let id = id.clone();
            std::thread::spawn(move || hub.wait_decision(&id, &AtomicBool::new(false)))
        };
        std::thread::sleep(Duration::from_millis(50));
        assert!(hub.decide(&id, true));
        assert_eq!(waiter.join().expect("join"), Some(true));
        assert!(hub.pending_for("moon-run-3").is_none());
        assert!(!hub.decide(&id, false), "already decided");

        let id = hub.request_approval(&run("moon-run-4"), &ship());
        assert_eq!(hub.wait_decision(&id, &AtomicBool::new(true)), None);
        assert!(hub.approvals().is_empty());
    }
}
