//! The grip library: hand poses kept outside any project, next to the
//! recipe library, each with every revision saved to it. One folder per
//! grip:
//!
//! ```text
//! library/grips/
//!   longsword-two-hands/
//!     entry.json       name, uid, every revision and where it came from
//!     r1.grip.json     revision 1, a grip file ([`crate::recipes::grips`])
//!     r2.grip.json
//! ```
//!
//! It is built as the recipe library is. A revision is written once and
//! never changed: saving again adds the next one, and removing one moves
//! its file to `library/grips/.removed/`. Revision numbers are never
//! handed out twice. `entry.json` keeps where each revision came from (the
//! project, recipe, pose key change and frame it was saved from, and
//! when). A write that read an entry sends its `version` (a hash of
//! `entry.json`) back as `expected`, so an entry that moved meanwhile is
//! refused (code `moved`).
//!
//! A recipe never reads the library to play or bake: its grip change holds
//! its own copy of the pose. The link back ([`GripFrom`]) names the entry
//! by id and uid, so an id taken again after a removal is never mistaken
//! for the grip a recipe came from, and it only serves the offer of a
//! newer revision ([`links`]). Agents read the library and lay grips on
//! recipes; only a person saves to it or removes from it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use crate::animations::recipes::library::read_capped;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::{latin_slug, windows_safe};
use crate::recipes::grips::{self, GRIP_SUFFIX, GripFile, GripHolds};
use crate::recipes::{BonePose, GripFrom, ID_MAX_CHARS, is_entry_uid, is_recipe_id, version_of};

mod apply;
mod from_keys;
mod remove;
mod save;

pub use apply::{ApplyGrip, GripApplied, GripLink, apply, links};
pub use from_keys::{GripFromKeys, GripKeyed, from_keys};
pub use remove::{REMOVED_DIR, remove_entry, remove_revision};
pub use save::{GripSave, GripSavePlan, GripSaved, GripSource, changes, plan_save, save_grip};

/// The grip library's folder, under the model library's.
pub const LIBRARY_GRIPS_DIR: &str = "grips";
pub const ENTRY_FILE: &str = "entry.json";
pub const ENTRY_VERSION: u32 = 1;
/// The biggest `entry.json` read.
const MAX_ENTRY_BYTES: u64 = 256 * 1024;
/// The most revisions one grip keeps listed.
pub const MAX_REVISIONS: usize = 500;

/// One revision of a library grip, with where it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryGripRevision {
    pub revision: u32,
    /// `r<revision>.grip.json`, in the grip's folder.
    pub file: String,
    /// RFC 3339, when it was saved.
    pub saved_at: String,
    /// The project, recipe, pose key change and bake frame it was saved from.
    pub project: Option<String>,
    pub recipe: Option<String>,
    pub layer: Option<String>,
    pub frame: Option<u32>,
}

/// `entry.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryGripEntry {
    pub version: u32,
    /// The folder name.
    pub id: String,
    /// Made once with the grip and never changed: a recipe's link names
    /// it, so an id taken again after a removal is not mistaken for the
    /// grip it came from.
    pub uid: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    /// The highest revision ever saved, so a removed one is never reused.
    pub last_revision: u32,
    /// Oldest first.
    pub revisions: Vec<LibraryGripRevision>,
}

impl LibraryGripEntry {
    pub fn latest(&self) -> Option<&LibraryGripRevision> {
        self.revisions.iter().max_by_key(|r| r.revision)
    }

    pub fn find(&self, revision: u32) -> Option<&LibraryGripRevision> {
        self.revisions.iter().find(|r| r.revision == revision)
    }
}

/// A line in the list of library grips: the entry and its newest revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryGripSummary {
    pub id: String,
    /// The entry's uid; empty when it could not be read.
    pub uid: String,
    /// The entry's name, or its id when it could not be read.
    pub name: String,
    /// A hash of `entry.json`, sent back as `expected` by a write.
    pub version: String,
    /// The newest revision; 0 when none could be read.
    pub latest: u32,
    pub revisions: Vec<LibraryGripRevision>,
    /// The newest revision's weapon and the group it sorts under.
    pub weapon: String,
    pub group: String,
    /// The newest revision's hands; none when it could not be read.
    pub hands: Option<GripHolds>,
    /// How those hands read: `both hands`, `right hand`, `either hand`.
    pub hands_words: String,
    /// The newest revision's pose, which its picture is drawn from.
    pub pose: BonePose,
    pub updated_at: String,
    /// Why the entry or its newest revision could not be read.
    pub problem: Option<String>,
}

/// A library grip's revision as the view and agents read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryGripDoc {
    pub entry: LibraryGripEntry,
    /// A hash of `entry.json`, sent back as `expected` by a write.
    pub version: String,
    /// The revision read.
    pub revision: u32,
    pub grip: GripFile,
}

/// The grip library folder of the workspace at `root`.
pub fn dir(root: &Path) -> PathBuf {
    crate::library::dir(root).join(LIBRARY_GRIPS_DIR)
}

/// The folder of grip `id`, which must be an id a folder can have (the
/// shape of a recipe id), so nothing outside the grip library is ever read
/// or written.
///
/// # Errors
///
/// [`CoreError::NotFound`] for anything that is not such an id.
pub fn entry_dir(root: &Path, id: &str) -> CoreResult<PathBuf> {
    if !is_recipe_id(id) {
        return Err(CoreError::NotFound(format!("the library grip {id}")));
    }
    Ok(dir(root).join(id))
}

/// The file name of revision `n`.
pub fn revision_file(n: u32) -> String {
    format!("r{n}{GRIP_SUFFIX}")
}

/// Where revision `n` of grip `id` sits, as a person reads it:
/// `library/grips/<id>/r<n>.grip.json`.
pub fn revision_path(id: &str, n: u32) -> String {
    format!(
        "{}/{LIBRARY_GRIPS_DIR}/{id}/{}",
        crate::library::LIBRARY_DIR,
        revision_file(n)
    )
}

fn check_entry(entry: &LibraryGripEntry) -> CoreResult<()> {
    if !is_entry_uid(&entry.uid) {
        return Err(CoreError::Invalid(format!(
            "The library grip {} has no uid it can be told apart by.",
            entry.id
        )));
    }
    if entry.revisions.len() > MAX_REVISIONS {
        return Err(CoreError::Invalid(format!(
            "The library grip {} lists more than {MAX_REVISIONS} revisions.",
            entry.id
        )));
    }
    for rev in &entry.revisions {
        if rev.revision == 0 || rev.file != revision_file(rev.revision) {
            return Err(CoreError::Invalid(format!(
                "The library grip {} names {:?}, which is not a revision file.",
                entry.id, rev.file
            )));
        }
    }
    Ok(())
}

/// Reads grip `id`'s entry with its version.
///
/// # Errors
///
/// Fails for an unknown grip, an unreadable or newer file, or one whose
/// revisions name other files.
pub fn load_entry(root: &Path, id: &str) -> CoreResult<(LibraryGripEntry, String)> {
    let file = entry_dir(root, id)?.join(ENTRY_FILE);
    let text = match read_capped(&file, MAX_ENTRY_BYTES) {
        Err(CoreError::NotFound(_)) => {
            return Err(CoreError::NotFound(format!("the library grip {id}")));
        }
        other => other?,
    };
    let mut entry: LibraryGripEntry =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if entry.version > ENTRY_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: entry.version,
            supported: ENTRY_VERSION,
        });
    }
    entry.id = id.to_string();
    check_entry(&entry)?;
    Ok((entry, version_of(&text)))
}

/// Refuses a write that read grip `id` at another version.
fn check_expected(id: &str, current: &str, expected: Option<&str>) -> CoreResult<()> {
    match expected {
        Some(read) if read != current => Err(CoreError::Moved(format!(
            "The library grip {id} changed since it was read (its version is now {current}). Read it again."
        ))),
        _ => Ok(()),
    }
}

/// Revision `revision` of `entry`, the newest when none is given.
///
/// # Errors
///
/// Fails for a revision the entry does not list, or a file that is missing,
/// unreadable or holds another revision number.
pub fn read_revision(
    root: &Path,
    entry: &LibraryGripEntry,
    revision: Option<u32>,
) -> CoreResult<GripFile> {
    let rev = match revision {
        Some(n) => entry.find(n),
        None => entry.latest(),
    }
    .ok_or_else(|| {
        CoreError::NotFound(match revision {
            Some(n) => format!("revision r{n} of the library grip {}", entry.id),
            None => format!("a revision of the library grip {}", entry.id),
        })
    })?;
    let grip = grips::load(&entry_dir(root, &entry.id)?.join(&rev.file))?;
    if grip.revision != rev.revision {
        return Err(CoreError::Invalid(format!(
            "{} of the library grip {} holds revision {}.",
            rev.file, entry.id, grip.revision
        )));
    }
    Ok(grip)
}

fn summary(root: &Path, id: &str) -> LibraryGripSummary {
    let mut line = LibraryGripSummary {
        id: id.to_string(),
        uid: String::new(),
        name: id.to_string(),
        version: String::new(),
        latest: 0,
        revisions: Vec::new(),
        weapon: String::new(),
        group: grips::weapon_group("").to_string(),
        hands: None,
        hands_words: String::new(),
        pose: BonePose::new(),
        updated_at: String::new(),
        problem: None,
    };
    let (entry, version) = match load_entry(root, id) {
        Ok(read) => read,
        Err(err) => {
            line.problem = Some(err.to_string());
            return line;
        }
    };
    line.name.clone_from(&entry.name);
    line.uid.clone_from(&entry.uid);
    line.version = version;
    line.updated_at.clone_from(&entry.updated_at);
    match read_revision(root, &entry, None) {
        Ok(grip) => {
            line.latest = grip.revision;
            line.group = grips::weapon_group(&grip.weapon).to_string();
            line.hands_words = grips::hands_words(&grip).to_string();
            line.hands = Some(grip.hands);
            line.weapon = grip.weapon;
            line.pose = grip.pose;
        }
        Err(err) => line.problem = Some(err.to_string()),
    }
    line.revisions = entry.revisions;
    line
}

/// Every library grip, by name. An entry that cannot be read still lists,
/// with its problem, so it can be removed.
///
/// # Errors
///
/// Fails when the folder exists but cannot be read.
pub fn list(root: &Path) -> CoreResult<Vec<LibraryGripSummary>> {
    let parent = dir(root);
    let read = match fs::read_dir(&parent) {
        Ok(read) => read,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&parent, err)),
    };
    let mut found: Vec<LibraryGripSummary> = read
        .filter_map(Result::ok)
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            (is_recipe_id(&id) && e.path().join(ENTRY_FILE).is_file()).then_some(id)
        })
        .map(|id| summary(root, &id))
        .collect();
    found.sort_by(|a, b| (a.name.to_lowercase(), &a.id).cmp(&(b.name.to_lowercase(), &b.id)));
    Ok(found)
}

/// Revision `revision` (the newest by default) of grip `id`.
///
/// # Errors
///
/// Fails for an unknown grip or revision, or an unreadable file.
pub fn get(root: &Path, id: &str, revision: Option<u32>) -> CoreResult<LibraryGripDoc> {
    let (entry, version) = load_entry(root, id)?;
    let grip = read_revision(root, &entry, revision)?;
    Ok(LibraryGripDoc {
        revision: grip.revision,
        entry,
        version,
        grip,
    })
}

/// The link to revision `revision` of `entry`.
pub fn link_to(entry: &LibraryGripEntry, revision: u32) -> GripFrom {
    GripFrom {
        entry: entry.id.clone(),
        uid: Some(entry.uid.clone()),
        revision,
    }
}

/// The entry `from` names, with its version, while it is still that grip:
/// none when it is gone or its id now holds another grip. A link with no
/// uid goes by the id alone.
///
/// # Errors
///
/// Fails when the entry is there but cannot be read.
pub fn linked_entry(
    root: &Path,
    from: &GripFrom,
) -> CoreResult<Option<(LibraryGripEntry, String)>> {
    match load_entry(root, &from.entry) {
        Ok((entry, version)) if from.uid.as_deref().is_none_or(|uid| uid == entry.uid) => {
            Ok(Some((entry, version)))
        }
        Ok(_) | Err(CoreError::NotFound(_)) => Ok(None),
        Err(err) => Err(err),
    }
}

/// The names of the library's grips; an entry that cannot be read has none.
fn names(root: &Path) -> Vec<String> {
    let Ok(read) = fs::read_dir(dir(root)) else {
        return Vec::new();
    };
    read.filter_map(Result::ok)
        .filter_map(|e| load_entry(root, &e.file_name().to_string_lossy()).ok())
        .map(|(entry, _)| entry.name)
        .collect()
}

/// A free grip id for `name`: its slug, then `-2`, `-3` and on.
fn free_entry_id(root: &Path, name: &str) -> CoreResult<String> {
    let base = latin_slug(name).map_or_else(
        || "grip".to_string(),
        |slug| {
            let short: String = slug.chars().take(ID_MAX_CHARS - 5).collect();
            windows_safe(short.trim_end_matches('-').to_string())
        },
    );
    for n in 1..10_000 {
        let id = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        if !entry_dir(root, &id)?.exists() {
            return Ok(id);
        }
    }
    Err(CoreError::Invalid(format!(
        "There are too many library grips called {name}."
    )))
}

/// Writes revision `grip` (its `revision` set) into `entry`'s folder and
/// the entry after it, the entry last so a reader never lists a revision
/// whose file is not there yet.
fn write_revision(root: &Path, entry: &LibraryGripEntry, grip: &GripFile) -> CoreResult<()> {
    check_entry(entry)?;
    let folder = entry_dir(root, &entry.id)?;
    let text = grips::text(grip)?;
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    let file = folder.join(revision_file(grip.revision));
    if file.exists() {
        return Err(CoreError::Invalid(format!(
            "Revision r{} of the library grip {} is there already.",
            grip.revision, entry.id
        )));
    }
    crate::fs::write_atomic(&file, text.as_bytes()).map_err(|e| CoreError::io(&file, e))?;
    write_json(&folder.join(ENTRY_FILE), entry)
}

/// One grip library write at a time in this process.
static WRITES: Mutex<()> = Mutex::new(());

fn turn() -> MutexGuard<'static, ()> {
    WRITES.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
