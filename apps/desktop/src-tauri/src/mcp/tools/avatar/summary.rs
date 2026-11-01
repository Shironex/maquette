//! The short answers of the avatar tools: a read without the bone and joint lists,
//! a set that says what changed.

use std::collections::BTreeSet;

use maquette_core::avatar::AvatarSpec;
use maquette_core::avatar::export::AvatarView;
use maquette_core::avatar::normalize::Normalized;
use maquette_core::avatar::reweight::Reweighted;
use maquette_core::inbox::vrm::{FindingLevel, VrmReadiness};
use serde_json::{Value, json};

use super::springs;
use crate::mcp::tools::to_short_value;

/// A reweight, short: the sheets to judge and its counts.
pub(super) fn reweight_summary(found: &Reweighted) -> Value {
    let report = &found.report;
    json!({
        "sourceVersion": found.source_version,
        "state": to_short_value(found.state),
        "sheets": to_short_value(&found.sheets),
        "moved": report.moved,
        "keptAsSleeve": report.kept_as_sleeve,
        "band": report.band,
        "bridges": to_short_value(&report.bridges),
        "bridgeFaces": to_short_value(found.bridges),
        "hiddenFaces": report.hidden_faces,
        "fromBones": to_short_value(&report.from_bones),
        "toBones": to_short_value(&report.to_bones),
    })
}

/// A normalised rest pose, short: the sheets to judge and its numbers.
pub(super) fn pose_summary(found: &Normalized) -> Value {
    json!({
        "sourceVersion": found.source_version,
        "state": to_short_value(found.state),
        "sheets": to_short_value(&found.sheets),
        "turnedDeg": found.report.turned_deg,
        "elbows": to_short_value(found.report.elbows),
        "fingers": to_short_value(found.report.fingers),
        "straightened": to_short_value(&found.report.straightened),
        "lifted": to_short_value(&found.report.lifted),
        "after": to_short_value(&found.report.after),
        "findingsAfter": to_short_value(&found.readiness.findings),
        "reweight": found.reweight.as_ref().map(|r| json!({
            "moved": r.moved,
            "keptAsSleeve": r.kept_as_sleeve,
        })),
    })
}

/// What a read of the avatar may add to its short answer.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Include {
    pub(super) bones: bool,
    pub(super) joints: bool,
}

impl Include {
    pub(super) fn parse(words: &[String]) -> Result<Self, String> {
        let mut out = Self::default();
        for word in words {
            match word.to_lowercase().as_str() {
                "bones" => out.bones = true,
                "joints" => out.joints = true,
                _ => return Err(format!("include takes bones or joints, not {word:?}.")),
            }
        }
        Ok(out)
    }
}

/// The readiness, short: counts, the missing required bones, the naming, the rest pose
/// measures and the findings. The 55 bones and every joint only when asked for.
pub(super) fn readiness_summary(r: &VrmReadiness, include: Include) -> Value {
    let missing: Vec<&str> = r
        .bones
        .iter()
        .filter(|b| b.required && b.joint.is_none())
        .map(|b| b.bone.as_str())
        .collect();
    let mut out = json!({
        "ready": r.ready,
        "mapped": r.mapped,
        "required": r.required,
        "missingRequired": missing,
        "naming": to_short_value(&r.naming),
        "pose": to_short_value(&r.pose),
        "findings": to_short_value(&r.findings),
        "jointCount": r.joints.len(),
    });
    if include.bones {
        out["bones"] = to_short_value(&r.bones);
    }
    if include.joints {
        out["joints"] = to_short_value(&r.joints);
    }
    out
}

/// The avatar, short: everything get_avatar says except the bone and joint lists.
pub(super) fn view_summary(view: &AvatarView, waiting: bool, include: Include) -> Value {
    json!({
        "project": view.project,
        "asset": view.asset,
        "name": view.name,
        "version": view.version,
        "model": view.model,
        "spec": to_short_value(&view.spec),
        "stored": view.stored,
        "readiness": view.readiness.as_ref().map(|r| readiness_summary(r, include)),
        "readinessError": view.readiness_error,
        "needsNormalize": view.needs_normalize,
        "needsReweight": view.needs_reweight,
        "reweighted": view.reweighted.as_ref().map(reweight_summary),
        "normalized": view.normalized.as_ref().map(pose_summary),
        "springs": {
            "ready": view.springs.ready,
            "found": view.springs.found.as_ref().map(springs::candidates_summary),
            "chained": view.springs.chained.as_ref().map(springs::chain_summary),
        },
        "export": to_short_value(&view.export),
        "exportPath": view.export_path,
        "metaProblem": view.meta_problem,
        "waitingApproval": waiting,
    })
}

/// The paths of the leaves that differ between two spec values, as `meta.name`.
pub(super) fn changed_fields(path: &str, before: &Value, after: &Value, out: &mut Vec<String>) {
    if let (Value::Object(a), Value::Object(b)) = (before, after) {
        let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
        for key in keys {
            let next = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            changed_fields(
                &next,
                a.get(key).unwrap_or(&Value::Null),
                b.get(key).unwrap_or(&Value::Null),
                out,
            );
        }
    } else if before != after {
        out.push(path.to_string());
    }
}

/// What a set_avatar answers: what changed, the spec, what still stops a build and the status.
pub(super) fn set_answer(before: &AvatarSpec, saved: &AvatarView) -> Value {
    let mut changed = Vec::new();
    changed_fields(
        "",
        &to_short_value(before),
        &to_short_value(&saved.spec),
        &mut changed,
    );
    let mut problems: Vec<String> = saved.meta_problem.iter().cloned().collect();
    problems.extend(saved.readiness_error.iter().cloned());
    if let Some(r) = &saved.readiness {
        problems.extend(
            r.findings
                .iter()
                .filter(|f| f.level == FindingLevel::Blocker)
                .map(|f| f.title.clone()),
        );
    }
    json!({
        "status": if changed.is_empty() { "unchanged" } else { "saved" },
        "changed": changed,
        "spec": to_short_value(&saved.spec),
        "problems": problems,
        "ready": saved.readiness.as_ref().is_some_and(|r| r.ready) && saved.meta_problem.is_none(),
        "needsReweight": saved.needs_reweight,
        "needsNormalize": saved.needs_normalize,
    })
}
