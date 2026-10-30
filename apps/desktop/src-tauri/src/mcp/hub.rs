//! What the MCP server shares with the app: the activity list, the runs an
//! agent can ask about, and the approvals (shipping, sends to 3D) waiting
//! for a person.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

use maquette_core::avatar::normalize::NormalizeSheets;
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
    /// The arguments in a few words, for example `frostveil, from compress`.
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
    /// An agent asked for Lab images on OpenAI's Image API, which spends
    /// dollars from the OpenAI key.
    ImageRound,
    /// An agent built an avatar whose new rest pose waits for a person to
    /// look at the before and after sheets.
    RestPose,
    /// An agent built an avatar whose weights, freeing cloth and hair from
    /// the arms, wait for a person to look at the before and after sheets.
    Reweight,
    /// An agent built an avatar whose new spring chains wait for a person
    /// to look at the parts and the sway sheet.
    Springs,
}

/// One send of a batch approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalItem {
    pub job: String,
    pub asset_name: String,
    /// Its settings in a few words.
    pub detail: String,
    /// What it should cost, from the price lists.
    #[serde(default)]
    pub estimate: Option<f64>,
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
    /// Send to 3D: the credits it should cost in all, from the price lists.
    #[serde(default)]
    pub estimate: Option<f64>,
    /// Image round: the Lab jobs it holds.
    #[serde(default)]
    pub concept_jobs: Vec<String>,
    /// Image round: the dollars it should cost in all.
    #[serde(default)]
    pub estimate_usd: Option<f64>,
    /// Rest pose and reweight: the asset id and the version.
    #[serde(default)]
    pub asset: Option<String>,
    #[serde(default)]
    pub version: Option<u32>,
    /// Rest pose and reweight: the before and after sheets.
    #[serde(default)]
    pub sheets: Option<NormalizeSheets>,
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

/// What [`Hub::request_image_round`] records.
pub struct ImageRound {
    pub project: String,
    /// The Lab direction the images land in.
    pub direction: Option<String>,
    pub jobs: Vec<String>,
    pub estimate_usd: f64,
    pub detail: String,
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
            estimate: None,
            concept_jobs: Vec::new(),
            estimate_usd: None,
            asset: None,
            version: None,
            sheets: None,
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
            estimate: job.estimate.or_else(|| job.estimate()),
            concept_jobs: Vec::new(),
            estimate_usd: None,
            asset: None,
            version: None,
            sheets: None,
        })
    }

    /// Records that a round of Lab images an agent asked for on the Image
    /// API waits before anything is sent. `detail` says what it is and how
    /// the month stands.
    pub fn request_image_round(&self, round: ImageRound) -> String {
        self.push(|id| Approval {
            id,
            kind: ApprovalKind::ImageRound,
            project: round.project,
            run_id: None,
            run_number: None,
            step: None,
            job: None,
            asset_name: round.direction,
            service: Some("OpenAI Images".to_string()),
            detail: Some(round.detail),
            items: Vec::new(),
            requested_at_ms: now_ms(),
            estimate: None,
            concept_jobs: round.jobs,
            estimate_usd: Some(round.estimate_usd),
            asset: None,
            version: None,
            sheets: None,
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
        let estimate: Option<f64> = jobs
            .iter()
            .map(|j| j.estimate.or_else(|| j.estimate()))
            .sum();
        let detail = match estimate {
            _ if credits > 0.0 => format!(
                "{models} in total, {} credits so far",
                format_credits(credits)
            ),
            Some(total) => format!("{models} in total, about {} credits", format_credits(total)),
            None => format!("{models} in total, one model per asset"),
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
                        estimate: j.estimate.or_else(|| j.estimate()),
                    })
                    .collect(),
                requested_at_ms: now_ms(),
                estimate,
                concept_jobs: Vec::new(),
                estimate_usd: None,
                asset: None,
                version: None,
                sheets: None,
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

mod rest_pose;

#[cfg(test)]
mod tests;
