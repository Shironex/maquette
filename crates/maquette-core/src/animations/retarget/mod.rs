//! Putting library clips on a rigged model. The person chooses clips, the
//! bone map lines the clip's skeleton up with the model's rig, each clip
//! gets its name, frames, loop, root motion and speed on that model, and
//! headless Blender bakes them onto a copy of the newest version
//! ([`draft`]). The review says per clip whether it looks right; [`bake`]
//! then adds the copy as the model's next version through the usual
//! checks, the older version kept, with the failed clips left out. A clip
//! whose planted feet slide can bake again with them pinned ([`feet`]).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{AnimationClip, SourceRig};
use crate::assets::{self, AssetVersion};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest};
use crate::workspace::WorkspaceConfig;

mod bake;
pub mod bones;
pub mod feet;
pub mod review;
pub mod settings;
pub mod uses;

pub use bake::{RETARGET_SCRIPT, bake, draft, draft_file};
pub use bones::{BonePair, MapChoice, PairMatch};
pub use review::{ClipReview, RetargetReview, ReviewStatus};
pub use settings::ClipSetting;
pub use uses::{BakedClip, ClipUse};

/// What a retarget onto one model starts from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RetargetPlan {
    pub project: String,
    pub asset: String,
    pub name: String,
    /// The version the clips go onto, and its file, absolute.
    pub version: u32,
    pub file: String,
    pub joints: u32,
    /// The clips the model has now.
    pub model_clips: Vec<String>,
    pub model_bytes: f64,
    /// The first chosen clip's rig, which the map's clip column shows.
    pub source_rig: Option<SourceRig>,
    pub source_bones: u32,
    pub map: Vec<BonePair>,
    /// A map is saved for this model.
    pub saved: bool,
    /// The bone map wants a look before baking.
    pub needs_look: bool,
    /// Model bones no humanoid bone maps to; they follow their parents.
    pub only_on_model: Vec<String>,
    /// Per clip, the settings it bakes with: the ones it was baked with
    /// before on this model, or its defaults.
    pub settings: Vec<ClipSetting>,
    /// Chosen clips this model baked at an older revision.
    pub outdated: Vec<String>,
}

/// Bake these clips onto the model's newest version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UseClips {
    pub project: String,
    pub asset: String,
    pub settings: Vec<ClipSetting>,
    /// The bone map as the person left it.
    pub map: Vec<MapChoice>,
    /// Keeps the map for this model's next retarget.
    pub save_map: bool,
}

/// A bake waiting for review, on a copy of the newest version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RetargetDraft {
    pub token: String,
    pub project: String,
    pub asset: String,
    pub name: String,
    pub from_version: u32,
    pub review: RetargetReview,
    /// The baked copy, absolute, for the side by side preview.
    pub file: String,
    pub settings: Vec<ClipSetting>,
}

/// A bake added as a new version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Baked {
    pub project: ProjectManifest,
    pub version: AssetVersion,
    pub review: RetargetReview,
    /// Checks the new version did not pass.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pending {
    project: String,
    asset: String,
    name: String,
    from_version: u32,
    settings: Vec<ClipSetting>,
    library: Vec<(String, u32, String)>,
}

struct Model {
    dir: PathBuf,
    name: String,
    version: u32,
    path: PathBuf,
    bytes: f64,
    joints: Vec<String>,
    clips: Vec<String>,
}

fn model(
    root: &Path,
    config: &WorkspaceConfig,
    project_id: &str,
    asset_id: &str,
) -> CoreResult<Model> {
    let manifest = project::load(root, &config.layout, project_id)?;
    let dir = project::dir(root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?;
    let latest = asset
        .latest()
        .ok_or_else(|| CoreError::Invalid(format!("{} has no model yet.", asset.name)))?;
    let path = dir.join(&latest.file);
    let (rig, clips) = crate::motion::rig_and_clips(&path)?;
    let rig = rig.ok_or_else(|| {
        CoreError::Invalid(format!(
            "{} has no rig, so clips cannot go on it. Rig it first.",
            asset.name
        ))
    })?;
    Ok(Model {
        name: asset.name.clone(),
        version: latest.version,
        bytes: latest.stats.as_ref().map_or(0.0, |s| s.bytes),
        path,
        joints: rig.joint_names,
        clips: clips.into_iter().map(|c| c.name).collect(),
        dir,
    })
}

fn library_clips(root: &Path, ids: &[String]) -> CoreResult<Vec<AnimationClip>> {
    ids.iter().map(|id| super::load(root, id)).collect()
}

fn bone_names(clip: &AnimationClip) -> Vec<String> {
    clip.rig.bones.iter().map(|b| b.name.clone()).collect()
}

/// The names the model keeps: its clips, apart from older bakes of the
/// chosen library clips, which the new bake replaces.
fn taken(model: &Model, entry: &uses::AssetAnimations, chosen: &[String]) -> Vec<String> {
    let replaced: Vec<&str> = entry
        .baked
        .iter()
        .filter(|b| chosen.contains(&b.setting.clip))
        .map(|b| b.setting.name.as_str())
        .collect();
    model
        .clips
        .iter()
        .filter(|c| !replaced.contains(&c.as_str()))
        .cloned()
        .collect()
}

/// Where `clips` would go on the model, with the suggested bone map and
/// each clip's settings. Reads, never writes.
///
/// # Errors
///
/// Fails for an unknown project, asset or clip, or a model with no rig.
pub fn plan(
    root: &Path,
    config: &WorkspaceConfig,
    (project_id, asset_id): (&str, &str),
    clip_ids: &[String],
    picked: &[MapChoice],
) -> CoreResult<RetargetPlan> {
    let model = model(root, config, project_id, asset_id)?;
    let clips = library_clips(root, clip_ids)?;
    let entry = uses::of_asset(&model.dir, asset_id)?;
    let source = clips.first().map(bone_names).unwrap_or_default();
    let map = bones::suggest(&source, &model.joints, &entry.map, picked);
    let used: Vec<&String> = map.iter().filter_map(|p| p.target.as_ref()).collect();
    let defaults: Vec<ClipSetting> = clips
        .iter()
        .map(|c| {
            entry
                .baked
                .iter()
                .find(|b| b.setting.clip == c.id)
                .map_or_else(|| settings::default_for(c), |b| b.setting.clone())
        })
        .collect();
    let settings = settings::normalise(&defaults, &clips, &taken(&model, &entry, clip_ids))?;
    let saved = !entry.map.is_empty();
    Ok(RetargetPlan {
        project: project_id.to_string(),
        asset: asset_id.to_string(),
        name: model.name.clone(),
        version: model.version,
        file: model.path.display().to_string(),
        joints: u32::try_from(model.joints.len()).unwrap_or(u32::MAX),
        model_bytes: model.bytes,
        source_rig: clips.first().map(|c| c.rig.kind),
        source_bones: u32::try_from(source.len()).unwrap_or(u32::MAX),
        needs_look: bones::needs_look(&map, saved),
        only_on_model: model
            .joints
            .iter()
            .filter(|j| !used.contains(j))
            .cloned()
            .collect(),
        outdated: uses::outdated(&entry, &clips)
            .into_iter()
            .map(|s| s.clip)
            .collect(),
        model_clips: model.clips,
        map,
        saved,
        settings,
    })
}

#[cfg(test)]
mod tests;
