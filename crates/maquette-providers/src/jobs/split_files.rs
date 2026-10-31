//! Where a split's or a re-texture's files live: its own folder in the
//! workspace cache, the copy of the model it went out with, and the file of
//! a version.

use std::path::PathBuf;

use super::{ProviderJob, Workspace};
use maquette_model::assets;
use maquette_model::project;

/// The copy of the model a split cuts, inside its job's folder.
pub(super) const SOURCE_FILE: &str = "source.glb";

/// The split's own folder in the workspace cache.
pub(super) fn split_dir(workspace: &Workspace, id: &str) -> PathBuf {
    workspace
        .root
        .join(&workspace.config.layout.cache)
        .join("splits")
        .join(id)
}

/// The model a split cuts, as it was when it went out; the version it
/// came from when an older job has no copy.
pub(super) fn source_of(workspace: &Workspace, job: &ProviderJob) -> Option<PathBuf> {
    let copy = split_dir(workspace, &job.id).join(SOURCE_FILE);
    if copy.is_file() {
        return Some(copy);
    }
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
