//! The Animations view: every library clip with its folder, for the
//! previews, and what the library weighs on disk.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{AnimationClip, clip_dir, dir, list};
use crate::error::CoreResult;

/// One clip as the Animations view shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AnimationItem {
    pub clip: AnimationClip,
    /// Its skeleton file, absolute, for the preview.
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AnimationsView {
    /// The animation library folder, absolute.
    pub dir: String,
    pub items: Vec<AnimationItem>,
    /// What every clip folder weighs, kept source files included.
    pub disk_bytes: f64,
}

/// The whole animation library.
///
/// # Errors
///
/// Fails when the library folder cannot be read.
pub fn view(root: &Path) -> CoreResult<AnimationsView> {
    let clips = list(root)?;
    let mut disk = 0u64;
    let mut items = Vec::new();
    for clip in clips {
        let folder = clip_dir(root, &clip.id)?;
        disk += crate::fs::dir_size(&folder);
        items.push(AnimationItem {
            file: folder.join(&clip.file).display().to_string(),
            clip,
        });
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "library sizes stay well under 2^52"
    )]
    let disk_bytes = disk as f64;
    Ok(AnimationsView {
        dir: dir(root).display().to_string(),
        items,
        disk_bytes,
    })
}
