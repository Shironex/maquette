//! A pack's record: its settings, every model with what its run and check
//! found, and where it stands. Kept in `<workspace>/exports/.packs/<id>.json`
//! and rewritten after every model, so a pack Blender stopped in goes on
//! from the model it reached, and the recent packs list reads them.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::plan::PackModel;
use super::{EXPORTS_DIR, PackSettings, PackSource, RECORDS_DIR};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;

pub(super) const RECORD_VERSION: u32 = 1;
/// The file that marks a folder as a pack Maquette wrote, so writing the
/// same pack again may replace it and nothing else is ever replaced.
pub(super) const MARKER: &str = ".maquette-pack";
/// How many packs the recent list shows.
pub(super) const RECENT: usize = 8;

/// Where one model of a pack stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PackItemState {
    Queued,
    Running,
    /// Written, opened again and as asked.
    Pass,
    /// Written, but the check found something a person should look at.
    Warn,
    /// Not written, or written wrong.
    Fail,
    /// A warning a person chose to keep.
    Kept,
}

impl PackItemState {
    /// Done with, one way or another.
    pub fn is_settled(self) -> bool {
        !matches!(self, Self::Queued | Self::Running)
    }

    /// Goes in the zip without asking.
    pub fn passes(self) -> bool {
        matches!(self, Self::Pass | Self::Kept)
    }
}

/// One model of a pack and what its run found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackItem {
    pub model: PackModel,
    pub state: PackItemState,
    /// The budget it is decimated to, after any raise.
    pub budget: Option<u32>,
    /// Its triangles once written.
    pub triangles: Option<u32>,
    /// Its height in metres, as the check measured it.
    pub height_m: Option<f32>,
    /// The files written, relative to the pack folder.
    pub files: Vec<String>,
    /// The check's render of what was written, an absolute path.
    pub picture: Option<String>,
    /// What the check found, in a line.
    pub note: Option<String>,
    /// For thin parts lost: the budget that keeps them, about.
    pub suggest: Option<u32>,
    /// Clips left out because they may be Mixamo's.
    pub clips_left_out: u32,
    /// How long its run took, in ms.
    pub took_ms: f64,
    /// What its files weigh.
    pub bytes: f64,
}

impl PackItem {
    pub fn new(model: PackModel, budget: Option<u32>) -> Self {
        Self {
            model,
            state: PackItemState::Queued,
            budget,
            triangles: None,
            height_m: None,
            files: Vec::new(),
            picture: None,
            note: None,
            suggest: None,
            clips_left_out: 0,
            took_ms: 0.0,
            bytes: 0.0,
        }
    }
}

/// Where a pack stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PackState {
    /// Models are being written and checked.
    Running,
    /// Every model ran; some need a person before the zip.
    Waiting,
    /// The README, manifest, contact sheet and zip are being written.
    Zipping,
    Done,
    /// Stopped by a person, or Maquette closed, before every model ran.
    Stopped,
    /// Something outside one model went wrong (the folder, Blender).
    Failed,
}

impl PackState {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Running | Self::Zipping)
    }
}

/// `exports/.packs/<id>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackRecord {
    pub version: u32,
    pub id: String,
    pub name: String,
    /// "12 models from the library".
    pub label: String,
    pub source: PackSource,
    pub settings: PackSettings,
    /// The pack folder, an absolute path.
    pub dir: String,
    /// The zip, once written.
    pub zip: Option<String>,
    pub state: PackState,
    pub items: Vec<PackItem>,
    pub created_at: String,
    pub finished_at: Option<String>,
    /// What the zip (or the folder, without one) weighs, once done.
    pub bytes: Option<f64>,
    /// Why it stopped or failed, or what finishing left out.
    pub message: Option<String>,
    /// How many models finishing left out of the pack.
    #[serde(default)]
    pub left_out: u32,
    pub preset: Option<String>,
}

/// What a record from before `leftOut` said it left out, in its message.
fn left_out_in(message: &str) -> u32 {
    message
        .split_once(" model")
        .filter(|(_, rest)| rest.contains("left out of the pack"))
        .and_then(|(count, _)| count.parse().ok())
        .unwrap_or(0)
}

impl PackRecord {
    pub fn done(&self) -> usize {
        self.items.iter().filter(|i| i.state.is_settled()).count()
    }

    pub fn triangles_before(&self) -> u64 {
        self.items
            .iter()
            .map(|i| u64::from(i.model.triangles()))
            .sum()
    }

    pub fn took_ms(&self) -> f64 {
        self.items.iter().map(|i| i.took_ms).sum()
    }
}

fn records_dir(root: &Path) -> PathBuf {
    root.join(EXPORTS_DIR).join(RECORDS_DIR)
}

fn record_path(root: &Path, id: &str) -> CoreResult<PathBuf> {
    if !crate::workspace::is_single_segment(id) || id.starts_with('.') {
        return Err(CoreError::Invalid(format!("{id:?} is not a pack id.")));
    }
    Ok(records_dir(root).join(format!("{id}.json")))
}

/// Writes a pack's record.
///
/// # Errors
///
/// Fails when the file cannot be written.
pub(super) fn save(root: &Path, record: &PackRecord) -> CoreResult<()> {
    let path = record_path(root, &record.id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    write_json(&path, record)
}

/// Reads one pack's record.
///
/// # Errors
///
/// Fails for an unknown pack or a file from a newer Maquette.
pub fn load(root: &Path, id: &str) -> CoreResult<PackRecord> {
    let path = record_path(root, id)?;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!("the pack {id}")));
        }
        Err(err) => return Err(CoreError::io(&path, err)),
    };
    let mut record: PackRecord =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&path, e))?;
    if record.left_out == 0 {
        record.left_out = record.message.as_deref().map_or(0, left_out_in);
    }
    if record.version > RECORD_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: record.version,
            supported: RECORD_VERSION,
        });
    }
    Ok(record)
}

/// The newest packs first, at most [`RECENT`]. Unreadable records are
/// skipped.
pub fn recent(root: &Path) -> Vec<PackRecord> {
    let Ok(read) = fs::read_dir(records_dir(root)) else {
        return Vec::new();
    };
    let mut found: Vec<PackRecord> = read
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let id = name.strip_suffix(".json")?.to_string();
            load(root, &id).ok()
        })
        .collect();
    found.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
    found.truncate(RECENT);
    found
}

fn inside(path: &Path, root: &Path) -> bool {
    let nearest = path.ancestors().find(|p| p.exists());
    nearest
        .and_then(|p| p.canonicalize().ok())
        .zip(root.canonicalize().ok())
        .is_some_and(|(p, r)| p.starts_with(r))
}

/// The pack folder and zip for a pack named `name` in `out` (the
/// workspace's exports folder when `None`). A relative `out` is read from
/// the workspace folder. The app may write anywhere a person picked
/// (`from_anywhere`); an agent only inside the workspace.
///
/// # Errors
///
/// Refuses a folder outside the workspace for an agent, a folder a person
/// picked that is not there, and a pack folder that holds files Maquette
/// did not write.
pub fn out_dir(
    root: &Path,
    out: Option<&str>,
    name: &str,
    from_anywhere: bool,
) -> CoreResult<(PathBuf, PathBuf)> {
    let parent = match out.map(str::trim).filter(|o| !o.is_empty()) {
        None => root.join(EXPORTS_DIR),
        Some(out) if crate::fs::looks_absolute(out) => PathBuf::from(out),
        Some(out) => crate::fs::from_workspace(root, out),
    };
    let in_workspace = inside(&parent, root);
    if !from_anywhere && !in_workspace {
        return Err(CoreError::Invalid(format!(
            "{} is outside the workspace. Give a folder inside the workspace, or leave it out for the exports folder.",
            parent.display()
        )));
    }
    if !in_workspace && !parent.is_dir() {
        return Err(CoreError::NotFound(format!(
            "the folder {}",
            parent.display()
        )));
    }
    let dir = parent.join(name);
    let foreign = fs::read_dir(&dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
        && !dir.join(MARKER).is_file();
    if foreign {
        return Err(CoreError::Invalid(format!(
            "{} is already there and Maquette did not write it. Pick another name or folder.",
            dir.display()
        )));
    }
    let zip = parent.join(format!("{name}.zip"));
    Ok((dir, zip))
}

#[cfg(test)]
mod left_out_tests {
    use super::left_out_in;

    #[test]
    fn an_old_message_says_how_many_were_left_out() {
        assert_eq!(left_out_in("1 model left out of the pack."), 1);
        assert_eq!(left_out_in("12 models left out of the pack."), 12);
        assert_eq!(left_out_in("Stopped at 3 models of 9."), 0);
    }
}
