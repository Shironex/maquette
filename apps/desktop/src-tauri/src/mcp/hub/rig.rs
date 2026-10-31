//! Rig doctor approvals: a fix an agent started waits for a person to look
//! at the before and after sheets. There is no tool that answers it.

use maquette_core::avatar::normalize::NormalizeSheets;
use maquette_core::pipeline::history::now_ms;
use maquette_core::rig::{RigRecord, RigStarter, RigState};

use super::{Approval, ApprovalKind, Hub};

impl Hub {
    /// A case changed: a draft an agent's run left asks a person, once.
    pub fn ask_about_rig_draft(&self, case: &RigRecord) {
        let agent = case.started_by == RigStarter::Agent || case.agent_opened;
        if agent && case.state == RigState::Fixed {
            self.request_rig_doctor(case);
        }
    }

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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use maquette_core::rig::{JointMove, JointMoveState, RigDoctors, RigSource};

    use super::*;

    /// A fixed case as the doctor leaves it, built by the record's JSON shape.
    fn case(started_by: RigStarter, state: RigState) -> RigRecord {
        let model = serde_json::json!({
            "rig": "Mixamo, 65 joints", "joints": 65, "skinnedMeshes": 1, "clips": [],
            "triangles": 1000, "height": 1.7, "textures": 1, "handFar": 0, "armFar": 0,
            "islands": 2, "oneIsland": false, "finding": null,
        });
        serde_json::from_value(serde_json::json!({
            "id": "hero-v1", "name": "Hero",
            "startedBy": started_by, "route": "humanoid", "state": state, "step": "land",
            "source": RigSource {
                inbox: None,
                project: Some("p".to_string()),
                asset: Some("hero".to_string()),
                version: Some(1),
            },
            "input": "in.glb", "dir": "work", "model": model, "issues": [], "creature": null,
            "before": null, "after": null, "defects": [], "stats": [], "fixSteps": [],
            "checks": [], "jointMove": null, "round": 1, "draft": "draft.glb",
            "progress": null, "stages": [], "log": [], "stop": null, "landed": null,
            "error": null, "updatedMs": 0.0,
        }))
        .expect("record")
    }

    #[test]
    fn an_agents_finished_draft_raises_one_rig_doctor_approval() {
        let hub = Hub::new(|| {});
        hub.ask_about_rig_draft(&case(RigStarter::Agent, RigState::Fixing));
        assert!(hub.approvals().is_empty(), "nothing to look at yet");
        let fixed = case(RigStarter::Agent, RigState::Fixed);
        hub.ask_about_rig_draft(&fixed);
        hub.ask_about_rig_draft(&fixed);
        let waiting = hub.approvals();
        assert_eq!(waiting.len(), 1, "one per case");
        assert_eq!(waiting[0].kind, ApprovalKind::RigDoctor);
        assert_eq!(waiting[0].job.as_deref(), Some("hero-v1"));
        assert_eq!(waiting[0].asset_name.as_deref(), Some("Hero v2"));
        hub.close_rig_doctor("hero-v1");
        assert!(hub.rig_doctor_for("hero-v1").is_none());
    }

    #[test]
    fn a_persons_own_draft_asks_nobody() {
        let hub = Hub::new(|| {});
        hub.ask_about_rig_draft(&case(RigStarter::Person, RigState::Fixed));
        assert!(hub.approvals().is_empty());
    }

    #[test]
    fn a_change_the_doctor_publishes_raises_it_through_the_shells_line() {
        struct Never;
        impl maquette_core::base::ports::ScriptRunner for Never {
            fn run_script(
                &self,
                _: &maquette_core::pipeline::tools::Toolbox,
                _: &Path,
                _: (&str, &str),
                _: &[String],
            ) -> maquette_core::CoreResult<()> {
                panic!("no script runs here");
            }
        }
        let dir = tempfile::tempdir().expect("dir");
        let hub = std::sync::Arc::new(Hub::new(|| {}));
        let asks = std::sync::Arc::clone(&hub);
        let doctors = RigDoctors::new(std::sync::Arc::new(Never), move |r: &RigRecord| {
            asks.ask_about_rig_draft(r);
        });
        let config = maquette_core::WorkspaceConfig::default();
        let mut on_disk = case(RigStarter::Agent, RigState::Fixed);
        on_disk.dir = maquette_core::rig::work_dir(dir.path(), &config, "hero-v1")
            .display()
            .to_string();
        on_disk.joint_move = Some(JointMove {
            state: JointMoveState::Asked,
            limb: "left arm".to_string(),
            off_cm: 4.0,
            joints: 3,
            breaks_clips: Vec::new(),
            draft: "moved.glb".to_string(),
            sheet: None,
        });
        maquette_core::rig::save(&on_disk).expect("save");
        assert!(hub.approvals().is_empty());
        // Any published change of a fixed case: here a person's joint answer.
        doctors
            .decide_joints(dir.path(), &config, "hero-v1", false)
            .expect("answer");
        let waiting = hub.approvals();
        assert_eq!(waiting.len(), 1);
        assert_eq!(waiting[0].kind, ApprovalKind::RigDoctor);
    }
}
