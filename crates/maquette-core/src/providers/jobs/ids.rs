//! Job ids: `<asset>-<n>`, unique across the workspace, not only within a
//! project, because every lookup (approve, cancel, the queue, the inbox
//! link, the MCP tools) goes by id alone. Saved jobs from before that rule
//! can share an id with a job of another project; opening the workspace
//! gives all but one of them a fresh id.

use std::collections::HashSet;
use std::path::Path;

use super::{Entry, JobState, Stored};
use crate::providers::handoff::HANDOFF_DIR;

/// The first free `<asset>-<n>`: no job of any project has it, and no
/// handoff folder of this project is named after it.
pub(super) fn next_id(jobs: &[Entry], project_dir: &Path, asset: &str) -> String {
    let taken = |id: &str| {
        jobs.iter().any(|e| e.stored.job.id == id)
            || project_dir.join(HANDOFF_DIR).join(id).exists()
    };
    (1u32..)
        .map(|n| format!("{asset}-{n}"))
        .find(|id| !taken(id))
        .unwrap_or_else(|| format!("{asset}-x"))
}

/// A job whose id is already out in the world and should keep it: a paid
/// task still to settle (its ledger lines carry the id), a By hand send
/// whose export name was handed out, or a model waiting in the inbox.
fn pinned(stored: &Stored) -> bool {
    match stored.job.state {
        JobState::Waiting | JobState::Inbox => true,
        JobState::Queued | JobState::Running => stored.chain.model_task.is_some(),
        _ => false,
    }
}

/// Gives every loaded job a workspace-unique id. Of jobs sharing one, the
/// pinned one (else the oldest) keeps it; each other one takes the next
/// free `<asset>-<n>`, and the redo links of its project follow it.
/// `handoff_taken(project, id)` says whether a handoff folder already uses
/// an id. Returns the projects whose jobs changed, so they can be saved.
pub(super) fn make_unique(
    jobs: &mut [Stored],
    handoff_taken: impl Fn(&str, &str) -> bool,
) -> Vec<String> {
    let mut order: Vec<usize> = (0..jobs.len()).collect();
    order.sort_by(|&a, &b| {
        let (a, b) = (&jobs[a], &jobs[b]);
        pinned(b)
            .cmp(&pinned(a))
            .then(a.job.created_at_ms.total_cmp(&b.job.created_at_ms))
            .then_with(|| a.job.project.cmp(&b.job.project))
    });
    let mut seen: HashSet<String> = HashSet::new();
    let mut all: HashSet<String> = jobs.iter().map(|s| s.job.id.clone()).collect();
    let mut touched = Vec::new();
    for index in order {
        if seen.insert(jobs[index].job.id.clone()) {
            continue;
        }
        let (project, asset, old) = {
            let job = &jobs[index].job;
            (job.project.clone(), job.asset.clone(), job.id.clone())
        };
        let fresh = (1u32..)
            .map(|n| format!("{asset}-{n}"))
            .find(|id| !all.contains(id) && !handoff_taken(&project, id))
            .unwrap_or_else(|| format!("{asset}-x"));
        tracing::info!(
            project,
            old,
            new = fresh,
            "gave a job a workspace-unique id"
        );
        for other in jobs.iter_mut().filter(|s| s.job.project == project) {
            if other.job.redo_of.as_deref() == Some(old.as_str()) {
                other.job.redo_of = Some(fresh.clone());
            }
        }
        jobs[index].job.id.clone_from(&fresh);
        all.insert(fresh.clone());
        seen.insert(fresh);
        if !touched.contains(&project) {
            touched.push(project);
        }
    }
    touched
}
