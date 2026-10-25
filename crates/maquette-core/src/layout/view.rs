//! The Inspector's read model of the layout: measured models, the first
//! view of each stop, and the whole view of a project.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::*;
use crate::prepare::{self, PrepFrame, ScenePrep, preview};

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
    /// Its bounds in its own frame once prepared (fix-ups and real size),
    /// metres; `None` when it cannot be read.
    pub min: Option<[f32; 3]>,
    pub max: Option<[f32; 3]>,
    /// How preparing moves and scales the model file's points, so the
    /// editor draws it the way the Build scene step places it.
    pub prep: PrepFrame,
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

pub(super) fn file_bytes(path: &Path) -> Option<f64> {
    #[expect(clippy::cast_precision_loss, reason = "model sizes stay below 2^53")]
    fs::metadata(path)
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.len() as f64)
}

pub(super) fn stem(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The assets with a model, newest version each, ordered the way the Build
/// scene starter imports them (`.glb` files by name, then `.gltf`).
pub(crate) fn placeable(manifest: &ProjectManifest) -> Vec<&Asset> {
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

/// A model's bounds once prepared, and the frame that gets them there.
/// The fix-ups' own bounds come from a preview that ran (`previews`), else
/// the model file's.
fn prepared(
    project_dir: &Path,
    previews: Option<&Path>,
    prep: &ScenePrep,
    asset: &str,
    file: &Path,
) -> (Option<Bounds>, PrepFrame) {
    let source = crate::inbox::analyse::bounds(file);
    let entry = prep.models.get(asset);
    let fixed = match (entry, previews) {
        (Some(entry), Some(root)) if !entry.fixups.is_empty() => {
            let script = preview::script_for(Some(project_dir));
            preview::fixed_bounds(root, file, entry, &script)
        }
        _ => None,
    };
    let base = fixed.or(source);
    let frame = prepare::frame_of(entry, base);
    (base.map(|b| frame.apply(b)), frame)
}

/// The models a layout places, measured: bounds from the model files (as
/// the real size and fix-ups leave them) and sizes from the shipped build
/// (else the raw export, else the source). `previews` is the fix-up preview
/// folder, when the caller has the workspace.
pub fn models(
    project_dir: &Path,
    manifest: &ProjectManifest,
    def: &PipelineDef,
    layout: &SceneLayout,
    previews: Option<&Path>,
) -> Vec<LayoutModel> {
    let (raw_dir, dist_dir) = crate::inspect::folders(project_dir, def);
    let shipped_names = assets::shipped_names(manifest);
    let prep = prepare::load(project_dir).unwrap_or_else(|err| {
        tracing::warn!(error = %err, "unreadable model preparation");
        ScenePrep::default()
    });
    let mut out: Vec<LayoutModel> = placeable(manifest)
        .into_iter()
        .filter_map(|asset| {
            let version = asset.latest()?;
            let file = project_dir.join(&version.file);
            let (bounds, frame) = prepared(project_dir, previews, &prep, &asset.id, &file);
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
                prep: frame,
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
    previews: Option<&Path>,
) -> CoreResult<LayoutView> {
    let layout = load(project_dir)?;
    Ok(view_of(
        project_dir,
        manifest,
        def,
        layout,
        first_view_mb,
        previews,
    ))
}

/// [`view`] for a layout in hand.
pub fn view_of(
    project_dir: &Path,
    manifest: &ProjectManifest,
    def: &PipelineDef,
    layout: SceneLayout,
    first_view_mb: f64,
    previews: Option<&Path>,
) -> LayoutView {
    let models = models(project_dir, manifest, def, &layout, previews);
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
    previews: Option<&Path>,
) -> FirstView {
    let layout = load(project_dir).unwrap_or_else(|err| {
        tracing::warn!(error = %err, "unreadable scene layout");
        SceneLayout::default()
    });
    let models = models(project_dir, manifest, def, &layout, previews);
    first_view(&layout, &models, first_view_mb)
}
