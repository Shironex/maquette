//! A model's measured facts in one answer: what an agent needs to know
//! about a model before it rebuilds or replaces it. Nothing here measures
//! anew. The counts and the file's bounds are the Inbox analysis
//! ([`crate::inbox::analyse`]), the real size and the box at real size are
//! the Layout editor's ([`crate::layout::prepared`] over `prepare.json`),
//! the layout entry is `layout.json`'s and the facing is read from the
//! analysis' rest pose ([`facing`]).

use std::path::Path;

use serde::Serialize;

use crate::assets::{Asset, AssetKind, AssetVersion};
use crate::error::{CoreError, CoreResult};
use crate::inbox::analyse::{self, GlbReport};
use crate::layout::{self, Bounds, Placement};
use crate::prepare::{self, RealSize};
use crate::refs::{AssetRef, RefView};

pub mod facing;

pub use facing::{Facing, FacingHow, Front, Sure};

/// What says how to read every number of the answer.
pub const AXES: &str = "Metres. glTF and three.js axes: y is up and a model's front is +z; x is its left to right in the front view. Blender imports that as z up with the front at -y, x unchanged. Turns are degrees about y from +z, towards +x positive, as a layout entry's y rotation.";

/// A box and its size, metres, x y z.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxFacts {
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// Width (x), height (y, up) and depth (z).
    pub size: [f32; 3],
}

/// A number to the hundredth of a millimetre, so a scaled 0.115 does not
/// read 0.11499999.
fn tidy(v: f32) -> f32 {
    (v * 1e5).round() / 1e5
}

impl BoxFacts {
    fn of((min, max): Bounds) -> Self {
        Self {
            min: min.map(tidy),
            max: max.map(tidy),
            size: prepare::extent((min, max)).map(tidy),
        }
    }
}

/// What the model file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub triangles: u32,
    pub vertices: u32,
    /// Meshes in the file.
    pub meshes: u32,
    /// Nodes that draw a mesh: the parts a page can move by name.
    pub objects: u32,
    /// One per mesh and material: the draw calls.
    pub primitives: u32,
    pub materials: u32,
    pub joints: u32,
    pub animations: u32,
}

/// The real size and what it does to the model file.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealSizeFacts {
    /// What `prepare.json` holds; `None` when the asset has no real size.
    pub set: Option<RealSize>,
    /// "45 cm high".
    pub text: Option<String>,
    /// The uniform scale Build scene gives the file to reach it; 1 without
    /// a real size.
    pub scale: f32,
    /// How far the origin fix-up moves the file's points first, metres.
    pub offset: [f32; 3],
}

/// Where the model stands in the scene.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutFacts {
    /// The asset has an entry in `layout.json`.
    pub placed: bool,
    /// The entry: position in metres, rotation in degrees (X, then Y, then
    /// Z) and the uniform scale on top of the real size.
    pub entry: Option<Placement>,
    /// The box the model fills in the scene, when it is placed and measured.
    pub in_scene: Option<BoxFacts>,
}

/// One ref of the asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefFact {
    /// An absolute path.
    pub file: String,
    pub view: RefView,
}

/// A model's facts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelFacts {
    pub project: String,
    pub asset: String,
    pub name: String,
    pub kind: AssetKind,
    pub version: u32,
    /// The asset's newest version, which the scene is built from.
    pub newest: u32,
    /// The model file, an absolute path.
    pub file: String,
    /// A Blender script of the project builds this asset.
    pub built: bool,
    pub counts: Counts,
    /// The file's own bounds, in its own units.
    pub in_file: BoxFacts,
    pub real_size: RealSizeFacts,
    /// The box once the fix-ups and the real size are applied: what the
    /// layout places. `None` when the file cannot be measured.
    pub at_real_size: Option<BoxFacts>,
    pub facing: Facing,
    pub layout: LayoutFacts,
    pub refs: Vec<RefFact>,
    pub axes: &'static str,
    /// What to make of the numbers, in sentences.
    pub notes: Vec<String>,
}

/// What [`facts`] reads from.
#[derive(Debug, Clone)]
pub struct FactsSource<'a> {
    pub project: &'a str,
    pub project_dir: &'a Path,
    pub asset: &'a Asset,
    pub version: &'a AssetVersion,
    /// The fix-up previews' folder, when the caller has the workspace.
    pub previews: Option<&'a Path>,
    /// The asset's refs and the view each shows, relative to the project.
    pub refs: &'a [AssetRef],
}

fn counts(report: &GlbReport) -> Counts {
    Counts {
        triangles: report.triangles,
        vertices: report.vertices,
        meshes: report.meshes,
        objects: report.objects,
        primitives: report.primitives,
        materials: report.materials,
        joints: report.joints,
        animations: u32::try_from(report.animations.len()).unwrap_or(u32::MAX),
    }
}

fn notes(found: &ModelFacts, fixups: usize, measured_fixed: bool) -> Vec<String> {
    let mut out = Vec::new();
    match (&found.real_size.text, found.at_real_size) {
        (Some(text), Some(_)) => out.push(format!(
            "The file is not at real size: Build scene scales it by {} so it measures {text}. atRealSize is the box the layout places.",
            prepare::trim_number(found.real_size.scale)
        )),
        (Some(text), None) => out.push(format!(
            "The asset's real size is {text}, but the file could not be measured, so the box at real size is unknown."
        )),
        (None, _) => out.push(
            "The asset has no real size, so the file's own size is what the layout places (set_real_size sets one).".to_string(),
        ),
    }
    if fixups > 0 && !measured_fixed {
        out.push(format!(
            "The asset has {fixups} fix-up{}. A fix-up can change the bounds (a rotate, removed loose parts); preview_fixups runs them and the box at real size then follows.",
            if fixups == 1 { "" } else { "s" }
        ));
    }
    if found.counts.objects <= 1 {
        out.push(
            "The model is one mesh, so its parts cannot be told apart by name: read them off render_views' pictures, or split it (add_fixups, op split-parts)."
                .to_string(),
        );
    }
    if let Some(entry) = &found.layout.entry
        && (entry.scale - 1.0).abs() > 1e-4
    {
        out.push(format!(
            "The layout scales it by {} on top of the real size: a replacement keeps its place by keeping this entry and matching atRealSize.",
            prepare::trim_number(entry.scale)
        ));
    }
    if !found.layout.placed {
        out.push(
            "No layout entry: Build scene stands it side by side with the other unplaced models (set_layout places it)."
                .to_string(),
        );
    }
    if found.version != found.newest {
        out.push(format!(
            "This is v{}, not the newest (v{}): the scene is built from the newest.",
            found.version, found.newest
        ));
    }
    if found.refs.is_empty() {
        out.push("The asset has no ref image to compare with.".to_string());
    }
    out
}

/// The facts of one version of an asset. Reads only, and never runs Blender.
///
/// # Errors
///
/// Fails when the model file is gone or is not glTF, or `prepare.json` or
/// `layout.json` cannot be read.
pub fn facts(source: &FactsSource<'_>) -> CoreResult<ModelFacts> {
    let FactsSource {
        project_dir,
        asset,
        version,
        ..
    } = *source;
    let file = project_dir.join(&version.file);
    if !file.is_file() {
        return Err(CoreError::NotFound(format!(
            "the model file {}",
            file.display()
        )));
    }
    let report = analyse::analyse(&file)?;
    let prep = prepare::load(project_dir)?;
    let entry = prep.models.get(&asset.id);
    let (prepared, frame) = layout::prepared(project_dir, source.previews, &prep, &asset.id, &file);
    let fixups = entry.map_or(0, |e| e.fixups.len());
    let measured_fixed = entry.is_some_and(|e| {
        let script = prepare::preview::script_for(Some(project_dir));
        source
            .previews
            .is_some_and(|root| prepare::preview::fixed_bounds(root, &file, e, &script).is_some())
    });
    let scene = layout::load(project_dir)?;
    let placement = scene.models.get(&asset.id).copied();
    let built = crate::built::load(project_dir).is_ok_and(|b| b.assets.contains_key(&asset.id));
    let size = entry.and_then(|e| e.size);
    let mut found = ModelFacts {
        project: source.project.to_string(),
        asset: asset.id.clone(),
        name: asset.name.clone(),
        kind: asset.kind,
        version: version.version,
        newest: asset.latest().map_or(version.version, |v| v.version),
        file: file.display().to_string(),
        built,
        counts: counts(&report),
        in_file: BoxFacts::of((report.min, report.max)),
        real_size: RealSizeFacts {
            set: size,
            text: size.map(|s| s.describe()),
            scale: tidy(frame.scale),
            offset: frame.offset.map(tidy),
        },
        at_real_size: prepared.map(BoxFacts::of),
        facing: facing::facing(&report, placement.as_ref(), built),
        layout: LayoutFacts {
            placed: placement.is_some(),
            entry: placement,
            in_scene: prepared
                .zip(placement)
                .map(|(b, p)| BoxFacts::of(layout::placed_bounds(b, &p))),
        },
        refs: source
            .refs
            .iter()
            .map(|r| RefFact {
                file: project_dir.join(&r.file).display().to_string(),
                view: r.view,
            })
            .collect(),
        axes: AXES,
        notes: Vec::new(),
    };
    found.notes = notes(&found, fixups, measured_fixed);
    Ok(found)
}

#[cfg(test)]
mod tests;
