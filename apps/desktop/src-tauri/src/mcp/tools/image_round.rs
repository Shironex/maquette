//! An agent's round on OpenAI's Image API waits for a person: the tools
//! that queue Lab images (start_concepts, make_object_refs,
//! make_other_views, clean_ref_background, retry_concept_jobs) end here.
//! When their jobs wait in the `approval` state, the round goes into the
//! app's approvals with what it should cost, a thread waits for the answer
//! and releases or denies the jobs, and the result says it is waiting.
//! A Codex round, and a round the person starts in the app, never wait.

use std::sync::Arc;

use maquette_core::pipeline::history::now_ms;
use maquette_core::providers::ledger::{self, month_to_date};
use serde_json::{Value, json};

use super::{McpTools, ToolError, WAITING};
use crate::mcp::host::Host;
use crate::mcp::hub::{Hub, ImageRound};

/// What the result tells the agent about a round that waits.
pub const ROUND_NOTE: &str = "A person approves or denies this OpenAI Images round in the Maquette app; nothing is sent or spent until then. Poll get_concepts (the jobs are in state approval, then queued once approved, or cancelled when denied); do not start the round again.";

/// The ids of the result's jobs that wait for approval.
fn held(value: &Value) -> Vec<String> {
    value
        .get("jobs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|j| j.get("state").and_then(Value::as_str) == Some("approval"))
        .filter_map(|j| j.get("id").and_then(Value::as_str).map(str::to_string))
        .collect()
}

/// The direction the result names, for the approval's title.
fn direction(value: &Value) -> Option<String> {
    value
        .pointer("/direction/name")
        .or_else(|| value.pointer("/jobs/0/direction"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// "4 images at medium quality, about $0.27. $1.20 of $10 spent this
/// month." for the approval card.
fn detail(host: &dyn Host, images: usize, estimate: f64) -> String {
    let what = match images {
        1 => "1 image".to_string(),
        n => format!("{n} images"),
    };
    let Ok((root, config)) = host.session().workspace() else {
        return format!("{what}, about ${estimate:.2}.");
    };
    let settings = config.lab.images;
    let spent = ledger::read(&root)
        .map(|l| month_to_date(&l.images, now_ms()))
        .unwrap_or_default();
    format!(
        "{what} at {} quality, about ${estimate:.2}. ${spent:.2} of ${} spent this month.",
        settings.quality.as_str(),
        settings.monthly_cap_usd
    )
}

/// Waits for the person's answer on its own thread, then sends or cancels
/// the jobs still waiting. Jobs cancelled in the Lab meanwhile withdraw
/// the approval once none is left waiting.
fn await_round(host: Arc<dyn Host>, hub: Arc<Hub>, approval: String, jobs: Vec<String>) {
    let spawned = std::thread::Builder::new()
        .name("image-round-approval".into())
        .spawn(move || {
            let gone = || host.concepts().awaiting_approval(&jobs).is_empty();
            match hub.wait_until(&approval, gone) {
                Some(true) => {
                    host.concepts().release(&jobs);
                }
                Some(false) => {
                    host.concepts().deny(&jobs);
                }
                None => {}
            }
        });
    if let Err(err) = spawned {
        tracing::warn!(error = %err, "could not wait for an image round approval");
    }
}

impl McpTools {
    /// Asks the person for the round when the result's jobs wait for
    /// approval, and marks the result as waiting. Anything else passes.
    pub(super) async fn ask_for_round(
        &self,
        project: String,
        found: Result<Value, ToolError>,
    ) -> Result<Value, ToolError> {
        let mut value = found?;
        let jobs = held(&value);
        if jobs.is_empty() {
            return Ok(value);
        }
        let host = Arc::clone(&self.host);
        let hub = Arc::clone(&self.hub);
        let direction = direction(&value);
        let (id, estimate) = tokio::task::spawn_blocking(move || {
            let estimate = host.concepts().estimate_usd(&jobs);
            let detail = detail(host.as_ref(), jobs.len(), estimate);
            let id = hub.request_image_round(ImageRound {
                project,
                direction,
                jobs: jobs.clone(),
                estimate_usd: estimate,
                detail,
            });
            await_round(host, hub, id.clone(), jobs);
            (id, estimate)
        })
        .await
        .map_err(|e| ToolError::from(format!("The task stopped unexpectedly. {e}")))?;
        value["status"] = json!(WAITING);
        value["approval"] = json!({
            "id": id,
            "estimateUsd": (estimate * 1000.0).round() / 1000.0,
            "note": ROUND_NOTE,
        });
        Ok(value)
    }
}
