//! Adding a model made in Blender (a texture bake back, an assembled prop)
//! to an asset as its next version, the way an Inbox accept does: the file
//! goes to `models/<asset>_v<n>.glb` and the asset waits for review in the
//! Blender column. The made file is copied; it stays where it was made.

use std::fs;
use std::path::Path;

use crate::assets::{self, Asset, AssetKind, AssetStatus, AssetVersion, NewAsset, Stage};
use crate::error::{CoreError, CoreResult};
use crate::inbox::{analyse, stats_of};
use crate::project::{self, ProjectManifest};
use crate::workspace::FolderLayout;

/// Which asset a made model goes to.
pub(crate) enum Target<'a> {
    /// An existing asset, by id.
    Asset(&'a str),
    /// The asset named so, created when there is none.
    Named { name: &'a str, kind: AssetKind },
}

/// What adding the version did.
#[derive(Debug, Clone)]
pub(crate) struct Landed {
    pub project: ProjectManifest,
    pub asset: Asset,
    pub version: AssetVersion,
}

/// Copies `model` into the project as the next version of `target`.
///
/// # Errors
///
/// Fails for an unknown project or asset, a model that is not glTF, or
/// when the file cannot be copied.
pub(crate) fn add(
    root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    target: Target<'_>,
    model: &Path,
) -> CoreResult<Landed> {
    let stats = Some(stats_of(&analyse::analyse(model)?));
    let models = project::dir(root, layout, project_id)?.join("models");
    let mut made: Option<(Asset, AssetVersion)> = None;
    let manifest = project::modify(root, layout, project_id, |manifest| {
        let id = match target {
            Target::Asset(id) => assets::find(manifest, id)?.id.clone(),
            Target::Named { name, kind } => match assets::find_by_name(manifest, name) {
                Some(asset) => asset.id.clone(),
                None => {
                    assets::add(
                        manifest,
                        NewAsset {
                            name: name.to_string(),
                            kind,
                            stage: Stage::Blender,
                        },
                    )?
                    .id
                }
            },
        };
        let number = assets::find(manifest, &id)?.next_version();
        let file_name = format!("{}_v{number}.glb", id.replace('-', "_"));
        let path = models.join(&file_name);
        if path.exists() {
            return Err(CoreError::Invalid(format!(
                "models/{file_name} already exists. Rename it or remove it first."
            )));
        }
        fs::create_dir_all(&models).map_err(|e| CoreError::io(&models, e))?;
        fs::copy(model, &path).map_err(|e| CoreError::io(&path, e))?;
        let version = assets::add_version(manifest, &id, format!("models/{file_name}"), stats)?;
        if assets::find(manifest, &id)?.stage < Stage::Blender {
            assets::move_to(manifest, &id, Stage::Blender)?;
        }
        assets::set_status(manifest, &id, AssetStatus::Review)?;
        made = Some((assets::find(manifest, &id)?.clone(), version));
        Ok(())
    })?;
    let (asset, version) =
        made.ok_or_else(|| CoreError::Invalid("The model was not added.".to_string()))?;
    Ok(Landed {
        project: manifest,
        asset,
        version,
    })
}
