//! Picking concepts one image at a time, the way an agent does it: each
//! image goes into `refs/` and becomes (or joins) a Board asset at Refs,
//! through the same steps as Send picks to Refs in the Lab. An image that
//! was picked before keeps its ref and answers with its asset.

use std::path::Path;

use super::objects::{check_source, image_title, ref_name};
use super::picks::place_at_refs;
use super::{CONCEPTS_DIR, Direction, REFS_DIR, list_directions, mark, modify_direction};
use crate::assets::copy::{is_ref_of, key};
use crate::assets::{self, Asset, AssetKind, clean_name};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest, slugify};
use crate::workspace::FolderLayout;

/// The most images one call picks or stars.
pub const MAX_PICKS: usize = 24;

/// One image of the Lab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageRef {
    /// Relative to the project folder (`concepts/<direction>/<file>`), or
    /// absolute inside it.
    File(String),
    /// A direction by id or name, and the image's index in it.
    Numbered { direction: String, index: u32 },
}

/// One image to pick, with what its asset should be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickRequest {
    pub image: ImageRef,
    /// Stars or unstars it too; `None` leaves the star alone.
    pub star: Option<bool>,
    /// The asset's name. By default the image's title in a direction of
    /// object refs, else the direction's name.
    pub name: Option<String>,
    /// What a new asset becomes. By default the direction's kind, else prop.
    pub kind: Option<AssetKind>,
}

/// What picking one image did.
#[derive(Debug, Clone, PartialEq)]
pub struct PickedImage {
    /// The direction id.
    pub direction: String,
    /// Relative to the project folder.
    pub file: String,
    pub index: u32,
    /// Its copy in `refs/`, relative to the project folder.
    pub ref_file: String,
    /// False when the image was picked before and kept its ref.
    pub copied: bool,
    /// The asset as the Board shows it now.
    pub asset: Asset,
}

/// What a pick call did, with what the app needs to update.
#[derive(Debug, Clone, PartialEq)]
pub struct Picked {
    pub images: Vec<PickedImage>,
    /// Every direction that changed, as saved.
    pub directions: Vec<Direction>,
    pub project: ProjectManifest,
}

/// A checked pick, before anything is written.
struct Plan {
    direction: String,
    file: String,
    index: u32,
    star: Option<bool>,
    name: String,
    kind: AssetKind,
    /// The ref is named after the asset (object refs, or a name of its own)
    /// rather than after the concept file.
    named_ref: bool,
}

fn check_count(n: usize) -> CoreResult<()> {
    if (1..=MAX_PICKS).contains(&n) {
        Ok(())
    } else {
        Err(CoreError::Invalid(format!(
            "Pick between 1 and {MAX_PICKS} images in one go."
        )))
    }
}

fn find_direction<'a>(directions: &'a [Direction], wanted: &str) -> Option<&'a Direction> {
    let wanted = wanted.trim();
    directions.iter().find(|d| d.id == wanted).or_else(|| {
        directions
            .iter()
            .find(|d| d.name.trim().eq_ignore_ascii_case(wanted))
    })
}

/// A path as given, made relative to the project folder when it is an
/// absolute path inside it.
fn relative(project_dir: &Path, raw: &str) -> String {
    let path = Path::new(raw.trim());
    path.strip_prefix(project_dir)
        .ok()
        .filter(|_| path.is_absolute())
        .map_or_else(
            || raw.trim().to_string(),
            |p| p.to_string_lossy().replace('\\', "/"),
        )
}

/// The direction and position of an image, checked to be a concept of the
/// Lab that is still on disk inside the project.
fn locate<'a>(
    project_dir: &Path,
    directions: &'a [Direction],
    image: &ImageRef,
) -> CoreResult<(&'a Direction, usize)> {
    let (direction, pos) = match image {
        ImageRef::File(raw) => {
            let clean = check_source(project_dir, &relative(project_dir, raw))?;
            let mut parts = clean.split('/');
            let id = (parts.next() == Some(CONCEPTS_DIR))
                .then(|| parts.next())
                .flatten()
                .ok_or_else(|| {
                    CoreError::Invalid(format!("{clean} is not a concept of the Lab."))
                })?;
            let direction = directions
                .iter()
                .find(|d| d.id == id)
                .ok_or_else(|| CoreError::NotFound(format!("the direction {id}")))?;
            let pos = direction
                .images
                .iter()
                .position(|i| i.file == clean)
                .ok_or_else(|| CoreError::NotFound(format!("the concept {clean}")))?;
            (direction, pos)
        }
        ImageRef::Numbered { direction, index } => {
            let found = find_direction(directions, direction)
                .ok_or_else(|| CoreError::NotFound(format!("the direction {direction}")))?;
            let pos = found
                .images
                .iter()
                .position(|i| i.index == *index)
                .ok_or_else(|| {
                    CoreError::NotFound(format!("image {index} of the direction {}", found.name))
                })?;
            (found, pos)
        }
    };
    // The file recorded in direction.json must stay inside the project too.
    check_source(project_dir, &direction.images[pos].file)?;
    Ok((direction, pos))
}

/// Checks every request before anything is written.
fn plan(
    project_dir: &Path,
    directions: &[Direction],
    requests: &[PickRequest],
) -> CoreResult<Vec<Plan>> {
    let mut plans: Vec<Plan> = Vec::new();
    for request in requests {
        let (direction, pos) = locate(project_dir, directions, &request.image)?;
        let image = &direction.images[pos];
        if plans.iter().any(|p| p.file == image.file) {
            return Err(CoreError::Invalid(format!(
                "{} is asked for twice.",
                image.file
            )));
        }
        let objects = direction.is_objects();
        let default = if objects {
            image_title(direction, image)
        } else {
            direction.name.clone()
        };
        let given = request.name.as_deref().filter(|n| !n.trim().is_empty());
        let name = clean_name(given.unwrap_or(&default))?;
        let kind = request
            .kind
            .or(if objects { direction.kind } else { None })
            .unwrap_or(AssetKind::Prop);
        if objects && kind == AssetKind::Scene {
            return Err(CoreError::Invalid(
                "Object refs are props or characters, not scenes.".to_string(),
            ));
        }
        if objects
            && given.is_none()
            && plans
                .iter()
                .any(|p| p.direction == direction.id && p.name.eq_ignore_ascii_case(&name))
        {
            return Err(CoreError::Invalid(format!(
                "Two picks in {} are both called {name}. Give one of them a name.",
                direction.name
            )));
        }
        plans.push(Plan {
            direction: direction.id.clone(),
            file: image.file.clone(),
            index: image.index,
            star: request.star,
            named_ref: objects || slugify(&name) != slugify(&default),
            name,
            kind,
        });
    }
    Ok(plans)
}

/// Where a new copy of the concept goes: named after the asset for object
/// refs and own names (so Send to 3D finds it), else after the concept file
/// as Send picks to Refs does.
fn ref_for(project_dir: &Path, plan: &Plan) -> String {
    let from = Path::new(&plan.file);
    if plan.named_ref {
        let ext = from
            .extension()
            .map_or_else(|| "png".to_string(), |e| e.to_string_lossy().to_lowercase());
        ref_name(project_dir, &plan.name, &ext)
    } else {
        let name = from
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        format!("{REFS_DIR}/{name}")
    }
}

/// Stars, copies and marks the plans of one direction; returns each plan's
/// ref and whether it was copied now, by plan position.
fn write_direction(
    project_dir: &Path,
    id: &str,
    plans: &[Plan],
    out: &mut [Option<(String, bool)>],
) -> CoreResult<Direction> {
    let (direction, ()) = modify_direction(project_dir, id, |d| {
        for (n, plan) in plans.iter().enumerate().filter(|(_, p)| p.direction == id) {
            let image = d
                .images
                .iter_mut()
                .find(|i| i.file == plan.file)
                .ok_or_else(|| CoreError::NotFound(format!("the concept {}", plan.file)))?;
            if let Some(star) = plan.star {
                image.starred = star;
            }
            // Sent picks are cleared, as the Lab clears them after a send.
            image.picked = false;
            let kept = image
                .ref_file
                .clone()
                .filter(|r| project_dir.join(r).is_file());
            out[n] = Some(match kept {
                Some(rel) => (rel, false),
                None => {
                    let rel = ref_for(project_dir, plan);
                    let from = project_dir.join(&plan.file);
                    std::fs::copy(&from, project_dir.join(&rel))
                        .map_err(|e| CoreError::io(&from, e))?;
                    image.ref_file = Some(rel.clone());
                    (rel, true)
                }
            });
        }
        Ok(())
    })?;
    Ok(direction)
}

/// The asset an earlier pick of `ref_file` went to: the one showing it, or
/// the one it is named after.
fn linked_asset(manifest: &ProjectManifest, ref_file: &str) -> Option<String> {
    let stem = Path::new(ref_file)
        .file_stem()?
        .to_string_lossy()
        .into_owned();
    manifest
        .assets
        .iter()
        .find(|a| a.thumbnail.as_deref() == Some(ref_file))
        .or_else(|| {
            manifest
                .assets
                .iter()
                .find(|a| is_ref_of(&stem, &key(&a.id)))
        })
        .map(|a| a.name.clone())
}

/// Picks the images: each is copied into `refs/` and its asset is put on
/// the Board at Refs (an existing one moved up from Concept, or a new one).
/// Every request is checked before anything is written.
///
/// # Errors
///
/// Fails when an image is not a concept of the project, a name or kind is
/// refused, or a file cannot be copied.
pub fn pick_images(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    requests: &[PickRequest],
) -> CoreResult<Picked> {
    check_count(requests.len())?;
    project::load(workspace_root, layout, project_id)?;
    let project_dir = project::dir(workspace_root, layout, project_id)?;
    let plans = plan(&project_dir, &list_directions(&project_dir), requests)?;
    let refs = project_dir.join(REFS_DIR);
    std::fs::create_dir_all(&refs).map_err(|e| CoreError::io(&refs, e))?;
    let mut ids: Vec<&str> = Vec::new();
    for p in &plans {
        if !ids.contains(&p.direction.as_str()) {
            ids.push(&p.direction);
        }
    }
    let mut sent: Vec<Option<(String, bool)>> = vec![None; plans.len()];
    let directions = ids
        .iter()
        .map(|id| write_direction(&project_dir, id, &plans, &mut sent))
        .collect::<CoreResult<Vec<_>>>()?;
    let mut images = Vec::new();
    let manifest = project::modify(workspace_root, layout, project_id, |manifest| {
        images.clear();
        let mut pictured: Vec<String> = Vec::new();
        for (plan, done) in plans.iter().zip(&sent) {
            let Some((rel, copied)) = done.clone() else {
                continue;
            };
            let earlier = (!copied).then(|| linked_asset(manifest, &rel)).flatten();
            let asset = place_at_refs(
                manifest,
                earlier.as_deref().unwrap_or(&plan.name),
                plan.kind,
            )?;
            // The card shows the first new pick of this call; a repeat
            // leaves the card as it is.
            if (copied || asset.thumbnail.is_none()) && !pictured.contains(&asset.id) {
                assets::set_ref_thumbnail(manifest, &asset.id, &rel)?;
                pictured.push(asset.id.clone());
            }
            images.push(PickedImage {
                direction: plan.direction.clone(),
                file: plan.file.clone(),
                index: plan.index,
                ref_file: rel,
                copied,
                asset: assets::find(manifest, &asset.id)?.clone(),
            });
        }
        Ok(())
    })?;
    // A later pick of the same asset may have changed it after it was read.
    for image in &mut images {
        if let Ok(now) = assets::find(&manifest, &image.asset.id) {
            image.asset = now.clone();
        }
    }
    Ok(Picked {
        images,
        directions,
        project: manifest,
    })
}

/// Stars or unstars images, as the Lab's Star does. Every image is checked
/// before any changes; returns each changed direction as saved.
///
/// # Errors
///
/// Fails when an image is not a concept of the project.
pub fn star_images(project_dir: &Path, stars: &[(ImageRef, bool)]) -> CoreResult<Vec<Direction>> {
    check_count(stars.len())?;
    let directions = list_directions(project_dir);
    let found = stars
        .iter()
        .map(|(image, star)| {
            locate(project_dir, &directions, image)
                .map(|(d, pos)| (d.id.clone(), d.images[pos].file.clone(), *star))
        })
        .collect::<CoreResult<Vec<_>>>()?;
    let mut changed: Vec<Direction> = Vec::new();
    for (id, file, star) in found {
        let saved = mark(project_dir, &id, &file, Some(star), None)?;
        changed.retain(|d| d.id != saved.id);
        changed.push(saved);
    }
    Ok(changed)
}

#[cfg(test)]
mod tests;
