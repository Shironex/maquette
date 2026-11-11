//! Removing a library grip's revision, or the whole grip. Nothing is
//! deleted: files move to `library/grips/.removed/`, which the list never
//! reads, so they can be got back by hand. Recipes that copied a revision
//! keep their copy of the pose; their link reads the grip or revision as
//! gone. Only a person removes: no agent tool reaches these.

use std::fs;
use std::path::{Path, PathBuf};

use super::{ENTRY_FILE, LibraryGripEntry, check_expected, dir, entry_dir, load_entry, turn};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::now_rfc3339;

/// Where removed files wait.
pub const REMOVED_DIR: &str = ".removed";

fn stamp() -> String {
    now_rfc3339().chars().filter(char::is_ascii_digit).collect()
}

fn removed_dir(root: &Path) -> CoreResult<PathBuf> {
    let to = dir(root).join(REMOVED_DIR);
    fs::create_dir_all(&to).map_err(|e| CoreError::io(&to, e))?;
    Ok(to)
}

/// Removes revision `revision` of grip `id`, read at `expected`. The only
/// revision left cannot go this way: remove the grip. Answers with the
/// entry as it is now.
///
/// # Errors
///
/// Fails for an unknown grip or revision, a grip that moved since
/// `expected`, the last revision, or files that cannot be moved.
pub fn remove_revision(
    root: &Path,
    id: &str,
    revision: u32,
    expected: Option<&str>,
) -> CoreResult<LibraryGripEntry> {
    let _turn = turn();
    let (mut entry, version) = load_entry(root, id)?;
    check_expected(id, &version, expected)?;
    let Some(at) = entry.revisions.iter().position(|r| r.revision == revision) else {
        return Err(CoreError::NotFound(format!(
            "revision r{revision} of the library grip {id}"
        )));
    };
    if entry.revisions.len() == 1 {
        return Err(CoreError::Invalid(format!(
            "r{revision} is the only revision of {}. Remove the whole grip instead.",
            entry.name
        )));
    }
    let gone = entry.revisions.remove(at);
    entry.updated_at = now_rfc3339();
    let folder = entry_dir(root, id)?;
    write_json(&folder.join(ENTRY_FILE), &entry)?;
    let from = folder.join(&gone.file);
    if from.is_file() {
        let to = removed_dir(root)?.join(format!("{id}-{}-{}", stamp(), gone.file));
        fs::rename(&from, &to).map_err(|e| CoreError::io(&from, e))?;
    }
    Ok(entry)
}

/// Removes grip `id`, read at `expected`, with every revision. Answers
/// with where its folder went, relative to the grip library. A grip whose
/// entry cannot be read has no version to have moved from: it still goes,
/// so a broken one can be cleared out of the list.
///
/// # Errors
///
/// Fails for an unknown grip, one that moved since `expected`, or a
/// folder that cannot be moved.
pub fn remove_entry(root: &Path, id: &str, expected: Option<&str>) -> CoreResult<String> {
    let _turn = turn();
    match load_entry(root, id) {
        Ok((_, version)) => check_expected(id, &version, expected)?,
        Err(err @ CoreError::NotFound(_)) => return Err(err),
        Err(_) => {}
    }
    let from = entry_dir(root, id)?;
    let name = format!("{id}-{}", stamp());
    let to = removed_dir(root)?.join(&name);
    fs::rename(&from, &to).map_err(|e| CoreError::io(&from, e))?;
    Ok(format!("{REMOVED_DIR}/{name}"))
}
