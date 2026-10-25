//! The credits ledger: every send that spends Tripo or Meshy credits, kept
//! per workspace in `credits.jsonl` next to `maquette.json`.
//!
//! The file only grows. A send writes a `sent` line once its first task
//! exists on the provider (that is when credits are held) and a `settled`
//! line when it ends, with what the provider reported. Reading folds the
//! lines of each send together, so a line that cannot be read (a crash
//! mid write, a hand edit) is skipped and counted instead of losing the
//! rest. Every line carries its version, so a newer Maquette can add
//! fields without breaking an older one.

use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::presets::ProviderId;
use crate::error::{CoreError, CoreResult};

pub const LEDGER_FILE: &str = "credits.jsonl";
const LINE_VERSION: u32 = 1;
const DAY_MS: f64 = 86_400_000.0;

/// One writer at a time, so two jobs settling together never interleave.
static WRITE: Mutex<()> = Mutex::new(());

/// How a send that spent credits ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SpendOutcome {
    /// Still on the provider: the credits are held, not yet charged.
    Pending,
    Succeeded,
    /// Neither service charges a failed task; a chain that failed after its
    /// first task keeps what that task cost.
    Failed,
    /// Cancelled while the provider could still give the credits back.
    Refunded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineEvent {
    Sent,
    Settled,
}

/// One line of `credits.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerLine {
    pub v: u32,
    pub at_ms: f64,
    pub event: LineEvent,
    pub project: String,
    /// The provider job id, `<asset>-<n>`.
    pub job: String,
    pub asset: String,
    pub asset_name: String,
    pub service: ProviderId,
    /// The provider's id of the first task.
    pub task: Option<String>,
    /// The settings in a few words.
    pub summary: String,
    /// From the price lists, for when the provider reports nothing.
    pub estimate: Option<f64>,
    /// What the provider reported, summed over the chain.
    pub credits: Option<f64>,
    pub outcome: SpendOutcome,
    /// The send this one redid, if it came from a Redo.
    pub redo_of: Option<String>,
    pub via_agent: bool,
}

impl LedgerLine {
    pub fn new(event: LineEvent, at_ms: f64, outcome: SpendOutcome) -> Self {
        Self {
            v: LINE_VERSION,
            at_ms,
            event,
            project: String::new(),
            job: String::new(),
            asset: String::new(),
            asset_name: String::new(),
            service: ProviderId::Tripo,
            task: None,
            summary: String::new(),
            estimate: None,
            credits: None,
            outcome,
            redo_of: None,
            via_agent: false,
        }
    }
}

/// One send as the Credits view lists it: its lines folded together.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Spend {
    pub project: String,
    pub job: String,
    pub asset: String,
    pub asset_name: String,
    pub service: ProviderId,
    pub task: Option<String>,
    pub summary: String,
    pub sent_at_ms: f64,
    pub settled_at_ms: Option<f64>,
    /// Charged, or held while pending.
    pub credits: f64,
    /// True when `credits` comes from the price lists, not the provider.
    pub estimated: bool,
    pub outcome: SpendOutcome,
    pub redo_of: Option<String>,
    pub via_agent: bool,
}

/// The ledger as read: newest send first.
#[derive(Debug, Clone, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Ledger {
    pub spends: Vec<Spend>,
    /// Lines that could not be read and were left out.
    pub skipped: u32,
}

pub fn path(root: &Path) -> PathBuf {
    root.join(LEDGER_FILE)
}

/// Adds one line to the end of the file.
pub fn append(root: &Path, line: &LedgerLine) -> CoreResult<()> {
    let path = path(root);
    let mut text = serde_json::to_string(line)
        .map_err(|e| CoreError::Invalid(format!("could not write a ledger line: {e}")))?;
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

/// Reads the ledger. A missing file is an empty ledger.
pub fn read(root: &Path) -> CoreResult<Ledger> {
    let path = path(root);
    match fs::read_to_string(&path) {
        Ok(text) => Ok(parse(&text)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Ledger::default()),
        Err(err) => Err(CoreError::io(&path, err)),
    }
}

/// Folds the lines of `text`, skipping and counting the ones that do not
/// parse or come from a newer version.
pub fn parse(text: &str) -> Ledger {
    let mut lines = Vec::new();
    let mut skipped = 0;
    for raw in text.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str::<LedgerLine>(raw) {
            Ok(line) if line.v <= LINE_VERSION => lines.push(line),
            _ => skipped += 1,
        }
    }
    Ledger {
        spends: fold(&lines),
        skipped,
    }
}

/// What a send was charged, and whether that is the price list's guess.
pub fn charged(outcome: SpendOutcome, reported: Option<f64>, estimate: Option<f64>) -> (f64, bool) {
    match (outcome, reported) {
        (SpendOutcome::Refunded, _) => (0.0, false),
        (SpendOutcome::Failed, reported) => (reported.unwrap_or(0.0), false),
        (SpendOutcome::Succeeded, Some(credits)) => (credits, false),
        (SpendOutcome::Pending | SpendOutcome::Succeeded, _) => {
            (estimate.unwrap_or(0.0), estimate.is_some())
        }
    }
}

/// One spend per send, the later lines filling in the earlier ones.
pub fn fold(lines: &[LedgerLine]) -> Vec<Spend> {
    let mut spends: Vec<(Spend, Option<f64>, Option<f64>)> = Vec::new();
    for line in lines {
        let at = spends
            .iter()
            .position(|(s, _, _)| s.project == line.project && s.job == line.job);
        let at = at.unwrap_or_else(|| {
            spends.push((
                Spend {
                    project: line.project.clone(),
                    job: line.job.clone(),
                    asset: line.asset.clone(),
                    asset_name: line.asset_name.clone(),
                    service: line.service,
                    task: None,
                    summary: String::new(),
                    sent_at_ms: line.at_ms,
                    settled_at_ms: None,
                    credits: 0.0,
                    estimated: false,
                    outcome: SpendOutcome::Pending,
                    redo_of: None,
                    via_agent: false,
                },
                None,
                None,
            ));
            spends.len() - 1
        });
        let (spend, reported, estimate) = &mut spends[at];
        if line.event == LineEvent::Settled {
            spend.settled_at_ms = Some(line.at_ms);
            spend.outcome = line.outcome;
        }
        spend.sent_at_ms = spend.sent_at_ms.min(line.at_ms);
        if !line.summary.is_empty() {
            spend.summary.clone_from(&line.summary);
        }
        spend.task = line.task.clone().or(spend.task.take());
        spend.redo_of = line.redo_of.clone().or(spend.redo_of.take());
        spend.via_agent |= line.via_agent;
        *reported = line.credits.or(*reported);
        *estimate = line.estimate.or(*estimate);
    }
    let mut out: Vec<Spend> = spends
        .into_iter()
        .map(|(mut spend, reported, estimate)| {
            (spend.credits, spend.estimated) = charged(spend.outcome, reported, estimate);
            spend
        })
        .collect();
    out.sort_by(|a, b| b.sent_at_ms.total_cmp(&a.sent_at_ms));
    out
}

/// Totals for one service.
#[derive(Debug, Clone, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServiceTotals {
    pub service: Option<ProviderId>,
    /// Sent in the last 7 days.
    pub week: f64,
    /// Sent in the last 30 days.
    pub month: f64,
    pub total: f64,
    pub sends: u32,
    pub redos: u32,
    /// Credits that went on redos, over all credits.
    pub redo_share: f64,
    /// Sends whose credits are the price list's guess.
    pub estimated: u32,
}

/// The whole ledger in a few numbers: one row per service, then the sum.
pub fn summarise(spends: &[Spend], now_ms: f64) -> Vec<ServiceTotals> {
    let totals = |service: Option<ProviderId>| {
        let mine = spends
            .iter()
            .filter(|s| service.is_none_or(|id| s.service == id));
        let mut t = ServiceTotals {
            service,
            ..ServiceTotals::default()
        };
        let mut redo_credits = 0.0;
        for spend in mine {
            let age = now_ms - spend.sent_at_ms;
            if age <= 7.0 * DAY_MS {
                t.week += spend.credits;
            }
            if age <= 30.0 * DAY_MS {
                t.month += spend.credits;
            }
            t.total += spend.credits;
            t.sends += 1;
            t.estimated += u32::from(spend.estimated);
            if spend.redo_of.is_some() {
                t.redos += 1;
                redo_credits += spend.credits;
            }
        }
        t.redo_share = if t.total > 0.0 {
            redo_credits / t.total
        } else {
            0.0
        };
        t
    };
    ProviderId::ALL
        .iter()
        .map(|&s| totals(Some(s)))
        .chain([totals(None)])
        .collect()
}

#[cfg(test)]
mod tests;
