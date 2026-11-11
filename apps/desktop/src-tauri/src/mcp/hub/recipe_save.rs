//! Save approvals for the recipe and grip libraries: an agent's
//! propose_recipe_save waits for a person to look at what changes and save
//! the recipe in the app's save to library sheet, and its propose_grip_save
//! waits for the person to save the grip in the save a grip sheet. No tool
//! answers either, and the approval itself writes nothing: only the
//! sheet's Save does.

use maquette_core::animations::grips::{GripSavePlan, GripSource};
use maquette_core::animations::recipes::library::RecipeSavePlan;
use maquette_core::pipeline::history::now_ms;
use maquette_core::providers::ledger::{DecidedBy, Decision, DecisionSource};
use maquette_core::recipes::grips::{GripHolds, hands_words};

use serde::{Deserialize, Serialize};

use super::{Approval, ApprovalKind, Hub};

/// Where an agent proposed saving to a library: a project's recipe in the
/// recipe library, or (with `grip`) a grip read from one of its pose key
/// changes in the grip library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSaveAsk {
    /// The project's recipe id.
    pub recipe: String,
    /// The library recipe (or grip) it would get a revision of; none for a
    /// new one.
    pub entry: Option<String>,
    /// The revision it would be.
    pub revision: u32,
    /// A grip save: what the agent proposed. None for a recipe save.
    #[serde(default)]
    #[specta(optional)]
    pub grip: Option<GripSaveAsk>,
}

/// The grip an agent proposed saving from a recipe's pose keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GripSaveAsk {
    /// The pose key change it reads, and the frame of the bake.
    pub layer: String,
    pub frame: u32,
    /// The grip as proposed.
    pub name: String,
    pub weapon: String,
    pub hands: GripHolds,
    /// The bones it keeps, by VRM name.
    pub bones: Vec<String>,
    /// The library grip's newest revision now; none for a new grip.
    pub latest: Option<u32>,
    /// Where the file would go: `library/grips/<id>/r<n>.grip.json`.
    pub file: String,
}

impl Hub {
    /// Records that saving a recipe to the library waits for a person. One
    /// per recipe: asking again returns the same id.
    pub fn request_recipe_save(&self, plan: &RecipeSavePlan) -> String {
        if let Some(found) = self.recipe_save_for(&plan.project, &plan.recipe) {
            return found.id;
        }
        let target = match &plan.entry {
            Some(_) => format!("as r{} of {}", plan.revision, plan.entry_name),
            None => format!("as a new library recipe, {}", plan.entry_name),
        };
        let changes = match plan.changes.len() {
            1 => "1 line of changes.".to_string(),
            n => format!("{n} lines of changes."),
        };
        self.push(|id| Approval {
            id,
            kind: ApprovalKind::RecipeSave,
            project: plan.project.clone(),
            run_id: None,
            run_number: None,
            step: None,
            job: Some(plan.recipe.clone()),
            asset_name: Some(plan.recipe_name.clone()),
            service: None,
            detail: Some(format!(
                "An agent proposes saving {} to the recipe library {target}. {changes}",
                plan.recipe_name
            )),
            items: Vec::new(),
            requested_at_ms: now_ms(),
            estimate: None,
            concept_jobs: Vec::new(),
            estimate_usd: None,
            asset: None,
            version: None,
            sheets: None,
            preview: None,
            recipe_save: Some(RecipeSaveAsk {
                recipe: plan.recipe.clone(),
                entry: plan.entry.clone(),
                revision: plan.revision,
                grip: None,
            }),
        })
    }

    /// Records that saving a grip to the library waits for a person. One
    /// per pose key change of a recipe: asking again for the same grip
    /// returns the same id, and asking for another grip while one waits is
    /// refused with the waiting id (`Err`), so the card never shows another
    /// grip than the agent was answered with.
    pub fn request_grip_save(
        &self,
        plan: &GripSavePlan,
        source: &GripSource,
        recipe_name: &str,
    ) -> Result<String, String> {
        let grip = &plan.grip;
        let ask = RecipeSaveAsk {
            recipe: source.recipe.clone(),
            entry: plan.entry.clone(),
            revision: plan.revision,
            grip: Some(GripSaveAsk {
                layer: source.layer.clone(),
                frame: source.frame,
                name: grip.name.clone(),
                weapon: grip.weapon.clone(),
                hands: grip.hands,
                bones: grip.pose.keys().cloned().collect(),
                latest: plan.latest,
                file: plan.file.clone(),
            }),
        };
        if let Some(found) = self.grip_save_for(&source.project, &source.recipe, &source.layer) {
            return if found.recipe_save.as_ref() == Some(&ask) {
                Ok(found.id)
            } else {
                Err(found.id)
            };
        }
        Ok(self.push(|id| Approval {
            id,
            kind: ApprovalKind::GripSave,
            project: source.project.clone(),
            run_id: None,
            run_number: None,
            step: None,
            job: Some(source.recipe.clone()),
            asset_name: Some(format!("{} r{}", plan.entry_name, plan.revision)),
            service: None,
            detail: Some(format!(
                "From the pose keys on {} in {recipe_name}, at frame {}.",
                hands_words(grip),
                source.frame
            )),
            items: Vec::new(),
            requested_at_ms: now_ms(),
            estimate: None,
            concept_jobs: Vec::new(),
            estimate_usd: None,
            asset: None,
            version: None,
            sheets: None,
            preview: None,
            recipe_save: Some(ask),
        }))
    }

    /// The approval a grip save from the pose key change `layer` of
    /// `recipe` in `project` waits for, if any.
    pub fn grip_save_for(&self, project: &str, recipe: &str, layer: &str) -> Option<Approval> {
        self.lock()
            .approvals
            .iter()
            .find(|a| {
                a.kind == ApprovalKind::GripSave
                    && a.project == project
                    && a.job.as_deref() == Some(recipe)
                    && a.recipe_save
                        .as_ref()
                        .and_then(|ask| ask.grip.as_ref())
                        .is_some_and(|grip| grip.layer == layer)
            })
            .cloned()
    }

    /// The person saved a grip from that pose key change in the sheet, to
    /// library grip `saved_to` (none for a new one): nothing waits any
    /// more. It ends as approved only when the save went where the agent
    /// asked; saved elsewhere, the agent's ask ends as cancelled.
    pub fn close_grip_save(
        &self,
        project: &str,
        recipe: &str,
        layer: &str,
        saved_to: Option<&str>,
    ) {
        if let Some(found) = self.grip_save_for(project, recipe, layer) {
            let asked = found.recipe_save.as_ref().map(|ask| ask.entry.as_deref());
            let decision = if asked == Some(saved_to) {
                Decision::Approved
            } else {
                Decision::Cancelled
            };
            let by = DecidedBy::from(DecisionSource::GripSave);
            if let Err(err) = self.settle(&found.id, decision == Decision::Approved, decision, by) {
                tracing::warn!(error = ?err, "could not close a grip save approval");
            }
        }
    }

    /// The approval a save of `recipe` in `project` waits for, if any.
    pub fn recipe_save_for(&self, project: &str, recipe: &str) -> Option<Approval> {
        self.lock()
            .approvals
            .iter()
            .find(|a| {
                a.kind == ApprovalKind::RecipeSave
                    && a.project == project
                    && a.job.as_deref() == Some(recipe)
            })
            .cloned()
    }

    /// The person saved the recipe in the sheet, to library recipe
    /// `saved_to` (none for a new one): nothing waits any more. It ends as
    /// approved only when the save went where the agent asked; saved
    /// elsewhere, the agent's ask ends as cancelled.
    pub fn close_recipe_save(&self, project: &str, recipe: &str, saved_to: Option<&str>) {
        if let Some(found) = self.recipe_save_for(project, recipe) {
            let asked = found.recipe_save.as_ref().map(|ask| ask.entry.as_deref());
            let decision = if asked == Some(saved_to) {
                Decision::Approved
            } else {
                Decision::Cancelled
            };
            let by = DecidedBy::from(DecisionSource::RecipeSave);
            if let Err(err) = self.settle(&found.id, decision == Decision::Approved, decision, by) {
                tracing::warn!(error = ?err, "could not close a recipe save approval");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(recipe: &str, entry: Option<&str>) -> RecipeSavePlan {
        RecipeSavePlan {
            project: "moon".into(),
            recipe: recipe.into(),
            recipe_name: "Reach out".into(),
            recipe_version: "v1".into(),
            entry: entry.map(Into::into),
            entry_name: "Reach out".into(),
            entry_version: entry.map(|_| "e1".into()),
            latest: entry.map(|_| 2),
            revision: if entry.is_some() { 3 } else { 1 },
            changes: vec!["Adds the wave on leftHand (l-2).".into()],
            same: false,
        }
    }

    #[test]
    fn a_proposed_save_raises_one_approval_per_recipe_until_the_person_saves() {
        let hub = Hub::new(|| {});
        let first = hub.request_recipe_save(&plan("reach", Some("reach")));
        assert_eq!(
            hub.request_recipe_save(&plan("reach", None)),
            first,
            "one per recipe"
        );
        let second = hub.request_recipe_save(&plan("wave", None));
        let waiting = hub.approvals();
        assert_eq!(waiting.len(), 2);
        assert_eq!(waiting[0].kind, ApprovalKind::RecipeSave);
        let ask = waiting[0].recipe_save.as_ref().expect("ask");
        assert_eq!((ask.entry.as_deref(), ask.revision), (Some("reach"), 3));
        assert!(
            waiting[0]
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("as r3 of Reach out")),
            "{:?}",
            waiting[0].detail
        );
        hub.close_recipe_save("moon", "reach", Some("reach"));
        assert!(hub.recipe_save_for("moon", "reach").is_none());
        assert!(
            hub.recipe_save_for("moon", "wave").is_some(),
            "the other waits"
        );
        assert_eq!(
            hub.wait_until(&first, || true),
            Some(true),
            "the waiter hears yes"
        );
        hub.close_recipe_save("moon", "wave", Some("reach"));
        assert!(hub.recipe_save_for("moon", "wave").is_none());
        assert_eq!(
            hub.wait_until(&second, || true),
            Some(false),
            "saved elsewhere is not the agent's yes"
        );
    }

    fn grip_plan(entry: Option<&str>) -> GripSavePlan {
        use maquette_core::recipes::grips::{GRIP_VERSION, GripFile, GripSide};
        GripSavePlan {
            entry: entry.map(Into::into),
            entry_name: "Scythe, staff grip".into(),
            entry_version: entry.map(|_| "e1".into()),
            latest: entry.map(|_| 2),
            revision: if entry.is_some() { 3 } else { 1 },
            file: "library/grips/scythe-staff-grip/r3.grip.json".into(),
            bytes: 900,
            grip: GripFile {
                version: GRIP_VERSION,
                name: "Scythe, staff grip".into(),
                revision: 3,
                weapon: "scythe".into(),
                hands: GripHolds {
                    holds: GripSide::Right,
                    supports: Some(GripSide::Left),
                },
                pose: [
                    ("rightHand", [4.0, 0.0, -8.0]),
                    ("leftHand", [0.0, 6.0, 0.0]),
                ]
                .into_iter()
                .map(|(bone, turn)| (bone.to_string(), turn))
                .collect(),
            },
            changes: vec!["Turns 1 bone another way.".into()],
            same: false,
        }
    }

    fn source(layer: &str) -> GripSource {
        GripSource {
            project: "moon".into(),
            recipe: "charge".into(),
            layer: layer.into(),
            frame: 30,
        }
    }

    #[test]
    fn a_proposed_grip_save_raises_one_approval_per_pose_key_change_until_the_person_saves() {
        let hub = Hub::new(|| {});
        let plan = grip_plan(Some("scythe-staff-grip"));
        let first = hub
            .request_grip_save(&plan, &source("l-2"), "Scythe charge, held")
            .expect("first");
        assert_eq!(
            hub.request_grip_save(&plan, &source("l-2"), "Scythe charge, held"),
            Ok(first.clone()),
            "the same grip again: the same approval"
        );
        assert_eq!(
            hub.request_grip_save(&grip_plan(None), &source("l-2"), "Scythe charge, held"),
            Err(first.clone()),
            "another grip while one waits: refused with the waiting id"
        );
        let second = hub
            .request_grip_save(&grip_plan(None), &source("l-5"), "Scythe charge, held")
            .expect("second");
        let waiting = hub.approvals();
        assert_eq!(waiting.len(), 2);
        assert_eq!(waiting[0].kind, ApprovalKind::GripSave);
        assert_eq!(
            waiting[0].asset_name.as_deref(),
            Some("Scythe, staff grip r3")
        );
        assert_eq!(
            waiting[0].detail.as_deref(),
            Some("From the pose keys on both hands in Scythe charge, held, at frame 30.")
        );
        let ask = waiting[0].recipe_save.as_ref().expect("ask");
        assert_eq!(
            (ask.entry.as_deref(), ask.revision),
            (Some("scythe-staff-grip"), 3)
        );
        let grip = ask.grip.as_ref().expect("grip");
        assert_eq!(
            (grip.layer.as_str(), grip.frame, grip.latest),
            ("l-2", 30, Some(2))
        );
        assert_eq!(grip.bones, ["leftHand", "rightHand"]);
        assert!(
            hub.recipe_save_for("moon", "charge").is_none(),
            "a grip save is not a recipe save"
        );

        // Saved where the agent asked: its yes. Saved elsewhere: not.
        hub.close_grip_save("moon", "charge", "l-2", Some("scythe-staff-grip"));
        assert!(hub.grip_save_for("moon", "charge", "l-2").is_none());
        assert!(
            hub.grip_save_for("moon", "charge", "l-5").is_some(),
            "the other waits"
        );
        assert_eq!(hub.wait_until(&first, || true), Some(true));
        hub.close_grip_save("moon", "charge", "l-5", Some("scythe-staff-grip"));
        assert_eq!(
            hub.wait_until(&second, || true),
            Some(false),
            "a new grip was asked for, a revision was saved"
        );
    }
}
