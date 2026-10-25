//! Changing the saved preparation without losing anyone else's: [`update`]
//! reads `prepare.json` on disk, applies one change and writes the result,
//! like the material tweaks. Agents change one asset part by part
//! ([`apply_patch`]); the app replaces one asset's entry whole
//! ([`save_entry`]).

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use serde::Deserialize;

use super::{
    Fixup, FixupKind, ModelPrep, PREPARE_FILE, RealSize, ScenePrep, load, normalized, save,
    validate,
};
use crate::error::{CoreError, CoreResult};
use crate::json::explicit;

/// A partial change of one asset's preparation.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepPatch {
    /// Left out keeps the size; null clears it.
    #[serde(default, deserialize_with = "explicit")]
    pub size: Option<Option<RealSize>>,
    /// Drops every fix-up before `add` applies.
    #[serde(default)]
    pub clear_fixups: bool,
    /// Fix-ups to add; one of a kind already there is replaced.
    #[serde(default)]
    pub add: Vec<Fixup>,
    /// Kinds of fix-up to remove.
    #[serde(default)]
    pub remove: Vec<FixupKind>,
    /// Left out keeps the choice; true skips the bake, false bakes, null
    /// forgets the choice.
    #[serde(default, deserialize_with = "explicit")]
    pub skip_bake: Option<Option<bool>>,
}

/// Applies `patch` to `asset` in `current`. Nothing changes when any part
/// is refused.
///
/// # Errors
///
/// Fails when a kind is added twice, or [`validate`] refuses the result.
pub fn apply_patch(current: &ScenePrep, asset: &str, patch: &PrepPatch) -> CoreResult<ScenePrep> {
    let mut entry = current.models.get(asset).cloned().unwrap_or_default();
    if let Some(size) = patch.size {
        entry.size = size;
    }
    if let Some(skip) = patch.skip_bake {
        entry.skip_bake = skip;
    }
    if patch.clear_fixups {
        entry.fixups.clear();
    }
    entry.fixups.retain(|f| !patch.remove.contains(&f.kind()));
    for (i, fixup) in patch.add.iter().enumerate() {
        if patch.add[..i].iter().any(|f| f.kind() == fixup.kind()) {
            return Err(CoreError::Invalid(format!(
                "Add {} once per call.",
                fixup.kind().label().to_lowercase()
            )));
        }
        entry.fixups.retain(|f| f.kind() != fixup.kind());
        entry.fixups.push(*fixup);
    }
    let mut next = current.clone();
    next.models.insert(asset.to_string(), entry);
    validate(&next)?;
    Ok(normalized(&next))
}

/// One read, change and write at a time, so the app and an agent saving
/// together both land.
static WRITES: Mutex<()> = Mutex::new(());

/// Reads `prepare.json` from disk, applies `change` and writes what comes
/// back, checked and tidied. Returns what was written.
///
/// # Errors
///
/// Refuses, writing nothing, when the file on disk comes from a newer
/// Maquette or cannot be read (so a broken file is never overwritten), and
/// fails when `change` or [`validate`] refuses or the write fails.
pub fn update(
    project_dir: &Path,
    change: impl FnOnce(&ScenePrep) -> CoreResult<ScenePrep>,
) -> CoreResult<ScenePrep> {
    let _one = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let current = load(project_dir).map_err(|err| match err {
        CoreError::UnsupportedVersion { .. } => err,
        other => CoreError::Invalid(format!(
            "{PREPARE_FILE} could not be read, so nothing was saved over it. Fix or remove the file, then save again. {other}"
        )),
    })?;
    save(project_dir, &change(&current)?)
}

/// Replaces `asset`'s entry in the file on disk; an empty one removes it.
/// Every other asset keeps what is saved. Returns the whole file.
///
/// # Errors
///
/// As [`update`].
pub fn save_entry(project_dir: &Path, asset: &str, entry: &ModelPrep) -> CoreResult<ScenePrep> {
    update(project_dir, |current| {
        let mut next = current.clone();
        if entry.is_empty() {
            next.models.remove(asset);
        } else {
            next.models.insert(asset.to_string(), entry.clone());
        }
        Ok(next)
    })
}
