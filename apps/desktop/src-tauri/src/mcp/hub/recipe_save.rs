//! Recipe save approvals: an agent's propose_recipe_save waits for a person
//! to look at what changes and save the recipe to the library in the app's
//! save to library sheet. No tool answers it, and the approval itself
//! writes nothing: only the sheet's Save does.

use maquette_core::animations::recipes::library::RecipeSavePlan;
use maquette_core::pipeline::history::now_ms;
use maquette_core::providers::ledger::{DecidedBy, Decision, DecisionSource};

use serde::{Deserialize, Serialize};

use super::{Approval, ApprovalKind, Hub};

/// Where an agent proposed saving a project's recipe in the recipe library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSaveAsk {
    /// The project's recipe id.
    pub recipe: String,
    /// The library recipe it would get a revision of; none for a new one.
    pub entry: Option<String>,
    /// The revision it would be.
    pub revision: u32,
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
            }),
        })
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

    /// The person saved the recipe in the sheet (approved): nothing waits
    /// any more.
    pub fn close_recipe_save(&self, project: &str, recipe: &str, decision: Decision) {
        if let Some(found) = self.recipe_save_for(project, recipe) {
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
        hub.request_recipe_save(&plan("wave", None));
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
        hub.close_recipe_save("moon", "reach", Decision::Approved);
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
    }
}
