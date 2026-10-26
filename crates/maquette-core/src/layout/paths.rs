//! Paths: curves drawn through the scene, like a cable laid along a trench
//! from one model to another, that the page lights up as it scrolls.
//!
//! A path is a list of points in the layout's space (metres, Y up) that the
//! site joins with straight runs (`polyline`) or a smooth Catmull-Rom curve
//! through every point (`catmull-rom`), drawn as a tube `width` metres
//! across in `color`. Its energize track in the timeline says how much of it
//! is lit, from the first point on, at each camera stop: the lit part glows
//! in `glow`. They live in `layout.json` under `paths`, and the tracks under
//! `timeline.paths`, keyed by path id:
//!
//! ```json
//! "paths": [
//!   { "id": "main-feed", "name": "Main feed", "curve": "catmull-rom",
//!     "points": [[0.2, 0, 1.1], [0.4, 0, 0.6], [0.4, 0.8, 0.2]],
//!     "start": "trench", "end": "panel", "width": 0.035,
//!     "color": "#f2c230", "glow": "#ffe08a" }
//! ],
//! "timeline": { "paths": { "main-feed": { "easing": "linear",
//!   "keys": [{ "stop": "porch", "amount": 0 }, { "stop": "canopy", "amount": 1 }] } } }
//! ```
//!
//! `start` and `end` anchor the first and last point to a model: when that
//! model's placement moves, the point moves with it ([`follow`]), so the
//! path keeps meeting the model. A path without a track stays unlit.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::timeline::check_keys;
use super::{Easing, MAX_REACH, Placement, SceneLayout, finite, invalid};
use crate::error::CoreResult;
use crate::json::explicit;
use crate::project::slugify;

pub const MAX_PATHS: usize = 32;
pub const MAX_POINTS: usize = 128;
pub const PATH_NAME_MAX_CHARS: usize = 60;
/// The thinnest and thickest a path's tube may be, in metres.
pub const MIN_WIDTH: f32 = 0.001;
pub const MAX_WIDTH: f32 = 1.0;
/// What a new path gets.
pub const DEFAULT_WIDTH: f32 = 0.035;
pub const DEFAULT_COLOR: &str = "#f2c230";
pub const DEFAULT_GLOW: &str = "#ffe08a";

/// How the site joins a path's points.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PathCurve {
    /// Straight runs from point to point.
    Polyline,
    /// A smooth curve through every point.
    #[default]
    CatmullRom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScenePath {
    /// Stays the same when the path is renamed.
    pub id: String,
    pub name: String,
    pub curve: PathCurve,
    /// Metres, Y up, in order from the start. At least two.
    pub points: Vec<[f32; 3]>,
    /// The asset whose placement the first point follows.
    pub start: Option<String>,
    /// The asset whose placement the last point follows.
    pub end: Option<String>,
    /// How thick the tube is, in metres.
    pub width: f32,
    /// The tube's colour, `#rrggbb`.
    pub color: String,
    /// The colour the lit part glows, `#rrggbb`.
    pub glow: String,
}

/// How much of a path is lit at one camera stop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PathKey {
    /// The camera stop's id.
    pub stop: String,
    /// 0 unlit, 1 lit from the start to the end.
    pub amount: f32,
}

/// One path's energize track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PathTrack {
    /// How the amount eases from one key to the next.
    pub easing: Easing,
    /// At most one per stop, in any order; the page sorts them by stop.
    pub keys: Vec<PathKey>,
}

impl Default for PathTrack {
    fn default() -> Self {
        Self {
            easing: Easing::Linear,
            keys: Vec::new(),
        }
    }
}

/// One change to a path: an existing id updates it, a new one (or none)
/// adds a path, which then needs a name and at least two points.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathPatch {
    pub id: Option<String>,
    pub name: Option<String>,
    pub curve: Option<PathCurve>,
    pub points: Option<Vec<[f32; 3]>>,
    /// `Some(None)` (an explicit null) lets the first point go.
    #[serde(default, deserialize_with = "explicit")]
    pub start: Option<Option<String>>,
    #[serde(default, deserialize_with = "explicit")]
    pub end: Option<Option<String>>,
    pub width: Option<f32>,
    pub color: Option<String>,
    pub glow: Option<String>,
    #[serde(default)]
    pub remove: bool,
}

/// True for `#rrggbb`.
pub fn is_colour(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn check_path(path: &ScenePath) -> CoreResult<()> {
    let name = path.name.trim();
    if name.is_empty() {
        return Err(invalid("Give every path a name."));
    }
    if name.chars().count() > PATH_NAME_MAX_CHARS {
        return Err(invalid(format!(
            "Keep the path name {name} to {PATH_NAME_MAX_CHARS} characters or fewer."
        )));
    }
    if path.id.is_empty() || slugify(&path.id) != path.id {
        return Err(invalid(format!(
            "The path {name} needs an id of lowercase letters, digits and dashes, like {}.",
            slugify(name)
        )));
    }
    if !(2..=MAX_POINTS).contains(&path.points.len()) {
        return Err(invalid(format!(
            "The path {name} has {} points. Give it 2 to {MAX_POINTS}.",
            path.points.len()
        )));
    }
    for point in &path.points {
        if !finite(point) {
            return Err(invalid(format!(
                "The path {name} has a point that is not a number."
            )));
        }
        if point.iter().any(|v| v.abs() > MAX_REACH) {
            return Err(invalid(format!(
                "The path {name} has a point more than {MAX_REACH} m from the middle."
            )));
        }
    }
    if !(path.width.is_finite() && (MIN_WIDTH..=MAX_WIDTH).contains(&path.width)) {
        return Err(invalid(format!(
            "The path {name} is {} m wide. Use {MIN_WIDTH} to {MAX_WIDTH} m.",
            path.width
        )));
    }
    for (what, colour) in [("colour", &path.color), ("glow", &path.glow)] {
        if !is_colour(colour) {
            return Err(invalid(format!(
                "The path {name} has the {what} {colour}. Use #rrggbb, like {DEFAULT_COLOR}."
            )));
        }
    }
    for anchor in [&path.start, &path.end].into_iter().flatten() {
        if anchor.trim().is_empty() {
            return Err(invalid(format!(
                "The path {name} is anchored to a model without an id."
            )));
        }
    }
    Ok(())
}

/// Checks the paths of `layout` and their energize tracks against its
/// camera stops.
pub(super) fn validate(layout: &SceneLayout) -> CoreResult<()> {
    if layout.paths.len() > MAX_PATHS {
        return Err(invalid(format!("Keep the paths to {MAX_PATHS} or fewer.")));
    }
    let mut ids = BTreeSet::new();
    for path in &layout.paths {
        check_path(path)?;
        if !ids.insert(path.id.as_str()) {
            return Err(invalid(format!("Two paths share the id {}.", path.id)));
        }
    }
    let stops: BTreeSet<&str> = layout.stops.iter().map(|s| s.id.as_str()).collect();
    for (id, track) in &layout.timeline.paths {
        if !ids.contains(id.as_str()) {
            return Err(invalid(format!(
                "The timeline lights the path {id}, which is not in the layout."
            )));
        }
        let keys = track.keys.iter().map(|k| (k.stop.as_str(), k.amount));
        check_keys(&format!("The energize track of {id}"), keys, &stops)?;
    }
    Ok(())
}

/// Refuses an anchor to an asset that is not in `known` (the assets with a
/// model).
///
/// # Errors
///
/// `Invalid`, naming the path and the asset.
pub fn check_anchors(paths: &[ScenePath], known: &[String]) -> CoreResult<()> {
    for path in paths {
        for anchor in [&path.start, &path.end].into_iter().flatten() {
            if !known.contains(anchor) {
                return Err(invalid(format!(
                    "The path {} is anchored to {anchor}, which has no model.",
                    path.name.trim()
                )));
            }
        }
    }
    Ok(())
}

/// Drops the keys at camera stops the layout no longer has. Returns how
/// many went.
pub fn prune_stops(layout: &mut SceneLayout) -> usize {
    let stops: BTreeSet<String> = layout.stops.iter().map(|s| s.id.clone()).collect();
    let mut dropped = 0;
    for track in layout.timeline.paths.values_mut() {
        let before = track.keys.len();
        track.keys.retain(|k| stops.contains(&k.stop));
        dropped += before - track.keys.len();
    }
    dropped
}

/// Drops the energize tracks of paths the layout no longer has.
pub fn drop_orphan_tracks(layout: &mut SceneLayout) {
    let ids: BTreeSet<String> = layout.paths.iter().map(|p| p.id.clone()).collect();
    layout.timeline.paths.retain(|id, _| ids.contains(id));
}

/// Moves each anchored end by how far its model's placement moved from
/// `before` to `after`, so a path keeps meeting the models it links.
/// Returns how many ends moved. A model without a placement on either side
/// moves nothing.
pub fn follow(before: &BTreeMap<String, Placement>, after: &mut SceneLayout) -> usize {
    let models = &after.models;
    let mut moved = 0;
    for path in &mut after.paths {
        let last = path.points.len().saturating_sub(1);
        for (anchor, index) in [(&path.start, 0), (&path.end, last)] {
            let Some(asset) = anchor else { continue };
            let (Some(old), Some(new)) = (before.get(asset), models.get(asset)) else {
                continue;
            };
            if old.position == new.position {
                continue;
            }
            if let Some(point) = path.points.get_mut(index) {
                for (axis, value) in point.iter_mut().enumerate() {
                    *value += new.position[axis] - old.position[axis];
                }
                moved += 1;
            }
        }
    }
    moved
}

/// A free path id for `name`: its slug, else the slug with `-2`, `-3`.
pub fn path_id(paths: &[ScenePath], name: &str) -> String {
    let base = slugify(name);
    let taken = |id: &str| paths.iter().any(|p| p.id == id);
    if !taken(&base) {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !taken(id))
        .unwrap_or(base)
}

fn new_path(paths: &[ScenePath], change: &PathPatch) -> CoreResult<ScenePath> {
    let name = change
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| invalid("A new path needs a name."))?;
    let Some(points) = change.points.clone() else {
        return Err(invalid(format!("The new path {name} needs its points.")));
    };
    Ok(ScenePath {
        id: change.id.clone().unwrap_or_else(|| path_id(paths, name)),
        name: name.to_string(),
        curve: change.curve.unwrap_or_default(),
        points,
        start: change.start.clone().flatten(),
        end: change.end.clone().flatten(),
        width: change.width.unwrap_or(DEFAULT_WIDTH),
        color: change
            .color
            .as_deref()
            .map_or_else(|| DEFAULT_COLOR.into(), |c| c.trim().to_lowercase()),
        glow: change
            .glow
            .as_deref()
            .map_or_else(|| DEFAULT_GLOW.into(), |c| c.trim().to_lowercase()),
    })
}

fn update(path: &mut ScenePath, change: &PathPatch) {
    if let Some(name) = &change.name {
        path.name = name.trim().to_string();
    }
    if let Some(curve) = change.curve {
        path.curve = curve;
    }
    if let Some(points) = &change.points {
        path.points.clone_from(points);
    }
    if let Some(start) = &change.start {
        path.start.clone_from(start);
    }
    if let Some(end) = &change.end {
        path.end.clone_from(end);
    }
    if let Some(width) = change.width {
        path.width = width;
    }
    if let Some(color) = &change.color {
        path.color = color.trim().to_lowercase();
    }
    if let Some(glow) = &change.glow {
        path.glow = glow.trim().to_lowercase();
    }
}

/// Applies path changes and energize tracks to `next`, in that order. A
/// removed path takes its track with it.
///
/// # Errors
///
/// `Invalid` for a path to remove that is not there, a new path without a
/// name or points, or an anchor to an asset not in `known`.
pub(super) fn apply_patch(
    next: &mut SceneLayout,
    changes: &[PathPatch],
    tracks: &BTreeMap<String, Option<PathTrack>>,
    known: &[String],
) -> CoreResult<()> {
    for change in changes {
        for anchor in [&change.start, &change.end].into_iter().flatten().flatten() {
            if !known.contains(anchor) {
                return Err(invalid(format!(
                    "There is no model for the asset {anchor} to anchor a path to."
                )));
            }
        }
        let found = change
            .id
            .as_deref()
            .and_then(|id| next.paths.iter().position(|p| p.id == id));
        match (found, change.remove) {
            (Some(index), true) => {
                next.paths.remove(index);
            }
            (None, true) => {
                return Err(invalid(format!(
                    "There is no path {} to remove.",
                    change.id.as_deref().unwrap_or("without an id")
                )));
            }
            (Some(index), false) => update(&mut next.paths[index], change),
            (None, false) => {
                let path = new_path(&next.paths, change)?;
                next.paths.push(path);
            }
        }
    }
    for (id, track) in tracks {
        match track {
            None => {
                next.timeline.paths.remove(id);
            }
            Some(track) => {
                next.timeline.paths.insert(id.clone(), track.clone());
            }
        }
    }
    drop_orphan_tracks(next);
    Ok(())
}

/// `current` with its paths and their energize tracks replaced whole, as
/// the Paths editor saves them.
///
/// # Errors
///
/// `Invalid` for an anchor to an asset without a model, or a result the
/// layout refuses.
pub fn with_paths(
    current: &SceneLayout,
    known: &[String],
    paths: Vec<ScenePath>,
    tracks: BTreeMap<String, PathTrack>,
) -> CoreResult<SceneLayout> {
    check_anchors(&paths, known)?;
    let mut next = current.clone();
    next.paths = paths;
    next.timeline.paths = tracks;
    drop_orphan_tracks(&mut next);
    super::validate(&next)?;
    Ok(next)
}

#[cfg(test)]
mod tests;
