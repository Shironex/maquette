//! Discarding an inbox model: the file moves into `<inbox>/.discarded/`
//! instead of being deleted, so a mistake can be undone by moving it back.
//! The folder sits inside the inbox, which neither the list nor the watcher
//! reads below its top level, and it travels with the workspace like the
//! rest of the inbox. The app has no OS trash helper; the workspace keeps
//! its own removed assets in `trash/` and this follows that habit.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{cache_dir, check_id, inbox_dir, set_aim};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::workspace::WorkspaceConfig;

/// The folder inside the inbox that discarded models go to.
pub const DISCARDED_DIR: &str = ".discarded";

/// Where a discarded model went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Discarded {
    /// The inbox id the model had.
    pub id: String,
    /// The file it moved to (an absolute path).
    pub file: String,
}

/// A name in `dir` that is free: `name`, else `stem-2.ext`, `stem-3.ext`.
pub(super) fn free_name(dir: &Path, id: &str) -> PathBuf {
    let first = dir.join(id);
    if !first.exists() {
        return first;
    }
    let path = Path::new(id);
    let stem = path
        .file_stem()
        .map_or_else(|| id.to_string(), |s| s.to_string_lossy().into_owned());
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2u32..)
        .map(|n| dir.join(format!("{stem}-{n}{ext}")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

/// Moves the inbox model `id` into `<inbox>/.discarded/` and forgets what
/// the inbox kept about it (its cached analysis, picked kind and aim). A
/// name already there gets a number, so an earlier discard is never
/// overwritten.
///
/// # Errors
///
/// Fails for a name outside the inbox, a missing file, or when it cannot be
/// moved.
pub fn discard(root: &Path, config: &WorkspaceConfig, id: &str) -> CoreResult<Discarded> {
    check_id(id)?;
    let source = inbox_dir(root, config).join(id);
    if !source.is_file() {
        return Err(CoreError::NotFound(format!("{id} in the inbox")));
    }
    let folder = inbox_dir(root, config).join(DISCARDED_DIR);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    let target = free_name(&folder, id);
    if fs::rename(&source, &target).is_err() {
        // A different volume: copy, then remove the original.
        fs::copy(&source, &target).map_err(|e| CoreError::io(&target, e))?;
        fs::remove_file(&source).map_err(|e| CoreError::io(&source, e))?;
    }
    let cache = cache_dir(root, config);
    let _ = fs::remove_file(cache.join(format!("{id}.json")));
    let _ = set_aim(root, config, id, None);
    forget_kind(&cache, id);
    Ok(Discarded {
        id: id.to_string(),
        file: target.display().to_string(),
    })
}

fn forget_kind(cache: &Path, id: &str) {
    let _one = super::choice_turn();
    let mut kinds = super::load_kinds(cache);
    if kinds.remove(id).is_some()
        && let Err(err) = write_json(&cache.join(super::KINDS_FILE), &kinds)
    {
        tracing::warn!(error = %err, "could not forget the discarded model's kind");
    }
}
