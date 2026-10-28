//! Import clips: FBX files or a pack zip of them. [`read`] copies the files
//! into a scratch folder, unzips packs and has Blender read them headless;
//! [`classify`] decides, per clip, whether it is new, the same keys as a
//! clip the library has (Skip or Keep both) or new keys for a clip it has
//! (a new revision); [`commit`] adds what the person chose. Nothing goes in
//! until then. A file with no keys, a single pose, is left out.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::blender::{self, ReadJob, ReadReport};
use super::{AnimationClip, RootMotion, SourceRig};
use crate::error::CoreResult;
use crate::pipeline::runner::PipelineRunner;
use crate::pipeline::tools::Toolbox;
use crate::workspace::WorkspaceConfig;

mod classify;
mod commit;
mod stage;

pub(crate) use classify::classify;
#[cfg(test)]
pub(crate) use classify::goes_in;
pub use commit::commit;
pub(crate) use stage::{StagedEntry, StagedFile};

/// What happens to a clip unless the person says otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ClipStatus {
    /// Goes in as a new clip.
    New,
    /// The library (or an earlier file here) has the same keys: skipped
    /// unless kept both.
    Duplicate,
    /// The pack has a clip of this name with other keys: it replaces that
    /// clip as its next revision, unless kept both.
    Revision,
}

/// The clip a duplicate or a revision matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MatchedClip {
    /// The library clip, or none for an earlier file in this import.
    pub id: Option<String>,
    pub name: String,
    pub pack: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanClip {
    /// `<file id>/<clip>`, which the request names it by.
    pub key: String,
    pub name: String,
    /// The FBX it is in.
    pub file: String,
    pub pack: String,
    pub tags: Vec<String>,
    pub rig: SourceRig,
    pub bones: u32,
    pub frames: u32,
    pub fps: f32,
    pub length: f32,
    pub root_motion: Option<RootMotion>,
    pub loops: bool,
    pub bytes: f64,
    pub status: ClipStatus,
    pub matched: Option<MatchedClip>,
}

/// A file or clip that does not go in, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LeftOut {
    pub file: String,
    pub reason: String,
}

/// One dropped file: a single FBX, or a pack zip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportFile {
    pub name: String,
    pub archive: bool,
    pub bytes: f64,
    /// FBX files read from it.
    pub fbx: u32,
    pub pack: String,
    pub rig: Option<SourceRig>,
    /// Bones of its (first) skeleton.
    pub bones: u32,
    /// The prefix its bones share, like `mixamorig:`.
    pub prefix: Option<String>,
    /// The rate every clip shares, when they do.
    pub fps: Option<f32>,
    /// FBX files that had a skin, which was dropped.
    pub skinned: u32,
    pub clips: Vec<PlanClip>,
    pub left_out: Vec<LeftOut>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TagCount {
    pub tag: String,
    pub count: u32,
}

/// What an import would add, before anything is added.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlan {
    /// Names the read files for [`commit`].
    pub token: String,
    pub files: Vec<ImportFile>,
    /// What was read, the FBX files and zips together.
    pub read_bytes: f64,
    /// What the clips that go in by default weigh, skeleton only.
    pub stored_bytes: f64,
    pub tags: Vec<TagCount>,
    /// Clips that go in by default.
    pub add: u32,
    pub left_out: u32,
}

/// The pack a dropped file's clips go to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackChoice {
    pub file: String,
    pub pack: String,
}

/// What the person chose in the Import sheet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub token: String,
    /// The pack for a zip's clips; left out, the zip's name.
    pub packs: Vec<PackChoice>,
    /// Where single files go; left out, Singles.
    pub singles: Option<String>,
    /// Keeps each FBX file next to its clip. Off by default.
    pub keep_files: bool,
    /// Duplicates and revisions to add as clips of their own.
    pub keep_both: Vec<String>,
    /// Clips not to add at all.
    pub skip: Vec<String>,
}

/// What an import added.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub added: Vec<AnimationClip>,
    /// Clips that took new keys as their next revision.
    pub revised: Vec<AnimationClip>,
    pub skipped: u32,
    pub left_out: u32,
}

/// Reads `paths` (FBX files and pack zips) headless and says what an import
/// would add. Nothing is added yet.
///
/// # Errors
///
/// Fails without Blender, for no readable file, or when the scratch folder
/// cannot be written.
pub fn read(
    root: &Path,
    config: &WorkspaceConfig,
    runner: &PipelineRunner,
    tools: &Toolbox,
    paths: &[String],
) -> CoreResult<ImportPlan> {
    let work = blender::scratch(root, config, "import")?;
    let staged = stage::stage(&work, paths)?;
    let jobs: Vec<ReadJob> = staged
        .iter()
        .flat_map(|f| &f.entries)
        .map(|e| ReadJob {
            id: e.id.clone(),
            file: e.path.clone(),
            name: e.file.clone(),
            clips: None,
            fps: None,
        })
        .collect();
    let report = if jobs.is_empty() {
        ReadReport::default()
    } else {
        blender::read(runner, tools, &work, &jobs)?
    };
    let token = token_of(&work);
    Ok(classify(
        &token,
        &staged,
        &report,
        &super::list(root)?,
        None,
    ))
}

#[cfg(test)]
pub(crate) fn stage_for_tests(work: &Path, paths: &[String]) -> CoreResult<Vec<StagedFile>> {
    stage::stage(work, paths)
}

fn token_of(work: &Path) -> String {
    work.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
