//! Copying model versions, pictures and refs between a project and a
//! library entry. Both are folders with `models/`, `thumbs/` and `refs/`,
//! so one set of helpers serves both ways.

use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::assets::copy::copy_file;
use crate::concepts::transfer::carry_ref;
use crate::error::{CoreError, CoreResult};
use crate::game::ship::files_under;

/// The model folder both sides keep versions in.
const MODELS: &str = "models";

/// The folder a version file sits in when it has one of its own: an
/// imported `.obj` lives in `models/crate_v1/` with its `.mtl` and
/// textures. `None` for a file straight in `models/`.
fn own_folder(file: &str) -> Option<&str> {
    let (folder, _) = file.rsplit_once('/')?;
    (folder != MODELS).then_some(folder)
}

/// The model file's SHA-256 as hex.
pub(super) fn hash(path: &Path) -> CoreResult<String> {
    let bytes = fs::read(path).map_err(|e| CoreError::io(path, e))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn ext_of(file: &str) -> String {
    Path::new(file)
        .extension()
        .map_or_else(|| "glb".to_string(), |e| e.to_string_lossy().to_lowercase())
}

/// Where version `version` of `key` goes: `models/desk_lamp_v2.glb`, or
/// `models/crate_v2/crate.obj` for a model with a folder of its own.
pub(super) fn version_path(file: &str, key: &str, version: u32) -> String {
    let base = format!("{MODELS}/{key}_v{version}");
    match (own_folder(file), file.rsplit_once('/')) {
        (Some(_), Some((_, name))) => format!("{base}/{name}"),
        _ => format!("{base}.{}", ext_of(file)),
    }
}

/// Copies the version `file` of `from` into `to` at [`version_path`],
/// with its folder for a model that has one. Refuses to write over
/// anything, so an older file is never replaced. Returns the new path,
/// relative to `to`.
pub(super) fn copy_version(
    from: &Path,
    file: &str,
    to: &Path,
    key: &str,
    version: u32,
) -> CoreResult<String> {
    let source = from.join(file);
    if !source.is_file() {
        return Err(CoreError::NotFound(format!("the model file {file}")));
    }
    let target = version_path(file, key, version);
    let landing = match own_folder(&target) {
        Some(folder) => to.join(folder),
        None => to.join(&target),
    };
    if landing.exists() {
        return Err(CoreError::Invalid(format!(
            "{} is already there. Move it away, then try again.",
            landing.strip_prefix(to).unwrap_or(&landing).display()
        )));
    }
    match (own_folder(file), own_folder(&target)) {
        (Some(src), Some(dst)) => {
            for rel in files_under(&from.join(src)) {
                copy_file(&from.join(src).join(&rel), &to.join(dst).join(&rel))?;
            }
        }
        _ => copy_file(&source, &to.join(&target))?,
    }
    Ok(target)
}

/// Removes what [`copy_version`] wrote, after a later step failed.
pub(super) fn remove_version(to: &Path, target: &str) {
    let path = match own_folder(target) {
        Some(folder) => to.join(folder),
        None => to.join(target),
    };
    let gone = if path.is_dir() {
        fs::remove_dir_all(&path)
    } else {
        fs::remove_file(&path)
    };
    if let Err(err) = gone {
        tracing::warn!(file = %path.display(), error = %err, "could not remove a copied model");
    }
}

/// Copies a picture: a render in `thumbs/` becomes `thumbs/<key>_v<n>`, a
/// ref picture goes into `refs/` like any ref. Returns its new path, or
/// `None` when the file is not there.
pub(super) fn copy_picture(
    from: &Path,
    picture: &str,
    to: &Path,
    key: &str,
    version: u32,
) -> CoreResult<Option<String>> {
    if !stays(picture) || !from.join(picture).is_file() {
        return Ok(None);
    }
    if picture.starts_with("refs/") {
        return carry_ref(from, to, picture);
    }
    let target = format!("thumbs/{key}_v{version}.{}", ext_of(picture));
    copy_file(&from.join(picture), &to.join(&target))?;
    Ok(Some(target))
}

/// Copies ref images into `to`'s `refs/`, reusing a file with the same
/// bytes. Returns where each landed, skipping the ones that are gone.
pub(super) fn copy_refs(from: &Path, refs: &[String], to: &Path) -> CoreResult<Vec<String>> {
    let mut landed = Vec::new();
    for rel in refs.iter().filter(|r| stays(r)) {
        if let Some(new) = carry_ref(from, to, rel)?
            && !landed.contains(&new)
        {
            landed.push(new);
        }
    }
    Ok(landed)
}

fn stays(path: &str) -> bool {
    crate::game::stays_inside(path)
}
