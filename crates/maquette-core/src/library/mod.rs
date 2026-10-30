//! The library: models kept outside any project so other projects can use
//! them. It lives in `library/` at the workspace root, one folder per entry:
//!
//! ```text
//! library/
//!   desk-lamp/
//!     entry.json            name, kind, tags, refs and every version
//!     models/desk_lamp_v1.glb
//!     thumbs/desk_lamp_v1.png
//!     refs/desk_lamp.png
//! ```
//!
//! The library copies, it never links. "Use in project" copies an entry's
//! version into a project as a new asset that records its origin (the entry
//! id and the library version, [`crate::assets::AssetOrigin`]); the project
//! owns that copy. Adding a newer model of the same asset adds a version to
//! its entry, and the projects that came from it show "update available"
//! until someone updates them, which brings the new model in as the asset's
//! next version. Nothing changes a project on its own. Which projects use
//! which version is read from the projects' origins, so it is never stale.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::assets::{AssetKind, ModelStats};
use crate::error::{CoreError, CoreResult};
use crate::game::stays_inside;
use crate::json::write_json;
use crate::materials::ModelMaterials;
use crate::prepare::ModelPrep;
use crate::workspace::is_single_segment;

mod add;
mod collections;
mod files;
mod look_alike;
mod merge;
mod merge_undo;
mod names;
mod plan_many;
mod remove;
mod source;
mod tags;
mod update;
mod use_in;
mod view;

pub use add::{AddToLibrary, LibraryAdded, add};
pub use collections::{CollectionChange, LibraryCollection, change_collection, collections};
pub use look_alike::{
    LookAlikeKind, LookAlikePair, LookAlikeSettings, LookAlikeVerdict, LookAlikes, decide,
    look_alikes, save_settings, settings as look_alike_settings,
};
pub use merge::{LibraryMerge, LibraryMerged, MergeEntries, merge};
pub use plan_many::{AddManyPlan, PlannedAdd, PlannedState, plan_add_many};
pub use remove::{LibraryRestored, REMOVED_DIR, remove, restore};
pub use source::detect as detect_source;
pub use tags::{TagEntries, rename, tag};
pub use update::{LibraryUpdated, SettingChange, UpdatePlan, plan_update, update};
pub use use_in::{LibraryUsed, use_in_project};
pub use view::{LibraryItem, LibraryUse, LibraryView, view};

/// The library's folder, under the workspace root.
pub const LIBRARY_DIR: &str = "library";
pub const ENTRY_FILE: &str = "entry.json";
pub const ENTRY_VERSION: u32 = 1;
pub const MAX_TAGS: usize = 16;
pub const TAG_MAX_CHARS: usize = 32;

/// What made a library model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum LibrarySource {
    Tripo,
    Meshy,
    /// Made on a provider's site and added by hand.
    ByHand,
    /// A Blender script in the project made it.
    Built,
    /// A file brought in: a game repo's model or one added from disk.
    Imported,
}

/// The project asset version a library version was copied from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VersionFrom {
    pub project: String,
    pub asset: String,
    pub version: u32,
}

/// One model of an entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryVersion {
    pub version: u32,
    /// Relative to the entry folder, with `/` separators.
    pub file: String,
    pub created_at: String,
    pub stats: Option<ModelStats>,
    pub source: LibrarySource,
    /// The credits the send that made it cost, when the app knows.
    pub credits: Option<f64>,
    /// The model file's SHA-256, so the same model is never added twice.
    pub hash: String,
    pub from: VersionFrom,
    /// Its picture, relative to the entry folder.
    pub thumbnail: Option<String>,
    /// The real size and fix-ups it had where it came from.
    pub prep: Option<ModelPrep>,
    /// The material tweaks it had where it came from.
    pub materials: Option<ModelMaterials>,
}

/// `entry.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub version: u32,
    /// The folder name.
    pub id: String,
    pub name: String,
    pub kind: AssetKind,
    pub tags: Vec<String>,
    /// Ref images, relative to the entry folder.
    pub refs: Vec<String>,
    /// Oldest first.
    pub versions: Vec<LibraryVersion>,
    pub created_at: String,
    pub updated_at: String,
}

impl LibraryEntry {
    pub fn latest(&self) -> Option<&LibraryVersion> {
        self.versions.last()
    }

    pub fn find(&self, version: u32) -> Option<&LibraryVersion> {
        self.versions.iter().find(|v| v.version == version)
    }

    pub fn next_version(&self) -> u32 {
        self.versions.iter().map(|v| v.version).max().unwrap_or(0) + 1
    }
}

/// The library folder of the workspace at `root`.
pub fn dir(root: &Path) -> PathBuf {
    root.join(LIBRARY_DIR)
}

/// The folder of entry `id`. Fails for anything that is not a plain id, so
/// nothing outside the library is ever read or written.
pub fn entry_dir(root: &Path, id: &str) -> CoreResult<PathBuf> {
    if !is_single_segment(id) || id.starts_with('.') {
        return Err(CoreError::Invalid(format!(
            "{id:?} is not a library entry id."
        )));
    }
    Ok(dir(root).join(id))
}

/// Checks that every file an entry names stays inside its folder.
///
/// # Errors
///
/// Names the first path that would leave it.
pub fn check_paths(entry: &LibraryEntry) -> CoreResult<()> {
    let files = entry
        .refs
        .iter()
        .chain(entry.versions.iter().map(|v| &v.file))
        .chain(entry.versions.iter().filter_map(|v| v.thumbnail.as_ref()));
    for file in files {
        if !stays_inside(file) {
            return Err(CoreError::Invalid(format!(
                "The library entry {} names {file:?}, which is outside its folder.",
                entry.id
            )));
        }
    }
    Ok(())
}

/// Reads one entry.
///
/// # Errors
///
/// Fails for an unknown entry, an unreadable or newer file, or one that
/// names files outside its folder.
pub fn load(root: &Path, id: &str) -> CoreResult<LibraryEntry> {
    let file = entry_dir(root, id)?.join(ENTRY_FILE);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!("the library entry {id}")));
        }
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let mut entry: LibraryEntry =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if entry.version > ENTRY_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: entry.version,
            supported: ENTRY_VERSION,
        });
    }
    entry.id = id.to_string();
    check_paths(&entry)?;
    Ok(entry)
}

/// Every readable entry, by name. Folders without a readable entry are
/// skipped with a warning.
pub fn list(root: &Path) -> CoreResult<Vec<LibraryEntry>> {
    let parent = dir(root);
    let read = match fs::read_dir(&parent) {
        Ok(read) => read,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&parent, err)),
    };
    let mut entries = Vec::new();
    for found in read.filter_map(Result::ok) {
        let id = found.file_name().to_string_lossy().into_owned();
        if crate::workspace::sync::is_sync_name(&id) || !found.path().join(ENTRY_FILE).is_file() {
            continue;
        }
        match load(root, &id) {
            Ok(entry) => entries.push(entry),
            Err(err) => {
                tracing::warn!(entry = %id, error = %err, "skipping unreadable library entry")
            }
        }
    }
    entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(entries)
}

fn save(root: &Path, entry: &LibraryEntry) -> CoreResult<()> {
    check_paths(entry)?;
    let folder = entry_dir(root, &entry.id)?;
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    write_json(&folder.join(ENTRY_FILE), entry)
}

/// One library write at a time in this process.
static WRITES: Mutex<()> = Mutex::new(());

fn turn() -> std::sync::MutexGuard<'static, ()> {
    WRITES.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Lowercase tags, trimmed, each once, in the order given.
///
/// # Errors
///
/// Refuses too many tags or one that is too long.
pub fn clean_tags(tags: &[String]) -> CoreResult<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag = tag.trim().to_lowercase();
        if tag.is_empty() || out.contains(&tag) {
            continue;
        }
        if tag.chars().count() > TAG_MAX_CHARS {
            return Err(CoreError::Invalid(format!(
                "Keep each tag to {TAG_MAX_CHARS} characters or fewer."
            )));
        }
        out.push(tag);
    }
    if out.len() > MAX_TAGS {
        return Err(CoreError::Invalid(format!(
            "Give a model {MAX_TAGS} tags or fewer."
        )));
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_batch;
#[cfg(test)]
mod tests_merge;
#[cfg(test)]
mod tests_plan;

/// Registers this module's IPC types (library entries, their uses, updates, tags,
/// collections, look-alikes, merges and batch adds) with the shell's
/// TypeScript export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<LibrarySource>()
        .register::<VersionFrom>()
        .register::<LibraryVersion>()
        .register::<LibraryEntry>()
        .register::<LibraryUse>()
        .register::<LibraryItem>()
        .register::<LibraryView>()
        .register::<AddToLibrary>()
        .register::<LibraryAdded>()
        .register::<LibraryUsed>()
        .register::<SettingChange>()
        .register::<UpdatePlan>()
        .register::<LibraryUpdated>()
        .register::<TagEntries>()
        .register::<LibraryCollection>()
        .register::<CollectionChange>()
        .register::<LibraryRestored>()
        .register::<LookAlikeSettings>()
        .register::<LookAlikeKind>()
        .register::<LookAlikeVerdict>()
        .register::<LookAlikePair>()
        .register::<LibraryMerge>()
        .register::<LookAlikes>()
        .register::<MergeEntries>()
        .register::<LibraryMerged>()
        .register::<PlannedState>()
        .register::<PlannedAdd>()
        .register::<AddManyPlan>()
}
