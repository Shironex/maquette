//! A By hand send stops waiting once its asset has a model that came in
//! another way: an Inbox file accepted for the asset under another name, a
//! library version, or any version newer than the send. Only By hand
//! waits close; a Tripo or Meshy send runs on and spends as it would.

use super::{JobState, Provider, Shared, settle, update_with};
use maquette_base::history::now_ms;
use maquette_model::assets;
use maquette_model::project;

/// Whether a send closed this way settles its subscription spend in the
/// credits ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Spend {
    /// The model was most likely made on the site, only saved under another
    /// name: the send's estimate counts as spent.
    Site,
    /// The model came from somewhere else (the library): nothing is written.
    None,
}

/// What a send that closed this way says.
pub(super) const ADDED_ANOTHER_WAY: &str = "Model added to the asset, no longer waiting";

/// Closes the By hand sends of one asset that `pick` says to, then sets the
/// card to what it is now. Returns how many closed.
fn close_where(
    shared: &Shared,
    project: &str,
    spend: Spend,
    pick: impl Fn(&str, f64) -> bool,
) -> usize {
    let ids: Vec<(String, String)> = shared
        .jobs()
        .iter()
        .map(|e| &e.stored.job)
        .filter(|j| j.project == project)
        .filter(|j| j.provider == Provider::ByHand && j.state == JobState::Waiting)
        .filter(|j| pick(&j.asset, j.created_at_ms))
        .map(|j| (j.id.clone(), j.asset.clone()))
        .collect();
    let mut closed = 0;
    let mut touched: Vec<String> = Vec::new();
    for (id, asset) in &ids {
        let mut done = false;
        let _ = update_with(shared, id, true, spend == Spend::Site, |stored| {
            // Checked again: a hand back may have come in meanwhile.
            if stored.job.state == JobState::Waiting {
                stored.job.state = JobState::Accepted;
                stored.job.progress = 1.0;
                stored.job.finished_at_ms = Some(now_ms());
                stored.job.message = Some(ADDED_ANOTHER_WAY.to_string());
                done = true;
            }
            Ok(())
        });
        if done {
            closed += 1;
            if !touched.contains(asset) {
                touched.push(asset.clone());
            }
        }
    }
    for asset in &touched {
        settle::settle_asset(shared, project, asset);
    }
    closed
}

/// Closes every By hand send of `asset` still waiting: a model for it just
/// landed some other way, from the Inbox, so most likely from the site.
pub(super) fn close_waits(shared: &Shared, project: &str, asset: &str) -> usize {
    close_where(shared, project, Spend::Site, |a, _| a == asset)
}

/// Closes the By hand sends of `project` still waiting while their asset
/// already has a version made after the send. For a workspace that opens
/// and for the ways a model lands that pass no send.
pub(super) fn close_outdated(shared: &Shared, project: &str, spend: Spend) -> usize {
    let Some(workspace) = shared.workspace() else {
        return 0;
    };
    let Ok(manifest) = project::load(&workspace.root, &workspace.config.layout, project) else {
        return 0;
    };
    close_where(shared, project, spend, |asset, sent_ms| {
        assets::find(&manifest, asset)
            .ok()
            .and_then(|a| {
                a.versions
                    .iter()
                    .filter_map(|v| version_ms(&v.created_at))
                    .reduce(f64::max)
            })
            .is_some_and(|newest| newest > sent_ms)
    })
}

/// When a version was made, in ms since the epoch.
fn version_ms(created_at: &str) -> Option<f64> {
    chrono::DateTime::parse_from_rfc3339(created_at)
        .ok()
        .map(|t| t.timestamp_millis() as f64)
}
