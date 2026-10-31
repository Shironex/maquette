//! Rig doctor approvals: a fix an agent started waits for a person to look
//! at the before and after sheets. There is no tool that answers it.

use maquette_core::avatar::normalize::NormalizeSheets;
use maquette_core::pipeline::history::now_ms;
use maquette_core::rig::RigRecord;

use super::{Approval, ApprovalKind, Hub};

impl Hub {
    /// Records that a case's draft waits for a person. One per case: asking
    /// again returns the same id.
    pub fn request_rig_doctor(&self, case: &RigRecord) -> String {
        if let Some(found) = self.rig_doctor_for(&case.id) {
            return found.id;
        }
        let sheet = |s: &Option<maquette_core::rig::RigSheet>| {
            s.as_ref().map(|s| s.file.clone()).unwrap_or_default()
        };
        let next = case.source.version.unwrap_or(1) + 1;
        let fixed = case.defects.iter().filter(|d| d.fixed).count();
        self.push(|id| Approval {
            id,
            kind: ApprovalKind::RigDoctor,
            project: case.source.project.clone().unwrap_or_default(),
            run_id: None,
            run_number: None,
            step: None,
            job: Some(case.id.clone()),
            asset_name: Some(format!("{} v{next}", case.name)),
            service: None,
            detail: Some(format!(
                "The rig doctor fixed {fixed} defect{}. Only joints and weights changed.",
                if fixed == 1 { "" } else { "s" }
            )),
            items: Vec::new(),
            requested_at_ms: now_ms(),
            estimate: None,
            concept_jobs: Vec::new(),
            estimate_usd: None,
            asset: case.source.asset.clone(),
            version: case.source.version,
            sheets: Some(NormalizeSheets {
                before: sheet(&case.before),
                after: sheet(&case.after),
            }),
        })
    }

    /// The approval case `id` waits for, if any.
    pub fn rig_doctor_for(&self, id: &str) -> Option<Approval> {
        self.lock()
            .approvals
            .iter()
            .find(|a| a.kind == ApprovalKind::RigDoctor && a.job.as_deref() == Some(id))
            .cloned()
    }

    /// The case was added or dropped: nothing waits any more.
    pub fn close_rig_doctor(&self, id: &str) {
        if let Some(found) = self.rig_doctor_for(id) {
            self.decide(&found.id, false);
        }
    }
}
