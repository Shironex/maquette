//! The recipe library: recipes kept outside any project, next to the clip
//! library, each with every revision saved to it. One folder per entry:
//!
//! ```text
//! library/recipes/
//!   reach-for-the-lantern/
//!     entry.json         name, every revision and where it came from
//!     r1.recipe.json     revision 1, a plain recipe file with revision 1
//!     r2.recipe.json
//! ```
//!
//! A revision is written once and never changed: saving a project's recipe
//! back adds the next one, and removing one moves its file to
//! `library/recipes/.removed/`. Revision numbers are never handed out
//! twice. A revision file is a version 1 recipe file with no `from`: only
//! clip refs, blanks and changes, never keys, frames or a mesh, so it is as
//! shareable as an exported file. `entry.json` keeps the provenance (the
//! project and recipe a revision was saved from, or the file it was
//! imported from, and when). A write that read an entry sends its
//! `version` (a hash of `entry.json`) back as `expected`, so an entry that
//! moved meanwhile is refused (code `moved`).

use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use super::{BaseMatch, match_base};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::{latin_slug, windows_safe};
use crate::recipes::{self, ID_MAX_CHARS, RECIPE_SUFFIX, RecipeFile, is_recipe_id, version_of};

mod diff;
mod link;
mod remove;
mod save;
pub mod starters;
mod transfer;
mod use_in;

pub use diff::changes;
pub use link::{link_to, linked_entry};
pub use remove::{REMOVED_DIR, remove_entry, remove_revision};
pub use save::{
    RecipeCompare, RecipeSave, RecipeSavePlan, RecipeSaved, compare, plan_save, save_back,
};
pub use transfer::{
    AGENT_EXPORTS, ExportSource, RecipeExported, RecipeImported, agent_export, agent_import_path,
    export, export_text, import_to_library, import_to_project, read_import, source_recipe,
};
pub use use_in::{RecipeUsed, UseRecipe, open_missing_props, use_recipe};

/// The recipe library's folder, under the model library's.
pub const LIBRARY_RECIPES_DIR: &str = "recipes";
pub const ENTRY_FILE: &str = "entry.json";
pub const ENTRY_VERSION: u32 = 1;
/// The biggest `entry.json` read.
const MAX_ENTRY_BYTES: u64 = 256 * 1024;
/// The most revisions one entry keeps listed.
pub const MAX_REVISIONS: usize = 500;

/// One revision of a library recipe, with where it came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecipeRevision {
    pub revision: u32,
    /// `r<revision>.recipe.json`, in the entry's folder.
    pub file: String,
    /// RFC 3339, when it was saved or imported.
    pub saved_at: String,
    /// The project and recipe it was saved back from.
    pub project: Option<String>,
    pub recipe: Option<String>,
    /// The file name it was imported from, without its folder.
    pub imported: Option<String>,
}

/// `entry.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecipeEntry {
    pub version: u32,
    /// The folder name.
    pub id: String,
    /// Made once with the entry and never changed: a project recipe's link
    /// names it, so an id taken again after a removal is not mistaken for
    /// the recipe it came from.
    pub uid: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    /// The highest revision ever saved, so a removed one is never reused.
    pub last_revision: u32,
    /// Oldest first.
    pub revisions: Vec<LibraryRecipeRevision>,
}

impl LibraryRecipeEntry {
    pub fn latest(&self) -> Option<&LibraryRecipeRevision> {
        self.revisions.iter().max_by_key(|r| r.revision)
    }

    pub fn find(&self, revision: u32) -> Option<&LibraryRecipeRevision> {
        self.revisions.iter().find(|r| r.revision == revision)
    }
}

/// A line in the list of library recipes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecipeSummary {
    pub id: String,
    /// The entry's uid; empty when it could not be read.
    pub uid: String,
    /// The entry's name, or its id when it could not be read.
    pub name: String,
    /// A hash of `entry.json`, sent back as `expected` by a write.
    pub version: String,
    /// The newest revision; 0 when none could be read.
    pub latest: u32,
    pub revisions: Vec<LibraryRecipeRevision>,
    /// The newest revision's base clip, frames and Mixamo flag.
    pub base: String,
    pub frames: u32,
    pub mixamo: bool,
    pub layers: u32,
    pub blanks: u32,
    pub updated_at: String,
    /// Why the entry or its newest revision could not be read.
    pub problem: Option<String>,
}

/// A library recipe's revision as the view and agents read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecipeDoc {
    pub entry: LibraryRecipeEntry,
    /// A hash of `entry.json`, sent back as `expected` by a write.
    pub version: String,
    /// The revision read.
    pub revision: u32,
    pub recipe: RecipeFile,
    /// Its base clip in this computer's clip library.
    pub base: BaseMatch,
    /// A bake of it counts as Mixamo's.
    pub mixamo: bool,
}

/// The recipe library folder of the workspace at `root`.
pub fn dir(root: &Path) -> PathBuf {
    crate::library::dir(root).join(LIBRARY_RECIPES_DIR)
}

/// The folder of entry `id`, which must be a recipe id, so nothing outside
/// the recipe library is ever read or written.
///
/// # Errors
///
/// [`CoreError::NotFound`] for anything that is not a recipe id.
pub fn entry_dir(root: &Path, id: &str) -> CoreResult<PathBuf> {
    if !is_recipe_id(id) {
        return Err(CoreError::NotFound(format!("the library recipe {id}")));
    }
    Ok(dir(root).join(id))
}

/// The file name of revision `n`.
pub fn revision_file(n: u32) -> String {
    format!("r{n}{RECIPE_SUFFIX}")
}

/// Reads `file` whole when it is at most `cap` bytes.
pub(super) fn read_capped(file: &Path, cap: u64) -> CoreResult<String> {
    let handle = match fs::File::open(file) {
        Ok(handle) => handle,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!("{}", file.display())));
        }
        Err(err) => return Err(CoreError::io(file, err)),
    };
    let too_big = || {
        CoreError::Invalid(format!(
            "{} is over {} KB, too big to read.",
            file.file_name().unwrap_or_default().to_string_lossy(),
            cap / 1024
        ))
    };
    let meta = handle.metadata().map_err(|e| CoreError::io(file, e))?;
    if !meta.is_file() {
        return Err(CoreError::Invalid(format!(
            "{} is not a file.",
            file.display()
        )));
    }
    if meta.len() > cap {
        return Err(too_big());
    }
    let mut text = String::new();
    handle
        .take(cap + 1)
        .read_to_string(&mut text)
        .map_err(|e| CoreError::io(file, e))?;
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > cap {
        return Err(too_big());
    }
    Ok(text)
}

fn check_entry(entry: &LibraryRecipeEntry) -> CoreResult<()> {
    if !recipes::is_entry_uid(&entry.uid) {
        return Err(CoreError::Invalid(format!(
            "The library recipe {} has no uid it can be told apart by.",
            entry.id
        )));
    }
    if entry.revisions.len() > MAX_REVISIONS {
        return Err(CoreError::Invalid(format!(
            "The library recipe {} lists more than {MAX_REVISIONS} revisions.",
            entry.id
        )));
    }
    for rev in &entry.revisions {
        if rev.revision == 0 || rev.file != revision_file(rev.revision) {
            return Err(CoreError::Invalid(format!(
                "The library recipe {} names {:?}, which is not a revision file.",
                entry.id, rev.file
            )));
        }
    }
    Ok(())
}

/// Reads entry `id` with its version.
///
/// # Errors
///
/// Fails for an unknown entry, an unreadable or newer file, or one whose
/// revisions name other files.
pub fn load_entry(root: &Path, id: &str) -> CoreResult<(LibraryRecipeEntry, String)> {
    let file = entry_dir(root, id)?.join(ENTRY_FILE);
    let text = match read_capped(&file, MAX_ENTRY_BYTES) {
        Err(CoreError::NotFound(_)) => {
            return Err(CoreError::NotFound(format!("the library recipe {id}")));
        }
        other => other?,
    };
    let mut entry: LibraryRecipeEntry =
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

/// The version of entry `id` as it is on disk.
///
/// # Errors
///
/// As [`load_entry`].
pub fn entry_version(root: &Path, id: &str) -> CoreResult<String> {
    Ok(load_entry(root, id)?.1)
}

/// Refuses a write that read entry `id` at another version.
pub(super) fn check_expected(id: &str, current: &str, expected: Option<&str>) -> CoreResult<()> {
    match expected {
        Some(read) if read != current => Err(CoreError::Moved(format!(
            "The library recipe {id} changed since it was read (its version is now {current}). Read it again."
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
    entry: &LibraryRecipeEntry,
    revision: Option<u32>,
) -> CoreResult<RecipeFile> {
    let rev = match revision {
        Some(n) => entry.find(n),
        None => entry.latest(),
    }
    .ok_or_else(|| {
        CoreError::NotFound(match revision {
            Some(n) => format!("revision r{n} of the library recipe {}", entry.id),
            None => format!("a revision of the library recipe {}", entry.id),
        })
    })?;
    let mut recipe = recipes::load_file(&entry_dir(root, &entry.id)?.join(&rev.file))?;
    if recipe.revision != rev.revision {
        return Err(CoreError::Invalid(format!(
            "{} of the library recipe {} holds revision {}.",
            rev.file, entry.id, recipe.revision
        )));
    }
    recipe.from = None;
    Ok(recipe)
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn summary(root: &Path, id: &str) -> LibraryRecipeSummary {
    let mut line = LibraryRecipeSummary {
        id: id.to_string(),
        uid: String::new(),
        name: id.to_string(),
        version: String::new(),
        latest: 0,
        revisions: Vec::new(),
        base: String::new(),
        frames: 0,
        mixamo: false,
        layers: 0,
        blanks: 0,
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
        Ok(recipe) => {
            line.latest = recipe.revision;
            line.base = recipe.base.name;
            line.frames = recipe.base.frames;
            line.mixamo = recipe.base.mixamo;
            line.layers = count(recipe.layers.len());
            line.blanks = count(recipe.blanks.len());
        }
        Err(err) => line.problem = Some(err.to_string()),
    }
    line.revisions = entry.revisions;
    line
}

/// Every library recipe, by name. An entry that cannot be read still
/// lists, with its problem, so it can be removed.
///
/// # Errors
///
/// Fails only when the folder exists but cannot be read.
pub fn list(root: &Path) -> CoreResult<Vec<LibraryRecipeSummary>> {
    let parent = dir(root);
    let read = match fs::read_dir(&parent) {
        Ok(read) => read,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&parent, err)),
    };
    let mut found: Vec<LibraryRecipeSummary> = read
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

/// Revision `revision` (the newest by default) of entry `id`, its base
/// matched against the clip library.
///
/// # Errors
///
/// Fails for an unknown entry or revision, or an unreadable file.
pub fn get(root: &Path, id: &str, revision: Option<u32>) -> CoreResult<LibraryRecipeDoc> {
    let (entry, version) = load_entry(root, id)?;
    let recipe = read_revision(root, &entry, revision)?;
    let clips = crate::animations::list(root)?;
    let (base, clip) = match_base(&clips, &recipe.base);
    let mixamo = recipe.base.mixamo
        || clip.is_some_and(|c| c.rig.kind == crate::animations::SourceRig::Mixamo);
    Ok(LibraryRecipeDoc {
        revision: recipe.revision,
        entry,
        version,
        recipe,
        base,
        mixamo,
    })
}

/// A free entry id for `seed` (a name or a file stem): its slug, then
/// `-2`, `-3` and on.
pub(super) fn free_entry_id(root: &Path, seed: &str) -> CoreResult<String> {
    let base = latin_slug(seed).map_or_else(
        || "recipe".to_string(),
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
        "There are too many library recipes called {seed}."
    )))
}

/// Writes revision `recipe` (its `revision` set) into entry `entry`'s
/// folder and the entry after it, the entry last so a reader never lists a
/// revision whose file is not there yet.
pub(super) fn write_revision(
    root: &Path,
    entry: &LibraryRecipeEntry,
    recipe: &RecipeFile,
) -> CoreResult<()> {
    check_entry(entry)?;
    let folder = entry_dir(root, &entry.id)?;
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    let mut stored = recipes::normalized(recipe);
    stored.from = None;
    recipes::validate(&stored)?;
    let file = folder.join(revision_file(stored.revision));
    if file.exists() {
        return Err(CoreError::Invalid(format!(
            "Revision r{} of the library recipe {} is there already.",
            stored.revision, entry.id
        )));
    }
    write_json(&file, &stored)?;
    write_json(&folder.join(ENTRY_FILE), entry)
}

/// One recipe library write at a time in this process.
static WRITES: Mutex<()> = Mutex::new(());

pub(super) fn turn() -> MutexGuard<'static, ()> {
    WRITES.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
