//! What a pack says about where each model came from, and which models
//! carry animation clips from Mixamo.
//!
//! Mixamo's clips may be used in your own work but not shared as files, so
//! a pack never writes them: a model whose clips may be Mixamo's is
//! exported with its bones and without its clips, and the card, the README
//! and the manifest say so. A clip counts as Mixamo's when the model baked
//! a library clip read from a Mixamo rig, or when the model's own rig uses
//! Mixamo's bone names and it did not come from Tripo (Tripo's rigs use
//! those names for clips Tripo made). That errs on the side of leaving a
//! clip out.

use std::path::Path;

use crate::animations::SourceRig;
use crate::animations::retarget::uses;
use crate::inbox::analyse::RigKind;
use crate::library::LibrarySource;

/// The line a pack writes for the clips it left out.
pub const MIXAMO_NOTE: &str = "Mixamo clips left out: Mixamo allows using them in your own work, not sharing them as files. The bones are kept, so the other side can add its own clips.";

/// "Tripo", "By hand" and so on, for the manifest and the cards.
pub fn source_label(source: Option<LibrarySource>) -> &'static str {
    match source {
        Some(LibrarySource::Tripo) => "Tripo",
        Some(LibrarySource::Meshy) => "Meshy",
        Some(LibrarySource::ByHand) => "By hand",
        Some(LibrarySource::Built) => "Built",
        Some(LibrarySource::Imported) => "Imported",
        None => "Unknown",
    }
}

/// The licence note of a model made by `source`. It says whose terms apply
/// and never more than that: Maquette does not know the plan a model was
/// made on.
pub fn note(source: Option<LibrarySource>) -> String {
    match source {
        Some(LibrarySource::Tripo) => {
            "Made with Tripo. What you may do with it is set by the Tripo plan it was made on."
        }
        Some(LibrarySource::Meshy) => {
            "Made with Meshy. What you may do with it is set by the Meshy plan it was made on."
        }
        Some(LibrarySource::ByHand) => {
            "Made by hand on a provider's site. That site's terms for the plan it was made on apply."
        }
        Some(LibrarySource::Built) => "Built by a Blender script in a Maquette project.",
        Some(LibrarySource::Imported) | None => {
            "Brought in from a file. Its licence is whatever its source gave it."
        }
    }
    .to_string()
}

/// Whether a model's clips may be Mixamo's: `rig` is the rig its file has,
/// `baked_mixamo` whether it baked a library clip read from a Mixamo rig.
pub fn may_be_mixamo(
    source: Option<LibrarySource>,
    rig: Option<RigKind>,
    baked_mixamo: bool,
) -> bool {
    baked_mixamo || (rig == Some(RigKind::Mixamo) && source != Some(LibrarySource::Tripo))
}

/// Whether the asset baked a library clip that came from a Mixamo rig, by
/// the project's `animations.json`. A clip no longer in the library counts
/// as Mixamo's, so a doubt leaves it out.
pub fn baked_mixamo(root: &Path, project_dir: &Path, asset: &str) -> bool {
    let Ok(record) = uses::of_asset(project_dir, asset) else {
        return false;
    };
    record.baked.iter().any(|baked| {
        crate::animations::load(root, &baked.setting.clip)
            .map_or(true, |clip| clip.rig.kind == SourceRig::Mixamo)
    })
}

/// The rig kind and clip count of a glTF model, read without Blender;
/// `None` for a file that is not glTF or cannot be read.
pub fn rig_of(file: &Path) -> Option<(Option<RigKind>, u32)> {
    let is_gltf = file
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("glb") || e.eq_ignore_ascii_case("gltf"));
    if !is_gltf {
        return None;
    }
    let (rig, clips) = crate::motion::rig_and_clips(file).ok()?;
    Some((
        rig.map(|r| r.kind),
        u32::try_from(clips.len()).unwrap_or(u32::MAX),
    ))
}
