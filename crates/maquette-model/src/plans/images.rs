//! Pictures on a canvas. A picture already inside the project is linked
//! where it is; one from elsewhere on disk is copied into `plans/images/`
//! first, named by its stem and the start of its content hash, so adding
//! the same picture twice keeps one copy and the canvas never points
//! outside the project folder.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{NodeKind, PLANS_DIR, PlanFile, is_project_relative};
use crate::error::{CoreError, CoreResult};
use crate::project::latin_slug;

/// Where copied pictures go, inside `plans/`.
pub const IMAGES_DIR: &str = "images";
/// The picture types a canvas shows. SVG stays out: it can carry script.
pub const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "avif"];
/// The largest picture a canvas copies in.
pub const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;

/// A picture ready for an image node.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanImage {
    /// What the node stores: relative to the project, `/` between folders.
    pub path: String,
    /// Where it is on this computer.
    pub file: String,
    /// The file name without its extension, a first caption.
    pub name: String,
    /// True when it was copied into `plans/images`.
    pub copied: bool,
}

fn extension_of(path: &Path) -> Option<String> {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .filter(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
}

/// `path` relative to `root` with `/` separators, when it is inside it.
fn relative_inside(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let joined = parts.join("/");
    is_project_relative(&joined).then_some(joined)
}

/// Readies the picture at `source` for a canvas in `project_dir`.
///
/// # Errors
///
/// Fails when `source` is not a picture file of a known type, is too large
/// or cannot be copied.
pub fn add(project_dir: &Path, source: &Path) -> CoreResult<PlanImage> {
    let ext = extension_of(source).ok_or_else(|| {
        CoreError::Invalid(format!(
            "A canvas shows {} pictures.",
            IMAGE_EXTENSIONS.join(", ")
        ))
    })?;
    let meta = fs::metadata(source).map_err(|e| CoreError::io(source, e))?;
    if !meta.is_file() {
        return Err(CoreError::Invalid("Pick a picture file.".into()));
    }
    if meta.len() > MAX_IMAGE_BYTES {
        return Err(CoreError::Invalid(format!(
            "The picture is over {} MB.",
            MAX_IMAGE_BYTES / 1024 / 1024
        )));
    }
    let name = source
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let root = maquette_base::fs::real_folder(project_dir);
    let real = maquette_base::fs::real_folder(source);
    if let Some(path) = relative_inside(&root, &real) {
        return Ok(PlanImage {
            file: real.display().to_string(),
            path,
            name,
            copied: false,
        });
    }
    let hash = maquette_base::hash::hash_file(source).map_err(|e| CoreError::io(source, e))?;
    let stem = latin_slug(&name).unwrap_or_else(|| "picture".to_string());
    let short = hash.get(..8).unwrap_or(&hash);
    let file_name = format!("{stem}-{short}.{ext}");
    let folder = project_dir.join(PLANS_DIR).join(IMAGES_DIR);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    let to = folder.join(&file_name);
    if !to.is_file() {
        copy_atomic(source, &to)?;
    }
    Ok(PlanImage {
        path: format!("{PLANS_DIR}/{IMAGES_DIR}/{file_name}"),
        file: to.display().to_string(),
        name,
        copied: true,
    })
}

/// Copies through a temp file beside the target, so a sync tool never
/// picks up half a picture.
fn copy_atomic(from: &Path, to: &Path) -> CoreResult<()> {
    let mut tmp = to.as_os_str().to_owned();
    tmp.push(".part");
    let tmp = PathBuf::from(tmp);
    fs::copy(from, &tmp).map_err(|e| CoreError::io(&tmp, e))?;
    maquette_base::fs::rename_retrying(&tmp, to).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        CoreError::io(to, e)
    })
}

/// Each image node's stored path to where it is on this computer.
pub fn paths_of(project_dir: &Path, plan: &PlanFile) -> BTreeMap<String, String> {
    plan.nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Image)
        .filter_map(|n| n.image.as_deref())
        .filter(|p| is_project_relative(p))
        .map(|p| {
            let file = maquette_base::fs::from_workspace(project_dir, p);
            (p.to_string(), file.display().to_string())
        })
        .collect()
}
