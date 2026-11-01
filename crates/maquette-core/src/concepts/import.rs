//! Images someone already has, dropped on the Board's Concept column.
//! They become a direction of the Lab, so starring, picking, Make object
//! refs and Send picks to Refs work on them like on generated images, and
//! a card of the same name at Concept, which a send then moves up.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::picks::capitalised;
use super::{
    CONCEPTS_DIR, Direction, RESERVED, add_image, direction_dir, direction_id, load_direction,
    next_free_index, now_rfc3339, save_direction, write_lock,
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
/// As far from either end as validating reads, however large the file.
const CHECK_BYTES: u64 = 4096;

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

/// True when the file ends where its format says. There is no image decoder
/// in this workspace, so damage in the middle only shows when it is opened.
fn is_whole(extension: &str, head: &[u8], tail: &[u8], size: u64) -> bool {
    match extension {
        "png" => tail.len() >= 8 && &tail[tail.len() - 8..tail.len() - 4] == b"IEND",
        // Some cameras pad after the end marker, so anywhere near the end will do.
        "jpg" => tail.windows(2).any(|w| w == [0xFF, 0xD9]),
        _ => head
            .get(4..8)
            .and_then(|b| b.try_into().ok())
            .is_some_and(|b| u64::from(u32::from_le_bytes(b)) + 8 <= size),
    }
}

/// The extension and size of a file an import may take. Reads both ends of
/// a regular file within the limit, and nothing of anything else.
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
    let mut file = fs::File::open(path).map_err(|e| CoreError::io(path, e))?;
    let mut read = |from: u64, len: u64| -> CoreResult<Vec<u8>> {
        file.seek(SeekFrom::Start(from))
            .map_err(|e| CoreError::io(path, e))?;
        let mut bytes = Vec::new();
        (&mut file)
            .take(len)
            .read_to_end(&mut bytes)
            .map_err(|e| CoreError::io(path, e))?;
        Ok(bytes)
    };
    let head = read(0, CHECK_BYTES)?;
    let tail = read(size.saturating_sub(CHECK_BYTES), CHECK_BYTES)?;
    let Some(extension) = image_extension(&head) else {
        return Err(CoreError::Invalid(format!(
            "{name} is not a PNG, JPEG or WebP image."
        )));
    };
    if !is_whole(extension, &head, &tail, size) {
        return Err(CoreError::Invalid(format!(
            "{name} looks cut off. Save or download it again."
        )));
    }
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

/// The direction called `name` as it is, or a new one with no brief.
fn open_direction(project_dir: &Path, name: &str) -> CoreResult<Direction> {
    let id = direction_id(name);
    direction_dir(project_dir, &id)?;
    let _guard = write_lock();
    if let Some(found) = load_direction(project_dir, &id)? {
        return Ok(found);
    }
    let now = now_rfc3339();
    let direction = Direction {
        id,
        name: name.to_string(),
        created_at: now.clone(),
        updated_at: now,
        // No board shaped these images.
        style_off: true,
        ..Direction::default()
    };
    save_direction(project_dir, &direction)?;
    Ok(direction)
}

/// Takes `count` indexes after every image the direction has, is making or
/// left behind, so a running job and this import never meet on one file.
fn reserve_indexes(project_dir: &Path, direction: &Direction, count: u32) -> u32 {
    let folder = project_dir.join(CONCEPTS_DIR).join(&direction.id);
    let mut reserved = RESERVED.lock().unwrap_or_else(|p| p.into_inner());
    let first = direction
        .next_index()
        .max(next_free_index(project_dir, &direction.id))
        .max(reserved.get(&folder).copied().unwrap_or(0) + 1);
    reserved.insert(folder, first + count - 1);
    first
}

/// The card called `name` at Concept showing `picture`. A card of that
/// name stays at its stage, and keeps a picture it already has.
fn place_card(manifest: &mut ProjectManifest, name: &str, picture: &str) -> CoreResult<Asset> {
    let id = match assets::find_by_name(manifest, name) {
        Some(found) => found.id.clone(),
        None => {
            let new = NewAsset {
                name: name.to_string(),
                kind: AssetKind::Prop,
                stage: Stage::Concept,
            };
            assets::add(manifest, new)?.id
        }
    };
    if assets::find(manifest, &id)?.thumbnail.is_none() {
        assets::set_ref_thumbnail(manifest, &id, picture)?;
    }
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
    let direction = open_direction(&project_dir, &name)?;
    let id = direction.id.clone();
    let start = reserve_indexes(&project_dir, &direction, paths.len() as u32);
    let folder = direction_dir(&project_dir, &id)?;
    fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
    let mut added = Vec::with_capacity(paths.len());
    let mut direction = direction;
    for (index, (from, extension)) in (start..).zip(paths.iter().zip(extensions)) {
        let file = format!("{CONCEPTS_DIR}/{id}/{id}_{index:02}.{extension}");
        fs::copy(from, project_dir.join(&file)).map_err(|e| CoreError::io(from, e))?;
        direction = add_image(&project_dir, &id, index, &file, None)?;
        added.push(file);
    }
    let mut asset = None;
    let manifest = project::modify(workspace_root, layout, project_id, |manifest| {
        asset = Some(place_card(manifest, &direction.name, &added[0])?);
        Ok(())
    })?;
    let asset = asset.ok_or_else(|| CoreError::Unavailable("The asset was not saved.".into()))?;
    Ok(ImportedConcepts {
        direction,
        asset,
        added,
        project: manifest,
    })
}

#[cfg(test)]
mod tests;
