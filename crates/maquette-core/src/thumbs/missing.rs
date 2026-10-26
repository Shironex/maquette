//! The models on the Board that have no thumbnail yet: what Import models
//! and the Board's Render previews ask the thumbnail service for.

use std::path::{Path, PathBuf};

use super::{ThumbTarget, is_renderable};
use crate::assets::ref_may_replace;
use crate::project::ProjectManifest;

/// An asset version to render, and its model file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingThumb {
    pub target: ThumbTarget,
    /// The newest version's model, inside the project folder.
    pub file: PathBuf,
}

/// Every asset whose newest model Blender can render and whose card shows
/// no render yet (nothing, or a ref image that stands in), in Board order.
/// `only` keeps the ones named, for a fresh import.
pub fn missing(
    project_dir: &Path,
    manifest: &ProjectManifest,
    only: Option<&[String]>,
) -> Vec<MissingThumb> {
    manifest
        .assets
        .iter()
        .filter(|a| only.is_none_or(|ids| ids.contains(&a.id)))
        .filter(|a| ref_may_replace(a.thumbnail.as_deref()))
        .filter_map(|asset| {
            let latest = asset.latest()?;
            let file = project_dir.join(&latest.file);
            (is_renderable(&file) && file.is_file()).then(|| MissingThumb {
                target: ThumbTarget {
                    project: manifest.id.clone(),
                    asset: asset.id.clone(),
                    version: latest.version,
                },
                file,
            })
        })
        .collect()
}
