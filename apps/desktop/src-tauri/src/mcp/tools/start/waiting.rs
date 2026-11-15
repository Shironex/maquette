//! get_started's `waiting` list: every approval waiting for a person and
//! every project's sends to 3D in flight, each with the tool that follows
//! it, so a new session sees what is under way before it starts anything.

use maquette_core::providers::ProviderJob;
use serde_json::{Value, json};

use super::super::hints::table::step_with;
use super::super::to_value;
use super::next::Sends;
use crate::mcp::hub::{Approval, ApprovalKind};

/// The tool that follows an approval, with its arguments.
fn poll(a: &Approval) -> Value {
    let p = a.project.as_str();
    let (tool, why, args) = match a.kind {
        ApprovalKind::Ship => (
            "get_run",
            "The run waits before Ship.",
            json!({ "project": p, "runId": a.run_id }),
        ),
        ApprovalKind::SendTo3d => match (&a.job, a.items.is_empty()) {
            (Some(job), true) => (
                "get_provider_job",
                "The send waits before it spends credits.",
                json!({ "project": p, "id": job }),
            ),
            _ => (
                "get_provider_job",
                "The batch waits before it spends credits.",
                json!({ "project": p }),
            ),
        },
        ApprovalKind::ImageRound => (
            "get_concepts",
            "The image round waits before it spends dollars.",
            json!({ "project": p }),
        ),
        ApprovalKind::RestPose | ApprovalKind::Reweight | ApprovalKind::Springs => (
            "get_avatar",
            "The avatar's change waits for a person to look at the sheets.",
            json!({ "project": p, "asset": a.asset }),
        ),
        ApprovalKind::RigDoctor => (
            "get_rig_doctor",
            "The rig doctor's draft waits for a person to add it as a version.",
            json!({ "id": a.job }),
        ),
        ApprovalKind::RecipeBake => (
            "get_recipe_bake",
            "The bake waits for a person to review it in the Motion view.",
            json!({ "project": p, "recipe": a.job }),
        ),
        ApprovalKind::RecipeSave => (
            "get_recipe",
            "The save to the recipe library waits for a person.",
            json!({ "project": p, "recipe": a.recipe_save.as_ref().map(|s| &s.recipe) }),
        ),
        ApprovalKind::GripSave => (
            "list_grips",
            "The save to the grip library waits for a person.",
            json!({}),
        ),
        ApprovalKind::PagePreview => (
            "get_page_preview",
            "The first start of the site's dev server waits for a person.",
            json!({ "project": p }),
        ),
    };
    step_with(tool, why, args)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "an age in whole seconds, never negative"
)]
fn age_seconds(requested_at_ms: f64, now_ms: f64) -> u64 {
    ((now_ms - requested_at_ms).max(0.0) / 1000.0) as u64
}

/// Every approval waiting for a person, oldest first, then each project's
/// sends in flight.
pub(super) fn waiting(approvals: &[Approval], jobs: &[ProviderJob], now_ms: f64) -> Vec<Value> {
    let mut projects: Vec<&str> = jobs.iter().map(|j| j.project.as_str()).collect();
    projects.sort_unstable();
    projects.dedup();
    let sends: Vec<(&str, Sends)> = projects
        .into_iter()
        .map(|p| (p, Sends::of(jobs.iter().filter(|j| j.project == p))))
        .collect();
    listed(approvals, &sends, now_ms)
}

/// [`waiting`] with each project's sends already counted.
pub(super) fn listed(approvals: &[Approval], sends: &[(&str, Sends)], now_ms: f64) -> Vec<Value> {
    let mut sorted: Vec<&Approval> = approvals.iter().collect();
    sorted.sort_by(|a, b| a.requested_at_ms.total_cmp(&b.requested_at_ms));
    let mut out: Vec<Value> = sorted
        .into_iter()
        .map(|a| {
            json!({
                "kind": to_value(a.kind),
                "project": a.project,
                "approval": a.id,
                "what": a.asset_name.as_ref().or(a.detail.as_ref()),
                "ageSeconds": age_seconds(a.requested_at_ms, now_ms),
                "poll": poll(a),
            })
        })
        .collect();
    for &(project, sends) in sends {
        if sends.approval + sends.running > 0 {
            out.push(json!({
                "kind": "sends",
                "project": project,
                "waitingForApproval": sends.approval,
                "inFlight": sends.running,
                "poll": step_with(
                    "get_provider_job",
                    "Sends to 3D on Tripo or Meshy are not over yet. Do not send again.",
                    json!({ "project": project }),
                ),
            }));
        }
        if sends.by_hand > 0 {
            out.push(json!({
                "kind": "by-hand",
                "project": project,
                "byHandWaiting": sends.by_hand,
                "poll": step_with(
                    "list_handoffs",
                    "By hand sends wait for a person to bring the model back. Do not send again.",
                    json!({ "project": project }),
                ),
            }));
        }
    }
    out
}
