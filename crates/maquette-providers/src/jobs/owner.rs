//! Which job a file in the inbox belongs to. The inbox is one folder for
//! the whole workspace and job ids are numbered per project, so a name such
//! as `desk-lamp-1.glb` can read as a send of several projects. The answer
//! is one job or none, never a guess between projects: a wrong one would
//! link the model, its accept and its redo to another project's asset.

use std::path::Path;

use super::{JobState, ProviderJob};
use crate::handback::SLACK_MS;
use crate::naming::is_named;

/// Lower case with `_` and spaces read as `-`, for matching file names to
/// job ids.
fn normalize(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c == '_' || c == ' ' { '-' } else { c })
        .collect()
}

/// The job id an inbox file name points at: the whole stem, or the
/// longest job id it starts with, followed by `-`.
pub(super) fn match_job<'a>(stem: &str, ids: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let stem = normalize(stem);
    ids.filter(|id| {
        let id = normalize(id);
        stem == id
            || stem
                .strip_prefix(&id)
                .is_some_and(|rest| rest.starts_with('-'))
    })
    .max_by_key(|id| id.len())
}

fn file_stem(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().into_owned())
}

/// Of the jobs a file could be for, the one it is for: a send still waiting
/// for its model, else one whose model is in the inbox, else the only one
/// there is. `None` as soon as that leaves two.
fn only<'a>(found: &[&'a ProviderJob]) -> Option<&'a ProviderJob> {
    for state in [Some(JobState::Waiting), Some(JobState::Inbox), None] {
        let in_state: Vec<&'a ProviderJob> = found
            .iter()
            .copied()
            .filter(|job| state.is_none_or(|s| job.state == s))
            .collect();
        match in_state.as_slice() {
            [] => {}
            [one] => return Some(one),
            _ => return None,
        }
    }
    None
}

/// The job whose model the inbox file `item` is: the one that saved it
/// there and still says so (it is in the Inbox state). A job that was
/// accepted or stopped keeps the name its model had, and the inbox gives a
/// free name out again, so of two that claim it the one whose model came
/// last has it.
pub(super) fn saved_owner<'a>(
    item: &str,
    jobs: impl Iterator<Item = &'a ProviderJob>,
) -> Option<&'a ProviderJob> {
    jobs.filter(|job| job.state == JobState::Inbox && job.inbox_file.as_deref() == Some(item))
        .max_by(|a, b| {
            let at = |job: &ProviderJob| job.finished_at_ms.unwrap_or(job.created_at_ms);
            at(a).total_cmp(&at(b))
        })
}

/// The job the inbox file `item` belongs to: the one that saved it
/// ([`saved_owner`]), else by its name: the export name a By hand send was
/// handed out with, else the longest job id the name starts with (how a
/// model saved into the inbox by hand under its send's id reads). A bare id
/// only reaches a send still waiting for its model or one whose model is in
/// the inbox, and only when the file was written after that send was made
/// (`modified_ms`, with the Downloads watch's slack; unknown means no). A
/// job that is over, or a send newer than the file, may share the id with
/// a send of another project that is gone (removed with its project, say),
/// and a link would hand that model, its accept and its redo to the wrong
/// asset.
pub(super) fn owner_of<'a>(
    item: &str,
    modified_ms: Option<f64>,
    jobs: impl Iterator<Item = &'a ProviderJob>,
) -> Option<&'a ProviderJob> {
    let jobs: Vec<&'a ProviderJob> = jobs.collect();
    if let Some(saved) = saved_owner(item, jobs.iter().copied()) {
        return Some(saved);
    }
    let expecting: Vec<&'a ProviderJob> = jobs
        .iter()
        .copied()
        .filter(|job| job.export_name().is_some_and(|name| is_named(item, &name)))
        .collect();
    if !expecting.is_empty() {
        return only(&expecting);
    }
    let open: Vec<&'a ProviderJob> = jobs
        .iter()
        .copied()
        .filter(|job| matches!(job.state, JobState::Waiting | JobState::Inbox))
        .filter(|job| modified_ms.is_some_and(|at| at + SLACK_MS >= job.created_at_ms))
        .collect();
    let id = match_job(&file_stem(item), open.iter().map(|job| job.id.as_str()))?;
    let named: Vec<&'a ProviderJob> = open.iter().copied().filter(|job| job.id == id).collect();
    only(&named)
}
