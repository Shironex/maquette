//! The small history behind the clear Inbox's numbers: what was accepted and
//! discarded lately, one line each in `round.json` in the inbox cache. It is
//! only ever counted, so a line that cannot be written is logged and the
//! model carries on. Redos are counted from their sends, in the shell.

use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::cache_dir;
use crate::json::write_json;
use crate::pipeline::history::now_ms;
use crate::workspace::WorkspaceConfig;

const FILE: &str = "round.json";
/// Lines older than this are dropped when the next one is written.
const KEEP_MS: f64 = 60.0 * 24.0 * 3600.0 * 1000.0;

/// One writer at a time: a line is read, added to and written back.
static WRITING: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    Accepted,
    Discarded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Line {
    kind: Kind,
    at_ms: f64,
    /// The inbox id it had.
    id: String,
    /// `project/asset`, for an accepted model.
    #[serde(default)]
    asset: Option<String>,
    /// The accepted asset went on to the library.
    #[serde(default)]
    library: bool,
}

/// The numbers of a round: everything since a moment the app chooses.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InboxRound {
    pub accepted: u32,
    pub to_library: u32,
    pub discarded: u32,
    pub redo_sent: u32,
    /// What those redos cost or are estimated to, as far as known.
    pub redo_credits: f64,
}

fn load(root: &Path, config: &WorkspaceConfig) -> Vec<Line> {
    std::fs::read_to_string(cache_dir(root, config).join(FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn change(root: &Path, config: &WorkspaceConfig, work: impl FnOnce(&mut Vec<Line>)) {
    let _one = WRITING.lock().unwrap_or_else(|p| p.into_inner());
    let mut lines = load(root, config);
    work(&mut lines);
    let oldest = now_ms() - KEEP_MS;
    lines.retain(|l| l.at_ms >= oldest);
    let dir = cache_dir(root, config);
    let written = std::fs::create_dir_all(&dir)
        .map_err(|e| crate::error::CoreError::io(&dir, e))
        .and_then(|()| write_json(&dir.join(FILE), &lines));
    if let Err(err) = written {
        tracing::warn!(error = %err, "could not note the inbox round");
    }
}

fn asset_key(project: &str, asset: &str) -> String {
    format!("{project}/{asset}")
}

pub(super) fn note_accepted(
    root: &Path,
    config: &WorkspaceConfig,
    id: &str,
    project: &str,
    asset: &str,
) {
    change(root, config, |lines| {
        lines.push(Line {
            kind: Kind::Accepted,
            at_ms: now_ms(),
            id: id.to_string(),
            asset: Some(asset_key(project, asset)),
            library: false,
        });
    });
}

pub(super) fn note_discarded(root: &Path, config: &WorkspaceConfig, id: &str, at_ms: f64) {
    change(root, config, |lines| {
        lines.push(Line {
            kind: Kind::Discarded,
            at_ms,
            id: id.to_string(),
            asset: None,
            library: false,
        });
    });
}

/// A restored model did not leave after all.
pub(super) fn note_restored(root: &Path, config: &WorkspaceConfig, id: &str) {
    change(root, config, |lines| {
        if let Some(at) = lines
            .iter()
            .rposition(|l| l.kind == Kind::Discarded && l.id == id)
        {
            lines.remove(at);
        }
    });
}

/// The asset went to the library: marks the model accepted for it, if any.
pub fn note_library(root: &Path, config: &WorkspaceConfig, project: &str, asset: &str) {
    let key = asset_key(project, asset);
    let known = load(root, config)
        .iter()
        .any(|l| l.asset.as_deref() == Some(&key));
    if !known {
        return;
    }
    change(root, config, |lines| {
        if let Some(line) = lines
            .iter_mut()
            .rev()
            .find(|l| l.asset.as_deref() == Some(&key))
        {
            line.library = true;
        }
    });
}

/// What was accepted and discarded since `since_ms`. The redo numbers are
/// the shell's to fill in, from the sends.
pub fn round(root: &Path, config: &WorkspaceConfig, since_ms: f64) -> InboxRound {
    let mut out = InboxRound::default();
    for line in load(root, config).iter().filter(|l| l.at_ms >= since_ms) {
        match line.kind {
            Kind::Accepted => {
                out.accepted += 1;
                out.to_library += u32::from(line.library);
            }
            Kind::Discarded => out.discarded += 1,
        }
    }
    out
}
