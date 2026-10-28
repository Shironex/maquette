//! The animation library: clips kept once, outside any project, next to the
//! model library. Each clip is skeleton only, the armature and its keys with
//! no mesh, in one folder:
//!
//! ```text
//! library/animations/
//!   standing-dodge-left/
//!     clip.json     name, pack, tags, source rig, frames, rate, where it came from
//!     clip.glb      the bones and the keys
//!     source.fbx    the file it came from, only when the import kept it
//! ```
//!
//! Clips come from a clip pack (a zip of FBX files, one clip each) or single
//! FBX files ([`import`]), from a rigged model already in a project, or from
//! a Tripo job's animations in the Inbox ([`save`]). Headless Blender reads
//! the files and drops the skin; the core decides what goes in. The library
//! lives on this machine only: Ship and Export never copy a raw clip, a clip
//! leaves it only baked into a model.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::workspace::is_single_segment;

mod blender;
pub mod import;
pub mod names;
pub mod rig;
pub mod save;
mod view;

pub use blender::{LANE, READ_SCRIPT};
pub use import::{ImportPlan, ImportRequest, Imported};
pub use rig::SourceRig;
pub use save::{ClipsFrom, SaveClips, SavePlan, Saved};
pub use view::{AnimationItem, AnimationsView, view};

/// The animation library's folder, under the model library's.
pub const ANIMATIONS_DIR: &str = "animations";
pub const CLIP_FILE: &str = "clip.json";
pub const CLIP_VERSION: u32 = 1;
/// The skeleton and keys of a clip, in its folder.
pub const CLIP_GLB: &str = "clip.glb";
/// The pack single files go to unless the import names another.
pub const SINGLES: &str = "Singles";

/// One bone of a clip's skeleton at rest, in metres with Y up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RestBone {
    pub name: String,
    pub parent: Option<String>,
    pub head: [f32; 3],
    pub tail: [f32; 3],
}

/// The skeleton a clip was made on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipRig {
    pub kind: SourceRig,
    /// Every bone with its rest pose, parents first as the file had them.
    pub bones: Vec<RestBone>,
    /// The hips' height at rest, metres, which root motion scales by.
    pub hips_height: f32,
}

/// How far and which way the root moves over a clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RootMotion {
    pub metres: f32,
    /// `forward`, `back`, `left` or `right`, as the character faces.
    pub direction: String,
}

/// Where a clip came into the library from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ClipSource {
    /// An FBX file or a pack of them.
    Import,
    /// A rigged model in a project.
    Model,
    /// A Tripo job's animations from the Inbox.
    Tripo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipOrigin {
    pub source: ClipSource,
    /// The file it was read from: `Standing Dodge Left.fbx`, or the model's.
    pub file: String,
    /// The zip it came in, for a pack.
    pub archive: Option<String>,
    /// The original file kept next to the clip, relative to its folder.
    pub kept: Option<String>,
    /// The project and asset version a model's clip came from.
    pub project: Option<String>,
    pub asset: Option<String>,
    pub version: Option<u32>,
}

/// `clip.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AnimationClip {
    pub version: u32,
    /// The folder name.
    pub id: String,
    pub name: String,
    pub pack: String,
    pub tags: Vec<String>,
    pub rig: ClipRig,
    pub frames: u32,
    pub fps: f32,
    /// Seconds.
    pub length: f32,
    pub root_motion: Option<RootMotion>,
    pub loops: bool,
    /// What the stored clip weighs.
    pub bytes: f64,
    /// The skeleton and keys, relative to the folder.
    pub file: String,
    pub origin: ClipOrigin,
    /// A hash of the keys, so the same clip is never added twice.
    pub hash: String,
    /// Goes up when an import replaces the keys; models that baked an older
    /// revision are offered an update.
    pub revision: u32,
    pub created_at: String,
    pub updated_at: String,
}

/// The animation library folder of the workspace at `root`.
pub fn dir(root: &Path) -> PathBuf {
    crate::library::dir(root).join(ANIMATIONS_DIR)
}

/// The folder of clip `id`. Fails for anything that is not a plain id, so
/// nothing outside the library is ever read or written.
pub fn clip_dir(root: &Path, id: &str) -> CoreResult<PathBuf> {
    if !is_single_segment(id) || id.starts_with('.') {
        return Err(CoreError::Invalid(format!(
            "{id:?} is not a library clip id."
        )));
    }
    Ok(dir(root).join(id))
}

fn check_paths(clip: &AnimationClip) -> CoreResult<()> {
    for file in std::iter::once(&clip.file).chain(clip.origin.kept.as_ref()) {
        if !crate::game::stays_inside(file) {
            return Err(CoreError::Invalid(format!(
                "The library clip {} names {file:?}, which is outside its folder.",
                clip.id
            )));
        }
    }
    Ok(())
}

/// Reads one clip.
///
/// # Errors
///
/// Fails for an unknown clip, an unreadable or newer file, or one that names
/// files outside its folder.
pub fn load(root: &Path, id: &str) -> CoreResult<AnimationClip> {
    let file = clip_dir(root, id)?.join(CLIP_FILE);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!("the library clip {id}")));
        }
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let mut clip: AnimationClip =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if clip.version > CLIP_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: clip.version,
            supported: CLIP_VERSION,
        });
    }
    clip.id = id.to_string();
    check_paths(&clip)?;
    Ok(clip)
}

/// Every readable clip, by pack then name. Folders without a readable clip
/// are skipped with a warning.
pub fn list(root: &Path) -> CoreResult<Vec<AnimationClip>> {
    let parent = dir(root);
    let read = match fs::read_dir(&parent) {
        Ok(read) => read,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&parent, err)),
    };
    let mut clips = Vec::new();
    for found in read.filter_map(Result::ok) {
        let id = found.file_name().to_string_lossy().into_owned();
        if crate::workspace::sync::is_sync_name(&id) || !found.path().join(CLIP_FILE).is_file() {
            continue;
        }
        match load(root, &id) {
            Ok(clip) => clips.push(clip),
            Err(err) => tracing::warn!(clip = %id, error = %err, "skipping unreadable clip"),
        }
    }
    clips.sort_by(|a, b| {
        (a.pack.to_lowercase(), a.name.to_lowercase(), &a.id).cmp(&(
            b.pack.to_lowercase(),
            b.name.to_lowercase(),
            &b.id,
        ))
    });
    Ok(clips)
}

fn store(root: &Path, clip: &AnimationClip) -> CoreResult<()> {
    check_paths(clip)?;
    let folder = clip_dir(root, &clip.id)?;
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    write_json(&folder.join(CLIP_FILE), clip)
}

/// A folder name for a clip called `name` that no clip has yet.
fn free_id(root: &Path, taken: &[String], name: &str) -> String {
    let base = match crate::project::slugify(name) {
        s if s.is_empty() => "clip".to_string(),
        s => s,
    };
    let used = |id: &str| taken.iter().any(|t| t == id) || dir(root).join(id).exists();
    if !used(&base) {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !used(id))
        .unwrap_or(base)
}

/// One library write at a time in this process.
static WRITES: Mutex<()> = Mutex::new(());

fn turn() -> std::sync::MutexGuard<'static, ()> {
    WRITES.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_blender;
