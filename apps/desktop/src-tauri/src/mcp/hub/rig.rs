//! Rig doctor approvals: a fix an agent started waits for a person to look
//! at the before and after sheets. There is no tool that answers it.

use maquette_core::avatar::normalize::NormalizeSheets;
use maquette_core::pipeline::history::now_ms;
use maquette_core::providers::ledger::{DecidedBy, Decision, DecisionSource};
use maquette_core::rig::{RigRecord, RigStarter, RigState, RigidState};

use super::{Approval, ApprovalKind, Hub};

/// An agent's bind on the case waits for a person.
fn agent_bind_asked(case: &RigRecord) -> bool {
    case.rigid
        .as_ref()
        .is_some_and(|r| r.asked_by == RigStarter::Agent && r.state == RigidState::Asked)
}

/// What the card says: the bind an agent asked for while it waits, else the fix.
fn rig_detail(case: &RigRecord) -> String {
    if let Some(r) = case.rigid.as_ref().filter(|_| agent_bind_asked(case)) {
        return format!(
            "An agent asks to bind {} part{} rigidly to {}. Only weights change.",
            r.parts.len(),
            if r.parts.len() == 1 { "" } else { "s" },
            maquette_core::inbox::analyse::bare_joint_name(&r.bone)
        );
    }
    let fixed = case.defects.iter().filter(|d| d.fixed).count();
    format!(
        "The rig doctor fixed {fixed} defect{}. Only joints and weights changed.",
        if fixed == 1 { "" } else { "s" }
    )
}

impl Hub {
    /// A case changed: a draft an agent's run left asks a person, once, and
    /// so does a rigid bind an agent asked for. A card already up follows
    /// the case's words, and one raised only for a bind closes once a
    /// person answered the bind.
    pub fn ask_about_rig_draft(&self, case: &RigRecord) {
        let agents_case = case.started_by == RigStarter::Agent || case.agent_opened;
        if case.state != RigState::Fixed || !(agents_case || agent_bind_asked(case)) {
            if !agents_case && case.state == RigState::Fixed {
                let answered = case.rigid.as_ref().map(|r| r.state);
                let decision = match answered {
                    Some(RigidState::Approved) => Some(Decision::Approved),
                    Some(RigidState::Declined) | None => Some(Decision::Denied),
                    _ => None,
                };
                if let Some(decision) = decision {
                    self.close_rig_doctor(&case.id, decision);
                }
            }
            return;
        }
        self.request_rig_doctor(case);
    }

    /// Records that a case's draft waits for a person. One per case: asking
    /// again returns the same id, with its words brought up to date.
    pub fn request_rig_doctor(&self, case: &RigRecord) -> String {
        let detail = rig_detail(case);
        if let Some(found) = self.rig_doctor_for(&case.id) {
            if found.detail.as_deref() != Some(detail.as_str()) {
                if let Some(card) = self.lock().approvals.iter_mut().find(|a| a.id == found.id) {
                    card.detail = Some(detail);
                }
                (self.on_change)();
            }
            return found.id;
        }
        let sheet = |s: &Option<maquette_core::rig::RigSheet>| {
            s.as_ref().map(|s| s.file.clone()).unwrap_or_default()
        };
        let next = case.source.version.unwrap_or(1) + 1;
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
            detail: Some(detail),
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
            preview: None,
            recipe_save: None,
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

    /// The case was added (approved), dropped (denied) or started over or
    /// held (cancelled) by a person in the Rig doctor: nothing waits any more.
    pub fn close_rig_doctor(&self, id: &str, decision: Decision) {
        if let Some(found) = self.rig_doctor_for(id) {
            let by = DecidedBy::from(DecisionSource::RigDoctor);
            if let Err(err) = self.settle(&found.id, false, decision, by) {
                tracing::warn!(error = ?err, "could not close a rig doctor approval");
            }
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
        hub.close_rig_doctor("hero-v1", Decision::Denied);
        assert!(hub.rig_doctor_for("hero-v1").is_none());
    }

    #[test]
    fn an_agents_rigid_bind_on_a_persons_draft_asks_a_person() {
        use maquette_core::rig::RigRigid;
        let hub = Hub::new(|| {});
        let mut fixed = case(RigStarter::Person, RigState::Fixed);
        let mut bind = RigRigid {
            parts: vec![1, 2],
            bone: "mixamorig:Spine2".to_string(),
            state: RigidState::Making,
            asked_by: RigStarter::Agent,
            vertices: 0,
            file: None,
            earlier: None,
            sheet: None,
            checks: Vec::new(),
            row: None,
            stretch_before: None,
            stretch_after: None,
        };
        fixed.rigid = Some(bind.clone());
        hub.ask_about_rig_draft(&fixed);
        assert!(hub.approvals().is_empty(), "nothing measured yet");
        bind.state = RigidState::Asked;
        fixed.rigid = Some(bind.clone());
        hub.ask_about_rig_draft(&fixed);
        let waiting = hub.approvals();
        assert_eq!(waiting.len(), 1);
        assert_eq!(
            waiting[0].detail.as_deref(),
            Some("An agent asks to bind 2 parts rigidly to Spine2. Only weights change.")
        );
        // The person turns it down in the Rig doctor: the card has nothing left to ask.
        bind.state = RigidState::Declined;
        fixed.rigid = Some(bind.clone());
        hub.ask_about_rig_draft(&fixed);
        assert!(hub.rig_doctor_for("hero-v1").is_none());

        // On an agent's own draft the card stays, and its words follow the bind.
        let mut agents = case(RigStarter::Agent, RigState::Fixed);
        hub.ask_about_rig_draft(&agents);
        let fix_words = hub.rig_doctor_for("hero-v1").and_then(|a| a.detail);
        bind.state = RigidState::Asked;
        agents.rigid = Some(bind.clone());
        hub.ask_about_rig_draft(&agents);
        let card = hub.rig_doctor_for("hero-v1").and_then(|a| a.detail);
        assert!(
            card.as_deref()
                .is_some_and(|d| d.starts_with("An agent asks to bind"))
        );
        bind.state = RigidState::Declined;
        agents.rigid = Some(bind);
        hub.ask_about_rig_draft(&agents);
        assert_eq!(
            hub.rig_doctor_for("hero-v1").and_then(|a| a.detail),
            fix_words
        );
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
