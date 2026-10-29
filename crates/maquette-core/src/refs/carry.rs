//! Copying a ref from one project's `refs/` to another's, as copying an
//! asset, moving a Lab direction and using a Library entry do.

use std::fs;
use std::path::Path;

use super::REFS_DIR;
use crate::error::{CoreError, CoreResult};

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
