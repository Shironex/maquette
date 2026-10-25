//! The shapes the Inspector view is made of.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ModelSource {
    /// The pipeline's uncompressed export.
    Raw,
    /// The compressed build that ships.
    Shipped,
    /// The copy in the site repo.
    Site,
    /// A version of a Board asset.
    Version,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TextureRow {
    pub name: String,
    /// What the texture is for: "base color", "normal" and so on.
    pub uses: Vec<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bytes: f64,
    /// `KTX2 ETC1S`, `KTX2 UASTC`, `PNG`, `JPEG`, `WebP` or `Unknown`.
    pub codec: String,
    pub gpu_bytes: f64,
}

/// The compression a model gets: its preset row, or the workspace defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreset {
    pub texture_size: u32,
    /// The smaller edge for metal, roughness, occlusion and normal maps.
    pub data_texture_size: Option<u32>,
    pub codec: TextureCodec,
    pub meshopt: MeshoptLevel,
    /// Why the row cannot change, from `pipeline.json`.
    pub lock: Option<String>,
    /// False when no row names the model and the defaults apply.
    pub has_row: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InspectModel {
    /// Unique in the view: `shipped/character`, `asset/character/v2`.
    pub id: String,
    /// The file stem presets and budgets use, for example `character`.
    pub model: String,
    pub label: String,
    pub source: ModelSource,
    pub file: String,
    /// Relative to the project (or the site repo for site copies).
    pub path: String,
    pub asset: Option<String>,
    pub version: Option<u32>,
    pub modified_at: Option<String>,
    pub bytes: f64,
    pub triangles: u32,
    pub vertices: u32,
    /// The bounds in metres, x y z.
    pub size: [f32; 3],
    pub materials: u32,
    pub joints: u32,
    pub animations: Vec<String>,
    pub textures: Vec<TextureRow>,
    pub gpu_bytes: f64,
    pub meshopt: bool,
    pub ktx2: bool,
    pub preset: ModelPreset,
    /// Why the file could not be read, when it could not.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SideMember {
    pub model: String,
    /// Where the model stands in the comparison, added to its own
    /// transform. Pipeline exports keep their place, so this is zero.
    pub position: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompareSide {
    pub id: String,
    /// What the badge says first: `v7`, `Raw`, `Shipped`.
    pub label: String,
    /// What it says after: "current", "on the site", a date.
    pub note: Option<String>,
    pub members: Vec<SideMember>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum CompareKind {
    Scene,
    Asset,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub id: String,
    pub label: String,
    pub kind: CompareKind,
    /// Oldest first; the slider opens on the last two.
    pub sides: Vec<CompareSide>,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelBudget {
    pub model: String,
    pub label: String,
    /// The shipped size, or the raw export's before the first compress.
    pub bytes: f64,
    pub target_mb: f64,
    /// True when `budgets.json` names the model.
    pub explicit: bool,
    pub triangles: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SceneTotal {
    pub bytes: f64,
    pub triangles: u32,
    pub models: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InspectView {
    pub project: String,
    pub models: Vec<InspectModel>,
    pub comparisons: Vec<Comparison>,
    pub stops: CameraStops,
    pub budgets: Vec<ModelBudget>,
    pub limits: Budgets,
    pub scene: SceneTotal,
}

/// The folders `inspect` reads, so tests can point them anywhere.
#[derive(Debug, Clone)]
pub struct Sources<'a> {
    pub project_dir: &'a Path,
    pub manifest: &'a ProjectManifest,
    pub def: &'a PipelineDef,
    pub defaults: CompressDefaults,
    pub limits: &'a Budgets,
}
