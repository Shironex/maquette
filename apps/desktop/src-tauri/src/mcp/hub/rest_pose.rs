//! Avatar approvals: an agent's avatar build whose new weights (cloth and
//! hair freed from the arms) or new rest pose wait for a person to look at
//! the before and after sheets.

use maquette_core::avatar::normalize::{NormalizeSheets, Normalized};
use maquette_core::avatar::reweight::Reweighted;
use maquette_core::pipeline::history::now_ms;

use super::{Approval, ApprovalKind, Hub};

/// What one avatar approval names.
struct AvatarAsk<'a> {
    kind: ApprovalKind,
    project: &'a str,
    asset: &'a str,
    version: u32,
    sheets: &'a NormalizeSheets,
    detail: String,
}

impl Hub {
    /// Records that an avatar's new rest pose, which an agent's build made,
    /// waits for a person. One per asset: asking again returns the same id.
    pub fn request_rest_pose(&self, normalized: &Normalized, asset_name: &str) -> String {
        let lifted: Vec<String> = normalized
            .report
            .lifted
            .iter()
            .map(|(side, lift)| format!("{side} arm {:.0}", lift.shoulder_deg.abs()))
            .collect();
        let mut detail = format!("v{}", normalized.source_version);
        if normalized.report.turned_deg.abs() >= 0.5 {
            detail.push_str(&format!(
                ", turned {:.0} degrees",
                normalized.report.turned_deg.abs()
            ));
        }
        if !lifted.is_empty() {
            detail.push_str(&format!(", lifted {} degrees", lifted.join(" and ")));
        }
        self.request_avatar(
            AvatarAsk {
                kind: ApprovalKind::RestPose,
                project: &normalized.project,
                asset: &normalized.asset,
                version: normalized.source_version,
                sheets: &normalized.sheets,
                detail,
            },
            asset_name,
        )
    }

    /// Records that an avatar's new weights, which an agent's build made,
    /// wait for a person. One per asset: asking again returns the same id.
    pub fn request_reweight(&self, reweighted: &Reweighted, asset_name: &str) -> String {
        let report = &reweighted.report;
        self.request_avatar(
            AvatarAsk {
                kind: ApprovalKind::Reweight,
                project: &reweighted.project,
                asset: &reweighted.asset,
                version: reweighted.source_version,
                sheets: &reweighted.sheets,
                detail: format!(
                    "v{}, {} vertices freed from the arms, {} kept as sleeve",
                    reweighted.source_version, report.moved, report.kept_as_sleeve
                ),
            },
            asset_name,
        )
    }

    fn request_avatar(&self, ask: AvatarAsk<'_>, asset_name: &str) -> String {
        if let Some(found) = self.avatar_approval(ask.kind, ask.project, ask.asset) {
            return found.id;
        }
        self.push(|id| Approval {
            id,
            kind: ask.kind,
            project: ask.project.to_string(),
            run_id: None,
            run_number: None,
            step: None,
            job: None,
            asset_name: Some(asset_name.to_string()),
            service: None,
            detail: Some(ask.detail),
            items: Vec::new(),
            requested_at_ms: now_ms(),
            estimate: None,
            concept_jobs: Vec::new(),
            estimate_usd: None,
            asset: Some(ask.asset.to_string()),
            version: Some(ask.version),
            sheets: Some(ask.sheets.clone()),
        })
    }

    /// The rest pose approval an asset waits for, if any.
    pub fn rest_pose_for(&self, project: &str, asset: &str) -> Option<Approval> {
        self.avatar_approval(ApprovalKind::RestPose, project, asset)
    }

    /// The reweight approval an asset waits for, if any.
    pub fn reweight_for(&self, project: &str, asset: &str) -> Option<Approval> {
        self.avatar_approval(ApprovalKind::Reweight, project, asset)
    }

    fn avatar_approval(&self, kind: ApprovalKind, project: &str, asset: &str) -> Option<Approval> {
        self.lock()
            .approvals
            .iter()
            .find(|a| a.kind == kind && a.project == project && a.asset.as_deref() == Some(asset))
            .cloned()
    }
}
