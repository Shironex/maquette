//! The settings a new game project starts with: its model folder and a
//! budget for every folder of models found in the game repo.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use super::{FolderBudget, GameExport, GameSettings, NewGame, folder_of};
use crate::error::{CoreError, CoreResult};
use crate::project::{has_ext, sorted_children};

/// What a new game gets per folder, by the folder's name.
const KNOWN: &[(&str, u32, u32)] = &[
    ("props", 12_000, 1024),
    ("enemies", 40_000, 2048),
    ("characters", 60_000, 2048),
];
/// Any other folder, and the model folder's top level.
const OTHER: (u32, u32) = (20_000, 1024);

/// How deep the model folder is searched.
const MAX_DEPTH: usize = 4;

/// True for a relative path with `/` that cannot leave the folder it is
/// joined to.
pub fn stays_inside(path: &str) -> bool {
    let p = Path::new(path);
    !path.trim().is_empty()
        && !path.starts_with(['/', '\\'])
        && !path.contains('\\')
        && !p.is_absolute()
        && p.components().all(|c| matches!(c, Component::Normal(_)))
}

/// The settings of a new game project in `repo`, with a budget for every
/// model folder found there.
pub fn new_settings(new: &NewGame, repo: Option<&Path>) -> CoreResult<GameSettings> {
    let models_dir = new.models_dir.trim().trim_end_matches('/').to_string();
    if !stays_inside(&models_dir) {
        return Err(CoreError::Invalid(format!(
            "The model folder {models_dir:?} must be a folder inside the game repo, without .."
        )));
    }
    let folders = repo
        .map(|r| model_folders(&r.join(&models_dir)))
        .unwrap_or_default();
    Ok(GameSettings {
        format: new.format,
        models_dir,
        export: GameExport::default(),
        bake: false,
        budgets: default_budgets(&folders),
        models: BTreeMap::new(),
        folders: BTreeMap::new(),
    })
}

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

/// Every model file under `dir`, relative to it with `/`, sorted.
pub fn model_files(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, depth: usize, out: &mut Vec<String>) {
        for (name, path) in sorted_children(dir) {
            if name.starts_with('.') {
                continue;
            }
            let rel = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            if path.is_dir() && depth < MAX_DEPTH {
                walk(&path, &rel, depth + 1, out);
            } else if path.is_file() && has_ext(&path, &["glb", "obj"]) {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", 0, &mut out);
    out
}

/// The folders under `dir` that hold model files (empty for its top level).
pub fn model_folders(dir: &Path) -> Vec<String> {
    let mut folders: Vec<String> = model_files(dir).iter().map(|f| folder_of(f)).collect();
    folders.sort();
    folders.dedup();
    folders
}
