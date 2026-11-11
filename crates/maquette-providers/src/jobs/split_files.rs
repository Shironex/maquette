//! Where a split's or a re-texture's files live: its own folder in the
//! workspace cache, the copy of the model it went out with, and the file of
//! a version.
//!
//! The folder is `<cache>/sends/<project>/<job id>/`: ids are numbered per
//! project, so the project is part of the path. A job from the older
//! numbering, whose id no other job of the workspace had, keeps its files
//! where it put them, in `<cache>/splits/<job id>/`. Nothing is moved.

use std::path::PathBuf;

use super::ids::IdScope;
use super::{Stored, Workspace};
use maquette_model::assets;
use maquette_model::project;

/// The copy of the model a split cuts, inside its job's folder.
pub(super) const SOURCE_FILE: &str = "source.glb";
/// The cache folder of the jobs numbered per project.
const FILES_DIR: &str = "sends";
/// The cache folder of the jobs from the older numbering.
const OLDER_DIR: &str = "splits";

/// The folder a job sent now gets: that of the job `id` of `project`.
pub(super) fn new_dir(workspace: &Workspace, project: &str, id: &str) -> PathBuf {
    workspace
        .root
        .join(&workspace.config.layout.cache)
        .join(FILES_DIR)
        .join(project)
        .join(id)
}

/// The job's own folder in the workspace cache. A job from the older
/// numbering has it under its id alone, unless a folder of its project and
/// id is there: only this job can have made that one.
pub(super) fn files_dir(workspace: &Workspace, stored: &Stored) -> PathBuf {
    let job = &stored.job;
    let own = new_dir(workspace, &job.project, &job.id);
    if stored.scope == IdScope::Project || own.is_dir() {
        return own;
    }
    workspace
        .root
        .join(&workspace.config.layout.cache)
        .join(OLDER_DIR)
        .join(&job.id)
}

/// The model a split cuts, as it was when it went out; the version it
/// came from when the job has no copy.
pub(super) fn source_of(workspace: &Workspace, stored: &Stored) -> Option<PathBuf> {
    let copy = files_dir(workspace, stored).join(SOURCE_FILE);
    if copy.is_file() {
        return Some(copy);
    }
    let job = &stored.job;
    let version = job.split.as_ref()?.from_version?;
    version_file(workspace, &job.project, &job.asset, version)
}

/// The file of one version of an asset.
pub(super) fn version_file(
    workspace: &Workspace,
    project_id: &str,
    asset: &str,
    version: u32,
) -> Option<PathBuf> {
    let manifest = project::load(&workspace.root, &workspace.config.layout, project_id).ok()?;
    let found = assets::find(&manifest, asset).ok()?;
    let file = &found.versions.iter().find(|v| v.version == version)?.file;
    Some(workspace.project_dir(project_id).ok()?.join(file))
}
