//! Taking a Lab candidate into an asset's refs: a drawn view goes into a
//! view slot, a clean copy takes the place of the ref it cleans. The
//! original of a clean copy moves to `refs/originals/` and is kept.

use std::fs;
use std::path::Path;

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
/// the slot `view` (its title's view when `None`). The ref that held the
/// slot becomes other. The other candidates stay in the Lab.
pub fn use_view_image(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    asset_id: &str,
    file: &str,
    view: Option<RefView>,
) -> CoreResult<ProjectManifest> {
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
    assign(&mut refs, &copy, view);
    save(workspace_root, layout, project_id, asset_id, |a| {
        a.refs = refs
    })
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
    let from = project_dir.join(&original);
    if from.is_file() {
        let keep = project_dir.join(ORIGINALS_DIR);
        fs::create_dir_all(&keep).map_err(|e| CoreError::io(&keep, e))?;
        let stem = from
            .file_stem()
            .map_or_else(|| "ref".to_string(), |s| s.to_string_lossy().into_owned());
        let to = free_name(&project_dir, ORIGINALS_DIR, &stem, &extension(&from));
        fs::rename(&from, project_dir.join(&to)).map_err(|e| CoreError::io(&from, e))?;
    }
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
