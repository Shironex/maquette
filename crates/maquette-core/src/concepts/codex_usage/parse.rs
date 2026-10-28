//! Reading the answer to `account/rateLimits/read`. Only what the app
//! shows is read, and leniently: a missing field is `None`, never an error.

use serde::Deserialize;
use serde_json::Value;

use super::{CodexUsage, UsageCredits, UsageWindow};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    rate_limits: Option<Snapshot>,
    #[serde(default)]
    ordinary_usage_allowed: Option<bool>,
    #[serde(default)]
    rate_limit_reset_credits: Option<ResetCredits>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    #[serde(default)]
    primary: Option<Window>,
    #[serde(default)]
    secondary: Option<Window>,
    #[serde(default)]
    credits: Option<Credits>,
    #[serde(default)]
    plan_type: Option<String>,
    #[serde(default)]
    rate_limit_reached_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Window {
    used_percent: f64,
    #[serde(default)]
    window_duration_mins: Option<f64>,
    /// Unix seconds.
    #[serde(default)]
    resets_at: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Credits {
    #[serde(default)]
    has_credits: bool,
    #[serde(default)]
    unlimited: bool,
    #[serde(default)]
    balance: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetCredits {
    #[serde(default)]
    available_count: Option<f64>,
}

impl From<Window> for UsageWindow {
    fn from(w: Window) -> Self {
        Self {
            used_percent: w.used_percent.clamp(0.0, 100.0),
            window_mins: w.window_duration_mins.filter(|m| *m > 0.0),
            resets_at_ms: w.resets_at.filter(|s| *s > 0.0).map(|s| s * 1000.0),
        }
    }
}

/// The usage in the `result` of an `account/rateLimits/read` answer, read
/// at `now_ms`. An answer without `rateLimits` reads as unknown.
pub fn from_response(result: &Value, now_ms: f64) -> CodexUsage {
    let Ok(response) = Response::deserialize(result) else {
        return CodexUsage::unknown("Codex sent a usage answer this app cannot read.", now_ms);
    };
    let Some(snapshot) = response.rate_limits else {
        return CodexUsage::unknown("Codex sent no usage windows.", now_ms);
    };
    CodexUsage {
        known: true,
        reason: None,
        read_at_ms: now_ms,
        primary: snapshot.primary.map(UsageWindow::from),
        secondary: snapshot.secondary.map(UsageWindow::from),
        plan: snapshot.plan_type,
        limit_reached: snapshot.rate_limit_reached_type,
        ordinary_usage_allowed: response.ordinary_usage_allowed,
        credits: snapshot.credits.map(|c| UsageCredits {
            has_credits: c.has_credits,
            unlimited: c.unlimited,
            balance: c.balance,
        }),
        reset_credits: response
            .rate_limit_reset_credits
            .and_then(|r| r.available_count),
    }
}
