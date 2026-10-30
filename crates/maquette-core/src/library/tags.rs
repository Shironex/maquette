//! Tagging many entries at once and renaming one. Only `entry.json`
//! changes: the folder (the entry's id) and its files stay where they are.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{LibraryEntry, MAX_TAGS, clean_tags, load, save, turn};
use crate::assets;
use crate::error::{CoreError, CoreResult};
use crate::project::now_rfc3339;

/// What a bulk tag change asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TagEntries {
    pub entries: Vec<String>,
    /// Tags put on every entry.
    pub add: Vec<String>,
    /// Tags taken off every entry.
    pub remove: Vec<String>,
}

/// `ids` once each, in the order given.
pub(super) fn unique(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

/// Puts `request.add` on and takes `request.remove` off every entry it
/// names. Entries that end up with the tags they had are not written.
///
/// # Errors
///
/// Fails for an unknown entry, a bad tag, or an entry that would carry
/// more than [`MAX_TAGS`] tags, before anything is written.
pub fn tag(root: &Path, request: &TagEntries) -> CoreResult<Vec<LibraryEntry>> {
    if request.entries.is_empty() {
        return Err(CoreError::Invalid("Pick at least one model.".to_string()));
    }
    let add = clean_tags(&request.add)?;
    let remove = clean_tags(&request.remove)?;
    let _turn = turn();
    let ids = unique(&request.entries);
    let mut entries = ids
        .iter()
        .map(|id| load(root, id))
        .collect::<CoreResult<Vec<_>>>()?;
    let mut changed = Vec::new();
    for entry in &mut entries {
        let mut next: Vec<String> = entry
            .tags
            .iter()
            .filter(|t| !remove.contains(t))
            .cloned()
            .collect();
        for t in &add {
            if !next.contains(t) && !remove.contains(t) {
                next.push(t.clone());
            }
        }
        if next.len() > MAX_TAGS {
            return Err(CoreError::Invalid(format!(
                "{} would have {} tags. Give a model {MAX_TAGS} tags or fewer.",
                entry.name,
                next.len()
            )));
        }
        if next != entry.tags {
            entry.tags = next;
            changed.push(entry.id.clone());
        }
    }
    let now = now_rfc3339();
    for entry in entries.iter_mut().filter(|e| changed.contains(&e.id)) {
        entry.updated_at.clone_from(&now);
        save(root, entry)?;
    }
    Ok(request
        .entries
        .iter()
        .filter_map(|id| entries.iter().find(|e| &e.id == id).cloned())
        .collect())
}

/// Renames an entry. Its id (the folder) stays, so projects that came from
/// it still find it.
///
/// # Errors
///
/// Fails for an unknown entry or a name that is empty or too long.
pub fn rename(root: &Path, entry: &str, name: &str) -> CoreResult<LibraryEntry> {
    let name = assets::clean_name(name)?;
    let _turn = turn();
    let mut found = load(root, entry)?;
    if found.name != name {
        found.name = name;
        found.updated_at = now_rfc3339();
        save(root, &found)?;
    }
    Ok(found)
}
