//! Images someone already has, dropped on the Board's Concept column.
//! They become a direction of the Lab, so starring, picking, Make object
//! refs and Send picks to Refs work on them like on generated images, and
//! a card of the same name at Concept, which a send then moves up.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::picks::capitalised;
use super::{
    CONCEPTS_DIR, ConceptImage, DIRECTION_FILE, Direction, RESERVED, direction_dir, direction_id,
    load_direction, next_free_index, now_rfc3339, save_direction, write_lock,
};
use crate::assets::{self, Asset, AssetKind, NewAsset, Stage};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest};
use crate::workspace::FolderLayout;

/// What importing images did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportedConcepts {
    pub direction: Direction,
    pub asset: Asset,
    /// The new files, relative to the project folder.
    pub added: Vec<String>,
    /// The project with the card added, for the Board.
    pub project: ProjectManifest,
}

const FALLBACK_NAME: &str = "Dropped images";

/// One image, as for a built asset's files.
pub const MAX_IMAGE_BYTES: u64 = crate::built::MAX_FILE_BYTES;
pub const MAX_IMAGES_PER_DROP: usize = 50;
/// One drop, as for a model handed back by hand.
pub const MAX_DROP_BYTES: u64 = 512 * 1024 * 1024;
/// As much of a file as telling its type reads.
const HEADER_BYTES: u64 = 12;

const MB: u64 = 1024 * 1024;

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The extension an image's first bytes say, whatever its name claims.
fn image_extension(head: &[u8]) -> Option<&'static str> {
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP") {
        Some("webp")
    } else {
        None
    }
}

/// The extension and size of a file an import may take. Reads the start of
/// a regular file within the limit, and nothing of anything else. Only the
/// header counts: valid images carry bytes after their end, like a phone's
/// Motion Photo, and a damaged one shows as a broken tile.
fn check_image(path: &Path) -> CoreResult<(&'static str, u64)> {
    let name = file_name(path);
    let meta = fs::metadata(path).map_err(|e| CoreError::io(path, e))?;
    if !meta.is_file() {
        return Err(CoreError::Invalid(format!("{name} is not a file.")));
    }
    let size = meta.len();
    if size > MAX_IMAGE_BYTES {
        return Err(CoreError::Invalid(format!(
            "{name} is {} MB. An image can be {} MB at most.",
            size.div_ceil(MB),
            MAX_IMAGE_BYTES / MB
        )));
    }
    let file = fs::File::open(path).map_err(|e| CoreError::io(path, e))?;
    let mut head = Vec::new();
    file.take(HEADER_BYTES)
        .read_to_end(&mut head)
        .map_err(|e| CoreError::io(path, e))?;
    let Some(extension) = image_extension(&head) else {
        return Err(CoreError::Invalid(format!(
            "{name} is not a PNG, JPEG or WebP image."
        )));
    };
    Ok((extension, size))
}

/// Every file checked before anything is written, and the limits of one drop.
fn check_drop(paths: &[PathBuf]) -> CoreResult<Vec<&'static str>> {
    if paths.is_empty() {
        return Err(CoreError::Invalid("Choose at least one image.".to_string()));
    }
    if paths.len() > MAX_IMAGES_PER_DROP {
        return Err(CoreError::Invalid(format!(
            "That is {} files. A drop can be {MAX_IMAGES_PER_DROP} images at most.",
            paths.len()
        )));
    }
    let mut total = 0;
    let mut extensions = Vec::with_capacity(paths.len());
    for path in paths {
        let meta_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if meta_size <= MAX_IMAGE_BYTES {
            total += meta_size;
        }
        if total > MAX_DROP_BYTES {
            return Err(CoreError::Invalid(format!(
                "These images come to more than {} MB, the most for one drop.",
                MAX_DROP_BYTES / MB
            )));
        }
    }
    for path in paths {
        extensions.push(check_image(path)?.0);
    }
    Ok(extensions)
}

/// `stone_lantern-02.png` reads `Stone lantern 02`, cut to fit a card's name.
fn name_from(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().replace(['_', '-'], " "))
        .unwrap_or_default();
    let name: String = capitalised(&stem)
        .chars()
        .take(assets::ASSET_NAME_MAX_CHARS)
        .collect();
    if name.chars().any(char::is_alphanumeric) {
        name.trim().to_string()
    } else {
        FALLBACK_NAME.to_string()
    }
}

/// How many other names to try before saying so.
const MAX_NAME_TRIES: u32 = 100;
/// Short enough for ` 100` to change the slug and fit a card's name.
const SUFFIXED_NAME_CHARS: usize = 50;

/// `name`, or `name 2` and on while its folder holds object refs, refs work
/// or a direction of another name: those stay as they are.
fn free_name(project_dir: &Path, name: &str) -> CoreResult<String> {
    let same = |a: &str, b: &str| a.trim().eq_ignore_ascii_case(b.trim());
    let short: String = name.chars().take(SUFFIXED_NAME_CHARS).collect();
    let mut candidate = name.to_string();
    for n in 2..=MAX_NAME_TRIES {
        match load_direction(project_dir, &direction_id(&candidate))? {
            Some(found)
                if found.is_objects()
                    || found.refs_job.is_some()
                    || !same(&found.name, &candidate) =>
            {
                candidate = format!("{} {n}", short.trim());
            }
            _ => return Ok(candidate),
        }
    }
    Err(CoreError::Invalid(format!(
        "Every direction name like {name} is taken. Rename the file and drop it again."
    )))
}

/// Takes `count` indexes after every image the direction has, is making or
/// left behind, so a running job and this import never meet on one file.
fn reserve_indexes(project_dir: &Path, id: &str, known_next: u32, count: u32) -> u32 {
    let folder = project_dir.join(CONCEPTS_DIR).join(id);
    let mut reserved = RESERVED.lock().unwrap_or_else(|p| p.into_inner());
    let first = known_next
        .max(next_free_index(project_dir, id))
        .max(reserved.get(&folder).copied().unwrap_or(0) + 1);
    reserved.insert(folder, first + count - 1);
    first
}

/// Saves the direction with the new images in one write, making it when it
/// is new, and says whether it did. Until this runs the Lab knows nothing of
/// them.
fn record(
    project_dir: &Path,
    id: &str,
    name: &str,
    added: &[(u32, String)],
) -> CoreResult<(Direction, bool)> {
    let _guard = write_lock();
    let now = now_rfc3339();
    let known = load_direction(project_dir, id)?;
    let created = known.is_none();
    let mut direction = known.unwrap_or_else(|| Direction {
        id: id.to_string(),
        name: name.to_string(),
        created_at: now.clone(),
        // No board shaped these images.
        style_off: true,
        ..Direction::default()
    });
    for (index, file) in added {
        let (width, height) = fs::read(project_dir.join(file))
            .ok()
            .and_then(|bytes| crate::inbox::images::dimensions(&bytes))
            .map_or((None, None), |(w, h)| (Some(w), Some(h)));
        direction.images.push(ConceptImage {
            file: file.clone(),
            index: *index,
            width,
            height,
            starred: false,
            picked: false,
            ref_file: None,
            created_at: now.clone(),
            title: None,
        });
    }
    direction.images.sort_by_key(|i| i.index);
    direction.updated_at = now;
    save_direction(project_dir, &direction)?;
    Ok((direction, created))
}

/// Copies through a temp name, so a failed copy leaves no partial image.
fn copy_whole(from: &Path, to: &Path) -> io::Result<u64> {
    let mut temp = to.as_os_str().to_owned();
    temp.push(".part");
    let temp = PathBuf::from(temp);
    let copied = fs::copy(from, &temp).and_then(|size| fs::rename(&temp, to).map(|()| size));
    if copied.is_err() {
        let _ = fs::remove_file(&temp);
    }
    copied
}

/// Takes back what this import wrote after a later step failed: its files,
/// its images in the direction, the direction if it `created` it and nothing
/// else came to it, and the folder if that leaves it empty. Another import
/// may be using the folder, so nothing else goes, and one failed step does
/// not stop the rest.
fn undo(project_dir: &Path, id: &str, added: &[(u32, String)], created: bool) {
    let warn = |error: &dyn std::fmt::Display| {
        tracing::warn!(%error, direction = id, "could not undo a failed image import");
    };
    let _guard = write_lock();
    for (_, file) in added {
        let path = project_dir.join(file);
        if let Err(error) = fs::remove_file(&path)
            && error.kind() != io::ErrorKind::NotFound
        {
            warn(&CoreError::io(&path, error));
        }
    }
    let folder = match direction_dir(project_dir, id) {
        Ok(folder) => folder,
        Err(error) => return warn(&error),
    };
    match load_direction(project_dir, id) {
        Ok(Some(mut direction)) => {
            direction
                .images
                .retain(|i| !added.iter().any(|(_, f)| *f == i.file));
            let saved = if created && direction.images.is_empty() {
                let path = folder.join(DIRECTION_FILE);
                fs::remove_file(&path).map_err(|e| CoreError::io(&path, e))
            } else {
                save_direction(project_dir, &direction)
            };
            if let Err(error) = saved {
                warn(&error);
            }
        }
        Ok(None) => {}
        Err(error) => warn(&error),
    }
    // Only an empty folder goes; a folder with anything in it refuses.
    let _ = fs::remove_dir(&folder);
}

/// The card called `name`: one already there stays as it is, and a new one
/// at Concept shows `picture`. Only that new card gets a picture from
/// `concepts/`, which is how a send knows its kind was a guess.
fn place_card(manifest: &mut ProjectManifest, name: &str, picture: &str) -> CoreResult<Asset> {
    if let Some(found) = assets::find_by_name(manifest, name) {
        return Ok(found.clone());
    }
    let new = NewAsset {
        name: name.to_string(),
        kind: AssetKind::Prop,
        stage: Stage::Concept,
    };
    let id = assets::add(manifest, new)?.id;
    assets::set_ref_thumbnail(manifest, &id, picture)?;
    Ok(assets::find(manifest, &id)?.clone())
}

/// Copies images into a direction named after the first one and puts its
/// card on the Board. Nothing is copied unless every file is a PNG, JPEG or
/// WebP image, whatever its extension says.
pub fn import_images(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    paths: &[PathBuf],
) -> CoreResult<ImportedConcepts> {
    let project_dir = project::dir(workspace_root, layout, project_id)?;
    let extensions = check_drop(paths)?;
    let name = free_name(&project_dir, &name_from(&paths[0]))?;
    let id = direction_id(&name);
    let folder = direction_dir(&project_dir, &id)?;
    let known_next = load_direction(&project_dir, &id)?
        .as_ref()
        .map_or(1, Direction::next_index);
    let start = reserve_indexes(&project_dir, &id, known_next, paths.len() as u32);
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    let mut added = Vec::with_capacity(paths.len());
    for (index, (from, extension)) in (start..).zip(paths.iter().zip(extensions)) {
        let file = format!("{CONCEPTS_DIR}/{id}/{id}_{index:02}.{extension}");
        if let Err(error) = copy_whole(from, &project_dir.join(&file)) {
            undo(&project_dir, &id, &added, false);
            return Err(CoreError::io(from, error));
        }
        added.push((index, file));
    }
    let mut created = false;
    let placed = record(&project_dir, &id, &name, &added).and_then(|(direction, made)| {
        created = made;
        let mut asset = None;
        let manifest = project::modify(workspace_root, layout, project_id, |manifest| {
            asset = Some(place_card(manifest, &direction.name, &added[0].1)?);
            Ok(())
        })?;
        let asset =
            asset.ok_or_else(|| CoreError::Unavailable("The asset was not saved.".into()))?;
        Ok((direction, asset, manifest))
    });
    match placed {
        Ok((direction, asset, manifest)) => Ok(ImportedConcepts {
            direction,
            asset,
            added: added.into_iter().map(|(_, file)| file).collect(),
            project: manifest,
        }),
        Err(error) => {
            undo(&project_dir, &id, &added, created);
            Err(error)
        }
    }
}

pub use refs::import_refs;

mod refs;

#[cfg(test)]
mod tests;
