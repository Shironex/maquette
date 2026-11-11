//! What a replace reports, before it happens and after: the old and the
//! new model side by side (file names, triangles, bytes, bounds), what each
//! setting does and what to check by eye.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::ModelStats;
use crate::layout::{self, Bounds};
use crate::prepare::{self, ModelPrep, RealSize, trim_number};
use crate::project::ProjectManifest;
use crate::ship::ShipSettings;

/// One side of a replace: a model file with its numbers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SwapSide {
    /// The version of the asset that keeps its place.
    pub version: u32,
    /// Relative to the project, for example `models/planner_v2.glb`.
    pub file: String,
    /// The file Ship writes for it: a site's `planner_v2.glb`, or a game's
    /// `props/planner.glb`. `None` for a model the project does not ship.
    pub ships_as: Option<String>,
    pub triangles: Option<u32>,
    /// Bytes as a JS safe number.
    pub bytes: Option<f64>,
    /// Its bounds in its own frame as it stands once prepared (fix-ups and
    /// real size, before the layout's placement), metres with y up.
    pub min: Option<[f32; 3]>,
    pub max: Option<[f32; 3]>,
    /// The scale its real size gives it, 1 without a real size.
    pub scale: f32,
}

/// What a replace does with one setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SwapAction {
    /// Keyed by the asset, which keeps its id: nothing is written.
    Kept,
    /// Recorded for the old model and right for the new one too.
    Carried,
    /// Recorded for the old model only: taken off, and back with an undo.
    Reset,
    /// The built asset's own, which comes along with its model.
    Brought,
}

/// One line of the settings table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SwapSetting {
    pub what: String,
    pub action: SwapAction,
    /// A sentence saying why.
    pub why: String,
}

impl SwapSetting {
    pub(super) fn new(what: impl Into<String>, action: SwapAction, why: &str) -> Self {
        Self {
            what: what.into(),
            action,
            why: why.to_string(),
        }
    }
}

/// What replacing a model with a built asset changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceReport {
    pub project: String,
    /// The asset that keeps its id, name and place.
    pub asset: String,
    pub name: String,
    /// The built asset whose script and model take the place.
    pub built: String,
    pub built_name: String,
    /// Its script, relative to the project.
    pub script: String,
    pub old: SwapSide,
    pub new: SwapSide,
    /// Whether the next run bakes the new model's light.
    pub bakes: bool,
    /// The parts of the new model, by the names they ship under.
    pub parts: Vec<String>,
    /// Setting by setting: kept, carried, reset or brought along.
    pub settings: Vec<SwapSetting>,
    /// What names a part or clip the new model does not have. It stays in
    /// the layout and does nothing until the model has it again.
    pub unresolved: Vec<String>,
    pub warnings: Vec<String>,
    /// What the built model has to match, since nothing is aligned.
    pub note: String,
    /// Where the built asset's card went, once the replace is done.
    pub trash: Option<String>,
}

/// What is not aligned for the person or agent.
pub const NOTE: &str = "Nothing is aligned. The layout keeps the asset's position, turn and scale, so the built model has to face the way the old model stood and sit around the same origin: compare the old and new bounds.";

/// "30 cm", "1.25 m", "4 mm".
pub(super) fn length(metres: f32) -> String {
    let m = metres.abs();
    if m < 0.01 {
        format!("{} mm", trim_number(metres * 1000.0))
    } else if m < 1.0 {
        format!("{} cm", trim_number(metres * 100.0))
    } else {
        format!("{} m", trim_number(metres))
    }
}

/// "30 x 2 x 21 cm": width, height and depth.
fn extent_words(extent: [f32; 3]) -> String {
    let longest = extent[0].max(extent[1]).max(extent[2]);
    let (unit, by) = if longest < 0.01 {
        ("mm", 1000.0)
    } else if longest < 1.0 {
        ("cm", 100.0)
    } else {
        ("m", 1.0)
    };
    let side = |v: f32| trim_number(v * by);
    format!(
        "{} x {} x {} {unit}",
        side(extent[0]),
        side(extent[1]),
        side(extent[2])
    )
}

fn bounds_of(side: &SwapSide) -> Option<Bounds> {
    Some((side.min?, side.max?))
}

fn centre((min, max): Bounds) -> [f32; 3] {
    [
        (min[0] + max[0]) / 2.0,
        (min[1] + max[1]) / 2.0,
        (min[2] + max[2]) / 2.0,
    ]
}

fn point(p: [f32; 3]) -> String {
    format!(
        "{}, {}, {}",
        trim_number(p[0]),
        trim_number(p[1]),
        trim_number(p[2])
    )
}

/// The size a model gets from its real size is within this of 1 when its
/// script builds it at that size.
const BUILT_TO_SIZE: std::ops::RangeInclusive<f32> = 0.9..=1.1;
/// A side this far off the old model's reads as another shape or facing.
const SAME_SHAPE: std::ops::RangeInclusive<f32> = 0.8..=1.25;
/// A middle this share of the old model's diagonal away reads as another origin.
const SAME_ORIGIN: f32 = 0.05;

/// What a look at the two models' bounds says: a size the script did not
/// build to, another shape or facing, another origin.
pub(super) fn standing_warnings(
    name: &str,
    old: &SwapSide,
    new: &SwapSide,
    size: Option<&RealSize>,
) -> Vec<String> {
    let (Some(was), Some(now)) = (bounds_of(old), bounds_of(new)) else {
        return vec![format!(
            "The bounds of {} could not be read, so its size and facing are not compared.",
            if bounds_of(old).is_none() {
                "the old model"
            } else {
                "the built model"
            }
        )];
    };
    let mut out = Vec::new();
    let (before, after) = (prepare::extent(was), prepare::extent(now));
    let longest = |e: [f32; 3]| e[0].max(e[1]).max(e[2]);
    match size {
        Some(size) if !BUILT_TO_SIZE.contains(&new.scale) => out.push(format!(
            "The real size ({}) scales the built model by {}: its script does not build it at that size. It stands at the real size all the same.",
            size.describe(),
            trim_number(new.scale)
        )),
        None if longest(before) > 1e-6
            && !BUILT_TO_SIZE.contains(&(longest(after) / longest(before))) =>
        {
            out.push(format!(
                "No real size is set for {name}, so the built model stands at its own size: {} on its longest side, where the old model stood {}. Set the real size, or build it to that size.",
                length(longest(after)),
                length(longest(before))
            ));
        }
        _ => {}
    }
    let same_shape =
        (0..3).all(|k| before[k] <= 1e-6 || SAME_SHAPE.contains(&(after[k] / before[k])));
    if out.is_empty() && !same_shape {
        out.push(format!(
            "The built model is {} (width, height, depth) where the old one was {}: check that it faces the same way.",
            extent_words(after),
            extent_words(before)
        ));
    }
    let (a, b) = (centre(was), centre(now));
    let apart = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    let diagonal = (before[0].powi(2) + before[1].powi(2) + before[2].powi(2)).sqrt();
    if diagonal > 1e-6 && apart > SAME_ORIGIN * diagonal {
        out.push(format!(
            "The built model's middle sits {} from the old model's in its own frame ({} against {}): build it around the same origin, or move it in the layout.",
            length(apart),
            point(b),
            point(a)
        ));
    }
    out
}

/// What one side of the swap is read from.
pub(super) struct SideFile<'a> {
    pub project_dir: &'a Path,
    /// The fix-up previews, for the bounds a rotate or a removed part leaves.
    pub previews: Option<&'a Path>,
    /// The model file to read, relative to the project.
    pub read: &'a str,
    pub version: u32,
    /// The file name the version has, or gets, in the asset.
    pub file: String,
    pub stats: Option<&'a ModelStats>,
    pub entry: Option<&'a ModelPrep>,
    pub ships_as: Option<String>,
}

/// A model with its numbers, measured the way the layout measures it.
pub(super) fn side(from: SideFile<'_>) -> SwapSide {
    let path = from.project_dir.join(from.read);
    let (bounds, frame) = layout::prepared_with(from.project_dir, from.previews, from.entry, &path);
    let (triangles, bytes) = match from.stats {
        Some(stats) => (Some(stats.triangles), Some(stats.bytes)),
        None => match crate::inbox::analyse(&path) {
            Ok(report) => (Some(report.triangles), Some(report.bytes)),
            Err(_) => (None, None),
        },
    };
    SwapSide {
        version: from.version,
        file: from.file,
        ships_as: from.ships_as,
        triangles,
        bytes,
        min: bounds.map(|b| b.0),
        max: bounds.map(|b| b.1),
        scale: frame.scale,
    }
}

/// The file Ship writes for the newest model of `asset`: a site's
/// `planner_v2.glb`, or a game's `props/planner.glb`.
pub(super) fn ships_as(
    manifest: &ProjectManifest,
    ship: &ShipSettings,
    asset: &str,
) -> Option<String> {
    if let Ok(game) = crate::game::settings_of(manifest) {
        let found = manifest.assets.iter().find(|a| a.id == asset)?;
        return Some(crate::game::file_of(game, found));
    }
    crate::site::ship_index(manifest, ship)
        .models
        .get(asset)
        .map(|entry| entry.file.clone())
}
