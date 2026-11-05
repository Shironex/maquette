//! What a recipe bake leaves behind so it is never taken for a model's own
//! clips. A staged bake carries a mark in its own GLB (`asset.extras`):
//! the recipe and version it was baked from, the new clip and whether it
//! counts as Mixamo's. Any Blender import and export drops that mark, so
//! bringing a marked file in as a version (inbox accept, a build, a library
//! entry, a game import) also records the bake on the asset in the
//! project's `recipe-bakes.json`, which every later version keeps.
//! Packs and Save clips to the library read both.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use maquette_glb::extras;

/// The key under the GLB's `asset.extras`.
pub const MARK_KEY: &str = "maquetteRecipeBake";
/// The project file the accepted bakes are recorded in.
pub const BAKES_FILE: &str = "recipe-bakes.json";
pub const BAKES_VERSION: u32 = 1;

/// The mark in a staged bake's GLB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBakeMark {
    pub recipe: String,
    pub recipe_version: String,
    /// The library clip the bake started from.
    pub base_clip: String,
    /// The clip the bake added.
    pub clip: String,
    pub mixamo: bool,
}

/// Writes `mark` into the GLB at `file`.
///
/// # Errors
///
/// Fails when the file cannot be read or written, or is no GLB.
pub fn write_mark(file: &Path, mark: &RecipeBakeMark) -> CoreResult<()> {
    let bytes = fs::read(file).map_err(|e| CoreError::io(file, e))?;
    let value = serde_json::to_value(mark).map_err(|e| CoreError::json(file, e))?;
    let marked = extras::with_asset_extra(&bytes, MARK_KEY, value)?;
    fs::write(file, marked).map_err(|e| CoreError::io(file, e))
}

/// The mark of the model at `file`; none for a model no recipe baked, or
/// one whose mark a Blender step dropped.
pub fn mark_of(file: &Path) -> Option<RecipeBakeMark> {
    serde_json::from_value(extras::asset_extra(file, MARK_KEY)?).ok()
}

/// Whether the model at `file` still carries a mark of a Mixamo bake.
pub fn marked_mixamo(file: &Path) -> bool {
    mark_of(file).is_some_and(|mark| mark.mixamo)
}

/// A recipe bake accepted as a version of an asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBakeUse {
    pub recipe: String,
    pub recipe_version: String,
    pub base_clip: String,
    /// The clip the bake added, by its name in the model.
    pub clip: String,
    pub mixamo: bool,
    /// The asset version it came in as.
    pub version: u32,
}

impl RecipeBakeUse {
    pub fn of(mark: &RecipeBakeMark, version: u32) -> Self {
        Self {
            recipe: mark.recipe.clone(),
            recipe_version: mark.recipe_version.clone(),
            base_clip: mark.base_clip.clone(),
            clip: mark.clip.clone(),
            mixamo: mark.mixamo,
            version,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct BakesFile {
    version: u32,
    /// Asset id to its accepted bakes, oldest first.
    assets: BTreeMap<String, Vec<RecipeBakeUse>>,
}

static WRITES: Mutex<()> = Mutex::new(());

fn load(project_dir: &Path) -> CoreResult<BakesFile> {
    let file = project_dir.join(BAKES_FILE);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(BakesFile::default()),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let found: BakesFile = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if found.version > BAKES_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: found.version,
            supported: BAKES_VERSION,
        });
    }
    Ok(found)
}

/// Records `bake` on `asset`.
///
/// # Errors
///
/// Fails when the record cannot be read or written.
pub fn record(project_dir: &Path, asset: &str, bake: RecipeBakeUse) -> CoreResult<()> {
    let _turn = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let mut file = load(project_dir)?;
    file.version = BAKES_VERSION;
    file.assets.entry(asset.to_string()).or_default().push(bake);
    write_json(&project_dir.join(BAKES_FILE), &file)
}

/// Records every bake in `bakes` on `asset`, oldest first, after the ones
/// it has. Nothing to record writes nothing.
///
/// # Errors
///
/// Fails when the record cannot be read or written.
pub fn record_all(project_dir: &Path, asset: &str, bakes: Vec<RecipeBakeUse>) -> CoreResult<()> {
    if bakes.is_empty() {
        return Ok(());
    }
    let _turn = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let mut file = load(project_dir)?;
    file.version = BAKES_VERSION;
    file.assets
        .entry(asset.to_string())
        .or_default()
        .extend(bakes);
    write_json(&project_dir.join(BAKES_FILE), &file)
}

/// Records the bake the model at `file` carries, when it carries one, as
/// `version` of `asset`. Every route that brings a model file in as a
/// version calls this, so the bake outlives a later step that drops the
/// mark.
///
/// # Errors
///
/// Fails when the record cannot be read or written.
pub fn note(project_dir: &Path, asset: &str, file: &Path, version: u32) -> CoreResult<()> {
    match mark_of(file) {
        Some(mark) => record(project_dir, asset, RecipeBakeUse::of(&mark, version)),
        None => Ok(()),
    }
}

/// Whether the project's record can be read: a missing one can.
///
/// # Errors
///
/// Fails when it is broken or comes from a newer Maquette.
pub fn check(project_dir: &Path) -> CoreResult<()> {
    load(project_dir).map(|_| ())
}

/// The bakes accepted as versions of `asset`, oldest first.
///
/// # Errors
///
/// Fails when the record cannot be read.
pub fn of_asset(project_dir: &Path, asset: &str) -> CoreResult<Vec<RecipeBakeUse>> {
    Ok(load(project_dir)?.assets.remove(asset).unwrap_or_default())
}

/// Whether a bake of a Mixamo clip came into `asset`. A record that cannot
/// be read counts as one, so a doubt leaves the clips out.
pub fn took_mixamo(project_dir: &Path, asset: &str) -> bool {
    of_asset(project_dir, asset).map_or(true, |bakes| bakes.iter().any(|b| b.mixamo))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bake(mixamo: bool) -> RecipeBakeUse {
        RecipeBakeUse {
            recipe: "wave".into(),
            recipe_version: "v1".into(),
            base_clip: "idle".into(),
            clip: "wave".into(),
            mixamo,
            version: 2,
        }
    }

    #[test]
    fn an_accepted_bake_is_kept_per_asset_and_a_doubt_counts_as_mixamo() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(!took_mixamo(dir.path(), "elsheria"), "nothing taken yet");
        record(dir.path(), "elsheria", bake(false)).expect("record");
        assert!(!took_mixamo(dir.path(), "elsheria"));
        record(dir.path(), "elsheria", bake(true)).expect("record");
        assert!(took_mixamo(dir.path(), "elsheria"));
        assert!(!took_mixamo(dir.path(), "lantern"));
        assert_eq!(of_asset(dir.path(), "elsheria").expect("read").len(), 2);
        fs::write(dir.path().join(BAKES_FILE), "{ broken").expect("broken");
        assert!(
            took_mixamo(dir.path(), "lantern"),
            "unreadable leaves clips out"
        );
    }
}
