//! What a render would be made of: the models of a source with where their
//! files go and whether they have the colour toon needs. It reads the same
//! plan as an export pack, so a project's newest models and a library's
//! entries come out the same way.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::record::EXPORTS_RENDERS;
use crate::error::{CoreError, CoreResult};
use crate::library::LibrarySource;
use crate::packs::{self, PackModel, PackSettings, PackSource};
use crate::project;
use crate::thumbs::source::{self, Source};
use crate::workspace::FolderLayout;

/// One model a render can start from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderTarget {
    /// The asset id in a project, the entry id in the library.
    pub key: String,
    pub name: String,
    /// Its folder under `exports/renders`.
    pub folder: String,
    pub version: u32,
    /// The file Blender reads, an absolute path.
    pub file: String,
    /// Its picture, an absolute path.
    pub thumbnail: Option<String>,
    pub height_m: Option<f32>,
    pub triangles: u32,
    /// It has a texture or material colours, so toon has something to show.
    pub has_colour: bool,
    /// Why Blender cannot read it, when it cannot.
    pub unreadable: Option<String>,
}

/// What a render of a source would hold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderPlan {
    /// "of Winter Spire", "from the library".
    pub label: String,
    /// Where the files go, an absolute path.
    pub dir: String,
    pub targets: Vec<RenderTarget>,
    /// Models left out, each with why.
    pub skipped: Vec<String>,
}

fn json_of(file: &Path) -> Option<serde_json::Value> {
    let bytes = fs::read(file).ok()?;
    let json: &[u8] = if bytes.starts_with(b"glTF") {
        let len = u32::from_le_bytes(bytes.get(12..16)?.try_into().ok()?) as usize;
        bytes.get(20..20 + len)?
    } else {
        &bytes
    };
    serde_json::from_slice(json).ok()
}

/// True when the glTF has a picture, or material colours that are not all
/// one grey. A file that is not glTF gets the benefit of the doubt.
pub(super) fn has_colour(file: &Path) -> bool {
    let is_gltf = file
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("glb") || e.eq_ignore_ascii_case("gltf"));
    if !is_gltf {
        return true;
    }
    let Some(json) = json_of(file) else {
        return true;
    };
    let pictures = json["images"].as_array().is_some_and(|i| !i.is_empty());
    if pictures {
        return true;
    }
    let mut seen: Vec<[i64; 3]> = Vec::new();
    for material in json["materials"].as_array().into_iter().flatten() {
        let factor = &material["pbrMetallicRoughness"]["baseColorFactor"];
        let channel = |i: usize| {
            #[expect(clippy::cast_possible_truncation, reason = "a colour rounded to 1/50")]
            let value = (factor[i].as_f64().unwrap_or(1.0) * 50.0).round() as i64;
            value
        };
        let colour = [channel(0), channel(1), channel(2)];
        if !seen.contains(&colour) {
            seen.push(colour);
        }
    }
    let tinted = |c: &[i64; 3]| c.iter().max().unwrap_or(&0) - c.iter().min().unwrap_or(&0) > 2;
    seen.len() > 1 || seen.iter().any(tinted)
}

fn target_of(model: &PackModel, key: String, taken: &mut Vec<String>) -> RenderTarget {
    let mut folder = project::slugify(&model.name);
    if folder.is_empty() {
        folder = "model".to_string();
    }
    let stem = folder.clone();
    let mut n = 2;
    while taken.contains(&folder) {
        folder = format!("{stem}-{n}");
        n += 1;
    }
    taken.push(folder.clone());
    let path = Path::new(&model.file);
    let (file, unreadable) = match source::pick(path) {
        Source::Original => (path.to_path_buf(), None),
        Source::Uncompressed(raw) => (raw, None),
        Source::Snapshot(why) => (path.to_path_buf(), Some(why)),
    };
    RenderTarget {
        key,
        name: model.name.clone(),
        folder,
        version: model.version,
        has_colour: has_colour(&file),
        file: file.display().to_string(),
        thumbnail: model.thumbnail.clone(),
        height_m: model.height_m,
        triangles: model.triangles(),
        unreadable,
    }
}

/// `<project>/exports/renders` or `<workspace>/exports/renders`.
pub(super) fn out_dir(
    root: &Path,
    layout: &FolderLayout,
    source: &PackSource,
) -> CoreResult<PathBuf> {
    let base = match source {
        PackSource::Project { project } => project::dir(root, layout, project)?,
        _ => root.to_path_buf(),
    };
    Ok(base.join(packs::EXPORTS_DIR).join(EXPORTS_RENDERS))
}

/// What a render of `source` would hold. Reads only.
///
/// # Errors
///
/// Fails for an unknown project or collection, and for a source with no
/// models.
pub fn plan(root: &Path, layout: &FolderLayout, source: &PackSource) -> CoreResult<RenderPlan> {
    let settings = PackSettings {
        name: "renders".to_string(),
        ..PackSettings::default()
    };
    let source_of = |_: &Path, _: &crate::Asset, _: u32| (LibrarySource::Imported, None);
    let planned = packs::plan(root, layout, source, &settings, &source_of)?;
    let mut taken = Vec::new();
    let targets = planned
        .models
        .iter()
        .filter_map(|m| {
            let key = m.asset.clone().or_else(|| m.entry.clone())?;
            Some(target_of(m, key, &mut taken))
        })
        .collect();
    Ok(RenderPlan {
        label: planned.label,
        dir: out_dir(root, layout, source)?.display().to_string(),
        targets,
        skipped: planned.skipped,
    })
}

/// The target `key` of `plan`.
pub(super) fn target<'a>(plan: &'a RenderPlan, key: &str) -> CoreResult<&'a RenderTarget> {
    plan.targets
        .iter()
        .find(|t| t.key == key)
        .ok_or_else(|| CoreError::NotFound(format!("a model for {key}")))
}
