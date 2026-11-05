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
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::{Spend, SpendSource};
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
    /// Still waiting when Maquette quit or another workspace was opened.
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
    /// A recipe bake's review in the Motion view: the bake was added, or a
    /// new bake of the recipe replaced it.
    RecipeReview,
    /// The save to library sheet: the person saved the recipe there, or an
    /// agent's later proposal replaced it.
    RecipeSave,
    /// The jobs it held left the approval state first: cancelled in Sends
    /// or the Lab, or by an agent's cancel of its own Lab jobs.
    JobsCancelled,
    /// The run it held was cancelled.
    RunCancelled,
    /// A page preview's folder or command changed, so the card was replaced.
    QuestionChanged,
    /// Still waiting when Maquette quit.
    Quit,
    /// Still waiting when another workspace was opened.
    WorkspaceChanged,
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

/// Who answered, from where and how. Two kinds of word, kept apart:
/// `source`, `input`, `trusted`, `page_focused` and `page_visible` are what
/// the app's own page reported with its answer, so they are only as good as
/// that page; `window_focused`, `window_visible` and `window_minimized` are
/// read by the app from the operating system when the answer arrives, and
/// replace anything the page said about them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DecidedBy {
    /// Reported by the page for a card's answer; set by the app otherwise.
    pub source: DecisionSource,
    /// Reported by the page: what pressed the button.
    #[serde(default)]
    pub input: Option<DecisionInput>,
    /// Reported by the page: the browser's `isTrusted` for the click, false
    /// when a script made it. True does not prove a person pressed it.
    #[serde(default)]
    pub trusted: Option<bool>,
    /// Reported by the page: it had the keyboard focus.
    #[serde(default)]
    pub page_focused: Option<bool>,
    /// Reported by the page: it was visible.
    #[serde(default)]
    pub page_visible: Option<bool>,
    /// Read from the operating system: the window had the focus.
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
    /// `rest-pose`, `reweight`, `springs`, `rig-doctor` or `recipe-bake`.
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
    // A line torn by a crash mid write gets its own line end first, so it
    // costs only itself, never the answer written after it.
    if ends_torn(&path) {
        text.insert(0, '\n');
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| CoreError::io(&path, e))?;
    file.write_all(text.as_bytes())
        .map_err(|e| CoreError::io(&path, e))
}

/// True when the file has bytes and does not end with a line end.
fn ends_torn(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut last = [0u8; 1];
    file.seek(SeekFrom::End(-1)).is_ok() && file.read_exact(&mut last).is_ok() && last[0] != b'\n'
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

/// What a send to 3D's approval is called in a record.
pub const SEND_KIND: &str = "send-to-3d";

/// The answer to the send-to-3D approval of job `job` of `project` made
/// after `after_ms` and at or before `before_ms`, the latest when there are
/// several. Job ids come back once finished jobs are cleared or trimmed, so
/// the window must start where this use of the id did. The one place both
/// the ledger and get_provider_job match a send to its answer.
pub fn send_answer<'a>(
    decisions: &'a [DecisionRecord],
    project: &str,
    job: &str,
    after_ms: f64,
    before_ms: f64,
) -> Option<&'a DecisionRecord> {
    decisions
        .iter()
        .filter(|d| d.kind == SEND_KIND && d.covers(project, job))
        .filter(|d| d.at_ms > after_ms && d.at_ms <= before_ms)
        .max_by(|a, b| a.at_ms.total_cmp(&b.at_ms))
}

/// Gives each API send an agent asked for the approval that let it go: an
/// approved answer made after the id's previous spend and before this one
/// reached the provider. A send started in the app, or one whose only
/// answer is a no or another use of the id, gets none.
pub fn attach(spends: &mut [Spend], decisions: &[DecisionRecord]) {
    let starts: Vec<(f64, &str, &str)> = spends
        .iter()
        .map(|s| (s.sent_at_ms, s.project.as_str(), s.job.as_str()))
        .collect();
    let found: Vec<Option<DecisionRecord>> = spends
        .iter()
        .map(|spend| {
            if !spend.via_agent || spend.source != SpendSource::Api {
                return None;
            }
            let after = starts
                .iter()
                .filter(|(at, project, job)| {
                    *project == spend.project && *job == spend.job && *at < spend.sent_at_ms
                })
                .map(|(at, _, _)| *at)
                .fold(f64::NEG_INFINITY, f64::max);
            send_answer(
                decisions,
                &spend.project,
                &spend.job,
                after,
                spend.sent_at_ms,
            )
            .filter(|d| d.decision == Decision::Approved)
            .cloned()
        })
        .collect();
    for (spend, decision) in spends.iter_mut().zip(found) {
        spend.decision = decision;
    }
}

#[cfg(test)]
mod tests;
