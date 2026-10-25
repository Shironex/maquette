//! Moving a Lab direction to another project of the workspace: its folder
//! (images, contact sheet, `direction.json` with the stars and picks), the
//! refs its picks were copied to and, when nothing else holds on to it, the
//! Board asset of the same name.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    CONCEPTS_DIR, DIRECTION_FILE, Direction, REFS_DIR, direction_dir, load_direction, write_lock,
};
use crate::assets::{self, Asset};
use crate::error::{CoreError, CoreResult};
use crate::fs::move_tree;
use crate::json::write_json;
use crate::project::{self, ProjectManifest, now_rfc3339};
use crate::workspace::FolderLayout;

/// What Move to project did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MovedDirection {
    /// The direction as it is now, in the other project. Its id changes
    /// only when that project already had one with the same id.
    pub direction: Direction,
    /// The project it left.
    pub from: ProjectManifest,
    /// The project it is in now.
    pub to: ProjectManifest,
    /// The Board asset that went along, when one did.
    pub asset: Option<Asset>,
    /// Its ref files in the other project's `refs/`, relative to that project.
    pub refs: Vec<String>,
}

/// A name in `dir` that is free: `name` itself, else `stem-2.ext`,
/// `stem-3.ext` and so on. A file with the same bytes counts as free, so a
/// ref that is already there is reused instead of copied twice.
fn free_name(dir: &Path, name: &str, same_as: &Path) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, Some(ext)),
        _ => (name, None),
    };
    let candidate = |n: u32| match (n, ext) {
        (1, _) => name.to_string(),
        (n, Some(ext)) => format!("{stem}-{n}.{ext}"),
        (n, None) => format!("{stem}-{n}"),
    };
    let same = |path: &Path| fs::read(path).ok() == fs::read(same_as).ok();
    (1u32..)
        .map(candidate)
        .find(|c| {
            let path = dir.join(c);
            !path.exists() || same(&path)
        })
        .unwrap_or_else(|| name.to_string())
}

fn free_direction_id(to_dir: &Path, id: &str) -> String {
    let taken = |c: &str| to_dir.join(CONCEPTS_DIR).join(c).exists();
    if !taken(id) {
        return id.to_string();
    }
    (2u32..)
        .map(|n| format!("{id}-{n}"))
        .find(|c| !taken(c))
        .unwrap_or_else(|| id.to_string())
}

/// Copies the project relative file `rel` from one project's `refs/` to the
/// other's and returns its new relative path. `None` when the file is gone.
pub(crate) fn carry_ref(from_dir: &Path, to_dir: &Path, rel: &str) -> CoreResult<Option<String>> {
    let source = from_dir.join(rel);
    if !source.is_file() {
        return Ok(None);
    }
    let Some(name) = source.file_name().map(|n| n.to_string_lossy().into_owned()) else {
        return Ok(None);
    };
    let refs = to_dir.join(REFS_DIR);
    fs::create_dir_all(&refs).map_err(|e| CoreError::io(&refs, e))?;
    let name = free_name(&refs, &name, &source);
    let target = refs.join(&name);
    if !target.exists() {
        fs::copy(&source, &target).map_err(|e| CoreError::io(&source, e))?;
    }
    Ok(Some(format!("{REFS_DIR}/{name}")))
}

/// True for a brief reference that points into the project's `refs/`.
fn is_project_ref(reference: &str) -> bool {
    reference.starts_with("refs/") || reference.starts_with("refs\\")
}

/// Moves the direction `direction_id` from project `from_id` to `to_id`.
///
/// The Board asset named like the direction moves along when it has no
/// model versions yet and `keep_asset` is false (the shell passes true
/// while a Send to 3D job still needs it where it is). The refs its picks
/// were copied to move with it then; otherwise they are copied, so the
/// asset left behind keeps its pictures. Brief references in `refs/` are
/// copied.
pub fn move_direction(
    workspace_root: &Path,
    layout: &FolderLayout,
    from_id: &str,
    direction_id: &str,
    to_id: &str,
    keep_asset: bool,
) -> CoreResult<MovedDirection> {
    if from_id == to_id {
        return Err(CoreError::Invalid(
            "The direction is already in this project.".to_string(),
        ));
    }
    let from_manifest = project::load(workspace_root, layout, from_id)?;
    project::load(workspace_root, layout, to_id)?;
    let from_dir = project::dir(workspace_root, layout, from_id)?;
    let to_dir = project::dir(workspace_root, layout, to_id)?;

    let _guard = write_lock();
    let mut direction = load_direction(&from_dir, direction_id)?
        .ok_or_else(|| CoreError::NotFound(format!("the direction {direction_id}")))?;
    let asset = assets::find_by_name(&from_manifest, &direction.name).cloned();
    let asset_moves = !keep_asset && asset.as_ref().is_some_and(|a| a.versions.is_empty());

    let new_id = free_direction_id(&to_dir, &direction.id);
    let source = direction_dir(&from_dir, &direction.id)?;
    let target = direction_dir(&to_dir, &new_id)?;
    let parent = to_dir.join(CONCEPTS_DIR);
    fs::create_dir_all(&parent).map_err(|e| CoreError::io(&parent, e))?;
    move_tree(&source, &target).map_err(|e| CoreError::io(&source, e))?;

    let carried = carry_files(&from_dir, &to_dir, &mut direction, &new_id);
    let written = carried
        .and_then(|refs| write_json(&target.join(DIRECTION_FILE), &direction).map(|()| refs));
    let (refs, renamed) = match written {
        Ok(found) => found,
        Err(err) => {
            // Put the folder back so the direction stays where it was.
            if let Err(back) = move_tree(&target, &source) {
                tracing::warn!(error = %back, "could not put a direction back");
            }
            return Err(err);
        }
    };

    let mut moved_asset = None;
    let (from, to) = if let (true, Some(asset)) = (asset_moves, asset) {
        let thumbnail = asset
            .thumbnail
            .as_ref()
            .map(|t| renamed_ref(&renamed, t).unwrap_or_else(|| t.clone()));
        let to = project::modify(workspace_root, layout, to_id, |m| {
            let placed = assets::adopt(
                m,
                Asset {
                    thumbnail,
                    ..asset.clone()
                },
            );
            moved_asset = Some(placed);
            Ok(())
        })?;
        let from = project::modify(workspace_root, layout, from_id, |m| {
            m.assets.retain(|a| a.id != asset.id);
            Ok(())
        })?;
        // The asset went along, so its refs did too: nothing here uses them.
        for (old, _) in &renamed {
            let path = from_dir.join(old);
            if let Err(err) = fs::remove_file(&path) {
                tracing::warn!(file = %path.display(), error = %err, "could not remove a moved ref");
            }
        }
        (from, to)
    } else {
        (
            project::load(workspace_root, layout, from_id)?,
            project::load(workspace_root, layout, to_id)?,
        )
    };
    Ok(MovedDirection {
        direction,
        from,
        to,
        asset: moved_asset,
        refs,
    })
}

type Renamed = Vec<(String, String)>;

fn renamed_ref(renamed: &Renamed, old: &str) -> Option<String> {
    renamed
        .iter()
        .find(|(o, _)| o == old)
        .map(|(_, n)| n.clone())
}

/// Points the direction at its new folder and copies its refs over.
/// Returns the refs now in the other project and each old to new path.
fn carry_files(
    from_dir: &Path,
    to_dir: &Path,
    direction: &mut Direction,
    new_id: &str,
) -> CoreResult<(Vec<String>, Renamed)> {
    let old_prefix = format!("{CONCEPTS_DIR}/{}/", direction.id);
    let new_prefix = format!("{CONCEPTS_DIR}/{new_id}/");
    let mut refs = Vec::new();
    let mut renamed: Renamed = Vec::new();
    for image in &mut direction.images {
        if let Some(rest) = image.file.strip_prefix(&old_prefix) {
            image.file = format!("{new_prefix}{rest}");
        }
        let Some(old) = image.ref_file.take() else {
            continue;
        };
        let new = match renamed_ref(&renamed, &old) {
            Some(new) => Some(new),
            None => carry_ref(from_dir, to_dir, &old)?,
        };
        if let Some(new) = &new {
            renamed.push((old, new.clone()));
            refs.push(new.clone());
        }
        image.ref_file = new;
    }
    let mut brief = Vec::new();
    for reference in &direction.refs {
        let kept = if is_project_ref(reference) {
            carry_ref(from_dir, to_dir, reference)?
        } else {
            Some(reference.clone())
        };
        brief.extend(kept);
    }
    direction.refs = brief;
    refs.dedup();
    direction.id = new_id.to_string();
    direction.updated_at = now_rfc3339();
    Ok((refs, renamed))
}

#[cfg(test)]
mod tests;
