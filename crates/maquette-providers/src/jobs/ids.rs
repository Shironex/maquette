//! Job ids: `<asset>-<n>`, numbered per project, so the first send of an
//! asset in a project is `<asset>-1` whatever other projects sent. An id
//! alone can therefore be a send of several projects: every lookup goes by
//! project and id ([`JobKey`]).
//!
//! For a while ids were unique across the whole workspace instead, and
//! before that they were per project but looked up by id alone. Records
//! from both times load as they are and keep their ids.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{JobState, ProviderJob, Stored};
use crate::handoff::HANDOFF_DIR;
use crate::naming::send_number;

/// What names one send: its project and its id in that project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobKey {
    pub project: String,
    pub id: String,
}

impl JobKey {
    pub fn new(project: &str, id: &str) -> Self {
        Self {
            project: project.to_string(),
            id: id.to_string(),
        }
    }

    /// The key of `job`.
    pub fn of(job: &ProviderJob) -> Self {
        Self::new(&job.project, &job.id)
    }

    /// True when it names `job`.
    pub fn is(&self, job: &ProviderJob) -> bool {
        job.project == self.project && job.id == self.id
    }
}

/// How a saved job's id was numbered, which says where its files outside
/// the project folder are.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum IdScope {
    /// From before ids were numbered per project: no other job of the
    /// workspace had its id, and its files sit under workspace wide names.
    /// What a record without the field is.
    #[default]
    Workspace,
    /// Numbered in its project; another project may hold the same id.
    Project,
}

impl IdScope {
    pub(super) fn is_workspace(&self) -> bool {
        *self == Self::Workspace
    }
}

/// The names in a project's handoff folder. A missing folder has none.
pub(super) fn handoff_names(project_dir: &Path) -> Vec<String> {
    std::fs::read_dir(project_dir.join(HANDOFF_DIR))
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The next id of `asset` in a project that uses the ids `used` (its jobs
/// and its handoff folders): one past the highest number the asset has
/// there, so a number never reads older than a record still on hand, and
/// never an id in use.
fn next_free(asset: &str, used: &[&str]) -> String {
    let highest = used
        .iter()
        .filter_map(|id| send_number(id, asset))
        .max()
        .unwrap_or(0);
    let fresh = (1u32..=u32::MAX - highest)
        .map(|step| format!("{asset}-{}", highest + step))
        .find(|id| !used.contains(&id.as_str()));
    fresh.unwrap_or_else(|| format!("{asset}-x"))
}

/// The id the next send of `asset` in `project` gets. `jobs` is every job
/// of the open workspace; only the project's own count.
pub(super) fn next_id<'a>(
    jobs: impl Iterator<Item = &'a ProviderJob>,
    project: &str,
    project_dir: &Path,
    asset: &str,
) -> String {
    let folders = handoff_names(project_dir);
    let used: Vec<&str> = jobs
        .filter(|job| job.project == project)
        .map(|job| job.id.as_str())
        .chain(folders.iter().map(String::as_str))
        .collect();
    next_free(asset, &used)
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

/// Gives every job of one project (`jobs`, as its file was read) an id no
/// other job of that project has: a hand edit or two computers saving at
/// once can leave two records with one id, and every lookup needs the pair
/// of project and id to name one job. Of jobs sharing an id the pinned one
/// (else the oldest) keeps it and each other one takes the asset's next id.
/// A redo link to the shared id stays with the job that kept it: which of
/// the two it meant cannot be told. `folders` are the names in the
/// project's handoff folder. True when a job changed.
///
/// Jobs of two projects sharing an id are left alone: that is what per
/// project numbering gives.
pub(super) fn make_unique(jobs: &mut [Stored], folders: &[String]) -> bool {
    let mut order: Vec<usize> = (0..jobs.len()).collect();
    order.sort_by(|&a, &b| {
        let (a, b) = (&jobs[a], &jobs[b]);
        pinned(b)
            .cmp(&pinned(a))
            .then(a.job.created_at_ms.total_cmp(&b.job.created_at_ms))
    });
    let mut kept: Vec<String> = Vec::new();
    let mut changed = false;
    for index in order {
        let old = jobs[index].job.id.clone();
        if !kept.contains(&old) {
            kept.push(old);
            continue;
        }
        let asset = jobs[index].job.asset.clone();
        let fresh = {
            let used: Vec<&str> = jobs
                .iter()
                .map(|s| s.job.id.as_str())
                .chain(folders.iter().map(String::as_str))
                .collect();
            next_free(&asset, &used)
        };
        tracing::info!(
            project = jobs[index].job.project,
            old,
            new = fresh,
            "gave a job an id of its own in its project"
        );
        jobs[index].job.id.clone_from(&fresh);
        kept.push(fresh);
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_id_is_one_past_the_highest_the_asset_has() {
        assert_eq!(next_free("desk-lamp", &[]), "desk-lamp-1");
        assert_eq!(next_free("desk-lamp", &["desk-lamp-1"]), "desk-lamp-2");
        // A project that holds only number 3 from the older numbering gets
        // 4: never 3 again, and never a 1 that reads older than the 3.
        assert_eq!(next_free("desk-lamp", &["desk-lamp-3"]), "desk-lamp-4");
        assert_eq!(
            next_free("desk-lamp", &["desk-lamp-5", "desk-lamp-2", "crate-9"]),
            "desk-lamp-6"
        );
        // The sends of the second asset named Desk lamp are not the first's.
        assert_eq!(
            next_free("desk-lamp", &["desk-lamp-2-1", "desk-lamp-2-7"]),
            "desk-lamp-1"
        );
        assert_eq!(
            next_free("desk-lamp-2", &["desk-lamp-2", "desk-lamp-2-7"]),
            "desk-lamp-2-8"
        );
        // An id with no number counts for nothing and is never handed out.
        assert_eq!(next_free("lamp", &["lamp-x"]), "lamp-1");
        let top = format!("lamp-{}", u32::MAX);
        assert_eq!(next_free("lamp", &[top.as_str()]), "lamp-x");
    }

    #[test]
    fn a_key_names_a_job_only_with_its_project() {
        let key = JobKey::new("moon-island", "desk-lamp-1");
        assert_eq!(key, JobKey::new("moon-island", "desk-lamp-1"));
        assert_ne!(key, JobKey::new("lantern-bay", "desk-lamp-1"));
        assert_ne!(key, JobKey::new("moon-island", "desk-lamp-2"));
    }

    #[test]
    fn a_record_without_a_scope_is_from_the_older_numbering() {
        assert_eq!(
            serde_json::from_str::<IdScope>("\"project\"").expect("project"),
            IdScope::Project
        );
        assert_eq!(IdScope::default(), IdScope::Workspace);
        assert!(IdScope::Workspace.is_workspace() && !IdScope::Project.is_workspace());
    }
}
