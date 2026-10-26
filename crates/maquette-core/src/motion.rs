//! Motion: the models of a project that move, and what their motion costs.
//!
//! [`view`] reads every asset's newest model file for its rig (skins and
//! joints, mapped onto a humanoid skeleton), its animation clips (length,
//! key rate, keyframe bytes, root travel, whether it loops) and its parts
//! (the nodes that draw a mesh, which object states move). States and clip
//! choices come from the layout ([`crate::layout::states`]).
//!
//! **Budget.** Motion has no budget of its own: clips are bytes inside the
//! model files, so they count in the scene budget and in the first view
//! like the rest of the model. A clip's size is its keyframe accessors in
//! the file the layout weighs the model by: the shipped build (meshopt
//! compressed, counted as each accessor's share of its compressed buffer
//! view), else the raw export, else the source model. A clip set not to
//! ship is left out of the shipped model by Build scene, so it costs
//! nothing once the next run has built it.

mod clips;
mod rig;
mod save;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::AssetKind;
use crate::error::{CoreError, CoreResult};
use crate::inbox::analyse::{self, RigKind};
use crate::layout::{self, FirstView, LayoutStop, Measured, ObjectStates, SceneLayout};
use crate::pipeline::PipelineDef;
use crate::project::ProjectManifest;
use crate::workspace::Budgets;

pub use rig::{BoneMap, BoneMatch, Rig, WEB_INFLUENCES};
pub use save::{check_parts, save, with_motion};

/// The marker of a Build scene script that leaves clips set not to ship
/// out of the model.
const CLIPS_MARKER: &str = "def drop_clips(";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    /// The animation's name in the file, or "Animation 2" without one.
    pub name: String,
    /// Its place among the file's animations.
    pub index: u32,
    /// Seconds from the first key to the last.
    pub duration: f32,
    /// Keys on its longest channel.
    pub keys: u32,
    /// Frames from the first key to the last, at `fps`.
    pub frames: u32,
    /// The rate its keys were sampled at.
    pub fps: f32,
    pub channels: u32,
    /// How many nodes it moves.
    pub targets: u32,
    /// Keyframe bytes in the source model.
    pub bytes: f64,
    /// Keyframe bytes in the file the model is weighed by, when that is a
    /// build (shipped or raw); `None` when the build has no such clip yet.
    pub shipped_bytes: Option<f64>,
    /// How far the root moves over the ground from the first key to the
    /// last, in metres; `None` for a clip that plays in place.
    pub root_travel: Option<f32>,
    /// The last key matches the first (root travel aside).
    pub loops: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MotionModel {
    pub asset: String,
    pub name: String,
    pub kind: AssetKind,
    pub version: u32,
    /// Absolute path of the newest model file.
    pub file: String,
    /// Relative to the project.
    pub path: String,
    pub thumbnail: Option<String>,
    /// When the newest version was added.
    pub added_at: String,
    /// What the whole model weighs on the page.
    pub bytes: f64,
    pub measured: Measured,
    pub rig: Option<Rig>,
    pub clips: Vec<Clip>,
    /// The bytes its clips add, in the file it is weighed by.
    pub motion_bytes: f64,
    /// The nodes that draw a mesh, in file order: the parts states move.
    pub parts: Vec<String>,
    /// Why the file could not be read, when it could not.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MotionBudget {
    /// Every model's bytes, against the scene budget.
    pub scene_bytes: f64,
    pub scene_target_mb: f64,
    /// Of those, the bytes of clips.
    pub motion_bytes: f64,
    pub first_view: FirstView,
    /// Clip bytes among what the first view loads.
    pub first_view_motion: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MotionView {
    pub project: String,
    /// Every model, moving or not, in Build scene order.
    pub models: Vec<MotionModel>,
    pub states: BTreeMap<String, ObjectStates>,
    /// Asset id to the clips that stay off the site.
    pub clips_off: BTreeMap<String, Vec<String>>,
    /// The layout's camera stops, which stop triggers name.
    pub stops: Vec<LayoutStop>,
    pub budget: MotionBudget,
    /// The project's Build scene leaves clips set not to ship out. False
    /// for an older or edited copy without that, or another template.
    pub clips_off_supported: bool,
}

/// What a model file holds that moves.
struct FileMotion {
    rig: Option<Rig>,
    clips: Vec<Clip>,
    parts: Vec<String>,
}

fn parse(bytes: &[u8], base: &Path) -> CoreResult<FileMotion> {
    let gltf = gltf::Gltf::from_slice_without_validation(bytes)
        .map_err(|e| CoreError::Invalid(format!("This is not a glTF model. {e}")))?;
    let doc = &gltf.document;
    let buffers: Vec<Option<Vec<u8>>> = doc
        .buffers()
        .map(|b| match b.source() {
            gltf::buffer::Source::Bin => gltf.blob.clone(),
            gltf::buffer::Source::Uri(uri) => analyse::load_uri(uri, base),
        })
        .collect();
    let json = clips::json_of(bytes).unwrap_or_default();
    let (sizes, _) = clips::clip_bytes(&json);
    let compressed = doc
        .extensions_used()
        .any(|e| e.contains("meshopt") || e.contains("draco"));
    let mut joint_names: Vec<String> = Vec::new();
    for skin in doc.skins() {
        for joint in skin.joints() {
            let name = joint.name().unwrap_or("joint").to_string();
            if !joint_names.contains(&name) {
                joint_names.push(name);
            }
        }
    }
    let rig = (!joint_names.is_empty()).then(|| {
        let map = rig::humanoid_map(&joint_names);
        Rig {
            joints: u32::try_from(joint_names.len()).unwrap_or(u32::MAX),
            kind: analyse_kind(&joint_names),
            humanoid: rig::is_humanoid(&map),
            map,
            max_influences: rig::max_influences(doc, &buffers, !compressed),
            joint_names,
        }
    });
    let mut parts = Vec::new();
    for node in doc.nodes().filter(|n| n.mesh().is_some()) {
        let name = node
            .name()
            .map_or_else(|| format!("Part {}", node.index() + 1), str::to_string);
        if !parts.contains(&name) {
            parts.push(name);
        }
    }
    Ok(FileMotion {
        rig,
        clips: clips::read(doc, &buffers, &sizes),
        parts,
    })
}

fn analyse_kind(joints: &[String]) -> RigKind {
    // The Inbox's own test: enough Mixamo names.
    let mixamo = rig::humanoid_map(joints)
        .iter()
        .filter(|m| m.how == BoneMatch::Exact)
        .count();
    if mixamo >= 18 {
        RigKind::Mixamo
    } else {
        RigKind::Other
    }
}

/// Reads a model file for its rig, clips and parts.
fn read_file(path: &Path) -> CoreResult<FileMotion> {
    let bytes = fs::read(path).map_err(|e| CoreError::io(path, e))?;
    let base = path.parent().unwrap_or(Path::new("."));
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse(&bytes, base))).unwrap_or_else(
        |_| {
            Err(CoreError::Invalid(
                "This file is not a well formed glTF model.".to_string(),
            ))
        },
    )
}

/// The clip bytes of a built file, by clip name, and all of them together.
fn built_bytes(path: &Path) -> Option<(BTreeMap<String, f64>, f64)> {
    let bytes = fs::read(path).ok()?;
    clips::json_of(&bytes).map(|json| clips::clip_bytes(&json))
}

/// The clips' bytes, the shipped sizes filled in from the file the model is
/// weighed by.
fn weigh(clips: &mut [Clip], measured: Measured, built: Option<&Path>) -> f64 {
    let source_total: f64 = clips.iter().map(|c| c.bytes).sum();
    if measured == Measured::Source {
        return source_total;
    }
    let Some((per, total)) = built.and_then(built_bytes) else {
        return source_total;
    };
    for clip in clips.iter_mut() {
        clip.shipped_bytes = per.get(&clip.name).copied();
    }
    total
}

fn clips_supported(project_dir: &Path) -> bool {
    fs::read_to_string(project_dir.join("tools/build_scene.py"))
        .is_ok_and(|text| text.contains(CLIPS_MARKER))
}

/// Everything the Motion view shows for a project.
///
/// # Errors
///
/// Fails when `layout.json` cannot be read.
pub fn view(
    project_dir: &Path,
    manifest: &ProjectManifest,
    def: &PipelineDef,
    budgets: &Budgets,
    previews: Option<&Path>,
) -> CoreResult<MotionView> {
    let layout = layout::load(project_dir)?;
    Ok(view_of(
        project_dir,
        manifest,
        def,
        budgets,
        previews,
        layout,
    ))
}

fn view_of(
    project_dir: &Path,
    manifest: &ProjectManifest,
    def: &PipelineDef,
    budgets: &Budgets,
    previews: Option<&Path>,
    layout: SceneLayout,
) -> MotionView {
    let placed = layout::models(project_dir, manifest, def, &layout, previews);
    let folders = crate::inspect::folders(project_dir, def);
    let shipped_names = crate::assets::shipped_names(manifest);
    let models: Vec<MotionModel> = placed
        .iter()
        .filter_map(|m| {
            let asset = manifest.assets.iter().find(|a| a.id == m.asset)?;
            let version = asset.latest()?;
            let source = project_dir.join(&version.file);
            let (built, _, measured) = layout::measured_file(
                (&folders.0, &folders.1),
                &shipped_names,
                &version.file,
                &source,
            );
            let (rig, mut clips, parts, error) = match read_file(&source) {
                Ok(found) => (found.rig, found.clips, found.parts, None),
                Err(err) => (None, Vec::new(), Vec::new(), Some(err.to_string())),
            };
            let motion_bytes = weigh(&mut clips, measured, Some(&built));
            Some(MotionModel {
                asset: m.asset.clone(),
                name: m.name.clone(),
                kind: m.kind,
                version: m.version,
                file: m.file.clone(),
                path: m.path.clone(),
                thumbnail: asset.thumbnail.clone(),
                added_at: version.created_at.clone(),
                bytes: m.bytes,
                measured: m.measured,
                rig,
                clips,
                motion_bytes,
                parts,
                error,
            })
        })
        .collect();
    let first_view = layout::first_view(&layout, &placed, budgets.first_view_mb);
    let first_view_motion = first_view
        .models
        .iter()
        .filter_map(|f| models.iter().find(|m| m.asset == f.asset))
        .map(|m| m.motion_bytes)
        .sum();
    let budget = MotionBudget {
        scene_bytes: placed.iter().map(|m| m.bytes).sum(),
        scene_target_mb: budgets.scene_mb,
        motion_bytes: models.iter().map(|m| m.motion_bytes).sum(),
        first_view,
        first_view_motion,
    };
    let clips_off = layout
        .clips
        .keys()
        .map(|asset| {
            let off = layout::states::clips_off(&layout, asset)
                .into_iter()
                .map(str::to_string)
                .collect();
            (asset.clone(), off)
        })
        .collect();
    MotionView {
        project: manifest.id.clone(),
        models,
        clips_off,
        stops: layout.stops.clone(),
        states: layout.states,
        budget,
        clips_off_supported: clips_supported(project_dir),
    }
}

/// The parts `states` move that the asset's newest model does not have,
/// as `(asset, part)`. An asset whose model cannot be read is skipped.
pub fn unknown_parts(
    project_dir: &Path,
    manifest: &ProjectManifest,
    states: &BTreeMap<String, ObjectStates>,
) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (asset, object) in states {
        let Some(version) = manifest
            .assets
            .iter()
            .find(|a| &a.id == asset)
            .and_then(|a| a.latest())
        else {
            continue;
        };
        let Ok(file) = read_file(&project_dir.join(&version.file)) else {
            continue;
        };
        let named = object
            .states
            .iter()
            .flat_map(|s| s.parts.keys())
            .chain(object.triggers.iter().filter_map(|t| t.part.as_ref()));
        for part in named {
            if !file.parts.contains(part) && !found.contains(&(asset.clone(), part.clone())) {
                found.push((asset.clone(), part.clone()));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests;
