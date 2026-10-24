//! The Inspector: what a project's models cost once they reach the web.
//!
//! [`inspect`] lists every model the view can open, each with its numbers
//! (triangles, textures with their codec and size, an estimate of the GPU
//! memory it takes, the transfer size), the compression preset it gets, and
//! the comparisons the compare slider offers:
//!
//! - **Scene**: the pipeline's raw export (`web/raw`), the shipped build
//!   (`web/dist`) and the copy on the site (`<site repo>/public/models`), each
//!   side being every model of that folder at its exported place.
//! - **Asset**: the versions of one Board asset, oldest first.
//!
//! **Budgets.** The workspace sets a scene budget (MB) and a triangle budget
//! per model. A model's transfer budget is its row in the project's
//! `budgets.json` (`{ "models": { "character": 1.0 } }`, in MB); models
//! without a row share what the rows leave of the scene budget evenly.
//!
//! **GPU memory** is an estimate: KTX2 textures stay block compressed on the
//! GPU (ETC1S at half a byte per pixel, UASTC at one), PNG and JPEG unpack to
//! four bytes per pixel, both with a third more for mipmaps; geometry counts
//! 32 bytes per vertex (48 when skinned) and 4 bytes per index.

pub mod stops;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::assets::Asset;
use crate::inbox::analyse::{self, GlbReport};
use crate::pipeline::presets::CompressPreset;
use crate::pipeline::{CompressDefaults, PipelineDef};
use crate::project::{ProjectManifest, rfc3339_utc};
use crate::workspace::{Budgets, MeshoptLevel, TextureCodec};

pub use stops::{CameraStopDef, CameraStops, StopsSource};

/// The project's per model transfer budgets, relative to the project.
pub const BUDGETS_FILE: &str = "budgets.json";
/// Where the Ship step copies the models, inside the site repo.
pub const SITE_MODELS: &str = "public/models";

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

fn file_bytes(path: &Path) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "model sizes stay below 2^53")]
    fs::metadata(path).map_or(0.0, |m| m.len() as f64)
}

/// `shop_house` reads `Shop house`.
pub fn model_label(stem: &str) -> String {
    let text = stem.replace(['_', '-'], " ");
    let mut chars = text.trim().chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => stem.to_string(),
    }
}

fn glbs(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("glb"))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// The supercompression of each KTX2 image, by image index.
fn ktx2_codecs(file: &Path) -> BTreeMap<usize, &'static str> {
    let mut out = BTreeMap::new();
    let Ok(bytes) = fs::read(file) else {
        return out;
    };
    let Ok(gltf) = gltf::Gltf::from_slice_without_validation(&bytes) else {
        return out;
    };
    let Some(blob) = gltf.blob.as_deref() else {
        return out;
    };
    for image in gltf.document.images() {
        let gltf::image::Source::View { view, .. } = image.source() else {
            continue;
        };
        if view.buffer().index() != 0 {
            continue;
        }
        let Some(data) = blob.get(view.offset()..view.offset() + view.length()) else {
            continue;
        };
        if data.get(1..7) != Some(b"KTX 20") {
            continue;
        }
        // The header: vkFormat at 12, supercompressionScheme at 44. Basis
        // ETC1S is BasisLZ (1); UASTC is uncompressed or Zstandard.
        let scheme = data
            .get(44..48)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes);
        out.insert(
            image.index(),
            if scheme == Some(1) { "ETC1S" } else { "UASTC" },
        );
    }
    out
}

fn texture_rows(file: &Path, report: &GlbReport) -> Vec<TextureRow> {
    let ktx = ktx2_codecs(file);
    report
        .textures
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mime = t.mime.as_deref().unwrap_or_default();
            let (codec, per_px) = match mime {
                "image/ktx2" => match ktx.get(&i).copied() {
                    Some("ETC1S") => ("KTX2 ETC1S".to_string(), 0.5),
                    Some(other) => (format!("KTX2 {other}"), 1.0),
                    None => ("KTX2".to_string(), 1.0),
                },
                "image/png" => ("PNG".to_string(), 4.0),
                "image/jpeg" => ("JPEG".to_string(), 4.0),
                "image/webp" => ("WebP".to_string(), 4.0),
                _ => ("Unknown".to_string(), 4.0),
            };
            let px = f64::from(t.width.unwrap_or(0)) * f64::from(t.height.unwrap_or(0));
            TextureRow {
                name: t.name.clone(),
                uses: t.uses.clone(),
                width: t.width,
                height: t.height,
                bytes: t.bytes,
                codec,
                gpu_bytes: px * per_px * 4.0 / 3.0,
            }
        })
        .collect()
}

fn geometry_bytes(report: &GlbReport) -> f64 {
    let stride = if report.skins > 0 { 48.0 } else { 32.0 };
    f64::from(report.vertices) * stride + f64::from(report.triangles) * 12.0
}

fn preset_for(presets: &[CompressPreset], model: &str, defaults: CompressDefaults) -> ModelPreset {
    match presets.iter().find(|p| p.model == model) {
        Some(p) => ModelPreset {
            texture_size: p.texture_size,
            codec: p.codec,
            meshopt: p.meshopt,
            lock: p.lock.clone(),
            has_row: true,
        },
        None => ModelPreset {
            texture_size: defaults.texture_size,
            codec: defaults.codec,
            meshopt: defaults.meshopt,
            lock: None,
            has_row: false,
        },
    }
}

struct Entry<'a> {
    id: String,
    model: String,
    label: String,
    source: ModelSource,
    file: PathBuf,
    path: String,
    asset: Option<&'a Asset>,
    version: Option<u32>,
}

fn measure(
    entry: Entry<'_>,
    presets: &[CompressPreset],
    defaults: CompressDefaults,
) -> InspectModel {
    let report = analyse::analyse(&entry.file);
    let modified_at = fs::metadata(&entry.file)
        .and_then(|m| m.modified())
        .ok()
        .map(rfc3339_utc);
    let preset = preset_for(presets, &entry.model, defaults);
    let mut model = InspectModel {
        id: entry.id,
        model: entry.model,
        label: entry.label,
        source: entry.source,
        file: entry.file.display().to_string(),
        path: entry.path,
        asset: entry.asset.map(|a| a.id.clone()),
        version: entry.version,
        modified_at,
        bytes: file_bytes(&entry.file),
        triangles: 0,
        vertices: 0,
        size: [0.0; 3],
        materials: 0,
        joints: 0,
        animations: Vec::new(),
        textures: Vec::new(),
        gpu_bytes: 0.0,
        meshopt: false,
        ktx2: false,
        preset,
        error: None,
    };
    match report {
        Ok(report) => {
            let textures = texture_rows(&entry.file, &report);
            model.gpu_bytes =
                textures.iter().map(|t| t.gpu_bytes).sum::<f64>() + geometry_bytes(&report);
            model.triangles = report.triangles;
            model.vertices = report.vertices;
            model.size = report.size;
            model.materials = report.materials;
            model.joints = report.joints;
            model.animations = report.animations;
            model.meshopt = report
                .extensions
                .iter()
                .any(|e| e == "EXT_meshopt_compression" || e == "KHR_meshopt_compression");
            model.ktx2 = report.extensions.iter().any(|e| e == "KHR_texture_basisu");
            model.textures = textures;
        }
        Err(err) => model.error = Some(err.to_string()),
    }
    model
}

#[derive(Deserialize, Default)]
struct BudgetsFile {
    #[serde(default)]
    models: BTreeMap<String, f64>,
}

fn read_budgets(project_dir: &Path) -> BTreeMap<String, f64> {
    let path = project_dir.join(BUDGETS_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    match serde_json::from_str::<BudgetsFile>(&text) {
        Ok(file) => file
            .models
            .into_iter()
            .filter(|(_, mb)| mb.is_finite() && *mb > 0.0)
            .collect(),
        Err(err) => {
            tracing::warn!(file = %path.display(), error = %err, "unreadable model budgets");
            BTreeMap::new()
        }
    }
}

/// Per model transfer budgets: explicit rows, then an even share of the rest.
pub fn split_budget(
    scene_mb: f64,
    models: &[String],
    explicit: &BTreeMap<String, f64>,
) -> Vec<(String, f64, bool)> {
    let named: f64 = models.iter().filter_map(|m| explicit.get(m)).sum();
    let open = models.iter().filter(|m| !explicit.contains_key(*m)).count();
    #[expect(clippy::cast_precision_loss, reason = "a handful of models")]
    let share = if open == 0 {
        0.0
    } else {
        ((scene_mb - named).max(0.0) / open as f64 * 100.0).round() / 100.0
    };
    models
        .iter()
        .map(|m| match explicit.get(m) {
            Some(mb) => (m.clone(), *mb, true),
            None => (m.clone(), share, false),
        })
        .collect()
}

fn rel(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The raw and shipped folders: the compress step's, or `web/raw` and `web/dist`.
fn folders(project_dir: &Path, def: &PipelineDef) -> (PathBuf, PathBuf) {
    let root = def.root_dir(project_dir);
    def.steps
        .iter()
        .find_map(|s| s.compress.as_ref())
        .map_or_else(
            || (project_dir.join("web/raw"), project_dir.join("web/dist")),
            |spec| (root.join(&spec.from), root.join(&spec.to)),
        )
}

/// Builds the Inspector's view of a project.
pub fn inspect(src: &Sources<'_>) -> InspectView {
    let presets = &src.def.presets;
    let label_of = |stem: &str| {
        presets
            .iter()
            .find(|p| p.model == stem)
            .map_or_else(|| model_label(stem), |p| p.label.clone())
    };
    let (raw_dir, dist_dir) = folders(src.project_dir, src.def);
    let site_dir = src
        .manifest
        .site_repo
        .as_deref()
        .map(|repo| PathBuf::from(repo).join(SITE_MODELS));

    let mut entries: Vec<Entry<'_>> = Vec::new();
    let mut scene_sides = Vec::new();
    let groups = [
        (
            ModelSource::Raw,
            "raw",
            "Raw",
            "before compression",
            Some(raw_dir),
        ),
        (
            ModelSource::Shipped,
            "shipped",
            "Shipped",
            "current build",
            Some(dist_dir),
        ),
        (
            ModelSource::Site,
            "site",
            "Site",
            "on the site now",
            site_dir,
        ),
    ];
    for (source, key, label, note, dir) in groups {
        let Some(dir) = dir else { continue };
        let files = glbs(&dir);
        if files.is_empty() {
            continue;
        }
        let base = if source == ModelSource::Site {
            dir.clone()
        } else {
            src.project_dir.to_path_buf()
        };
        let mut members = Vec::new();
        for file in files {
            let stem = file
                .file_stem()
                .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            let id = format!("{key}/{stem}");
            members.push(SideMember {
                model: id.clone(),
                position: [0.0; 3],
            });
            entries.push(Entry {
                id,
                label: label_of(&stem),
                model: stem,
                source,
                path: rel(&base, &file),
                file,
                asset: None,
                version: None,
            });
        }
        scene_sides.push(CompareSide {
            id: format!("scene/{key}"),
            label: label.to_string(),
            note: Some(note.to_string()),
            members,
        });
    }

    let mut comparisons = Vec::new();
    if !scene_sides.is_empty() {
        comparisons.push(Comparison {
            id: "scene".to_string(),
            label: "Scene".to_string(),
            kind: CompareKind::Scene,
            sides: scene_sides,
        });
    }
    for asset in &src.manifest.assets {
        let mut sides = Vec::new();
        let last = asset.versions.last().map(|v| v.version);
        for version in &asset.versions {
            let file = src.project_dir.join(&version.file);
            if !file.is_file() {
                continue;
            }
            let stem = file
                .file_stem()
                .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            // `character_v2` shares the `character` preset.
            let model = stem
                .rsplit_once("_v")
                .filter(|(_, n)| n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty())
                .map_or(stem.clone(), |(base, _)| base.to_string());
            let id = format!("asset/{}/v{}", asset.id, version.version);
            entries.push(Entry {
                id: id.clone(),
                label: asset.name.clone(),
                model,
                source: ModelSource::Version,
                path: version.file.clone(),
                file,
                asset: Some(asset),
                version: Some(version.version),
            });
            sides.push(CompareSide {
                id: format!("{id}/side"),
                label: format!("v{}", version.version),
                note: Some(if Some(version.version) == last {
                    "current".to_string()
                } else {
                    version.created_at.get(..10).unwrap_or_default().to_string()
                }),
                members: vec![SideMember {
                    model: id,
                    position: [0.0; 3],
                }],
            });
        }
        if !sides.is_empty() {
            comparisons.push(Comparison {
                id: format!("asset/{}", asset.id),
                label: asset.name.clone(),
                kind: CompareKind::Asset,
                sides,
            });
        }
    }

    let models: Vec<InspectModel> = entries
        .into_iter()
        .map(|e| measure(e, presets, src.defaults))
        .collect();

    // The scene's models: the shipped build, or the raw export before it.
    let scene: Vec<&InspectModel> = {
        let shipped: Vec<&InspectModel> = models
            .iter()
            .filter(|m| m.source == ModelSource::Shipped)
            .collect();
        if shipped.is_empty() {
            models
                .iter()
                .filter(|m| m.source == ModelSource::Raw)
                .collect()
        } else {
            shipped
        }
    };
    let stems: Vec<String> = scene.iter().map(|m| m.model.clone()).collect();
    let explicit = read_budgets(src.project_dir);
    let budgets = split_budget(src.limits.scene_mb, &stems, &explicit)
        .into_iter()
        .zip(&scene)
        .map(|((model, target_mb, explicit), m)| ModelBudget {
            label: m.label.clone(),
            model,
            bytes: m.bytes,
            target_mb,
            explicit,
            triangles: m.triangles,
        })
        .collect();
    let total = SceneTotal {
        bytes: scene.iter().map(|m| m.bytes).sum(),
        triangles: scene.iter().map(|m| m.triangles).sum(),
        models: u32::try_from(scene.len()).unwrap_or(u32::MAX),
    };
    let sized: Vec<(PathBuf, [f32; 3])> = scene
        .iter()
        .map(|m| (PathBuf::from(&m.file), m.size))
        .collect();
    let stops = stops::load(src.project_dir, &sized);
    InspectView {
        project: src.manifest.id.clone(),
        models,
        comparisons,
        stops,
        budgets,
        limits: src.limits.clone(),
        scene: total,
    }
}

#[cfg(test)]
mod tests;
