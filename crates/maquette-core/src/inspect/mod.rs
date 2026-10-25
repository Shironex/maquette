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

mod budgets;
mod measure;
pub mod stops;
mod types;
use budgets::read_budgets;
pub use budgets::split_budget;

pub use measure::model_label;
use measure::{Entry, glbs, measure};

pub use types::{
    CompareKind, CompareSide, Comparison, InspectModel, InspectView, ModelBudget, ModelPreset,
    ModelSource, SceneTotal, SideMember, Sources, TextureRow,
};

use std::path::{Path, PathBuf};

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

fn rel(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The raw and shipped folders: the compress step's, or `web/raw` and `web/dist`.
pub fn folders(project_dir: &Path, def: &PipelineDef) -> (PathBuf, PathBuf) {
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
            // A shipped file of a renamed asset carries the new name; its
            // preset row is still the model file's.
            let renamed = (source == ModelSource::Shipped)
                .then(|| crate::assets::for_shipped_stem(src.manifest, &stem))
                .flatten();
            // An export of an asset version reads as the asset: "clay", v1.
            let from_asset = renamed
                .as_ref()
                .map(|(asset, version, _)| (*asset, *version))
                .or_else(|| crate::assets::for_model_stem(src.manifest, &stem));
            let model = renamed.map_or_else(|| stem.clone(), |(_, _, raw)| raw);
            entries.push(Entry {
                id,
                label: from_asset.map_or_else(|| label_of(&model), |(a, _)| a.name.clone()),
                model,
                source,
                path: rel(&base, &file),
                file,
                asset: None,
                version: from_asset.map(|(_, v)| v),
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
