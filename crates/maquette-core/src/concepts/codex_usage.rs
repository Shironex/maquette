//! How much Codex usage is left: one read of `account/rateLimits/read`
//! through `codex app-server`.
//!
//! The app server speaks newline delimited JSON over stdio, with no
//! `jsonrpc` field: an `initialize` request, an `initialized` notification,
//! then the request. Each read starts its own server, asks once and stops
//! it. The protocol is marked experimental, so anything unexpected (an old
//! Codex, an error answer, a timeout, a shape that does not parse) reads as
//! [`CodexUsage::unknown`] and never as an error.
//!
//! The server runs in its own process group (a job object on Windows) and
//! is killed when the read ends, however it ends. Its standard input is a
//! pipe, so it also stops on its own if Maquette goes away mid read.

mod parse;
mod server;
#[cfg(test)]
mod tests;

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::pipeline::tools::Toolbox;
use crate::toolchain::ToolId;

pub use parse::from_response;

/// The longest one read may take, start to stop.
pub const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// The oldest Codex CLI asked: older ones may not have the app server, or
/// may wait on a request they do not know instead of answering.
const OLDEST: (u32, u32) = (0, 130);

/// One usage window: the five hour one or the weekly one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    /// 0 to 100.
    pub used_percent: f64,
    /// 300 for the five hour window, 10080 for the week.
    pub window_mins: Option<f64>,
    /// When the window starts over, ms since the epoch.
    pub resets_at_ms: Option<f64>,
}

impl UsageWindow {
    pub fn is_full(&self) -> bool {
        self.used_percent >= 100.0
    }
}

/// The account's Codex credits, as the app server reports them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageCredits {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
}

/// What one read found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CodexUsage {
    /// False when the read failed or this Codex cannot say; `reason` says why.
    pub known: bool,
    pub reason: Option<String>,
    /// When the read happened, ms since the epoch.
    pub read_at_ms: f64,
    /// The five hour window.
    pub primary: Option<UsageWindow>,
    /// The weekly window.
    pub secondary: Option<UsageWindow>,
    /// "plus", "pro" and so on.
    pub plan: Option<String>,
    /// Set while a limit holds, for example "rate_limit_reached".
    pub limit_reached: Option<String>,
    /// Whether the account may use its included usage now. `None` when the
    /// server does not say; then nothing is read into the percentages.
    pub ordinary_usage_allowed: Option<bool>,
    pub credits: Option<UsageCredits>,
    /// Limit reset credits the account may spend, when the server says.
    pub reset_credits: Option<f64>,
}

impl CodexUsage {
    pub fn unknown(reason: impl Into<String>, read_at_ms: f64) -> Self {
        Self {
            known: false,
            reason: Some(reason.into()),
            read_at_ms,
            primary: None,
            secondary: None,
            plan: None,
            limit_reached: None,
            ordinary_usage_allowed: None,
            credits: None,
            reset_credits: None,
        }
    }

    fn windows(&self) -> impl Iterator<Item = &UsageWindow> {
        self.primary.iter().chain(self.secondary.iter())
    }

    /// True while Codex's usage limit holds by this read.
    pub fn limit_active(&self) -> bool {
        self.known
            && (self.limit_reached.is_some()
                || self.ordinary_usage_allowed == Some(false)
                || self.windows().any(UsageWindow::is_full))
    }

    /// When the limit that holds lifts, ms since the epoch: the latest reset
    /// of a full window, else the five hour window's. `None` when no limit
    /// holds or the server named no time.
    pub fn resets_at_ms(&self) -> Option<f64> {
        if !self.limit_active() {
            return None;
        }
        let full = self
            .windows()
            .filter(|w| w.is_full())
            .filter_map(|w| w.resets_at_ms)
            .reduce(f64::max);
        full.or_else(|| self.primary.as_ref().and_then(|w| w.resets_at_ms))
    }

    /// True when jobs held for the usage limit may run now: the server says
    /// ordinary usage is allowed and the five hour window is not full.
    pub fn can_run_now(&self) -> bool {
        self.known
            && self.ordinary_usage_allowed == Some(true)
            && self.primary.as_ref().is_some_and(|w| !w.is_full())
    }
}

/// "Tue 16:22", ms since the epoch in local time, for messages.
pub fn local_time(ms: f64) -> String {
    use chrono::TimeZone as _;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "ms since the epoch fit i64"
    )]
    chrono::Local
        .timestamp_millis_opt(ms as i64)
        .single()
        .map(|t| t.format("%a %H:%M").to_string())
        .unwrap_or_default()
}

/// True when this Codex CLI version is new enough to ask. An unknown
/// version is not asked.
pub fn supported(version: Option<&str>) -> bool {
    let Some(version) = version else {
        return false;
    };
    let mut parts = version
        .trim()
        .split(['.', '-', '+'])
        .map(|p| p.parse::<u32>().ok());
    match (parts.next().flatten(), parts.next().flatten()) {
        (Some(major), Some(minor)) => (major, minor) >= OLDEST,
        _ => false,
    }
}

/// Reads the usage with the Codex CLI the tool check found.
pub fn read(tools: &Toolbox) -> CodexUsage {
    let now = crate::pipeline::history::now_ms();
    let Some(codex) = tools.tools.get(&ToolId::Codex) else {
        return CodexUsage::unknown(
            "Codex CLI is not installed or did not pass the tool check.",
            now,
        );
    };
    if !supported(Some(&codex.version)) {
        return CodexUsage::unknown(
            format!(
                "Codex CLI {} cannot report its usage. Update it to read the usage here.",
                codex.version
            ),
            now,
        );
    }
    server::ask(&codex.program, &tools.path_env, READ_TIMEOUT)
}
