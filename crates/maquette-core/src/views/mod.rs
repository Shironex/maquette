//! Standard views: one model (or two versions of it) rendered headless from
//! a fixed camera set, in the page light, so a rebuild can be judged against
//! its concept ref without anyone writing a render script.
//!
//! ```text
//! <project>/exports/renders/<asset>/views/v3/three-quarter.png, front.png,
//!     top.png, close-up-1.png, close-up-2.png, sheet.png, views.json
//! <project>/exports/renders/<asset>/views/v3-vs-v2/v3_front.png, v2_front.png, ...
//! <project>/exports/renders/<asset>/views/v3-1a2b3c4d/...   (another angle or close ups)
//! ```
//!
//! [`cameras`] holds the set, [`render_views.py`] makes the pictures with
//! the thumbnail's stage and the preview rig's light ([`crate::thumbs`]),
//! and [`Views`] runs it on a runner lane of its own: a set that is already
//! there answers at once (`views.json` keeps the key of what made it), a
//! second ask for a set on its way joins it, and a run can be cancelled.
//! The pictures of a run land in the folder together, when all are done.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub mod cameras;
mod job;
mod spec;

pub use cameras::{StandardView, ViewAngle, ViewTarget, standard_set};
pub use job::Views;
pub use spec::file_name;

/// The script, written beside the thumbnail script and the rig before a run.
pub const SCRIPT: &str = include_str!("render_views.py");
pub const SCRIPT_FILE: &str = "render_views.py";
/// The runner lane every set shares, one at a time.
pub const LANE: &str = "views";
/// The edge of every picture an agent gets, in pixels.
pub const SIZE: u32 = 1024;
/// The sizes a request may ask for.
pub const SIZES: std::ops::RangeInclusive<u32> = 64..=4096;
/// Cycles samples per picture; the denoiser does the rest.
pub const SAMPLES: u32 = 32;
/// The edge of a picture on the sheet.
pub const SHEET_TILE: u32 = 512;
pub const SHEET_FILE: &str = "sheet.png";
/// The record of a finished set, beside its pictures.
pub const RECORD_FILE: &str = "views.json";
pub const RECORD_VERSION: u32 = 1;
/// The folder under an asset's renders the sets go in.
pub const VIEWS_DIR: &str = "views";

/// One model of a set: an asset version as Blender reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewModel {
    /// `v3`: its row in the answer and, side by side, the start of its
    /// pictures' names.
    pub id: String,
    pub version: u32,
    /// The file Blender reads: the prepared copy, or the model itself.
    pub file: PathBuf,
}

/// Everything a set is made from.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewsRequest {
    /// Where the pictures go.
    pub dir: PathBuf,
    /// Scratch space for the run, under the workspace cache.
    pub work: PathBuf,
    /// One model, or two for a side by side: they share every camera.
    pub models: Vec<ViewModel>,
    pub views: Vec<StandardView>,
    /// The asset's ref images, which lead the sheet.
    pub refs: Vec<PathBuf>,
    /// The edge of each picture in pixels: [`SIZE`] for every set an agent
    /// asks for.
    pub size: u32,
}

/// One picture and the camera that took it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewShot {
    pub id: String,
    /// An absolute path.
    pub file: String,
    pub azimuth: f32,
    pub elevation: f32,
    /// What it frames: "the whole model", "the top half", "the part mug".
    pub frames: String,
    /// The camera in glTF terms (Y up, metres), at the size it was rendered.
    pub position: [f32; 3],
    pub target: [f32; 3],
    /// Its field of view in degrees.
    pub fov: f32,
}

/// One model of a finished set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewedModel {
    pub id: String,
    pub version: u32,
    /// The file that was rendered, an absolute path.
    pub file: String,
    /// Its bounds as rendered, glTF axes, metres.
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub triangles: u64,
    pub shots: Vec<ViewShot>,
}

/// The numbers of the light the pictures were made in, as the rig holds them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageLight {
    /// The sun's strength, as Build scene sets it.
    pub sun: f32,
    /// The grey world's even fill.
    pub fill: f32,
    /// The tone mapping, three.js' name for it in kebab case.
    pub tone: String,
    pub exposure: f32,
}

/// A finished set: what `views.json` keeps and a request answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewSet {
    pub version: u32,
    /// What made it (see `spec::key_of`): the same key is the same set.
    pub key: String,
    pub dir: String,
    /// The edge of each picture in pixels.
    pub size: u32,
    /// The model asked for first, then the one it is compared with.
    pub models: Vec<ViewedModel>,
    /// Every picture on one PNG: a row per model, the refs leading the first.
    pub sheet: String,
    pub light: PageLight,
    /// How long Blender took.
    pub seconds: f64,
    pub rendered_at: String,
}

/// What asking for a set led to.
#[derive(Debug, Clone, PartialEq)]
pub enum Asked {
    Ready(Box<ViewSet>),
    /// Blender is on it; ask again to read it.
    Rendering,
}

/// The folder of a set under the project: `exports/renders/<asset>/views/`
/// then `v3`, or `v3-vs-v2` side by side, with `-as-is` when the models are
/// rendered as their files are, and `-<8 hex>` when its cameras are not the
/// standard set (another angle or other close ups): each set of cameras
/// keeps its own pictures and its own render.
pub fn set_dir(
    project_dir: &Path,
    asset: &str,
    version: u32,
    compare: Option<u32>,
    as_is: bool,
    views: &[StandardView],
) -> PathBuf {
    let mut name = match compare {
        Some(other) => format!("v{version}-vs-v{other}"),
        None => format!("v{version}"),
    };
    if as_is {
        name.push_str("-as-is");
    }
    if let Some(tag) = cameras_tag(views) {
        name.push('-');
        name.push_str(&tag);
    }
    project_dir
        .join(crate::packs::EXPORTS_DIR)
        .join("renders")
        .join(asset)
        .join(VIEWS_DIR)
        .join(name)
}

/// Eight hex digits of the cameras, or `None` for the standard set.
fn cameras_tag(views: &[StandardView]) -> Option<String> {
    use sha2::{Digest, Sha256};
    if standard_set(None, &[]).is_ok_and(|standard| standard == views) {
        return None;
    }
    let json = serde_json::to_string(views).unwrap_or_default();
    Some(hex::encode(Sha256::digest(json.as_bytes()))[..8].to_string())
}

/// The set a folder holds, when one finished there.
pub fn stored(dir: &Path) -> Option<ViewSet> {
    let text = std::fs::read_to_string(dir.join(RECORD_FILE)).ok()?;
    let set: ViewSet = serde_json::from_str(&text).ok()?;
    (set.version == RECORD_VERSION).then_some(set)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_blender;
