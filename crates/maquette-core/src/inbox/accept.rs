//! Accepting an inbox model into a project as a new asset version.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::*;
use crate::prepare::{self, ModelPrep, ScenePrep};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AcceptRequest {
    pub item: String,
    pub project: String,
    /// An existing asset to add a version to. Without it, an asset named
    /// `name` (or the item's name) is found or created.
    pub asset: Option<String>,
    pub name: Option<String>,
    /// The real size and fix-ups set in the Inbox: they replace the asset's
    /// in `prepare.json`. Left out, the asset keeps what it has. Either
    /// way an asset nobody chose a bake for skips it from now on
    /// ([`prepare::SKIP_BAKE_FROM_INBOX`]): Inbox models are scans.
    #[serde(default)]
    pub prep: Option<ModelPrep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AcceptResult {
    pub asset: Asset,
    pub version: AssetVersion,
    /// Where the file went.
    pub file: String,
    /// The project after the change, so the Board can update.
    pub project: crate::project::ProjectManifest,
    /// Why the real size and fix-ups were not saved, when they were not.
    pub note: Option<String>,
}

fn move_file(from: &Path, to: &Path) -> CoreResult<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    // A different volume: copy, then remove the original.
    fs::copy(from, to).map_err(|e| CoreError::io(to, e))?;
    fs::remove_file(from).map_err(|e| CoreError::io(from, e))
}

pub fn stats_of(report: &GlbReport) -> ModelStats {
    ModelStats {
        triangles: report.triangles,
        bytes: report.bytes,
        textures: u32::try_from(report.textures.len()).unwrap_or(u32::MAX),
        max_texture: report.max_texture,
        joints: report.joints,
        animations: u32::try_from(report.animations.len()).unwrap_or(u32::MAX),
    }
}

/// Moves the model into the project's `models/` folder as the next version
/// of its asset and records it, with the asset moved to the Blender column
/// for review.
///
/// # Errors
///
/// Fails for a missing item, a glTF that depends on files next to it, an
/// unknown project or asset, or when the file cannot be moved.
pub fn accept(
    root: &Path,
    config: &WorkspaceConfig,
    request: &AcceptRequest,
) -> CoreResult<AcceptResult> {
    let found = item(root, config, &request.item)?;
    let source = PathBuf::from(&found.file);
    if external_files(&source) {
        return Err(CoreError::Invalid(
            "This glTF uses files next to it. Export it as a single .glb, then accept it."
                .to_string(),
        ));
    }
    let project_dir = project::dir(root, &config.layout, &request.project)?;
    // Checked before the file moves, so a bad size refuses the whole accept.
    if let Some(prep) = &request.prep {
        let mut check = ScenePrep::default();
        check.models.insert("model".to_string(), prep.clone());
        prepare::validate(&check)?;
    }
    let models = project_dir.join("models");
    let ext = source
        .extension()
        .map_or_else(|| "glb".to_string(), |e| e.to_string_lossy().to_lowercase());
    let stats = found.report.as_ref().map(stats_of);
    let mut result: Option<(Asset, AssetVersion, String)> = None;
    let manifest = project::modify(root, &config.layout, &request.project, |manifest| {
        let id = match &request.asset {
            Some(id) => assets::find(manifest, id)?.id.clone(),
            None => {
                let name = request.name.clone().unwrap_or_else(|| found.name.clone());
                match assets::find_by_name(manifest, &name) {
                    Some(asset) => asset.id.clone(),
                    None => {
                        assets::add(
                            manifest,
                            NewAsset {
                                name,
                                kind: found.kind,
                                stage: Stage::Tripo,
                            },
                        )?
                        .id
                    }
                }
            }
        };
        let number = assets::find(manifest, &id)?.next_version();
        let file_name = format!("{}_v{number}.{ext}", id.replace('-', "_"));
        let target = models.join(&file_name);
        if target.exists() {
            return Err(CoreError::Invalid(format!(
                "models/{file_name} already exists. Rename it or remove it first."
            )));
        }
        fs::create_dir_all(&models).map_err(|e| CoreError::io(&models, e))?;
        move_file(&source, &target)?;
        let version =
            assets::add_version(manifest, &id, format!("models/{file_name}"), stats.clone())?;
        let current = assets::find(manifest, &id)?.stage;
        if current < Stage::Blender {
            assets::move_to(manifest, &id, Stage::Blender)?;
        }
        assets::set_status(manifest, &id, AssetStatus::Review)?;
        result = Some((
            assets::find(manifest, &id)?.clone(),
            version,
            target.display().to_string(),
        ));
        Ok(())
    })?;
    let _ = fs::remove_file(cache_dir(root, config).join(format!("{}.json", request.item)));
    let (asset, version, file) =
        result.ok_or_else(|| CoreError::Invalid("The model was not accepted.".to_string()))?;
    let note = save_prep(&project_dir, &asset.id, request.prep.as_ref())
        .err()
        .map(|err| format!("The model is in, but its real size and fix-ups were not saved. {err}"));
    Ok(AcceptResult {
        asset,
        version,
        file,
        project: manifest,
        note,
    })
}

/// Saves the Inbox's real size and fix-ups for the asset (when it set any)
/// and fills in the scan's bake default when neither the Inbox nor the
/// saved entry chose one. An entry that already says nothing new is not
/// written again.
fn save_prep(project_dir: &Path, asset: &str, prep: Option<&ModelPrep>) -> CoreResult<()> {
    prepare::update(project_dir, |current| {
        let saved = current.models.get(asset).cloned().unwrap_or_default();
        let mut entry = prep.cloned().unwrap_or_else(|| saved.clone());
        entry.skip_bake = entry
            .skip_bake
            .or(saved.skip_bake)
            .or(Some(prepare::SKIP_BAKE_FROM_INBOX));
        let mut next = current.clone();
        next.models.insert(asset.to_string(), entry);
        Ok(next)
    })
    .map(|_| ())
}

/// The arguments that open `file` in Blender's window: a `.blend` opens as
/// is; a model is imported into an empty scene. Only for the explicit Open
/// in Blender action, never for pipeline steps.
pub fn open_in_blender_args(file: &Path) -> Vec<String> {
    let is_blend = file
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("blend"));
    if is_blend {
        return vec![file.display().to_string()];
    }
    vec![
        "--python-expr".to_string(),
        "import bpy, sys; bpy.ops.wm.read_factory_settings(use_empty=True); \
         bpy.ops.import_scene.gltf(filepath=sys.argv[sys.argv.index('--') + 1])"
            .to_string(),
        "--".to_string(),
        file.display().to_string(),
    ]
}
