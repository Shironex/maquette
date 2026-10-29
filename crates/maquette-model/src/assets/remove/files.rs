//! Which files in a project are one asset's own, and moving them between
//! the project and its trash folder.
//!
//! A file another asset still uses stays: a ref it was sent with or shows
//! as its picture, a model file its version names, a built helper its
//! script imports.

use std::fs;
use std::io;
use std::path::{Component, Path};

use super::Sent;
use crate::assets::copy::{is_ref_of, key, refs_of, thumb_of};
use crate::assets::{Asset, HANDOFF_DIR};
use crate::built::BuiltFile;
use crate::error::{CoreError, CoreResult};
use crate::fs::move_tree;
use crate::project::ProjectManifest;

/// A relative path made only of plain names, with `/` separators: the only
/// kind a remove moves or a restore writes back.
pub(super) fn is_plain(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.contains('\\')
        && Path::new(rel)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

/// What other assets of the project still use.
struct Others<'a> {
    assets: Vec<&'a Asset>,
    sent: Vec<&'a str>,
}

impl Others<'_> {
    fn use_ref(&self, rel: &str) -> bool {
        let stem = Path::new(rel)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.sent.contains(&rel)
            || self
                .assets
                .iter()
                .any(|a| a.thumbnail.as_deref() == Some(rel) || is_ref_of(&stem, &key(&a.id)))
    }

    fn use_model(&self, rel: &str) -> bool {
        self.assets.iter().any(|a| {
            a.versions
                .iter()
                .any(|v| v.file == rel || v.file.starts_with(&format!("{rel}/")))
        })
    }
}

/// The model file of a version, or the folder an `.obj` version keeps
/// with its companions (`models/<key>_v2/lamp.obj` moves `models/<key>_v2`).
fn version_path(file: &str, key: &str) -> String {
    let parts: Vec<&str> = file.split('/').collect();
    match parts.as_slice() {
        [models, folder, _] if *models == "models" && folder.starts_with(key) => {
            format!("{models}/{folder}")
        }
        _ => file.to_string(),
    }
}

/// The paths, relative to the project folder, that are the asset's own and
/// are there: its model versions, thumbnails, refs, handoff folders and
/// built script and helpers. Each once, files before folders.
pub(super) fn of_asset(
    project_dir: &Path,
    manifest: &ProjectManifest,
    asset: &Asset,
    jobs: &[Sent],
    built: &BuiltFile,
) -> Vec<String> {
    let own_key = key(&asset.id);
    let others = Others {
        assets: manifest
            .assets
            .iter()
            .filter(|a| a.id != asset.id)
            .collect(),
        sent: jobs
            .iter()
            .filter(|j| j.asset != asset.id)
            .flat_map(|j| j.images.iter().map(String::as_str))
            .collect(),
    };
    let sent: Vec<String> = jobs
        .iter()
        .filter(|j| j.asset == asset.id)
        .flat_map(|j| j.images.iter().cloned())
        .collect();

    let mut paths: Vec<String> = Vec::new();
    for v in &asset.versions {
        let path = version_path(&v.file, &own_key);
        if !others.use_model(&path) {
            paths.push(path);
        }
        paths.push(thumb_of(&asset.id, v.version));
    }
    if let Some(thumb) = asset
        .thumbnail
        .as_deref()
        .filter(|t| t.starts_with("thumbs/"))
        && !others
            .assets
            .iter()
            .any(|a| a.thumbnail.as_deref() == Some(thumb))
    {
        paths.push(thumb.to_string());
    }
    paths.extend(
        refs_of(project_dir, asset, &sent)
            .into_iter()
            .filter(|r| !others.use_ref(r)),
    );
    if let Some(source) = built.assets.get(&asset.id) {
        let used_elsewhere = |file: &str| {
            built
                .assets
                .iter()
                .filter(|(id, _)| *id != &asset.id)
                .any(|(_, s)| s.script == file || s.helpers.iter().any(|h| h == file))
        };
        paths.extend(
            std::iter::once(&source.script)
                .chain(&source.helpers)
                .filter(|f| !used_elsewhere(f))
                .cloned(),
        );
    }
    for job in jobs.iter().filter(|j| j.asset == asset.id) {
        paths.push(format!("{HANDOFF_DIR}/{}", job.id));
    }

    let mut seen = Vec::new();
    paths.retain(|p| {
        let keep = is_plain(p) && !seen.contains(p) && project_dir.join(p).exists();
        seen.push(p.clone());
        keep
    });
    paths
}

/// Moves one file or folder, making its parent folders. A rename when it
/// can, else a copy and a removal.
fn move_path(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", to.display()),
        ));
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    if from.is_dir() {
        return move_tree(from, to);
    }
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to)?;
    if let Err(err) = fs::remove_file(from) {
        let _ = fs::remove_file(to);
        return Err(err);
    }
    Ok(())
}

/// Moves each of `paths` from under `from` to the same place under `to`.
/// When one fails, the ones already moved go back and nothing has changed.
pub(super) fn move_all(from: &Path, to: &Path, paths: &[String]) -> CoreResult<()> {
    let mut done: Vec<&String> = Vec::new();
    for rel in paths {
        if let Err(err) = move_path(&from.join(rel), &to.join(rel)) {
            for back in done.iter().rev() {
                if let Err(undo) = move_path(&to.join(back), &from.join(back)) {
                    tracing::warn!(error = %undo, path = %back, "could not move a file back");
                }
            }
            return Err(CoreError::io(from.join(rel), err));
        }
        done.push(rel);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_relative_paths_move() {
        assert!(is_plain("refs/clay_bowl.png"));
        assert!(is_plain("handoff/clay-bowl-1"));
        assert!(!is_plain("../outside.png"));
        assert!(!is_plain("/etc/hosts"));
        assert!(!is_plain("refs/../../x"));
        assert!(!is_plain("refs\\x.png"));
        assert!(!is_plain(""));
    }

    #[test]
    fn an_obj_version_moves_its_folder() {
        assert_eq!(
            version_path("models/lamp_v2/lamp.obj", "lamp"),
            "models/lamp_v2"
        );
        assert_eq!(
            version_path("models/lamp_v2.glb", "lamp"),
            "models/lamp_v2.glb"
        );
        assert_eq!(
            version_path("models/other_v1/x.obj", "lamp"),
            "models/other_v1/x.obj"
        );
    }
}
