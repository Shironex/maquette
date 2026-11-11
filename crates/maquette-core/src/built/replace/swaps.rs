//! The replaces no Ship has written yet, for the person who approves the
//! next one: which asset is a built model now, the file it shipped as and
//! the file it ships as, and the numbers of both models. An undo after a
//! Ship wrote the built model is listed the same way, the other way round.

use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::report::ships_as;
use crate::assets::Asset;
use crate::built::{BuiltFile, ReplacedModel, UndoneReplace, load};
use crate::pipeline::history::{RunRecord, StepRecord, StepState};
use crate::project::{ProjectManifest, now_rfc3339, rfc3339_utc};
use crate::ship::ShipSettings;

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
    /// An undo after a Ship wrote the built model: the old model goes back,
    /// so `old` is the built model and `new` the one that is current again.
    #[serde(default)]
    pub undone: bool,
}

/// Whether `run` shipped what the project builds now into the project's own
/// repo: its Ship step ran, no repo was given for this run only, and every
/// step before Ship ran or was up to date. A run from Ship, say, ships the
/// export from before the replace, so it does not count.
fn shipped_the_project(run: &RunRecord) -> bool {
    if !matches!(run.ship_outcome(), Some((true, _))) || run.repo_for_run.is_some() {
        return false;
    }
    let is_ship = |s: &StepRecord| {
        s.id.trim().eq_ignore_ascii_case("ship") || s.name.trim().eq_ignore_ascii_case("ship")
    };
    run.steps
        .iter()
        .take_while(|s| !is_ship(s))
        .all(|s| matches!(s.state, StepState::Done | StepState::Cached))
}

/// When each run that shipped the project finished, as RFC 3339.
fn shipped_times(runs: &[RunRecord]) -> impl Iterator<Item = String> + '_ {
    runs.iter()
        .filter(|run| shipped_the_project(run))
        .filter_map(|run| run.finished_at_ms)
        .filter(|ms| ms.is_finite() && *ms >= 0.0)
        // A finite, positive count of milliseconds.
        .map(|ms| rfc3339_utc(UNIX_EPOCH + Duration::from_millis(ms as u64)))
}

/// Whether a run that shipped the project finished after `at` (RFC 3339).
pub(super) fn shipped_since(runs: &[RunRecord], at: &str) -> bool {
    shipped_times(runs).any(|done| done.as_str() > at)
}

/// What an undo of `asset`'s replace leaves for the next Ship to name, when
/// the built model shipped since the replace: its newest model, which
/// leaves, and the file it shipped as. `None` when no Ship wrote it, since
/// the old model is then still what the site has.
pub(super) fn undone_record(
    project_dir: &Path,
    manifest: &ProjectManifest,
    asset: &Asset,
    script: &str,
    record: &ReplacedModel,
    runs: &[RunRecord],
) -> Option<UndoneReplace> {
    if !shipped_since(runs, &record.at) {
        return None;
    }
    let built = asset.latest()?;
    let ship = crate::ship::load_or_default(project_dir);
    Some(UndoneReplace {
        at: now_rfc3339(),
        script: script.to_string(),
        version: built.version,
        shipped_as: ships_as(manifest, &ship, &asset.id),
        triangles: built.stats.as_ref().map(|s| s.triangles),
        bytes: built.stats.as_ref().map(|s| s.bytes),
    })
}

/// The undos no run has shipped yet: the old model goes back at the next Ship.
fn swaps_back(
    built: &BuiltFile,
    manifest: &ProjectManifest,
    ship: &ShipSettings,
    runs: &[RunRecord],
) -> Vec<ShipSwap> {
    built
        .undone
        .iter()
        .filter(|(id, undone)| !built.assets.contains_key(*id) && !shipped_since(runs, &undone.at))
        .filter_map(|(id, undone)| {
            let asset = manifest.assets.iter().find(|a| &a.id == id)?;
            let now = asset.latest()?;
            Some(ShipSwap {
                asset: id.clone(),
                name: asset.name.clone(),
                script: undone.script.clone(),
                at: undone.at.clone(),
                old_file: undone.shipped_as.clone(),
                new_file: ships_as(manifest, ship, id),
                old_version: undone.version,
                new_version: now.version,
                old_triangles: undone.triangles,
                new_triangles: now.stats.as_ref().map(|s| s.triangles),
                old_bytes: undone.bytes,
                new_bytes: now.stats.as_ref().map(|s| s.bytes),
                undone: true,
            })
        })
        .collect()
}

/// The replaces of `manifest` that no run in `runs` has shipped: a swap
/// stays listed until a run that built and shipped into the project's own
/// repo finishes after it. `runs` is
/// the project's run history, which each computer keeps for itself, so a
/// swap shipped from another computer is listed here once more.
pub fn swaps(project_dir: &Path, manifest: &ProjectManifest, runs: &[RunRecord]) -> Vec<ShipSwap> {
    let Ok(built) = load(project_dir) else {
        return Vec::new();
    };
    let ship = crate::ship::load_or_default(project_dir);
    let shipped = shipped_times(runs).max();
    let mut out: Vec<ShipSwap> = manifest
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
            let mut swap = ShipSwap {
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
                undone: false,
            };
            // Replaced again after an undo no Ship wrote: the site still has
            // the built model that undo took off, so that is what leaves.
            if let Some(undone) = built.undone.get(&asset.id)
                && !shipped_since(runs, &undone.at)
            {
                swap.old_file.clone_from(&undone.shipped_as);
                swap.old_version = undone.version;
                swap.old_triangles = undone.triangles;
                swap.old_bytes = undone.bytes;
            }
            Some(swap)
        })
        .collect();
    out.extend(swaps_back(&built, manifest, &ship, runs));
    out
}
