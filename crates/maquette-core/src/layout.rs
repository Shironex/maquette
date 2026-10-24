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

/// One change to a model's placement. Fields left out keep their value; a
/// new placement starts at the origin, unrotated, at scale 1.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementPatch {
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub scale: Option<f32>,
}

/// One change to a stop: an existing id updates it, a new one (or none)
/// adds a stop, which then needs a name, a position and a target.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopPatch {
    pub id: Option<String>,
    pub name: Option<String>,
    pub position: Option<[f32; 3]>,
    pub target: Option<[f32; 3]>,
    pub fov: Option<f32>,
    /// `Some(None)` (an explicit null) goes back to measuring the view.
    #[serde(default, deserialize_with = "explicit")]
    pub models: Option<Option<Vec<String>>>,
    #[serde(default)]
    pub remove: bool,
}

/// A partial update: only what it names changes.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutPatch {
    /// Asset id to its change; `None` drops the placement, so the model
    /// stands side by side again.
    #[serde(default)]
    pub models: BTreeMap<String, Option<PlacementPatch>>,
    #[serde(default)]
    pub stops: Vec<StopPatch>,
    /// Every stop id in the new order.
    pub stop_order: Option<Vec<String>>,
    /// `Some(None)` (an explicit null) removes the desk.
    #[serde(default, deserialize_with = "explicit")]
    pub desk: Option<Option<Desk>>,
}

/// Applies `patch` to `layout`. `known` are the asset ids a placement or a
/// stop's model list may name (the assets with a model). Nothing changes
/// when any part is refused.
///
/// # Errors
///
/// Fails on an unknown asset or stop, a new stop without a name, position
/// or target, a stop order that is not every stop once, or a result that
/// [`validate`] refuses.
pub fn apply(
    layout: &SceneLayout,
    patch: &LayoutPatch,
    known: &[String],
) -> CoreResult<SceneLayout> {
    let mut next = layout.clone();
    let check_asset = |id: &str| {
        if known.iter().any(|k| k == id) {
            Ok(())
        } else {
            Err(invalid(format!(
                "There is no model for the asset {id}. Use one of: {}.",
                known.join(", ")
            )))
        }
    };
    for (id, change) in &patch.models {
        match change {
            None => {
                next.models.remove(id);
            }
            Some(change) => {
                check_asset(id)?;
                let placement = next.models.entry(id.clone()).or_default();
                if let Some(position) = change.position {
                    placement.position = position;
                }
                if let Some(rotation) = change.rotation {
                    placement.rotation = rotation;
                }
                if let Some(scale) = change.scale {
                    placement.scale = scale;
                }
            }
        }
    }
    for change in &patch.stops {
        let found = change
            .id
            .as_deref()
            .and_then(|id| next.stops.iter().position(|s| s.id == id));
        if change.remove {
            let index = found.ok_or_else(|| {
                invalid(format!(
                    "There is no camera stop {} to remove.",
                    change.id.as_deref().unwrap_or("without an id")
                ))
            })?;
            next.stops.remove(index);
            continue;
        }
        if let Some(Some(models)) = &change.models {
            for id in models {
                check_asset(id)?;
            }
        }
        match found {
            Some(index) => {
                let stop = &mut next.stops[index];
                if let Some(name) = &change.name {
                    stop.name = name.trim().to_string();
                }
                if let Some(position) = change.position {
                    stop.position = position;
                }
                if let Some(target) = change.target {
                    stop.target = target;
                }
                if let Some(fov) = change.fov {
                    stop.fov = fov;
                }
                if let Some(models) = &change.models {
                    stop.models.clone_from(models);
                }
            }
            None => {
                let name = change
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .ok_or_else(|| invalid("A new camera stop needs a name."))?;
                let (Some(position), Some(target)) = (change.position, change.target) else {
                    return Err(invalid(format!(
                        "The new stop {name} needs a position and a target."
                    )));
                };
                let id = match &change.id {
                    Some(id) => id.clone(),
                    None => stop_id(&next, name),
                };
                next.stops.push(LayoutStop {
                    id,
                    name: name.to_string(),
                    position,
                    target,
                    fov: change.fov.unwrap_or(DEFAULT_FOV),
                    models: change.models.clone().flatten(),
                });
            }
        }
    }
    if let Some(order) = &patch.stop_order {
        let wanted: BTreeSet<&String> = order.iter().collect();
        let have: BTreeSet<&String> = next.stops.iter().map(|s| &s.id).collect();
        if wanted != have || order.len() != next.stops.len() {
            return Err(invalid(format!(
                "stopOrder must name every stop once: {}.",
                next.stops
                    .iter()
                    .map(|s| s.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        next.stops
            .sort_by_key(|s| order.iter().position(|id| *id == s.id));
    }
    if let Some(desk) = patch.desk {
        next.desk = desk;
    }
    validate(&next)?;
    Ok(next)
}

/// Column major, like glTF: `m[column][row]`.
pub type Mat4 = [[f32; 4]; 4];

/// The matrix of a placement: translate, then rotate X, Y, Z, then scale,
/// as three.js composes an object with the default euler order.
pub fn matrix(p: &Placement) -> Mat4 {
    let [x, y, z] = p.rotation.map(f32::to_radians);
    let (sx, cx) = x.sin_cos();
    let (sy, cy) = y.sin_cos();
    let (sz, cz) = z.sin_cos();
    // Rx * Ry * Rz, row by row.
    let r = [
        [cy * cz, -cy * sz, sy],
        [cx * sz + sx * sy * cz, cx * cz - sx * sy * sz, -sx * cy],
        [sx * sz - cx * sy * cz, sx * cz + cx * sy * sz, cx * cy],
    ];
    let s = p.scale;
    let mut m = [[0.0; 4]; 4];
    for (col, column) in m.iter_mut().take(3).enumerate() {
        for (row, cell) in column.iter_mut().take(3).enumerate() {
            *cell = r[row][col] * s;
        }
    }
    m[3] = [p.position[0], p.position[1], p.position[2], 1.0];
    m
}

fn transform(m: &Mat4, v: [f32; 3]) -> [f32; 3] {
    crate::inbox::math::point(m, v)
}

/// Bounds in a model's own frame, min and max.
pub type Bounds = ([f32; 3], [f32; 3]);

fn corners((min, max): Bounds) -> [[f32; 3]; 8] {
    std::array::from_fn(|i| {
        [
            if i & 1 == 0 { min[0] } else { max[0] },
            if i & 2 == 0 { min[1] } else { max[1] },
            if i & 4 == 0 { min[2] } else { max[2] },
        ]
    })
}

/// The world bounds of `bounds` moved by `p`.
pub fn placed_bounds(bounds: Bounds, p: &Placement) -> Bounds {
    let m = matrix(p);
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for corner in corners(bounds) {
        let q = transform(&m, corner);
        for k in 0..3 {
            min[k] = min[k].min(q[k]);
            max[k] = max[k].max(q[k]);
        }
    }
    (min, max)
}

fn overlaps(a: Bounds, b: Bounds) -> bool {
    (0..3).all(|k| a.0[k] < b.1[k] && b.0[k] < a.1[k])
}

/// Where the Build scene starter puts models without a placement, as the
/// shift from their own origin: when two of them overlap, all of them go
/// into a row along X, centred on the origin, with a quarter of the largest
/// diagonal between them. Otherwise they stay where they are. `models` is
/// in the order the starter imports them (by file name).
pub fn fallback_row(models: &[(String, Bounds)]) -> BTreeMap<String, [f32; 3]> {
    let mut out: BTreeMap<String, [f32; 3]> = models
        .iter()
        .map(|(id, _)| (id.clone(), [0.0; 3]))
        .collect();
    let crowded = models
        .iter()
        .enumerate()
        .any(|(i, a)| models[i + 1..].iter().any(|b| overlaps(a.1, b.1)));
    if models.len() < 2 || !crowded {
        return out;
    }
    let diagonal = |(min, max): Bounds| {
        (0..3)
            .map(|k| (max[k] - min[k]).powi(2))
            .sum::<f32>()
            .sqrt()
    };
    let gap = 0.25 * models.iter().map(|(_, b)| diagonal(*b)).fold(0.0, f32::max);
    let widths: Vec<f32> = models
        .iter()
        .map(|(_, (min, max))| max[0] - min[0])
        .collect();
    #[expect(clippy::cast_precision_loss, reason = "a handful of models")]
    let mut x = -(widths.iter().sum::<f32>() + gap * (models.len() - 1) as f32) / 2.0;
    for ((id, (min, _)), width) in models.iter().zip(&widths) {
        out.insert(id.clone(), [x - min[0], 0.0, 0.0]);
        x += width + gap;
    }
    out
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalized(v: [f32; 3]) -> Option<[f32; 3]> {
    let len = dot(v, v).sqrt();
    (len > 1e-6).then(|| [v[0] / len, v[1] / len, v[2] / len])
}

/// True when any part of the world box `bounds` may be in the stop's view,
/// a 16:9 frame ([`ASPECT`]). A box is only left out when all its corners
/// are behind the camera or outside one side of the view.
pub fn in_view(stop: &LayoutStop, bounds: Bounds) -> bool {
    let Some(forward) = normalized(sub(stop.target, stop.position)) else {
        return false;
    };
    let up = if forward[1].abs() > 0.999 {
        [0.0, 0.0, -1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let Some(right) = normalized(cross(forward, up)) else {
        return false;
    };
    let up = cross(right, forward);
    let tan_y = (stop.fov.to_radians() / 2.0).tan();
    let tan_x = tan_y * ASPECT;
    let near = 0.01;
    // Each side as a test that is positive inside the view.
    let sides: [&dyn Fn(f32, f32, f32) -> f32; 5] = [
        &|_, _, z| z - near,
        &|x, _, z| x + z * tan_x,
        &|x, _, z| z * tan_x - x,
        &|_, y, z| y + z * tan_y,
        &|_, y, z| z * tan_y - y,
    ];
    let local: Vec<[f32; 3]> = corners(bounds)
        .iter()
        .map(|c| {
            let rel = sub(*c, stop.position);
            [dot(rel, right), dot(rel, up), dot(rel, forward)]
        })
        .collect();
    sides
        .iter()
        .all(|side| local.iter().any(|[x, y, z]| side(*x, *y, *z) >= 0.0))
}

/// Which built file a model's size is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Measured {
    /// The compressed build that ships.
    Shipped,
    /// The raw export, before the first Compress.
    Raw,
    /// The source model, before the first export.
    Source,
}

/// A model the layout can place: an asset's newest version.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LayoutModel {
    pub asset: String,
    pub name: String,
    pub kind: AssetKind,
    pub version: u32,
    /// Absolute path of the model file.
    pub file: String,
    /// Relative to the project.
    pub path: String,
    /// Its bounds in its own frame, metres; `None` when it cannot be read.
    pub min: Option<[f32; 3]>,
    pub max: Option<[f32; 3]>,
    /// Where it stands without a placement: the shift from its origin.
    pub fallback: [f32; 3],
    /// What it weighs on the page.
    pub bytes: f64,
    pub measured: Measured,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FirstViewModel {
    pub asset: String,
    pub name: String,
    pub bytes: f64,
    pub measured: Measured,
}

/// What the first camera stop sees and what that costs to load.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FirstView {
    /// The first stop's name; `None` without stops.
    pub stop: Option<String>,
    /// True when the stop lists its models by hand.
    pub custom: bool,
    pub models: Vec<FirstViewModel>,
    pub bytes: f64,
    pub target_mb: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LayoutView {
    pub project: String,
    /// True when `layout.json` exists.
    pub saved: bool,
    pub layout: SceneLayout,
    pub models: Vec<LayoutModel>,
    pub first_view: FirstView,
}

fn file_bytes(path: &Path) -> Option<f64> {
    #[expect(clippy::cast_precision_loss, reason = "model sizes stay below 2^53")]
    fs::metadata(path)
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.len() as f64)
}

fn stem(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The assets with a model, newest version each, ordered the way the Build
/// scene starter imports them (`.glb` files by name, then `.gltf`).
fn placeable(manifest: &ProjectManifest) -> Vec<&Asset> {
    let mut list: Vec<&Asset> = manifest
        .assets
        .iter()
        .filter(|a| a.latest().is_some())
        .collect();
    list.sort_by_key(|a| {
        let file = a
            .latest()
            .map(|v| v.file.to_lowercase())
            .unwrap_or_default();
        (!file.ends_with(".glb"), stem(&file))
    });
    list
}

/// The asset ids [`apply`] accepts for `manifest`.
pub fn known_assets(manifest: &ProjectManifest) -> Vec<String> {
    placeable(manifest).iter().map(|a| a.id.clone()).collect()
}

/// The models a layout places, measured: bounds from the model files and
/// sizes from the shipped build (else the raw export, else the source).
pub fn models(
    project_dir: &Path,
    manifest: &ProjectManifest,
    def: &PipelineDef,
    layout: &SceneLayout,
) -> Vec<LayoutModel> {
    let (raw_dir, dist_dir) = crate::inspect::folders(project_dir, def);
    let shipped_names = assets::shipped_names(manifest);
    let mut out: Vec<LayoutModel> = placeable(manifest)
        .into_iter()
        .filter_map(|asset| {
            let version = asset.latest()?;
            let file = project_dir.join(&version.file);
            let bounds = crate::inbox::analyse::bounds(&file);
            let model = stem(&version.file);
            let shipped_stem = shipped_names
                .get(&model.to_lowercase())
                .cloned()
                .unwrap_or_else(|| model.clone());
            let (bytes, measured) = file_bytes(&dist_dir.join(format!("{shipped_stem}.glb")))
                .map(|b| (b, Measured::Shipped))
                .or_else(|| {
                    file_bytes(&raw_dir.join(format!("{model}.glb"))).map(|b| (b, Measured::Raw))
                })
                .unwrap_or_else(|| (file_bytes(&file).unwrap_or(0.0), Measured::Source));
            Some(LayoutModel {
                asset: asset.id.clone(),
                name: asset.name.clone(),
                kind: asset.kind,
                version: version.version,
                file: file.display().to_string(),
                path: version.file.clone(),
                min: bounds.map(|b| b.0),
                max: bounds.map(|b| b.1),
                fallback: [0.0; 3],
                bytes,
                measured,
            })
        })
        .collect();
    let unplaced: Vec<(String, Bounds)> = out
        .iter()
        .filter(|m| !layout.models.contains_key(&m.asset))
        .filter_map(|m| Some((m.asset.clone(), (m.min?, m.max?))))
        .collect();
    let row = fallback_row(&unplaced);
    for model in &mut out {
        if let Some(shift) = row.get(&model.asset) {
            model.fallback = *shift;
        }
    }
    out
}

/// Where a model stands: its placement, or its fallback shift.
pub fn placement_of(layout: &SceneLayout, model: &LayoutModel) -> Placement {
    layout
        .models
        .get(&model.asset)
        .copied()
        .unwrap_or(Placement {
            position: model.fallback,
            ..Placement::default()
        })
}

/// The models the first stop sees: its own list, or every model whose
/// placed bounds reach into its view.
pub fn first_view(layout: &SceneLayout, models: &[LayoutModel], target_mb: f64) -> FirstView {
    let Some(stop) = layout.stops.first() else {
        return FirstView {
            stop: None,
            custom: false,
            models: Vec::new(),
            bytes: 0.0,
            target_mb,
        };
    };
    let seen: Vec<&LayoutModel> = match &stop.models {
        Some(list) => models.iter().filter(|m| list.contains(&m.asset)).collect(),
        None => models
            .iter()
            .filter(|m| match (m.min, m.max) {
                (Some(min), Some(max)) => {
                    in_view(stop, placed_bounds((min, max), &placement_of(layout, m)))
                }
                // A model that cannot be measured counts, to be safe.
                _ => true,
            })
            .collect(),
    };
    FirstView {
        stop: Some(stop.name.clone()),
        custom: stop.models.is_some(),
        bytes: seen.iter().map(|m| m.bytes).sum(),
        models: seen
            .into_iter()
            .map(|m| FirstViewModel {
                asset: m.asset.clone(),
                name: m.name.clone(),
                bytes: m.bytes,
                measured: m.measured,
            })
            .collect(),
        target_mb,
    }
}

/// Everything the layout editor shows for a project.
///
/// # Errors
///
/// Fails when `layout.json` cannot be read.
pub fn view(
    project_dir: &Path,
    manifest: &ProjectManifest,
    def: &PipelineDef,
    first_view_mb: f64,
) -> CoreResult<LayoutView> {
    let layout = load(project_dir)?;
    Ok(view_of(project_dir, manifest, def, layout, first_view_mb))
}

/// [`view`] for a layout in hand.
pub fn view_of(
    project_dir: &Path,
    manifest: &ProjectManifest,
    def: &PipelineDef,
    layout: SceneLayout,
    first_view_mb: f64,
) -> LayoutView {
    let models = models(project_dir, manifest, def, &layout);
    LayoutView {
        project: manifest.id.clone(),
        saved: path(project_dir).is_file(),
        first_view: first_view(&layout, &models, first_view_mb),
        models,
        layout,
    }
}

/// The first view of a project, for the budget meters. A layout that
/// cannot be read counts as no layout.
pub fn first_view_of(
    project_dir: &Path,
    manifest: &ProjectManifest,
    def: &PipelineDef,
    first_view_mb: f64,
) -> FirstView {
    let layout = load(project_dir).unwrap_or_else(|err| {
        tracing::warn!(error = %err, "unreadable scene layout");
        SceneLayout::default()
    });
    let models = models(project_dir, manifest, def, &layout);
    first_view(&layout, &models, first_view_mb)
}

#[cfg(test)]
mod tests;
