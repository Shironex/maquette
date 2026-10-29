//! Run records and their logs, kept in the project's cache folder so the
//! Pipeline view and the status bar still know the last run after a restart.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::CoreResult;
use crate::json::write_json;

const RUNS_FILE: &str = "runs.json";
/// Runs kept in `runs.json`.
pub const KEEP_RUNS: usize = 20;
/// Runs whose full log is kept.
pub const KEEP_LOGS: usize = 5;
/// Lines kept per run log. Blender can print a lot; the tail matters most.
pub const MAX_LOG_LINES: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RunState {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl RunState {
    pub fn is_finished(self) -> bool {
        matches!(
            self,
            RunState::Done | RunState::Failed | RunState::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum StepState {
    /// Planned, not started yet.
    Waiting,
    Running,
    /// Inputs unchanged, skipped.
    Cached,
    Done,
    Failed,
    /// Not part of this run, or not reached because an earlier step failed.
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StepRecord {
    pub id: String,
    pub name: String,
    pub state: StepState,
    pub started_at_ms: Option<f64>,
    pub duration_ms: Option<f64>,
    pub exit_code: Option<i32>,
    /// Why it failed or was skipped.
    pub message: Option<String>,
    /// From 0 to 1 while the step runs, when its tool reports how far it
    /// is (see `pipeline::progress`). `None` draws an indeterminate bar.
    #[serde(default)]
    pub progress: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    /// `<project>-run-<number>`, unique across projects.
    pub id: String,
    /// Counts up per project, for "Run 25".
    pub number: u32,
    pub project: String,
    /// The step "Run from here" started at.
    pub from: Option<String>,
    pub force: bool,
    /// The run stopped before Ship.
    #[serde(default)]
    pub build_only: bool,
    pub state: RunState,
    pub queued_at_ms: f64,
    pub started_at_ms: Option<f64>,
    pub finished_at_ms: Option<f64>,
    /// In run order.
    pub steps: Vec<StepRecord>,
    /// The failure or cancel reason, for the banner.
    pub message: Option<String>,
}

impl RunRecord {
    pub fn failed_step(&self) -> Option<&StepRecord> {
        self.steps.iter().find(|s| s.state == StepState::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum LogLevel {
    Info,
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum LogStream {
    Stdout,
    Stderr,
    /// Lines the runner writes itself.
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub run_id: String,
    pub step: Option<String>,
    pub stream: LogStream,
    pub level: LogLevel,
    pub text: String,
    pub at_ms: f64,
}

/// Milliseconds since the epoch, as a JS safe number.
#[expect(
    clippy::cast_precision_loss,
    reason = "epoch milliseconds stay far below 2^53"
)]
pub fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// A level for a tool's output line: errors and tracebacks, warnings, the
/// rest plain.
pub fn classify(text: &str) -> LogLevel {
    let lower = text.trim_start().to_ascii_lowercase();
    let error = lower.starts_with("error")
        || lower.starts_with("traceback")
        || lower.starts_with("fatal")
        || lower.contains(" error:")
        || lower.contains("exception:")
        || lower.starts_with("raise ")
        || (lower.contains("error") && lower.contains("line "));
    if error {
        return LogLevel::Error;
    }
    if lower.starts_with("warning") || lower.starts_with("warn") || lower.contains(" warning:") {
        return LogLevel::Warn;
    }
    LogLevel::Info
}

pub fn load_runs(dir: &Path) -> Vec<RunRecord> {
    fs::read_to_string(dir.join(RUNS_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Puts `run` first (replacing an older copy of it) and keeps the newest
/// [`KEEP_RUNS`].
pub fn record_run(dir: &Path, run: &RunRecord) -> CoreResult<Vec<RunRecord>> {
    let mut runs = load_runs(dir);
    runs.retain(|r| r.id != run.id);
    runs.insert(0, run.clone());
    runs.truncate(KEEP_RUNS);
    fs::create_dir_all(dir).map_err(|e| crate::error::CoreError::io(dir, e))?;
    write_json(&dir.join(RUNS_FILE), &runs)?;
    Ok(runs)
}

fn log_path(dir: &Path, number: u32) -> PathBuf {
    dir.join("logs").join(format!("run-{number}.json"))
}

/// Saves a run's log (its last [`MAX_LOG_LINES`] lines) and drops logs
/// older than the last [`KEEP_LOGS`] runs.
pub fn save_log(dir: &Path, number: u32, entries: &[LogEntry]) -> CoreResult<()> {
    let logs = dir.join("logs");
    fs::create_dir_all(&logs).map_err(|e| crate::error::CoreError::io(&logs, e))?;
    let start = entries.len().saturating_sub(MAX_LOG_LINES);
    write_json(&log_path(dir, number), &entries[start..])?;
    if let Ok(read) = fs::read_dir(&logs) {
        for entry in read.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let old = name
                .strip_prefix("run-")
                .and_then(|n| n.strip_suffix(".json"))
                .and_then(|n| n.parse::<u32>().ok())
                .is_some_and(|n| n + u32::try_from(KEEP_LOGS).unwrap_or(5) <= number);
            if old {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

pub fn load_log(dir: &Path, number: u32) -> Vec<LogEntry> {
    fs::read_to_string(log_path(dir, number))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(number: u32) -> RunRecord {
        RunRecord {
            id: format!("moon-run-{number}"),
            number,
            project: "moon".into(),
            from: None,
            force: false,
            build_only: false,
            state: RunState::Done,
            queued_at_ms: 1.0,
            started_at_ms: Some(2.0),
            finished_at_ms: Some(3.0),
            steps: Vec::new(),
            message: None,
        }
    }

    fn entry(text: &str) -> LogEntry {
        LogEntry {
            run_id: "r".into(),
            step: None,
            stream: LogStream::Stdout,
            level: LogLevel::Info,
            text: text.into(),
            at_ms: 0.0,
        }
    }

    #[test]
    fn runs_keep_the_newest_first() {
        let dir = tempfile::tempdir().expect("tempdir");
        for n in 1..=25 {
            record_run(dir.path(), &run(n)).expect("record");
        }
        let mut updated = run(25);
        updated.state = RunState::Failed;
        let runs = record_run(dir.path(), &updated).expect("update");
        assert_eq!(runs.len(), KEEP_RUNS);
        assert_eq!(runs[0].number, 25);
        assert_eq!(runs[0].state, RunState::Failed);
        assert_eq!(runs[1].number, 24);
        assert_eq!(load_runs(dir.path()), runs);
    }

    #[test]
    fn logs_are_capped_and_rotated() {
        let dir = tempfile::tempdir().expect("tempdir");
        let many: Vec<LogEntry> = (0..MAX_LOG_LINES + 10)
            .map(|i| entry(&i.to_string()))
            .collect();
        save_log(dir.path(), 1, &many).expect("save");
        let back = load_log(dir.path(), 1);
        assert_eq!(back.len(), MAX_LOG_LINES);
        assert_eq!(back[0].text, "10");
        for n in 2..=7 {
            save_log(dir.path(), n, &[entry("x")]).expect("save");
        }
        assert!(load_log(dir.path(), 1).is_empty(), "rotated out");
        assert_eq!(load_log(dir.path(), 7).len(), 1);
        assert_eq!(load_log(dir.path(), 3).len(), 1);
    }

    #[test]
    fn levels() {
        assert_eq!(
            classify("Traceback (most recent call last):"),
            LogLevel::Error
        );
        assert_eq!(classify("Error: Python: camera not found"), LogLevel::Error);
        assert_eq!(
            classify("  File \"x.py\", line 3, in <module> NameError"),
            LogLevel::Error
        );
        assert_eq!(classify("Warning: 2 textures over 2048"), LogLevel::Warn);
        assert_eq!(
            classify("Blender 4.3.2 (hash 32f5fdce0a0a built 2024-12-17)"),
            LogLevel::Info
        );
        assert_eq!(classify("Saved \"island.blend\""), LogLevel::Info);
    }
}
