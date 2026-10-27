//! What an asset's card shows once its jobs stop driving it: after a
//! stop, and for cards an older stop left on Queued.

use super::{JobState, ProviderEvent, Shared};
use crate::assets::{self, AssetStatus};
use crate::project;

/// The status the live jobs of `asset` ask for: Running while one runs,
/// Queued while one waits, nothing when none is active.
fn active_status(shared: &Shared, project: &str, asset: &str) -> Option<AssetStatus> {
    let jobs = shared.jobs();
    let mut active = jobs
        .iter()
        .map(|e| &e.stored.job)
        .filter(|j| j.project == project && j.asset == asset && j.state.is_active())
        .map(|j| j.state);
    let first = active.next()?;
    let running = first == JobState::Running || active.any(|s| s == JobState::Running);
    Some(if running {
        AssetStatus::Running
    } else {
        AssetStatus::Queued
    })
}

/// Sets `asset` to what it really is after a job of it stopped: the status
/// of another live job, else Review when it has a model, else Queued.
pub(super) fn settle_asset(shared: &Shared, project: &str, asset: &str) {
    let Some(workspace) = shared.workspace() else {
        return;
    };
    let active = active_status(shared, project, asset);
    let changed = project::modify(&workspace.root, &workspace.config.layout, project, |m| {
        let status = match active {
            Some(status) => status,
            None => assets::resting_status(assets::find(m, asset)?),
        };
        assets::set_status(m, asset, status)
    });
    match changed {
        Ok(manifest) => shared.emit(ProviderEvent::Project { project: manifest }),
        Err(err) => tracing::warn!(error = %err, "could not update the asset"),
    }
}

/// Fixes cards a stopped send left on Queued or Running with no live job
/// behind them. Run when a workspace opens, before any job starts, so a
/// card with a live job is never touched. Returns how many it fixed.
pub(super) fn heal_stuck(shared: &Shared, project: &str) -> usize {
    let Some(workspace) = shared.workspace() else {
        return 0;
    };
    let Ok(manifest) = project::load(&workspace.root, &workspace.config.layout, project) else {
        return 0;
    };
    // A refs-only card is rightly Queued; every other Queued or Running one is wrong.
    let stuck: Vec<String> = manifest
        .assets
        .iter()
        .filter(|a| matches!(a.status, AssetStatus::Queued | AssetStatus::Running))
        .filter(|a| !a.versions.is_empty() || a.status == AssetStatus::Running)
        .filter(|a| active_status(shared, project, &a.id).is_none())
        .map(|a| a.id.clone())
        .collect();
    if stuck.is_empty() {
        return 0;
    }
    let healed = project::modify(&workspace.root, &workspace.config.layout, project, |m| {
        for id in &stuck {
            // Checked again under the write lock.
            let current = assets::find(m, id)?;
            if matches!(current.status, AssetStatus::Queued | AssetStatus::Running) {
                let status = assets::resting_status(current);
                assets::set_status(m, id, status)?;
            }
        }
        Ok(())
    });
    match healed {
        Ok(manifest) => {
            shared.emit(ProviderEvent::Project { project: manifest });
            stuck.len()
        }
        Err(err) => {
            tracing::warn!(error = %err, "could not heal stuck cards");
            0
        }
    }
}
