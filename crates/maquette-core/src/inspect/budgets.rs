//! The project's `budgets.json` and the transfer budget each model gets.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::*;

#[derive(Deserialize, Default)]
pub(super) struct BudgetsFile {
    #[serde(default)]
    models: BTreeMap<String, f64>,
}

pub(super) fn read_budgets(project_dir: &Path) -> BTreeMap<String, f64> {
    let path = project_dir.join(BUDGETS_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    match serde_json::from_str::<BudgetsFile>(&text) {
        Ok(file) => file
            .models
            .into_iter()
            .filter(|(_, mb)| mb.is_finite() && *mb > 0.0)
            .collect(),
        Err(err) => {
            tracing::warn!(file = %path.display(), error = %err, "unreadable model budgets");
            BTreeMap::new()
        }
    }
}

/// Per model transfer budgets: explicit rows, then an even share of the rest.
pub fn split_budget(
    scene_mb: f64,
    models: &[String],
    explicit: &BTreeMap<String, f64>,
) -> Vec<(String, f64, bool)> {
    let named: f64 = models.iter().filter_map(|m| explicit.get(m)).sum();
    let open = models.iter().filter(|m| !explicit.contains_key(*m)).count();
    #[expect(clippy::cast_precision_loss, reason = "a handful of models")]
    let share = if open == 0 {
        0.0
    } else {
        ((scene_mb - named).max(0.0) / open as f64 * 100.0).round() / 100.0
    };
    models
        .iter()
        .map(|m| match explicit.get(m) {
            Some(mb) => (m.clone(), *mb, true),
            None => (m.clone(), share, false),
        })
        .collect()
}
