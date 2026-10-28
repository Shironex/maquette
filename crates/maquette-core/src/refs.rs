//! An asset's refs and the view each one shows: front, left, back, right,
//! or other. The views live on the asset in `project.json` (`refs`), never
//! in the file names, and every send reads them: the front goes first,
//! then left, back and right, and a ref tagged other stays home.
//!
//! An asset nobody tagged has no `refs` yet. Its views are read from the
//! order its refs always had (the Lab picks, else the files named after
//! it): the first is the front, then left, back and right, and any more
//! are other. That is how sends went before the tags, so nothing changes
//! until someone tags a ref.

mod files;
mod suggested;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::{self, Asset};
use crate::concepts::REFS_DIR;
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest};
use crate::workspace::{FolderLayout, is_single_segment};

pub use files::{ORIGINALS_DIR, use_clean_copy, use_view_image};
pub use suggested::{ref_files, suggested};

/// The view a ref shows of its object.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum RefView {
    Front,
    Left,
    Back,
    Right,
    /// Kept with the asset, never sent as a view.
    Other,
}

impl RefView {
    /// The four view slots, in the order a send carries them.
    pub const SLOTS: [RefView; 4] = [RefView::Front, RefView::Left, RefView::Back, RefView::Right];

    /// `front`, `left`, ...: the name Tripo's multiview endpoint and the
    /// By hand files use.
    pub fn name(self) -> &'static str {
        match self {
            RefView::Front => "front",
            RefView::Left => "left",
            RefView::Back => "back",
            RefView::Right => "right",
            RefView::Other => "other",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RefView::Front => "Front",
            RefView::Left => "Left",
            RefView::Back => "Back",
            RefView::Right => "Right",
            RefView::Other => "Other",
        }
    }

    pub fn is_slot(self) -> bool {
        self != RefView::Other
    }
}

/// One ref of an asset and the view it shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetRef {
    /// Relative to the project folder, in `refs/`.
    pub file: String,
    pub view: RefView,
}

/// Views read from an order: the first four fill front, left, back and
/// right, the rest are other.
pub fn inferred(files: &[String]) -> Vec<AssetRef> {
    files
        .iter()
        .enumerate()
        .map(|(n, file)| AssetRef {
            file: file.clone(),
            view: RefView::SLOTS.get(n).copied().unwrap_or(RefView::Other),
        })
        .collect()
}

/// The asset's refs with their views: the tags it carries (minus files
/// that are gone), with `suggested` refs it has not met yet as other; or,
/// for an asset nobody tagged, the views read from `suggested`'s order.
pub fn views_of(project_dir: &Path, asset: &Asset, suggested: &[String]) -> Vec<AssetRef> {
    if asset.refs.is_empty() {
        return inferred(suggested);
    }
    let mut out: Vec<AssetRef> = asset
        .refs
        .iter()
        .filter(|r| project_dir.join(&r.file).is_file())
        .cloned()
        .collect();
    for file in suggested {
        if !out.iter().any(|r| &r.file == file) {
            out.push(AssetRef {
                file: file.clone(),
                view: RefView::Other,
            });
        }
    }
    out
}

/// The files in the view slots, front first, skipping empty slots: what a
/// send picks by default.
pub fn slot_files(refs: &[AssetRef]) -> Vec<String> {
    RefView::SLOTS
        .iter()
        .filter_map(|v| refs.iter().find(|r| r.view == *v))
        .map(|r| r.file.clone())
        .collect()
}

/// Gives `file` the view `view`. A slot holds one ref, so the ref that held
/// it takes `file`'s old view (other when `file` is new to the asset).
pub fn assign(refs: &mut Vec<AssetRef>, file: &str, view: RefView) {
    let old = refs.iter().find(|r| r.file == file).map(|r| r.view);
    if view.is_slot()
        && let Some(holder) = refs.iter_mut().find(|r| r.view == view && r.file != file)
    {
        holder.view = old.unwrap_or(RefView::Other);
    }
    match refs.iter_mut().find(|r| r.file == file) {
        Some(own) => own.view = view,
        None => refs.push(AssetRef {
            file: file.to_string(),
            view,
        }),
    }
}

/// True for `refs/<one file>`, the only place a ref may live.
fn in_refs(file: &str) -> bool {
    file.strip_prefix(&format!("{REFS_DIR}/"))
        .is_some_and(|name| is_single_segment(name) && !name.starts_with('.'))
}

/// Checks a full set of tags: each file a ref of the project that exists,
/// listed once, and at most one ref per view slot.
pub fn check(project_dir: &Path, refs: &[AssetRef]) -> CoreResult<Vec<AssetRef>> {
    let mut out: Vec<AssetRef> = Vec::new();
    for r in refs {
        let file = r.file.trim().replace('\\', "/");
        if !in_refs(&file) {
            return Err(CoreError::Invalid(format!(
                "{} is not a ref of this project; refs live in refs/.",
                r.file
            )));
        }
        if !project_dir.join(&file).is_file() {
            return Err(CoreError::NotFound(format!("the ref {file}")));
        }
        if out.iter().any(|o| o.file == file) {
            return Err(CoreError::Invalid(format!("{file} is listed twice.")));
        }
        if r.view.is_slot()
            && let Some(taken) = out.iter().find(|o| o.view == r.view)
        {
            return Err(CoreError::Invalid(format!(
                "{} and {file} are both tagged {}. A view holds one ref.",
                taken.file,
                r.view.label()
            )));
        }
        out.push(AssetRef { file, view: r.view });
    }
    Ok(out)
}

/// Saves an asset's tags (checked with [`check`]) and returns the project.
pub fn set_views(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    asset_id: &str,
    refs: &[AssetRef],
) -> CoreResult<ProjectManifest> {
    let project_dir = project::dir(workspace_root, layout, project_id)?;
    let refs = check(&project_dir, refs)?;
    project::modify(workspace_root, layout, project_id, |manifest| {
        let asset = assets::find_mut(manifest, asset_id)?;
        asset.refs = refs;
        asset.updated_at = project::now_rfc3339();
        Ok(())
    })
}

/// The images a send carries and the view of each, in send order.
///
/// An asset with tags sends the images tagged with a view slot, front
/// first, then left, back and right; an image tagged other, or not tagged
/// at all, stays out. When none of `images` has a view tag (an asset
/// nobody tagged, or only other refs picked), they go as given, the first
/// as the front, the way sends always went.
pub fn send_order(asset: &Asset, images: &[String]) -> (Vec<String>, Vec<String>) {
    let tagged: Vec<(RefView, &String)> = images
        .iter()
        .filter_map(|file| {
            let view = asset.refs.iter().find(|r| &r.file == file)?.view;
            view.is_slot().then_some((view, file))
        })
        .collect();
    if tagged.is_empty() {
        let views = images
            .iter()
            .zip(RefView::SLOTS)
            .map(|(_, v)| v.name().to_string())
            .collect();
        return (images.to_vec(), views);
    }
    let mut tagged = tagged;
    tagged.sort_by_key(|(view, _)| *view);
    tagged
        .into_iter()
        .map(|(view, file)| (file.clone(), view.name().to_string()))
        .unzip()
}

#[cfg(test)]
mod tests;
