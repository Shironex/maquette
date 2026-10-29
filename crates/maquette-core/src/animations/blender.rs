//! The Blender side of the animation library: a scratch folder under the
//! workspace cache with the files to read and a script, and one headless
//! run of it on the runner's own lane.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use serde::{Deserialize, Serialize};

use super::{RestBone, RootMotion};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::runner::PipelineRunner;
use crate::pipeline::tools::Toolbox;
use crate::workspace::WorkspaceConfig;

// One headless Blender script run lives with the pipeline runner, which
// implements the script runner port with it; these keep the old paths.
pub use crate::pipeline::script::LANE;
pub(crate) use crate::pipeline::script::run_script;
/// Reads clips out of FBX and glTF files, skeleton only.
pub const READ_SCRIPT: &str = include_str!("read_clips.py");
const READ_FILE: &str = "read_clips.py";
pub(super) const JOBS_FILE: &str = "jobs.json";
pub(super) const OUT_DIR: &str = "clips";
pub(super) const REPORT_FILE: &str = "report.json";

/// One file for the reader.
#[derive(Debug, Clone, Serialize)]
pub(super) struct ReadJob {
    pub id: String,
    /// Relative to the scratch folder, with `/` separators.
    pub file: String,
    /// The file's own name, which a lone take is named after.
    pub name: String,
    /// The clips to keep, for a model that holds others too.
    pub clips: Option<Vec<String>>,
    /// The rate a glTF model's keys were sampled at.
    pub fps: Option<f32>,
}

/// One clip as the reader found it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClipReport {
    pub name: String,
    pub hash: String,
    pub has_keys: bool,
    pub frames: u32,
    pub fps: f32,
    pub length: f32,
    pub root_motion: Option<RootMotion>,
    pub loops: bool,
    /// The written skeleton file's size; none for a clip without keys.
    pub bytes: Option<u64>,
    /// Relative to the clips folder.
    pub out: Option<String>,
}

/// One file as the reader found it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileReport {
    pub id: String,
    pub error: Option<String>,
    #[serde(default)]
    pub skinned: bool,
    #[serde(default)]
    pub bones: Vec<RestBone>,
    #[serde(default)]
    pub hips_height: f32,
    #[serde(default)]
    pub clips: Vec<ClipReport>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ReadReport {
    #[serde(default)]
    pub files: Vec<FileReport>,
}

/// A fresh scratch folder for one job: `<workspace cache>/animations/<kind>-<n>`.
pub(super) fn scratch(root: &Path, config: &WorkspaceConfig, kind: &str) -> CoreResult<PathBuf> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let parent = root.join(&config.layout.cache).join(super::ANIMATIONS_DIR);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let name = format!("{kind}-{stamp}-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let work = parent.join(name);
    fs::create_dir_all(&work).map_err(|e| CoreError::io(&work, e))?;
    Ok(work)
}

/// Reads `jobs` (files already in `work`) and returns what was found.
pub(super) fn read(
    runner: &PipelineRunner,
    tools: &Toolbox,
    work: &Path,
    jobs: &[ReadJob],
) -> CoreResult<ReadReport> {
    crate::json::write_json(&work.join(JOBS_FILE), &jobs)?;
    let extra: Vec<String> = [
        "--jobs",
        JOBS_FILE,
        "--out",
        OUT_DIR,
        "--report",
        REPORT_FILE,
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    run_script(runner, tools, work, (READ_FILE, READ_SCRIPT), &extra)?;
    read_report(work)
}

pub(super) fn read_report(work: &Path) -> CoreResult<ReadReport> {
    let file = work.join(REPORT_FILE);
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))
}
