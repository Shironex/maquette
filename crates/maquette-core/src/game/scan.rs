//! The import list: every model in the game's model folder with its
//! format, triangles, textures, size and budget check.

use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::budget::{BudgetCheck, budget_for, check_stats};
use super::detect::model_files;
use super::{GameFormat, GameSettings, folder_of, obj};
use crate::assets::ModelStats;
use crate::error::{CoreError, CoreResult};
use crate::inbox::{analyse, stats_of};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScannedModel {
    /// Relative to the model folder, with `/`: `props/brazier.glb`.
    pub file: String,
    /// `props`, or empty at the top level.
    pub folder: String,
    pub format: GameFormat,
    pub stats: Option<ModelStats>,
    /// Why the file could not be read, when it could not.
    pub error: Option<String>,
    pub budget: Option<BudgetCheck>,
    /// The asset that already looks after it.
    pub asset: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameScan {
    pub repo: String,
    pub models_dir: String,
    pub models: Vec<ScannedModel>,
    /// How long the scan took.
    pub millis: f64,
}

/// The numbers of one model file.
pub(crate) fn stats_of_file(path: &Path) -> CoreResult<ModelStats> {
    match GameFormat::of(&path.to_string_lossy()) {
        Some(GameFormat::Obj) => Ok(obj::read(path)?.stats),
        Some(GameFormat::Glb) => Ok(stats_of(&analyse(path)?)),
        None => Err(CoreError::Invalid(format!(
            "{} is not a .glb or .obj model.",
            path.display()
        ))),
    }
}

/// Lists the models under `<repo>/<models_dir>`. Reads only.
///
/// # Errors
///
/// Fails when the model folder does not exist.
pub fn scan_models(repo: &Path, settings: &GameSettings) -> CoreResult<GameScan> {
    let started = Instant::now();
    let dir = repo.join(&settings.models_dir);
    if !dir.is_dir() {
        return Err(CoreError::NotFound(format!(
            "the model folder {}",
            dir.display()
        )));
    }
    let models = model_files(&dir)
        .into_iter()
        .filter_map(|file| {
            let format = GameFormat::of(&file)?;
            let (stats, error) = match stats_of_file(&dir.join(&file)) {
                Ok(stats) => (Some(stats), None),
                Err(err) => (None, Some(err.to_string())),
            };
            let budget = stats
                .as_ref()
                .zip(budget_for(settings, &file))
                .map(|(s, b)| check_stats(b, s));
            let asset = settings
                .models
                .iter()
                .find(|(_, m)| m.file == file)
                .map(|(id, _)| id.clone());
            Some(ScannedModel {
                folder: folder_of(&file),
                file,
                format,
                stats,
                error,
                budget,
                asset,
            })
        })
        .collect();
    Ok(GameScan {
        repo: repo.display().to_string(),
        models_dir: settings.models_dir.clone(),
        models,
        millis: started.elapsed().as_secs_f64() * 1000.0,
    })
}
