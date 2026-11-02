//! Who answered what an agent asked: every approve, deny, expiry and cancel
//! of an approval (a paid send, an Image API round, a ship, a page preview,
//! an avatar or rig step), kept per workspace in `approvals.jsonl` next to
//! `credits.jsonl`.
//!
//! A sibling file, not new lines in the credits ledger, so an older Maquette
//! reads its ledger exactly as before and never counts a decision as a line
//! it could not read. The ledger links to it on read: a paid send carries
//! the decision that let it go (the same project and job, the latest one
//! made before the send reached the provider). A workspace without the file
//! has no decisions, so old ledgers read as they always did.
//!
//! Like the ledger, the file only grows, a line that cannot be read is
//! skipped, and every line carries its version.

use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::Spend;
use crate::error::{CoreError, CoreResult};

pub const DECISIONS_FILE: &str = "approvals.jsonl";
const DECISION_VERSION: u32 = 1;

/// One writer at a time, so two answers never interleave.
static WRITE: Mutex<()> = Mutex::new(());

/// How an approval ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Decision {
    Approved,
    Denied,
    /// Still waiting when Maquette quit.
    Expired,
    /// It went away before anyone answered: its jobs or its run were
    /// cancelled, or the question changed under it.
    Cancelled,
}

/// Where the answer came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum DecisionSource {
    /// The approval card floating over every view.
    FloatingCard,
    /// The approval card in Settings > Agents.
    AgentsPanel,
    /// The Ship diff in the Pipeline view of a game project.
    ShipView,
    /// The before and after sheets in the Avatar view, or a card's Deny for
    /// an avatar step.
    Avatar,
    /// The Rig doctor: the draft was added, dropped or started over.
    RigDoctor,
    /// The jobs it held left the approval state first: cancelled in Sends
    /// or the Lab, or by an agent's cancel of its own Lab jobs.
    JobsCancelled,
    /// The run it held was cancelled.
    RunCancelled,
    /// A page preview's folder or command changed, so the card was replaced.
    QuestionChanged,
    /// Still waiting when Maquette quit.
    Quit,
    /// A caller that did not say.
    Unknown,
}

impl DecisionSource {
    /// The surfaces the app's windows answer from with Approve and Deny.
    pub fn is_card(self) -> bool {
        matches!(
            self,
            Self::FloatingCard | Self::AgentsPanel | Self::ShipView
        )
    }
}

/// What pressed it, when the window could tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum DecisionInput {
    /// A mouse, pen or touch press.
    Pointer,
    /// Enter or Space on the focused button.
    Keyboard,
}

/// What the window tells about an answer it sends: where, how, and whether
/// the page was in front. The app adds what the operating system says
/// about the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DecidedBy {
    pub source: DecisionSource,
    #[serde(default)]
    pub input: Option<DecisionInput>,
    /// False when a script made the click, not a person's input.
    #[serde(default)]
    pub trusted: Option<bool>,
    /// The page had the keyboard focus.
    #[serde(default)]
    pub page_focused: Option<bool>,
    /// The page was visible (not a hidden tab or a minimised window).
    #[serde(default)]
    pub page_visible: Option<bool>,
    /// The operating system's word on the window.
    #[serde(default)]
    pub window_focused: Option<bool>,
    #[serde(default)]
    pub window_visible: Option<bool>,
    #[serde(default)]
    pub window_minimized: Option<bool>,
}

impl DecidedBy {
    /// An answer that carries only where it came from.
    pub fn from(source: DecisionSource) -> Self {
        Self {
            source,
            input: None,
            trusted: None,
            page_focused: None,
            page_visible: None,
            window_focused: None,
            window_visible: None,
            window_minimized: None,
        }
    }
}

/// One line of `approvals.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRecord {
    pub v: u32,
    /// When it was answered.
    pub at_ms: f64,
    /// The approval's id in the app, `approval-<n>` (counted again from 1
    /// each time the app starts, so read it with `requested_at_ms`).
    pub approval: String,
    /// What waited: `send-to-3d`, `image-round`, `ship`, `page-preview`,
    /// `rest-pose`, `reweight`, `springs` or `rig-doctor`.
    pub kind: String,
    pub decision: Decision,
    /// Where it was answered from, how, and the window at that moment.
    pub by: DecidedBy,
    pub project: String,
    /// The provider jobs it covered.
    #[serde(default)]
    pub jobs: Vec<String>,
    /// The Lab jobs of an Image API round.
    #[serde(default)]
    pub concept_jobs: Vec<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    /// Tripo, Meshy or OpenAI Images.
    #[serde(default)]
    pub provider: Option<String>,
    /// The asset's name, or every asset of a batch.
    #[serde(default)]
    pub asset: Option<String>,
    /// Credits it should cost, from the price lists.
    #[serde(default)]
    pub estimate: Option<f64>,
    /// Dollars an Image API round should cost.
    #[serde(default)]
    pub estimate_usd: Option<f64>,
    pub requested_at_ms: f64,
}

impl DecisionRecord {
    pub fn new(approval: &str, kind: &str, decision: Decision, by: DecidedBy, at_ms: f64) -> Self {
        Self {
            v: DECISION_VERSION,
            at_ms,
            approval: approval.to_string(),
            kind: kind.to_string(),
            decision,
            by,
            project: String::new(),
            jobs: Vec::new(),
            concept_jobs: Vec::new(),
            run_id: None,
            provider: None,
            asset: None,
            estimate: None,
            estimate_usd: None,
            requested_at_ms: at_ms,
        }
    }

    /// True when it covers provider job `job` of `project`.
    pub fn covers(&self, project: &str, job: &str) -> bool {
        self.project == project && self.jobs.iter().any(|j| j == job)
    }
}

pub fn path(root: &Path) -> PathBuf {
    root.join(DECISIONS_FILE)
}

/// Adds one decision to the end of the file.
pub fn append(root: &Path, record: &DecisionRecord) -> CoreResult<()> {
    let path = path(root);
    let mut text = serde_json::to_string(record)
        .map_err(|e| CoreError::Invalid(format!("could not write a decision line: {e}")))?;
    text.push('\n');
    let _guard = WRITE.lock().unwrap_or_else(|p| p.into_inner());
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| CoreError::io(&path, e))?;
    file.write_all(text.as_bytes())
        .map_err(|e| CoreError::io(&path, e))
}

/// Every decision, oldest first. A missing file has none.
pub fn read(root: &Path) -> CoreResult<Vec<DecisionRecord>> {
    let path = path(root);
    match fs::read_to_string(&path) {
        Ok(text) => Ok(parse(&text)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(CoreError::io(&path, err)),
    }
}

/// The lines of `text` that parse and are not from a newer version.
pub fn parse(text: &str) -> Vec<DecisionRecord> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|raw| serde_json::from_str::<DecisionRecord>(raw).ok())
        .filter(|r| r.v <= DECISION_VERSION)
        .collect()
}

/// The decision on job `job` of `project` made at or before `before_ms`, the
/// latest one when there are several. Job ids come back once finished jobs
/// are cleared, so an answer after the moment asked about is another send's.
pub fn for_job<'a>(
    decisions: &'a [DecisionRecord],
    project: &str,
    job: &str,
    before_ms: f64,
) -> Option<&'a DecisionRecord> {
    decisions
        .iter()
        .filter(|d| d.covers(project, job) && d.at_ms <= before_ms)
        .max_by(|a, b| a.at_ms.total_cmp(&b.at_ms))
}

/// Gives each spend the decision that let it go: a send reaches the
/// provider (its first ledger line) only after it was approved.
pub fn attach(spends: &mut [Spend], decisions: &[DecisionRecord]) {
    for spend in spends {
        spend.decision = for_job(decisions, &spend.project, &spend.job, spend.sent_at_ms).cloned();
    }
}

#[cfg(test)]
mod tests;
