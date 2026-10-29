//! Taking a Lab candidate into an asset's refs: a drawn view goes into a
//! view slot, a clean copy takes the place of the ref it cleans. The
//! original of a clean copy moves to `refs/originals/` and is kept, and so
//! does the ref a drawn view replaces, unless something else still uses it.

use std::fs;
use std::path::Path;

use serde::Serialize;

use super::{AssetRef, RefView, assign, ref_files, suggested, views_of};
use crate::assets::{self, Asset};
use crate::concepts::objects::free_name;
use crate::concepts::{ConceptImage, Direction, REFS_DIR, RefsJobKind, modify_direction};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest};
use crate::workspace::FolderLayout;

/// Where the original of a cleaned ref is kept, relative to the project.
pub const ORIGINALS_DIR: &str = "refs/originals";

/// The direction id of a Lab image, `concepts/<id>/<file>`.
fn direction_of(file: &str) -> CoreResult<&str> {
    let mut parts = file.split('/');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(crate::concepts::CONCEPTS_DIR), Some(id), Some(_), None) => Ok(id),
        _ => Err(CoreError::Invalid(format!(
            "{file} is not a candidate from the Lab."
        ))),
    }
}

fn extension(path: &Path) -> String {
    path.extension()
        .map_or_else(|| "png".to_string(), |e| e.to_string_lossy().to_lowercase())
}

/// Copies the candidate `file` of the asset's refs direction of `kind`
/// into `refs/` (once; a second use keeps the first copy) and returns the
/// copy and the candidate's title. `name` makes the copy's file name.
fn take_candidate(
    project_dir: &Path,
    asset: &Asset,
    file: &str,
    kind: RefsJobKind,
    name: impl FnOnce(&ConceptImage) -> String,
) -> CoreResult<(String, String)> {
    let id = direction_of(file)?;
    let (_, out) = modify_direction(project_dir, id, |d: &mut Direction| {
        if !d
            .refs_job
            .as_ref()
            .is_some_and(|j| j.asset == asset.id && j.kind == kind)
        {
            return Err(CoreError::Invalid(format!(
                "{file} was not made for the refs of {}.",
                asset.name
            )));
        }
        let image = d
            .images
            .iter_mut()
            .find(|i| i.file == file)
            .ok_or_else(|| CoreError::NotFound(format!("the candidate {file}")))?;
        let title = image.title.clone().unwrap_or_default();
        if let Some(kept) = image
            .ref_file
            .clone()
            .filter(|r| project_dir.join(r).is_file())
        {
            return Ok((kept, title));
        }
        let from = project_dir.join(file);
        let refs = project_dir.join(REFS_DIR);
        fs::create_dir_all(&refs).map_err(|e| CoreError::io(&refs, e))?;
        let rel = free_name(project_dir, REFS_DIR, &name(image), &extension(&from));
        fs::copy(&from, project_dir.join(&rel)).map_err(|e| CoreError::io(&from, e))?;
        image.ref_file = Some(rel.clone());
        image.picked = true;
        Ok((rel, title))
    })?;
    Ok(out)
}

/// Moves the ref `rel` to `refs/originals/` under a free name and returns
/// where it went. A file that is gone moves nothing.
fn keep_original(project_dir: &Path, rel: &str) -> CoreResult<Option<String>> {
    let from = project_dir.join(rel);
    if !from.is_file() {
        return Ok(None);
    }
    let keep = project_dir.join(ORIGINALS_DIR);
    fs::create_dir_all(&keep).map_err(|e| CoreError::io(&keep, e))?;
    let stem = from
        .file_stem()
        .map_or_else(|| "ref".to_string(), |s| s.to_string_lossy().into_owned());
    let to = free_name(project_dir, ORIGINALS_DIR, &stem, &extension(&from));
    fs::rename(&from, project_dir.join(&to)).map_err(|e| CoreError::io(&from, e))?;
    Ok(Some(to))
}

/// What happens to the ref that held the slot a drawn view takes.
#[derive(Debug, Clone, Copy)]
pub enum OldView<'a> {
    /// It stays with the asset, tagged other.
    KeepAsOther,
    /// It leaves the asset's refs and its file moves to `refs/originals/`,
    /// unless another asset of the project shows it or a send lists it
    /// (`sent`: the images of every send of the project), in which case the
    /// file stays where it is.
    Replace { sent: &'a [String] },
}

/// The ref a drawn view replaced and where its file went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Replaced {
    pub file: String,
    pub view: RefView,
    /// `refs/originals/<name>`, or `None` when the file stayed in place.
    pub moved_to: Option<String>,
    /// Why the file stayed in place.
    pub kept_because: Option<String>,
}

/// The project after a drawn view went in, and the ref it replaced.
#[derive(Debug, Clone)]
pub struct UsedView {
    pub manifest: ProjectManifest,
    pub replaced: Option<Replaced>,
}

/// Why `file` must stay where it is, if anything else uses it: another
/// asset that shows it or a send that lists it.
fn still_used(
    project_dir: &Path,
    manifest: &ProjectManifest,
    asset_id: &str,
    file: &str,
    sent: &[String],
) -> Option<String> {
    if let Some(other) = manifest.assets.iter().find(|a| {
        a.id != asset_id
            && (a.thumbnail.as_deref() == Some(file)
                || current(project_dir, a).iter().any(|r| r.file == file))
    }) {
        return Some(format!("{} uses it too", other.name));
    }
    sent.iter()
        .any(|s| s == file)
        .then(|| "a send lists it".to_string())
}

/// The asset's refs as they stand, tagged or read from their order.
fn current(project_dir: &Path, asset: &Asset) -> Vec<AssetRef> {
    let files = ref_files(project_dir);
    views_of(project_dir, asset, &suggested(project_dir, asset, &files))
}

fn save(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    asset_id: &str,
    change: impl FnOnce(&mut Asset),
) -> CoreResult<ProjectManifest> {
    project::modify(workspace_root, layout, project_id, |manifest| {
        let asset = assets::find_mut(manifest, asset_id)?;
        change(asset);
        asset.updated_at = project::now_rfc3339();
        Ok(())
    })
}

/// "Use as Left": the view candidate `file` goes into `refs/` and into
/// the slot `view` (its title's view when `None`). `old` says what becomes
/// of the ref that held the slot. The other candidates stay in the Lab.
pub fn use_view_image(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    asset_id: &str,
    file: &str,
    view: Option<RefView>,
    old: OldView<'_>,
) -> CoreResult<UsedView> {
    let project_dir = project::dir(workspace_root, layout, project_id)?;
    let manifest = project::load(workspace_root, layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?.clone();
    let mut refs = current(&project_dir, &asset);
    let (copy, title) = take_candidate(&project_dir, &asset, file, RefsJobKind::Views, |i| {
        format!("{} {}", asset.id, i.title.as_deref().unwrap_or("view"))
    })?;
    let view = view
        .or_else(|| RefView::SLOTS.into_iter().find(|v| v.name() == title))
        .ok_or_else(|| CoreError::Invalid("Say which view this image shows.".to_string()))?;
    if view == RefView::Front {
        return Err(CoreError::Invalid(
            "The front stays as it is; a drawn view fills left, back or right.".to_string(),
        ));
    }
    let holder = refs
        .iter()
        .find(|r| r.view == view && r.file != copy)
        .map(|r| r.file.clone());
    let replaced = match (old, holder) {
        (OldView::Replace { sent }, Some(held)) => {
            let kept_because = still_used(&project_dir, &manifest, asset_id, &held, sent);
            let moved_to = match kept_because {
                Some(_) => None,
                None => keep_original(&project_dir, &held)?,
            };
            refs.retain(|r| r.file != held);
            Some(Replaced {
                file: held,
                view,
                moved_to,
                kept_because,
            })
        }
        _ => None,
    };
    assign(&mut refs, &copy, view);
    let manifest = save(workspace_root, layout, project_id, asset_id, |a| {
        a.refs = refs;
        if let Some(r) = replaced.as_ref().filter(|r| r.moved_to.is_some())
            && a.thumbnail.as_deref() == Some(r.file.as_str())
        {
            a.thumbnail = Some(copy);
        }
    })?;
    Ok(UsedView { manifest, replaced })
}

/// "Use the clean copy": the clean candidate `file` goes into `refs/` as
/// `<ref>_clean`, takes the place and view of the ref it cleans, and that
/// original moves to `refs/originals/`, untouched.
pub fn use_clean_copy(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    asset_id: &str,
    file: &str,
) -> CoreResult<ProjectManifest> {
    let project_dir = project::dir(workspace_root, layout, project_id)?;
    let manifest = project::load(workspace_root, layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?.clone();
    let mut refs = current(&project_dir, &asset);
    let (copy, original) = take_candidate(&project_dir, &asset, file, RefsJobKind::Clean, |i| {
        let of = i.title.as_deref().unwrap_or("ref");
        let stem = Path::new(of)
            .file_stem()
            .map_or_else(|| "ref".to_string(), |s| s.to_string_lossy().into_owned());
        format!("{stem}_clean")
    })?;
    keep_original(&project_dir, &original)?;
    let view = refs
        .iter()
        .find(|r| r.file == original)
        .map_or(RefView::Other, |r| r.view);
    refs.retain(|r| r.file != original && r.file != copy);
    refs.push(AssetRef {
        file: copy.clone(),
        view,
    });
    save(workspace_root, layout, project_id, asset_id, |a| {
        a.refs = refs;
        if a.thumbnail.as_deref() == Some(original.as_str()) {
            a.thumbnail = Some(copy);
        }
    })
}

#[cfg(test)]
mod tests;
