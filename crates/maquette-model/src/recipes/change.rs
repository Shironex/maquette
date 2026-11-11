//! Changing a recipe: what a change replaces, and the one checked write
//! that applies it under the write lock.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::clip_moves::replace_clips;
use super::files::{RecipeRead, settled, version, write_recipe, writing};
use super::{
    BlankKind, RecipeBase, RecipeBlank, RecipeFile, RecipeLayer, TimeEdit, check_name, check_props,
    load_versioned, normalized, path, validate,
};
use crate::error::{CoreError, CoreResult};
use crate::plans::trimmed;
use crate::project::{ProjectManifest, now_rfc3339};

/// What a change to a recipe replaces: its name, the changes, the blanks,
/// the clips, the time edits, mirror, or any of them, each whole. A change
/// with an empty id gets a free one (`l-1`, `l-2`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeChange {
    /// A new name; the file and id stay.
    #[serde(default)]
    pub name: Option<String>,
    pub layers: Option<Vec<RecipeLayer>>,
    pub blanks: Option<Vec<RecipeBlank>>,
    /// Every clip in order, the base first, each with its trim and (after
    /// the first) its blend window. The keys, changes and time edits the
    /// recipe keeps (the lists not sent with it) follow their clip to its
    /// new place, a change's start and end moved to cover the same motion;
    /// taking away a clip one of them sits on, or splitting the clips a
    /// change runs across, is refused. Lists sent with it count clips, and
    /// read shares, in the new order.
    #[serde(default)]
    #[specta(optional)]
    pub clips: Option<Vec<RecipeBase>>,
    /// For each clip of `clips`, the index of the recipe's clip it keeps (0
    /// the base), or none for a new one. Left out, a clip sent keeps the
    /// first of the recipe's clips with its name, frames and hash.
    #[serde(default)]
    #[specta(optional)]
    pub kept: Option<Vec<Option<u32>>>,
    /// Every time edit.
    #[serde(default)]
    #[specta(optional)]
    pub time: Option<Vec<TimeEdit>>,
    /// Left and right swapped across the whole recipe, or not.
    #[serde(default)]
    #[specta(optional)]
    pub mirror: Option<bool>,
    /// The `version` of the recipe as it was read (a hash of the file's
    /// bytes). Given, a file that changed since in any way is refused (code
    /// `moved`) and nothing is written.
    #[serde(default)]
    pub expected: Option<String>,
    /// Prop blanks to empty for good, even when the prop they name is no
    /// longer an asset (which a blank sent open otherwise keeps). A blank
    /// sent with a prop keeps that prop.
    #[serde(default)]
    pub clear: Vec<String>,
}

/// Free ids for the changes that came without one.
fn with_ids(layers: Vec<RecipeLayer>) -> Vec<RecipeLayer> {
    let mut taken: Vec<String> = layers
        .iter()
        .map(|l| trimmed(&l.id).to_string())
        .filter(|id| !id.is_empty())
        .collect();
    let mut next = 1u32;
    layers
        .into_iter()
        .map(|mut layer| {
            if trimmed(&layer.id).is_empty() {
                while taken.contains(&format!("l-{next}")) {
                    next += 1;
                }
                layer.id = format!("l-{next}");
                taken.push(layer.id.clone());
            }
            layer
        })
        .collect()
}

/// Puts back the stored prop of each prop blank sent open whose prop the
/// project no longer has: it read as open, so sending it open again is not
/// a choice to empty it, and restoring the asset fills it again. A blank
/// named in `clear` is left empty.
fn keep_stale_props(
    next: &mut RecipeFile,
    current: &RecipeFile,
    manifest: &ProjectManifest,
    clear: &[String],
) {
    for blank in &mut next.blanks {
        if blank.kind != BlankKind::Prop || blank.prop.is_some() || clear.contains(&blank.id) {
            continue;
        }
        let stale = current
            .blanks
            .iter()
            .find(|b| b.id == blank.id && b.kind == BlankKind::Prop)
            .and_then(|b| b.prop.as_ref())
            .filter(|p| !manifest.assets.iter().any(|a| &a.id == *p));
        if let Some(prop) = stale {
            blank.prop = Some(prop.clone());
        }
    }
}

/// Refuses a `clear` naming something that is not a prop blank of `next`.
fn check_clear(next: &RecipeFile, clear: &[String]) -> CoreResult<()> {
    match clear.iter().find(|id| {
        !next
            .blanks
            .iter()
            .any(|b| &&b.id == id && b.kind == BlankKind::Prop)
    }) {
        Some(id) => Err(CoreError::Invalid(format!(
            "The recipe has no prop blank {id} to clear."
        ))),
        None => Ok(()),
    }
}

/// Refuses a write that read the recipe at another version.
fn check_expected(id: &str, current: &str, expected: Option<&str>) -> CoreResult<()> {
    match expected {
        Some(read) if read != current => Err(CoreError::Moved(format!(
            "The recipe {id} changed since it was read (its version is now {current}). Read it again and send the change once more."
        ))),
        _ => Ok(()),
    }
}

/// Replaces the name, the changes, the blanks or any of them of the recipe
/// `id`, as the file holds it now, in one write after checking the whole
/// result. Blanks sent must name assets of `manifest`'s project. When the
/// blanks are kept, they stay in the file as they are: a prop blank naming
/// an asset the project no longer has keeps its prop, so restoring the
/// asset fills it again, and only the answer reads it as open, with a
/// warning, as [`get`](super::get) does. A prop blank sent open whose stored prop is
/// such a gone asset keeps it too, since it only read as open, unless
/// `clear` names it. With `expected`, a file whose version (a hash of its
/// bytes) moved since it was read is refused under the write lock, so no
/// write landing in between, nor a hand edit or a sync, is overwritten.
/// `edited` moves only when something changed; the revision never does.
///
/// # Errors
///
/// Fails on a name [`check_name`] refuses, when the recipe is gone or
/// changed since `expected`, `clips` take away a clip kept keys or time
/// edits sit on, `clear` names no prop blank, the result does not validate,
/// would be too big to write, or the file cannot be written.
pub fn update(
    project_dir: &Path,
    manifest: &ProjectManifest,
    id: &str,
    change: RecipeChange,
) -> CoreResult<RecipeRead> {
    changed(project_dir, manifest, id, change, false)
}

/// A person's save of the recipe `id`: [`update`], with the draft marks
/// cleared under the same write lock once the recipe is written, so no
/// agent write lands between the two and a save that fails keeps them.
/// Marks that cannot be cleared are only logged: the save has landed.
///
/// # Errors
///
/// As [`update`].
pub fn save(
    project_dir: &Path,
    manifest: &ProjectManifest,
    id: &str,
    change: RecipeChange,
) -> CoreResult<RecipeRead> {
    changed(project_dir, manifest, id, change, true)
}

fn changed(
    project_dir: &Path,
    manifest: &ProjectManifest,
    id: &str,
    change: RecipeChange,
    by_person: bool,
) -> CoreResult<RecipeRead> {
    if let Some(name) = &change.name {
        check_name(name)?;
    }
    let _lock = writing();
    let file = path(project_dir, id)?;
    let (current, current_version) = load_versioned(&file)?;
    check_expected(id, &current_version, change.expected.as_deref())?;
    let mut next = current.clone();
    if let Some(name) = change.name {
        next.name = trimmed(&name).to_string();
    }
    if let Some(clips) = change.clips {
        replace_clips(
            &mut next,
            &current,
            clips,
            change.kept.as_deref(),
            change.layers.is_some(),
            change.time.is_some(),
        )?;
    }
    if let Some(layers) = change.layers {
        next.layers = with_ids(layers);
    }
    if let Some(time) = change.time {
        next.time = time;
    }
    if let Some(mirror) = change.mirror {
        next.mirror = mirror;
    }
    let sent_blanks = change.blanks.is_some();
    if let Some(blanks) = change.blanks {
        next.blanks = blanks;
    }
    let mut next = normalized(&next);
    validate(&next)?;
    super::pins::check_ik_overlap(&next)?;
    check_clear(&next, &change.clear)?;
    if sent_blanks {
        check_props(&next, manifest)?;
        keep_stale_props(&mut next, &current, manifest, &change.clear);
    } else {
        for blank in &mut next.blanks {
            if change.clear.contains(&blank.id) {
                blank.prop = None;
            }
        }
    }
    let version = if next == current {
        current_version
    } else {
        next.edited = now_rfc3339();
        write(&file, &next)?;
        version(project_dir, id)?
    };
    // A person's save clears the marks once it has landed, still under the
    // lock, so a save that fails keeps them. Marks are only a hint: one
    // that cannot be cleared never turns the landed save into a failure.
    if by_person && let Err(err) = super::drafts::clear_held(project_dir, id) {
        tracing::warn!("recipe draft marks for {id} stayed after a save: {err}");
    }
    Ok(settled(next, version, manifest))
}

#[cfg(test)]
thread_local! {
    /// Fails this thread's next recipe write, for tests of a failed save.
    pub(super) static FAIL_NEXT_WRITE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn write(file: &Path, recipe: &RecipeFile) -> CoreResult<()> {
    #[cfg(test)]
    if FAIL_NEXT_WRITE.with(|f| f.replace(false)) {
        return Err(CoreError::io(
            file,
            std::io::Error::other("a test failed this write"),
        ));
    }
    write_recipe(file, recipe)
}
