//! Per folder budgets: every model checks its triangles and its largest
//! texture against the budget of the folder it lives in.

use serde::{Deserialize, Serialize};

use super::{FolderBudget, GameSettings, file_of};
use crate::assets::{Asset, ModelStats};
use crate::inbox::checks::short_count;

/// What a new game gets per folder, by the folder's name.
const KNOWN: &[(&str, u32, u32)] = &[
    ("props", 12_000, 1024),
    ("enemies", 40_000, 2048),
    ("characters", 60_000, 2048),
];
/// Any other folder, and the model folder's top level.
const OTHER: (u32, u32) = (20_000, 1024);

/// A budget row for every folder, the known names first.
pub fn default_budgets(folders: &[String]) -> Vec<FolderBudget> {
    let mut rows: Vec<FolderBudget> = folders
        .iter()
        .map(|folder| {
            let (triangles, texture) = KNOWN
                .iter()
                .find(|(name, _, _)| name == folder)
                .map_or(OTHER, |(_, t, x)| (*t, *x));
            FolderBudget {
                folder: folder.clone(),
                triangles,
                texture,
            }
        })
        .collect();
    rows.sort_by_key(|r| {
        (
            KNOWN
                .iter()
                .position(|(n, _, _)| *n == r.folder)
                .unwrap_or(KNOWN.len()),
            r.folder.clone(),
        )
    });
    if rows.is_empty() {
        rows.push(FolderBudget {
            folder: String::new(),
            triangles: OTHER.0,
            texture: OTHER.1,
        });
    }
    rows
}

/// `props` for `props/brazier.glb`, empty at the top level.
pub fn folder_of(file: &str) -> String {
    file.rsplit_once('/')
        .map(|(folder, _)| folder.to_string())
        .unwrap_or_default()
}

/// The budget a file in the model folder checks against: its folder's, or
/// the nearest parent folder's.
pub fn budget_for<'a>(settings: &'a GameSettings, file: &str) -> Option<&'a FolderBudget> {
    let mut folder = folder_of(file);
    loop {
        if let Some(found) = settings.budgets.iter().find(|b| b.folder == folder) {
            return Some(found);
        }
        if folder.is_empty() {
            return None;
        }
        folder = folder_of(&folder);
    }
}

/// One model against its folder's budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetCheck {
    pub folder: String,
    pub triangles: u32,
    pub max_triangles: u32,
    pub texture: u32,
    pub max_texture: u32,
    /// The model has a skin or bones. Decimate would break its weights and
    /// shape keys, so its triangles are reduced by hand.
    pub rigged: bool,
    /// What is over, as the list says it: "Over 12k triangles".
    pub problems: Vec<String>,
}

impl BudgetCheck {
    pub fn over(&self) -> bool {
        !self.problems.is_empty()
    }
}

/// Checks model numbers against a folder budget.
pub fn check_stats(budget: &FolderBudget, stats: &ModelStats) -> BudgetCheck {
    let mut problems = Vec::new();
    if stats.triangles > budget.triangles {
        problems.push(format!("Over {} triangles", short_count(budget.triangles)));
    }
    if stats.max_texture > budget.texture {
        problems.push(format!("Texture over {} px", budget.texture));
    }
    BudgetCheck {
        folder: budget.folder.clone(),
        triangles: stats.triangles,
        max_triangles: budget.triangles,
        texture: stats.max_texture,
        max_texture: budget.texture,
        rigged: stats.joints > 0,
        problems,
    }
}

/// The asset's newest model against the budget of its folder, when both
/// are known.
pub fn check_asset(settings: &GameSettings, asset: &Asset) -> Option<BudgetCheck> {
    let stats = asset.latest()?.stats.as_ref()?;
    let budget = budget_for(settings, &file_of(settings, asset))?;
    Some(check_stats(budget, stats))
}
