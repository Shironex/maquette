//! Rest pose approvals: an agent's avatar build whose new rest pose waits
//! for a person to look at the before and after sheets.

use maquette_core::avatar::normalize::Normalized;
use maquette_core::pipeline::history::now_ms;

use super::{Approval, ApprovalKind, Hub};

impl Hub {
    /// Records that an avatar's new rest pose, which an agent's build made,
    /// waits for a person. One per asset: asking again returns the same id.
    pub fn request_rest_pose(&self, normalized: &Normalized, asset_name: &str) -> String {
        if let Some(found) = self.rest_pose_for(&normalized.project, &normalized.asset) {
            return found.id;
        }
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
        self.push(|id| Approval {
            id,
            kind: ApprovalKind::RestPose,
            project: normalized.project.clone(),
            run_id: None,
            run_number: None,
            step: None,
            job: None,
            asset_name: Some(asset_name.to_string()),
            service: None,
            detail: Some(detail),
            items: Vec::new(),
            requested_at_ms: now_ms(),
            estimate: None,
            concept_jobs: Vec::new(),
            estimate_usd: None,
            asset: Some(normalized.asset.clone()),
            version: Some(normalized.source_version),
            sheets: Some(normalized.sheets.clone()),
        })
    }

    /// The rest pose approval an asset waits for, if any.
    pub fn rest_pose_for(&self, project: &str, asset: &str) -> Option<Approval> {
        self.lock()
            .approvals
            .iter()
            .find(|a| {
                a.kind == ApprovalKind::RestPose
                    && a.project == project
                    && a.asset.as_deref() == Some(asset)
            })
            .cloned()
    }
}
