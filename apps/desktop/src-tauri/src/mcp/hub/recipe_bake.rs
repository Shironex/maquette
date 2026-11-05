//! Recipe bake approvals: an agent's add_recipe_bake waits for a person to
//! review the staged bake in the Motion view and add it there. There is no
//! tool that answers it.

use maquette_core::animations::recipes::bake::RecipeBakeRecord;
use maquette_core::pipeline::history::now_ms;
use maquette_core::providers::ledger::{DecidedBy, Decision, DecisionSource};

use super::{Approval, ApprovalKind, Hub};

impl Hub {
    /// Records that a finished bake waits for a person to add it. One per
    /// recipe: asking again returns the same id.
    pub fn request_recipe_bake(&self, bake: &RecipeBakeRecord) -> String {
        if let Some(found) = self.recipe_bake_for(&bake.project, &bake.recipe) {
            return found.id;
        }
        let result = bake.result.as_ref();
        let from = result.map_or(1, |r| r.from_version);
        let model = result.map_or_else(|| bake.asset.clone(), |r| r.asset_name.clone());
        let warns = bake.review.as_ref().map_or(0, |r| r.warns);
        let clip = result.map_or_else(String::new, |r| format!(" with the new clip {}", r.clip));
        let numbers = match warns {
            0 => "Every number of the review passes.".to_string(),
            1 => "1 number of the review warns.".to_string(),
            n => format!("{n} numbers of the review warn."),
        };
        self.push(|id| Approval {
            id,
            kind: ApprovalKind::RecipeBake,
            project: bake.project.clone(),
            run_id: None,
            run_number: None,
            step: None,
            job: Some(bake.recipe.clone()),
            asset_name: Some(format!("{model} v{}", from + 1)),
            service: None,
            detail: Some(format!(
                "{} baked onto {model} v{from}{clip}. {numbers}",
                bake.recipe_name
            )),
            items: Vec::new(),
            requested_at_ms: now_ms(),
            estimate: None,
            concept_jobs: Vec::new(),
            estimate_usd: None,
            asset: Some(bake.asset.clone()),
            version: Some(from),
            sheets: None,
            preview: None,
            recipe_save: None,
        })
    }

    /// The approval the bake of `recipe` in `project` waits for, if any.
    pub fn recipe_bake_for(&self, project: &str, recipe: &str) -> Option<Approval> {
        self.lock()
            .approvals
            .iter()
            .find(|a| {
                a.kind == ApprovalKind::RecipeBake
                    && a.project == project
                    && a.job.as_deref() == Some(recipe)
            })
            .cloned()
    }

    /// The bake was added (approved) in the review, or a new bake of the
    /// recipe started (cancelled): nothing waits any more.
    pub fn close_recipe_bake(&self, project: &str, recipe: &str, decision: Decision) {
        if let Some(found) = self.recipe_bake_for(project, recipe) {
            let by = DecidedBy::from(DecisionSource::RecipeReview);
            if let Err(err) = self.settle(&found.id, decision == Decision::Approved, decision, by) {
                tracing::warn!(error = ?err, "could not close a recipe bake approval");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A finished bake as the service reads it, built by the record's JSON shape.
    fn bake(recipe: &str, warns: u32) -> RecipeBakeRecord {
        serde_json::from_value(serde_json::json!({
            "project": "moon", "recipe": recipe, "recipeName": "Wave at the gate",
            "asset": "elsheria", "state": "done", "startedBy": "agent",
            "progress": null, "error": null, "startedMs": 0.0, "updatedMs": 0.0,
            "finishedMs": 1.0, "result": null,
            "review": { "lines": [], "warns": warns },
        }))
        .expect("record")
    }

    #[test]
    fn an_agents_add_raises_one_approval_per_recipe_until_it_is_closed() {
        let hub = Hub::new(|| {});
        let first = hub.request_recipe_bake(&bake("wave", 2));
        assert_eq!(
            hub.request_recipe_bake(&bake("wave", 2)),
            first,
            "one per recipe"
        );
        hub.request_recipe_bake(&bake("nod", 0));
        let waiting = hub.approvals();
        assert_eq!(waiting.len(), 2);
        assert_eq!(waiting[0].kind, ApprovalKind::RecipeBake);
        assert_eq!(waiting[0].job.as_deref(), Some("wave"));
        assert_eq!(waiting[0].asset_name.as_deref(), Some("elsheria v2"));
        assert!(
            waiting[0]
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("2 numbers of the review warn")),
            "{:?}",
            waiting[0].detail
        );
        hub.close_recipe_bake("moon", "wave", Decision::Approved);
        assert!(hub.recipe_bake_for("moon", "wave").is_none());
        assert!(
            hub.recipe_bake_for("moon", "nod").is_some(),
            "the other waits"
        );
        assert_eq!(
            hub.wait_until(&first, || true),
            Some(true),
            "the waiter hears yes"
        );
    }
}
