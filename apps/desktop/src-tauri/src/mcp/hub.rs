//! What the MCP server shares with the app: the activity list, the runs an
//! agent can ask about, and the approvals (shipping, sends to 3D) waiting
//! for a person.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

use maquette_core::pipeline::StepDef;
use maquette_core::pipeline::history::{LogEntry, RunRecord, now_ms};
use maquette_core::pipeline::runner::PipelineEvent;
use maquette_core::providers::{ProviderJob, format_credits};
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

/// What an agent asked that waits for a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalKind {
    /// A run an agent started stopped before its Ship step.
    Ship,
    /// An agent asked to send an asset to Tripo or Meshy, which spends credits.
    #[serde(rename = "send-to-3d")]
    SendTo3d,
}

/// One send of a batch approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalItem {
    pub job: String,
    pub asset_name: String,
    /// Its settings in a few words.
    pub detail: String,
}

/// Something an agent asked for that waits for a yes or no in the app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub id: String,
    pub kind: ApprovalKind,
    pub project: String,
    /// Ship: the run and the step it stopped before.
    pub run_id: Option<String>,
    pub run_number: Option<u32>,
    pub step: Option<String>,
    /// Send to 3D: the provider job, its asset and settings in a few words.
    pub job: Option<String>,
    pub asset_name: Option<String>,
    pub service: Option<String>,
    pub detail: Option<String>,
    /// A batch send: every send it holds, all approved or denied at once.
    pub items: Vec<ApprovalItem>,
    pub requested_at_ms: f64,
}

impl Approval {
    /// The provider jobs this approval releases.
    pub fn jobs(&self) -> Vec<String> {
        self.job
            .iter()
            .cloned()
            .chain(self.items.iter().map(|i| i.job.clone()))
            .collect()
    }
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
            .find(|a| a.run_id.as_deref() == Some(run_id))
            .cloned()
    }

    /// The approval a provider job waits for, if any.
    pub fn pending_for_job(&self, job_id: &str) -> Option<Approval> {
        self.lock()
            .approvals
            .iter()
            .find(|a| a.job.as_deref() == Some(job_id) || a.items.iter().any(|i| i.job == job_id))
            .cloned()
    }

    fn push(&self, approval: impl FnOnce(String) -> Approval) -> String {
        let id = {
            let mut inner = self.lock();
            inner.next_approval += 1;
            let id = format!("approval-{}", inner.next_approval);
            inner.approvals.push(approval(id.clone()));
            id
        };
        (self.on_change)();
        id
    }

    /// Records that `run` waits before `step` and returns the approval's id.
    pub fn request_approval(&self, run: &RunRecord, step: &StepDef) -> String {
        self.push(|id| Approval {
            id,
            kind: ApprovalKind::Ship,
            project: run.project.clone(),
            run_id: Some(run.id.clone()),
            run_number: Some(run.number),
            step: Some(step.name.clone()),
            job: None,
            asset_name: None,
            service: None,
            detail: None,
            items: Vec::new(),
            requested_at_ms: now_ms(),
        })
    }

    /// Records that a send an agent asked for waits before spending credits.
    pub fn request_send(&self, job: &ProviderJob) -> String {
        self.push(|id| Approval {
            id,
            kind: ApprovalKind::SendTo3d,
            project: job.project.clone(),
            run_id: None,
            run_number: None,
            step: None,
            job: Some(job.id.clone()),
            asset_name: Some(job.asset_name.clone()),
            service: Some(job.provider.label().to_string()),
            detail: Some(job.summary()),
            items: Vec::new(),
            requested_at_ms: now_ms(),
        })
    }

    /// Records one approval for a batch of sends an agent asked for. The
    /// detail says the total: how many models, and the credits already
    /// known (none before anything is sent, so it counts models).
    pub fn request_batch(&self, jobs: &[ProviderJob]) -> Option<String> {
        let first = jobs.first()?;
        let models = match jobs.len() {
            1 => "1 model".to_string(),
            n => format!("{n} models"),
        };
        let credits: f64 = jobs.iter().filter_map(|j| j.credits).sum();
        let detail = if credits > 0.0 {
            format!(
                "{models} in total, {} credits so far",
                format_credits(credits)
            )
        } else {
            format!("{models} in total, one model per asset")
        };
        Some(self.push(|id| {
            Approval {
                id,
                kind: ApprovalKind::SendTo3d,
                project: first.project.clone(),
                run_id: None,
                run_number: None,
                step: None,
                job: None,
                asset_name: None,
                service: Some(first.provider.label().to_string()),
                detail: Some(detail),
                items: jobs
                    .iter()
                    .map(|j| ApprovalItem {
                        job: j.id.clone(),
                        asset_name: j.asset_name.clone(),
                        detail: j.summary(),
                    })
                    .collect(),
                requested_at_ms: now_ms(),
            }
        }))
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
        self.wait_until(approval_id, || cancelled.load(Ordering::SeqCst))
    }

    /// Like [`Hub::wait_decision`], with any test for "it went away first".
    pub fn wait_until(&self, approval_id: &str, gone: impl Fn() -> bool) -> Option<bool> {
        let mut inner = self.lock();
        loop {
            if let Some(answer) = inner.decisions.remove(approval_id) {
                return Some(answer);
            }
            if gone() {
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

    #[test]
    fn a_batch_approval_covers_every_job_of_it() {
        let hub = Hub::new(|| {});
        assert!(hub.request_batch(&[]).is_none());
        let job = |id: &str, name: &str| {
            let json = serde_json::json!({
                "id": id, "project": "landing", "asset": id, "assetName": name,
                "provider": "tripo", "service": "tripo", "variant": "prop",
                "images": ["refs/a.png"], "tripo": null, "meshy": null, "state": "approval",
                "phase": null, "progress": 0.0, "remoteId": null, "credits": null,
                "createdAtMs": 0.0, "startedAtMs": null, "finishedAtMs": null, "message": null,
                "inboxFile": null, "handoff": null, "card": null, "version": null,
                "viaAgent": true, "cancel": "free", "batch": "batch-1"
            });
            serde_json::from_value::<ProviderJob>(json).expect("job")
        };
        let id = hub
            .request_batch(&[job("lamp-1", "Desk lamp"), job("cup-1", "Brush cup")])
            .expect("approval");
        let pending = hub.pending_for_job("cup-1").expect("covered");
        assert_eq!(pending.id, id);
        assert_eq!(
            pending.jobs(),
            vec!["lamp-1".to_string(), "cup-1".to_string()]
        );
        assert_eq!(
            pending.detail.as_deref(),
            Some("2 models in total, one model per asset")
        );
        assert!(hub.decide(&id, true));
        assert!(hub.pending_for_job("lamp-1").is_none());
    }
}
