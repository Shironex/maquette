//! What each project took from the animation library, in its own
//! `animations.json`: per asset, the bone map saved for its rig, the clips
//! baked into it with the revision and settings they were baked with, and
//! the last retarget review. From it the library says where a clip is
//! used, and offers models an update when a clip they baked gets a newer
//! revision.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use super::bones::MapChoice;
use super::rest::FollowChoice;
use super::review::RetargetReview;
use super::settings::ClipSetting;
use crate::animations::AnimationClip;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project;
use crate::workspace::FolderLayout;

pub const USES_FILE: &str = "animations.json";
pub const USES_VERSION: u32 = 1;

/// A library clip baked into a model version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BakedClip {
    pub setting: ClipSetting,
    /// The clip's revision when it was baked.
    pub revision: u32,
    /// The library clip's name then.
    pub library_name: String,
    /// The asset version it went into.
    pub version: u32,
    /// The library clip's whole length, frames; 0 for a bake recorded
    /// before this was kept.
    #[serde(default)]
    pub library_frames: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AssetAnimations {
    /// The bone map saved for this model's rig.
    pub map: Vec<MapChoice>,
    /// What the bones only on this model follow, saved with the map.
    pub follow: Vec<FollowChoice>,
    /// Every library clip baked in, newest bake last; a clip baked again
    /// replaces its older entry.
    pub baked: Vec<BakedClip>,
    /// The last retarget review and the version it made.
    pub review: Option<RetargetReview>,
    pub review_version: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectAnimations {
    pub version: u32,
    pub assets: BTreeMap<String, AssetAnimations>,
}

fn path(project_dir: &Path) -> PathBuf {
    project_dir.join(USES_FILE)
}

/// Reads `animations.json`, or nothing taken when there is none.
///
/// # Errors
///
/// Fails when the file cannot be read or comes from a newer Maquette.
pub fn load(project_dir: &Path) -> CoreResult<ProjectAnimations> {
    let file = path(project_dir);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(ProjectAnimations::default());
        }
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let found: ProjectAnimations =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if found.version > USES_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: found.version,
            supported: USES_VERSION,
        });
    }
    Ok(found)
}

static WRITES: Mutex<()> = Mutex::new(());

/// Reads the record, applies `change` to the asset's entry and writes it.
///
/// # Errors
///
/// As [`load`], or when the file cannot be written.
pub fn update<T>(
    project_dir: &Path,
    asset: &str,
    change: impl FnOnce(&mut AssetAnimations) -> T,
) -> CoreResult<T> {
    let _turn = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let mut file = load(project_dir)?;
    let out = change(file.assets.entry(asset.to_string()).or_default());
    file.version = USES_VERSION;
    write_json(&path(project_dir), &file)?;
    Ok(out)
}

/// The asset's entry, empty when it took nothing.
///
/// # Errors
///
/// As [`load`].
pub fn of_asset(project_dir: &Path, asset: &str) -> CoreResult<AssetAnimations> {
    Ok(load(project_dir)?.assets.remove(asset).unwrap_or_default())
}

/// Records a bake: the clips replace older bakes of the same clip, and the
/// review is kept with the version it made.
pub(crate) fn record_bake(
    entry: &mut AssetAnimations,
    baked: Vec<BakedClip>,
    review: RetargetReview,
    version: u32,
) {
    entry
        .baked
        .retain(|old| !baked.iter().any(|b| b.setting.clip == old.setting.clip));
    entry.baked.extend(baked);
    entry.review = Some(review);
    entry.review_version = Some(version);
}

/// One model that baked a library clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipUse {
    pub project: String,
    pub project_name: String,
    pub asset: String,
    pub asset_name: String,
    /// The clip's name in the model.
    pub name: String,
    pub version: u32,
    pub revision: u32,
    /// The library has a newer revision than the one baked.
    pub outdated: bool,
}

/// Every use of every library clip across the workspace's projects, by
/// clip id. Projects whose record cannot be read are skipped.
///
/// # Errors
///
/// Fails when the projects cannot be listed.
pub fn uses(
    root: &Path,
    layout: &FolderLayout,
    clips: &[AnimationClip],
) -> CoreResult<BTreeMap<String, Vec<ClipUse>>> {
    let mut out: BTreeMap<String, Vec<ClipUse>> = BTreeMap::new();
    for manifest in project::list(root, layout)? {
        let Ok(dir) = project::dir(root, layout, &manifest.id) else {
            continue;
        };
        let record = match load(&dir) {
            Ok(record) => record,
            Err(err) => {
                tracing::warn!(project = %manifest.id, error = %err, "skipping its animations");
                continue;
            }
        };
        for (asset_id, entry) in record.assets {
            let Some(asset) = manifest.assets.iter().find(|a| a.id == asset_id) else {
                continue;
            };
            for baked in entry.baked {
                let revision = clips
                    .iter()
                    .find(|c| c.id == baked.setting.clip)
                    .map(|c| c.revision);
                out.entry(baked.setting.clip.clone())
                    .or_default()
                    .push(ClipUse {
                        project: manifest.id.clone(),
                        project_name: manifest.name.clone(),
                        asset: asset.id.clone(),
                        asset_name: asset.name.clone(),
                        name: baked.setting.name,
                        version: baked.version,
                        revision: baked.revision,
                        outdated: revision.is_some_and(|r| r > baked.revision),
                    });
            }
        }
    }
    Ok(out)
}

/// The clips a model baked that the library has fixed since, as the
/// settings to bake them again with.
pub fn outdated(entry: &AssetAnimations, clips: &[AnimationClip]) -> Vec<ClipSetting> {
    entry
        .baked
        .iter()
        .filter(|b| {
            clips
                .iter()
                .any(|c| c.id == b.setting.clip && c.revision > b.revision)
        })
        .map(|b| b.setting.clone())
        .collect()
}
