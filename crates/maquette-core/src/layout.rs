//! The scene layout: where each model of a project stands, the camera stops
//! the site tours and an optional desk the models sit on. It lives in
//! `layout.json` next to `project.json`, keyed by asset id, so renaming an
//! asset keeps its place. Everything is in glTF coordinates (Y up, metres),
//! like the Inspector's stops.
//!
//! ```json
//! {
//!   "version": 1,
//!   "models": {
//!     "desk-lamp": { "position": [0.42, 0, -0.1], "rotation": [0, -35, 0], "scale": 0.4 }
//!   },
//!   "stops": [
//!     { "id": "concept", "name": "Concept", "position": [0.9, 0.5, 1.4],
//!       "target": [0, 0.15, 0], "fov": 35, "models": null }
//!   ],
//!   "desk": { "size": [1.6, 0.8] }
//! }
//! ```
//!
//! A placement moves the asset's newest model: `rotation` is in degrees,
//! applied X, then Y, then Z (three.js' default order), and `scale` is
//! uniform. The Build scene starter applies it in Blender; models without a
//! placement stand side by side as before ([`fallback_row`]). Each stop
//! becomes a camera there, which Capture posters renders. A stop's
//! `models` lists what it sees by hand; `null` means whatever the camera's
//! view holds ([`first_view`]).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::assets::{self, Asset, AssetKind};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::pipeline::PipelineDef;
use crate::project::{ProjectManifest, slugify};

mod patch;
mod placement;
mod view;

pub use view::{
    FirstView, FirstViewModel, LayoutModel, LayoutView, Measured, first_view, first_view_of,
    known_assets, models, placement_of, view, view_of,
};

pub use placement::{Bounds, Mat4, fallback_row, in_view, matrix, placed_bounds};

pub use patch::{LayoutPatch, PlacementPatch, StopPatch, apply};

pub const LAYOUT_FILE: &str = "layout.json";
pub const LAYOUT_VERSION: u32 = 1;
/// The poster frame the first view is measured in: Blender's default
/// 1920 by 1080.
pub const ASPECT: f32 = 16.0 / 9.0;
/// The field of view a new stop gets, in degrees.
pub const DEFAULT_FOV: f32 = 35.0;
pub const MAX_STOPS: usize = 24;
pub const STOP_NAME_MAX_CHARS: usize = 60;
/// How far a model may stand from the origin, in metres.
const MAX_REACH: f32 = 10_000.0;
const MIN_SCALE: f32 = 0.0001;
const MAX_SCALE: f32 = 10_000.0;
/// The desk is this thick, its top at y = 0.
pub const DESK_THICKNESS: f32 = 0.04;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    /// Metres.
    pub position: [f32; 3],
    /// Degrees, applied X, then Y, then Z.
    pub rotation: [f32; 3],
    /// Uniform.
    pub scale: f32,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LayoutStop {
    /// Stays the same when the stop is renamed or moved in the order.
    pub id: String,
    pub name: String,
    pub position: [f32; 3],
    pub target: [f32; 3],
    /// Vertical field of view in degrees.
    pub fov: f32,
    /// The assets this stop sees, set by hand. `None` measures the view.
    pub models: Option<Vec<String>>,
}

/// A plain slab the models stand on, so they cast their shadows somewhere.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Desk {
    /// Width (X) and depth (Z) in metres. The top is at y = 0.
    pub size: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SceneLayout {
    pub version: u32,
    /// Asset id to where its newest model stands.
    pub models: BTreeMap<String, Placement>,
    /// In tour order. The first one is the view the page opens on.
    pub stops: Vec<LayoutStop>,
    pub desk: Option<Desk>,
}

impl Default for SceneLayout {
    fn default() -> Self {
        Self {
            version: LAYOUT_VERSION,
            models: BTreeMap::new(),
            stops: Vec::new(),
            desk: None,
        }
    }
}

pub fn path(project_dir: &Path) -> PathBuf {
    project_dir.join(LAYOUT_FILE)
}

/// Reads `layout.json`, or an empty layout when the project has none.
///
/// # Errors
///
/// Fails when the file exists but cannot be read, is not a layout, comes
/// from a newer Maquette or holds values [`validate`] refuses.
pub fn load(project_dir: &Path) -> CoreResult<SceneLayout> {
    let file = path(project_dir);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(SceneLayout::default()),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let layout: SceneLayout = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if layout.version > LAYOUT_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: layout.version,
            supported: LAYOUT_VERSION,
        });
    }
    validate(&layout)?;
    Ok(layout)
}

/// Checks the layout and writes it atomically. A refused layout writes
/// nothing.
///
/// # Errors
///
/// Fails when [`validate`] refuses it or the file cannot be written.
pub fn save(project_dir: &Path, layout: &SceneLayout) -> CoreResult<()> {
    validate(layout)?;
    let stored = SceneLayout {
        version: LAYOUT_VERSION,
        ..layout.clone()
    };
    write_json(&path(project_dir), &stored)
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

fn finite(values: &[f32]) -> bool {
    values.iter().all(|v| v.is_finite())
}

fn check_placement(id: &str, p: &Placement) -> CoreResult<()> {
    if !finite(&p.position) || !finite(&p.rotation) || !p.scale.is_finite() {
        return Err(invalid(format!(
            "{id} has a position, rotation or scale that is not a number."
        )));
    }
    if p.position.iter().any(|v| v.abs() > MAX_REACH) {
        return Err(invalid(format!(
            "{id} stands more than {MAX_REACH} m from the middle. Keep positions within that."
        )));
    }
    if !(MIN_SCALE..=MAX_SCALE).contains(&p.scale) {
        return Err(invalid(format!(
            "{id} has a scale of {}. Use more than 0 and at most {MAX_SCALE}.",
            p.scale
        )));
    }
    Ok(())
}

fn check_stop(stop: &LayoutStop) -> CoreResult<()> {
    let name = stop.name.trim();
    if name.is_empty() {
        return Err(invalid("Give every camera stop a name."));
    }
    if name.chars().count() > STOP_NAME_MAX_CHARS {
        return Err(invalid(format!(
            "Keep the stop name {name} to {STOP_NAME_MAX_CHARS} characters or fewer."
        )));
    }
    if stop.id.is_empty() || slugify(&stop.id) != stop.id {
        return Err(invalid(format!(
            "The stop {name} needs an id of lowercase letters, digits and dashes, like {}.",
            slugify(name)
        )));
    }
    if !finite(&stop.position) || !finite(&stop.target) || !stop.fov.is_finite() {
        return Err(invalid(format!(
            "The stop {name} has a value that is not a number."
        )));
    }
    if stop
        .position
        .iter()
        .chain(&stop.target)
        .any(|v| v.abs() > MAX_REACH)
    {
        return Err(invalid(format!(
            "The stop {name} is more than {MAX_REACH} m from the middle."
        )));
    }
    if crate::inbox::math::distance(stop.position, stop.target) < 1e-4 {
        return Err(invalid(format!(
            "The stop {name} looks at the point it stands on. Move its target away from the camera."
        )));
    }
    if !(stop.fov > 1.0 && stop.fov < 170.0) {
        return Err(invalid(format!(
            "The stop {name} has a field of view of {}. Use more than 1 and less than 170 degrees.",
            stop.fov
        )));
    }
    if let Some(models) = &stop.models {
        let unique: BTreeSet<&String> = models.iter().collect();
        if unique.len() != models.len() {
            return Err(invalid(format!("The stop {name} lists a model twice.")));
        }
    }
    Ok(())
}

/// Checks every value: finite numbers, a scale above zero, named stops with
/// unique ids, a target away from the camera, a field of view between 1 and
/// 170 degrees and a desk of sensible size.
///
/// # Errors
///
/// Returns the first problem as a sentence for the person or agent.
pub fn validate(layout: &SceneLayout) -> CoreResult<()> {
    for (id, placement) in &layout.models {
        check_placement(id, placement)?;
    }
    if layout.stops.len() > MAX_STOPS {
        return Err(invalid(format!(
            "Keep the camera stops to {MAX_STOPS} or fewer."
        )));
    }
    let mut ids = BTreeSet::new();
    for stop in &layout.stops {
        check_stop(stop)?;
        if !ids.insert(stop.id.as_str()) {
            return Err(invalid(format!(
                "Two camera stops share the id {}.",
                stop.id
            )));
        }
    }
    if let Some(desk) = layout.desk
        && !desk
            .size
            .iter()
            .all(|s| s.is_finite() && *s >= 0.05 && *s <= 100.0)
    {
        return Err(invalid(
            "Make the desk between 0.05 and 100 m wide and deep.",
        ));
    }
    Ok(())
}

/// A free stop id for `name`: its slug, else the slug with `-2`, `-3`.
pub fn stop_id(layout: &SceneLayout, name: &str) -> String {
    let base = slugify(name);
    let taken = |id: &str| layout.stops.iter().any(|s| s.id == id);
    if !taken(&base) {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !taken(id))
        .unwrap_or(base)
}

/// Tells a missing field (`None`) from an explicit null (`Some(None)`).
fn explicit<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[cfg(test)]
mod tests;
