//! The link from a project's recipe to the library recipe it came from.
//! An entry's id is its folder name, which can be taken again once the
//! entry is removed, so every entry also gets a uid when it is made, and a
//! link names both. A link whose entry is gone, or whose entry now has
//! another uid, reads as gone: it never points at an unrelated recipe.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{LibraryRecipeEntry, load_entry};
use crate::error::{CoreError, CoreResult};
use crate::recipes::RecipeFrom;

/// A fresh uid for a library recipe made from `seed`: 16 hex digits of a
/// hash of the seed, the clock, the process and a counter.
pub(super) fn new_uid(seed: &str) -> String {
    use sha2::{Digest, Sha256};
    static MADE: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let count = MADE.fetch_add(1, Ordering::Relaxed);
    let text = format!("{seed}\n{nanos}\n{}\n{count}", std::process::id());
    hex::encode(Sha256::digest(text.as_bytes()))[..16].to_string()
}

/// The link to revision `revision` of `entry`.
pub fn link_to(entry: &LibraryRecipeEntry, revision: u32) -> RecipeFrom {
    RecipeFrom {
        entry: entry.id.clone(),
        uid: entry.uid.clone(),
        revision,
    }
}

/// The entry `from` names, with its version, while it is still that
/// recipe: none when it is gone or its id now holds another recipe.
///
/// # Errors
///
/// Fails when the entry is there but cannot be read.
pub fn linked_entry(
    root: &Path,
    from: &RecipeFrom,
) -> CoreResult<Option<(LibraryRecipeEntry, String)>> {
    match load_entry(root, &from.entry) {
        Ok((entry, version)) if entry.uid == from.uid => Ok(Some((entry, version))),
        Ok(_) | Err(CoreError::NotFound(_)) => Ok(None),
        Err(err) => Err(err),
    }
}
