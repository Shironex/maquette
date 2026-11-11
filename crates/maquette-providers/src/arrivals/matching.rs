//! Which waiting By hand send a new download is for. Only the export name
//! a send was handed out with (`maq_<project>_<asset>_v<n>_<provider>`, the
//! older `maq_<asset>_v<n>_<provider>`, or the still older handoff name,
//! the job id) places a file by itself, and only when it names one waiting
//! send: sends of every project wait together, and two of them may share an
//! id. A file that fits one send (the only send waiting, or the one its
//! name is close to) is only suggested: a person looks at it first, however
//! long after its handoff it came. Anything else is asked, and its picture
//! ([`super::picture`]) may suggest a send later.

use crate::handback::{SLACK_MS, name_fit};
use crate::jobs::{JobKey, ProviderJob};
use crate::naming;

/// The least name fit that picks one send out of several.
const FIT: f64 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Verdict {
    /// Named for this job: take it.
    Exact { job: JobKey, reason: String },
    /// Most likely this job, but only a guess: wait for a look.
    Close { job: JobKey, reason: String },
    /// Ask which asset it is.
    Ask { reason: String },
}

fn exact(job: &ProviderJob, reason: String) -> Verdict {
    Verdict::Exact {
        job: JobKey::of(job),
        reason,
    }
}

fn close(job: &ProviderJob, reason: String) -> Verdict {
    Verdict::Close {
        job: JobKey::of(job),
        reason,
    }
}

/// What a file named for a send that is not waiting says.
fn not_waiting(named: &naming::ExportName) -> String {
    let project = named
        .project
        .as_deref()
        .map(|p| format!(" in {p}"))
        .unwrap_or_default();
    format!(
        "Named for {} v{} on {}{project}, which is not waiting for a model",
        named.asset,
        named.version,
        named.provider.label()
    )
}

fn stem(name: &str) -> &str {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    base.rsplit_once('.').map_or(base, |(s, _)| s)
}

/// Decides for a file `name` (and the GLB inside, for a zip) changed at
/// `modified_ms`, given the By hand sends waiting for a model.
pub(super) fn decide(
    name: &str,
    inner: Option<&str>,
    modified_ms: f64,
    waiting: &[ProviderJob],
) -> Verdict {
    let files = [Some(name), inner];
    // The name each send was handed out with, in either form.
    let expecting: Vec<&ProviderJob> = waiting
        .iter()
        .filter(|job| {
            job.export_name().is_some_and(|expected| {
                files
                    .iter()
                    .flatten()
                    .any(|f| naming::is_named(f, &expected))
            })
        })
        .collect();
    match expecting.as_slice() {
        [found] => return exact(found, format!("Named for {}", found.asset_name)),
        [] => {}
        several => {
            return Verdict::Ask {
                reason: format!("{} sends wait for a file of that name", several.len()),
            };
        }
    }
    if let Some(named) = files.iter().flatten().find_map(|f| naming::parse(f)) {
        return Verdict::Ask {
            reason: not_waiting(&named),
        };
    }
    // The handoff name from before export names: the job id itself. An id
    // is numbered per project, so it places a file only while one waiting
    // send has it.
    let stems = [Some(stem(name)), inner.map(stem)];
    let same_id: Vec<&ProviderJob> = waiting
        .iter()
        .filter(|job| {
            stems
                .iter()
                .flatten()
                .any(|s| s.eq_ignore_ascii_case(&job.id))
        })
        .collect();
    match same_id.as_slice() {
        [found] => return exact(found, format!("Named for {}", found.asset_name)),
        [] => {}
        several => {
            return Verdict::Ask {
                reason: format!(
                    "{} sends named {} are waiting, in {}",
                    several.len(),
                    several[0].id,
                    projects(several)
                ),
            };
        }
    }
    // Late is fine: a model made days after its handoff is still its model.
    let recent: Vec<&ProviderJob> = waiting
        .iter()
        .filter(|j| modified_ms + SLACK_MS >= j.created_at_ms)
        .collect();
    let mut fits: Vec<(f64, &ProviderJob)> = recent
        .iter()
        .map(|j| (name_fit(name, inner, j), *j))
        .collect();
    fits.sort_by(|a, b| b.0.total_cmp(&a.0));
    match (recent.as_slice(), fits.as_slice()) {
        // A stale send still waiting makes "the only one" a guess.
        ([only], _) if waiting.len() == 1 => close(
            only,
            format!("The only send waiting, for {}", only.asset_name),
        ),
        (_, [(best, found), rest @ ..])
            if *best >= FIT && rest.first().is_none_or(|(next, _)| next < best) =>
        {
            close(found, format!("Name close to {}", found.asset_name))
        }
        ([], _) => Verdict::Ask {
            reason: "It is older than the waiting handoffs".to_string(),
        },
        _ => Verdict::Ask {
            reason: match waiting.len() {
                1 => "1 send is waiting".to_string(),
                n => format!("{n} sends are waiting"),
            },
        },
    }
}

/// The projects of `jobs`, as a list in a sentence.
fn projects(jobs: &[&ProviderJob]) -> String {
    let mut names: Vec<&str> = jobs.iter().map(|job| job.project.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    names.join(" and ")
}

#[cfg(test)]
pub(super) mod tests;
