//! Rig doctor records: one `record.json` in each case's work folder.

use std::fs;
use std::path::{Path, PathBuf};

use super::{RigModel, RigRecord, RigSource, RigStarter, RigState, RigStep, WORK_DIR};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::pipeline::history::now_ms;
use crate::workspace::WorkspaceConfig;

const RECORD_FILE: &str = "record.json";

/// The work folder of case `id`.
pub fn work_dir(root: &Path, config: &WorkspaceConfig, id: &str) -> PathBuf {
    root.join(&config.layout.cache).join(WORK_DIR).join(id)
}

/// Writes a record.
///
/// # Errors
///
/// Fails when the folder cannot be written.
pub fn save(record: &RigRecord) -> CoreResult<()> {
    let dir = Path::new(&record.dir);
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    write_json(&dir.join(RECORD_FILE), record)
}

/// Case `id`, or `None` when there is none.
///
/// # Errors
///
/// Fails when the record cannot be read.
pub fn load(root: &Path, config: &WorkspaceConfig, id: &str) -> CoreResult<Option<RigRecord>> {
    if id.contains(['/', '\\']) || id.contains("..") {
        return Err(CoreError::Invalid(format!(
            "{id} is not a rig doctor case."
        )));
    }
    let file = work_dir(root, config, id).join(RECORD_FILE);
    if !file.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| CoreError::json(&file, e))
}

/// Every case, newest first. A record an older version wrote is skipped.
pub fn recent(root: &Path, config: &WorkspaceConfig) -> Vec<RigRecord> {
    let folder = root.join(&config.layout.cache).join(WORK_DIR);
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found: Vec<RigRecord> = entries
        .flatten()
        .filter_map(|e| fs::read_to_string(e.path().join(RECORD_FILE)).ok())
        .filter_map(|text| serde_json::from_str(&text).ok())
        .collect();
    found.sort_by(|a, b| b.updated_ms.total_cmp(&a.updated_ms));
    found
}

/// A new case, before its model is read.
pub(super) fn blank(
    id: String,
    name: String,
    dir: &Path,
    input: &Path,
    source: RigSource,
) -> RigRecord {
    RigRecord {
        id,
        name,
        started_by: RigStarter::Person,
        route: None,
        state: RigState::Ready,
        step: RigStep::Read,
        source,
        input: input.display().to_string(),
        dir: dir.display().to_string(),
        model: RigModel {
            rig: String::new(),
            joints: 0,
            skinned_meshes: 0,
            clips: Vec::new(),
            triangles: 0,
            height: 0.0,
            textures: 0,
            hand_far: 0,
            arm_far: 0,
            islands: 0,
            one_island: false,
            finding: None,
        },
        issues: Vec::new(),
        creature: None,
        before: None,
        after: None,
        defects: Vec::new(),
        stats: Vec::new(),
        fix_steps: Vec::new(),
        checks: Vec::new(),
        joint_move: None,
        round: 0,
        draft: None,
        progress: None,
        stages: Vec::new(),
        log: Vec::new(),
        stop: None,
        landed: None,
        error: None,
        updated_ms: now_ms(),
    }
}

/// Registers this module's IPC types with the shell's TypeScript export.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<super::RigRoute>()
        .register::<super::RigState>()
        .register::<super::RigStep>()
        .register::<super::RigStarter>()
        .register::<super::RigRun>()
        .register::<super::RigSource>()
        .register::<super::RigIssue>()
        .register::<super::RigModel>()
        .register::<super::RigRow>()
        .register::<super::RigSheet>()
        .register::<super::DefectKind>()
        .register::<super::RigDefect>()
        .register::<super::RigChain>()
        .register::<super::RigCreature>()
        .register::<super::RigFixStep>()
        .register::<super::RigCheck>()
        .register::<super::RigStat>()
        .register::<super::JointMoveState>()
        .register::<super::JointMove>()
        .register::<super::RigStage>()
        .register::<super::RigLogLine>()
        .register::<super::RigProgress>()
        .register::<super::RigStopKind>()
        .register::<super::RigStop>()
        .register::<super::RigRecord>()
}
