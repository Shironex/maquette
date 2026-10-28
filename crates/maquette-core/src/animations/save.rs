//! Save clips to the library from a rigged model: the newest version of a
//! project asset, or a Tripo job's animations waiting in the Inbox. Only the
//! skeleton and the chosen clips' keys go in; the mesh stays where it is.
//! A clip whose keys the library has already is skipped.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::blender::{self, ReadJob};
use super::{AnimationClip, ClipOrigin, ClipRig, ClipSource, SourceRig, names, rig};
use crate::assets;
use crate::error::{CoreError, CoreResult};
use crate::motion::Clip;
use crate::pipeline::runner::PipelineRunner;
use crate::pipeline::tools::Toolbox;
use crate::project::{self, now_rfc3339};
use crate::workspace::WorkspaceConfig;

/// The model clips are saved from: a project asset's newest version, or an
/// Inbox file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipsFrom {
    pub project: Option<String>,
    pub asset: Option<String>,
    /// An Inbox file name.
    pub inbox: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveClip {
    pub name: String,
    pub frames: u32,
    pub fps: f32,
    pub root_travel: Option<f32>,
    pub loops: Option<bool>,
    pub bytes: f64,
    /// The library clip it came from, which is never saved back.
    pub from_library: Option<String>,
}

/// What Save clips to the library offers for a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SavePlan {
    pub from: ClipsFrom,
    /// The model's name.
    pub name: String,
    pub version: Option<u32>,
    /// The model file, relative to its project or the Inbox.
    pub file: String,
    pub rig: SourceRig,
    pub bones: u32,
    pub clips: Vec<SaveClip>,
    /// A new pack named after the model.
    pub pack: String,
    /// Tags the clip names suggest.
    pub tags: Vec<String>,
    /// What the mesh and textures weigh, which stay where they are.
    pub model_bytes: f64,
}

/// What the person chose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveClips {
    pub from: ClipsFrom,
    pub clips: Vec<String>,
    pub pack: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    pub added: Vec<AnimationClip>,
    /// Clips the library already had, by name.
    pub skipped: Vec<String>,
}

struct Model {
    path: PathBuf,
    name: String,
    version: Option<u32>,
    file: String,
    bytes: f64,
}

fn model_of(root: &Path, config: &WorkspaceConfig, from: &ClipsFrom) -> CoreResult<Model> {
    if let Some(inbox) = &from.inbox {
        if !crate::workspace::is_single_segment(inbox) {
            return Err(CoreError::Invalid(format!(
                "{inbox:?} is not an Inbox file."
            )));
        }
        let path = crate::inbox::inbox_dir(root, config).join(inbox);
        let bytes = std::fs::metadata(&path).map_or(0.0, |m| m.len() as f64);
        return Ok(Model {
            name: names::clip_name(inbox),
            version: None,
            file: inbox.clone(),
            bytes,
            path,
        });
    }
    let (Some(project_id), Some(asset_id)) = (&from.project, &from.asset) else {
        return Err(CoreError::Invalid(
            "Name a project asset or an Inbox file to save clips from.".to_string(),
        ));
    };
    let manifest = project::load(root, &config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?;
    let latest = asset
        .latest()
        .ok_or_else(|| CoreError::Invalid(format!("{} has no model yet.", asset.name)))?;
    Ok(Model {
        path: project::dir(root, &config.layout, project_id)?.join(&latest.file),
        name: asset.name.clone(),
        version: Some(latest.version),
        file: latest.file.clone(),
        bytes: latest.stats.as_ref().map_or(0.0, |s| s.bytes),
    })
}

/// The clips a model holds and the rig they move, read without Blender.
/// `from_tripo` says the model came from a Tripo job; `baked` names the
/// clips that came from the library.
///
/// # Errors
///
/// Fails for an unknown model, one that cannot be read, or one with no rig
/// or no clips.
pub fn plan(
    root: &Path,
    config: &WorkspaceConfig,
    from: &ClipsFrom,
    from_tripo: bool,
    baked: &[(String, String)],
) -> CoreResult<SavePlan> {
    let model = model_of(root, config, from)?;
    let (found_rig, clips) = crate::motion::rig_and_clips(&model.path)?;
    let found_rig = found_rig.ok_or_else(|| {
        CoreError::Invalid(format!("{} has no rig to save clips from.", model.name))
    })?;
    if clips.is_empty() {
        return Err(CoreError::Invalid(format!("{} has no clips.", model.name)));
    }
    let mut tags: Vec<String> = Vec::new();
    for tag in clips.iter().flat_map(|c| names::tags_for(&c.name)) {
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    let motion_bytes: f64 = clips.iter().map(|c| c.bytes).sum();
    Ok(SavePlan {
        from: from.clone(),
        version: model.version,
        file: model.file,
        rig: rig::detect(&found_rig.joint_names, from_tripo),
        bones: found_rig.joints,
        clips: clips.iter().map(|c| save_clip(c, baked)).collect(),
        pack: format!("{} moves", model.name),
        tags,
        model_bytes: (model.bytes - motion_bytes).max(0.0),
        name: model.name,
    })
}

fn save_clip(clip: &Clip, baked: &[(String, String)]) -> SaveClip {
    SaveClip {
        name: clip.name.clone(),
        frames: clip.frames,
        fps: clip.fps,
        root_travel: clip.root_travel,
        loops: clip.loops,
        bytes: clip.bytes,
        from_library: baked
            .iter()
            .find(|(name, _)| *name == clip.name)
            .map(|(_, from)| from.clone()),
    }
}

/// Reads the chosen clips headless and adds them as a pack, skeleton only.
///
/// # Errors
///
/// As [`plan`], for a bad pack name, without Blender, or when a clip cannot
/// be written.
pub fn save(
    root: &Path,
    config: &WorkspaceConfig,
    (runner, tools): (&PipelineRunner, &Toolbox),
    request: &SaveClips,
    from_tripo: bool,
    baked: &[(String, String)],
) -> CoreResult<Saved> {
    let found = plan(root, config, &request.from, from_tripo, baked)?;
    let pack = names::clean_pack(&request.pack)
        .ok_or_else(|| CoreError::Invalid("Name the pack the clips go to.".to_string()))?;
    let tags = crate::library::clean_tags(&request.tags)?;
    let wanted: Vec<String> = found
        .clips
        .iter()
        .filter(|c| request.clips.contains(&c.name) && c.from_library.is_none())
        .map(|c| c.name.clone())
        .collect();
    if wanted.is_empty() {
        return Err(CoreError::Invalid(
            "Pick at least one clip to save.".to_string(),
        ));
    }
    let model = model_of(root, config, &request.from)?;
    let work = blender::scratch(root, config, "save")?;
    crate::assets::copy::copy_file(&model.path, &work.join("in/model.glb"))?;
    let fps = found.clips.first().map(|c| c.fps).filter(|f| *f > 0.0);
    let job = ReadJob {
        id: "0".to_string(),
        file: "in/model.glb".to_string(),
        name: model.file.clone(),
        clips: Some(wanted),
        fps,
    };
    let report = blender::read(runner, tools, &work, &[job])?;
    let read = report
        .files
        .first()
        .ok_or_else(|| CoreError::Invalid("Blender read nothing from the model.".to_string()))?;
    if let Some(err) = &read.error {
        return Err(CoreError::Invalid(format!(
            "Blender could not read the model: {err}"
        )));
    }
    let _turn = super::turn();
    let existing = super::list(root)?;
    let mut taken: Vec<String> = existing.iter().map(|c| c.id.clone()).collect();
    let mut saved = Saved {
        added: Vec::new(),
        skipped: Vec::new(),
    };
    for clip in read.clips.iter().filter(|c| c.has_keys) {
        if existing.iter().any(|c| c.hash == clip.hash) {
            saved.skipped.push(clip.name.clone());
            continue;
        }
        let Some(out) = &clip.out else { continue };
        let id = super::free_id(root, &taken, &clip.name);
        taken.push(id.clone());
        let from = work.join(blender::OUT_DIR).join(out);
        crate::assets::copy::copy_file(&from, &super::clip_dir(root, &id)?.join(super::CLIP_GLB))?;
        let now = now_rfc3339();
        let made = AnimationClip {
            version: super::CLIP_VERSION,
            id,
            name: clip.name.clone(),
            pack: pack.clone(),
            tags: if tags.is_empty() {
                names::tags_for(&clip.name)
            } else {
                tags.clone()
            },
            rig: ClipRig {
                kind: found.rig,
                bones: read.bones.clone(),
                hips_height: read.hips_height,
            },
            frames: clip.frames,
            fps: clip.fps,
            length: clip.length,
            root_motion: clip.root_motion.clone(),
            loops: clip.loops,
            bytes: clip.bytes.unwrap_or(0) as f64,
            file: super::CLIP_GLB.to_string(),
            origin: ClipOrigin {
                source: if from_tripo && request.from.inbox.is_some() {
                    ClipSource::Tripo
                } else {
                    ClipSource::Model
                },
                file: names::stem(&model.file) + ".glb",
                archive: None,
                kept: None,
                project: request.from.project.clone(),
                asset: request.from.asset.clone(),
                version: model.version,
            },
            hash: clip.hash.clone(),
            revision: 1,
            created_at: now.clone(),
            updated_at: now,
        };
        super::store(root, &made)?;
        saved.added.push(made);
    }
    if let Err(err) = std::fs::remove_dir_all(&work) {
        tracing::warn!(error = %err, "could not remove the save's scratch folder");
    }
    Ok(saved)
}
