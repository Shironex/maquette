//! The replaces no Ship has written yet, for the person who approves the
//! next one: which asset is a built model now, the file it shipped as and
//! the file it ships as, and the numbers of both models.

use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::report::ships_as;
use crate::built::load;
use crate::pipeline::history::RunRecord;
use crate::project::{ProjectManifest, rfc3339_utc};

/// One asset whose model a built asset replaced since the last Ship.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShipSwap {
    pub asset: String,
    pub name: String,
    /// The script that builds it now, relative to the project.
    pub script: String,
    /// RFC 3339 UTC, when the built model took the place.
    pub at: String,
    /// The file the old model shipped as, and the one the built model ships
    /// as: a site's `planner_v2.glb` and `planner_v3.glb`, or a game's one
    /// `props/planner.glb` for both.
    pub old_file: Option<String>,
    pub new_file: Option<String>,
    pub old_version: u32,
    pub new_version: u32,
    /// The numbers of the two model files, before the pipeline compresses them.
    pub old_triangles: Option<u32>,
    pub new_triangles: Option<u32>,
    pub old_bytes: Option<f64>,
    pub new_bytes: Option<f64>,
}

/// When the newest run that shipped finished, as RFC 3339.
fn last_shipped(runs: &[RunRecord]) -> Option<String> {
    runs.iter()
        .filter(|run| matches!(run.ship_outcome(), Some((true, _))))
        .filter_map(|run| run.finished_at_ms)
        .filter(|ms| ms.is_finite() && *ms >= 0.0)
        // A finite, positive count of milliseconds.
        .map(|ms| rfc3339_utc(UNIX_EPOCH + Duration::from_millis(ms as u64)))
        .max()
}

/// The replaces of `manifest` that no run in `runs` has shipped: a swap
/// stays listed until a run whose Ship step ran finishes after it. `runs` is
/// the project's run history, which each computer keeps for itself, so a
/// swap shipped from another computer is listed here once more.
pub fn swaps(project_dir: &Path, manifest: &ProjectManifest, runs: &[RunRecord]) -> Vec<ShipSwap> {
    let Ok(built) = load(project_dir) else {
        return Vec::new();
    };
    let ship = crate::ship::load_or_default(project_dir);
    let shipped = last_shipped(runs);
    manifest
        .assets
        .iter()
        .filter_map(|asset| {
            let source = built.assets.get(&asset.id)?;
            let record = source.replaced.as_ref()?;
            if shipped.as_deref().is_some_and(|at| at > record.at.as_str()) {
                return None;
            }
            let old = asset
                .versions
                .iter()
                .find(|v| Some(v.version) == record.version())?;
            let new = asset.latest().filter(|v| v.version != old.version)?;
            // The project as it was, to name the file the old model shipped as.
            let mut before = manifest.clone();
            if let Some(was) = before.assets.iter_mut().find(|a| a.id == asset.id) {
                was.versions
                    .retain(|v| record.versions.contains(&v.version));
            }
            Some(ShipSwap {
                asset: asset.id.clone(),
                name: asset.name.clone(),
                script: source.script.clone(),
                at: record.at.clone(),
                old_file: ships_as(&before, &ship, &asset.id),
                new_file: ships_as(manifest, &ship, &asset.id),
                old_version: old.version,
                new_version: new.version,
                old_triangles: old.stats.as_ref().map(|s| s.triangles),
                new_triangles: new.stats.as_ref().map(|s| s.triangles),
                old_bytes: old.stats.as_ref().map(|s| s.bytes),
                new_bytes: new.stats.as_ref().map(|s| s.bytes),
            })
        })
        .collect()
}
