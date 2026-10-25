//! Settling a finished concept job: the image at its path, or copied out
//! of the folder where Codex keeps its own, or a failure that says why.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::{ConceptJob, ConceptJobState};
use crate::concepts::{Direction, add_image};
use crate::pipeline::history::RunState;

fn is_image(path: &Path) -> bool {
    fs::read(path)
        .ok()
        .and_then(|b| crate::inbox::images::dimensions(&b))
        .is_some()
}

/// The newest image Codex made in `session` since `since`.
fn generated_image(codex_home: &Path, session: &str, since: Option<SystemTime>) -> Option<PathBuf> {
    let dir = codex_home.join("generated_images").join(session);
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if !matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg" | "webp")) {
            continue;
        }
        let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        if since.is_some_and(|s| modified + Duration::from_secs(2) < s) {
            continue;
        }
        if best.as_ref().is_none_or(|(t, _)| modified > *t) {
            best = Some((modified, path));
        }
    }
    best.map(|(_, p)| p)
}

/// Settles a finished job: done when the image is there (copied out of
/// Codex's folder when it was left there), failed with a reason otherwise.
/// Returns the direction when an image was added.
pub(super) fn finish(
    job: &mut ConceptJob,
    state: RunState,
    reason: Option<String>,
    last_error: Option<String>,
    project_dir: &Path,
    codex_home: Option<&Path>,
    session: Option<&str>,
    started: Option<SystemTime>,
) -> Option<Direction> {
    let target = project_dir.join(&job.file);
    if state == RunState::Cancelled {
        job.state = ConceptJobState::Cancelled;
        job.message = Some(reason.unwrap_or_else(|| "Cancelled.".into()));
        return None;
    }
    let mut note = None;
    if !is_image(&target)
        && let (Some(home), Some(session)) = (codex_home, session)
        && let Some(found) = generated_image(home, session, started)
    {
        if let Some(parent) = target.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::copy(&found, &target).is_ok() {
            note = Some(format!(
                "Codex left the image in its own folder; copied {}.",
                found.display()
            ));
        }
    }
    if is_image(&target) {
        match add_image(
            project_dir,
            &job.direction,
            job.index,
            &job.file,
            job.title.as_deref(),
        ) {
            Ok(direction) => {
                job.state = ConceptJobState::Done;
                if let Some(image) = direction.images.iter().find(|i| i.file == job.file) {
                    job.width = image.width;
                    job.height = image.height;
                }
                job.message = note.or_else(|| {
                    (state == RunState::Failed).then(|| {
                        format!(
                            "The image arrived, but Codex then stopped with an error: {}",
                            reason.clone().unwrap_or_default()
                        )
                    })
                });
                return Some(direction);
            }
            Err(err) => {
                job.state = ConceptJobState::Failed;
                job.message = Some(format!("The image arrived but was not recorded. {err}"));
                return None;
            }
        }
    }
    job.state = ConceptJobState::Failed;
    let last = last_error.or_else(|| {
        job.log_tail
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .cloned()
    });
    job.message = Some(match (state, reason) {
        (RunState::Done, _) => format!(
            "Codex finished without saving the image to {}. Its log shows what it did.",
            job.file
        ),
        (_, Some(reason)) => match last {
            Some(last) if !reason.contains(&last) => format!("{reason} Last line: {last}"),
            _ => reason,
        },
        (_, None) => "Codex stopped before the image arrived.".into(),
    });
    None
}
