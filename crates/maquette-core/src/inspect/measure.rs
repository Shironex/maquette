//! Measuring one model: its textures, geometry, GPU memory estimate and the
//! compression preset it gets.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::*;

pub(super) fn file_bytes(path: &Path) -> f64 {
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

pub(super) fn glbs(dir: &Path) -> Vec<PathBuf> {
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
pub(super) fn ktx2_codecs(file: &Path) -> BTreeMap<usize, &'static str> {
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

pub(super) fn texture_rows(file: &Path, report: &GlbReport) -> Vec<TextureRow> {
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

pub(super) fn geometry_bytes(report: &GlbReport) -> f64 {
    let stride = if report.skins > 0 { 48.0 } else { 32.0 };
    f64::from(report.vertices) * stride + f64::from(report.triangles) * 12.0
}

pub(super) fn preset_for(
    presets: &[CompressPreset],
    model: &str,
    defaults: CompressDefaults,
) -> ModelPreset {
    match presets.iter().find(|p| p.model == model) {
        Some(p) => ModelPreset {
            texture_size: p.texture_size,
            data_texture_size: p.data_texture_size.filter(|d| *d != p.texture_size),
            codec: p.codec,
            map_codecs: p.map_codecs,
            codecs: p.codecs(),
            meshopt: p.meshopt,
            lock: p.lock.clone(),
            has_row: true,
        },
        None => ModelPreset {
            texture_size: defaults.texture_size,
            data_texture_size: None,
            codec: defaults.codec,
            map_codecs: None,
            codecs: MapCodecs::default().resolve(defaults.codec),
            meshopt: defaults.meshopt,
            lock: None,
            has_row: false,
        },
    }
}

pub(super) struct Entry<'a> {
    pub(super) id: String,
    pub(super) model: String,
    pub(super) label: String,
    pub(super) source: ModelSource,
    pub(super) file: PathBuf,
    pub(super) path: String,
    pub(super) asset: Option<&'a Asset>,
    pub(super) version: Option<u32>,
    pub(super) tweak_asset: Option<String>,
}

pub(super) fn measure(
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
        material_names: Vec::new(),
        tweak_asset: entry.tweak_asset,
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
            model.material_names = crate::materials::material_names(&entry.file);
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
