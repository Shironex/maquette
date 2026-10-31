//! A render's record: its models, what each file is doing and how it ended.
//! Written to `<workspace>/exports/.renders/<id>.json` after every change,
//! so recent renders list and a stopped one goes on.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::RenderSettings;
use super::{RenderStyle, plan::RenderTarget};
use crate::error::{CoreError, CoreResult};
use crate::packs::{EXPORTS_DIR, PackSource};

/// The folder under `exports` the pictures go in.
pub(super) const EXPORTS_RENDERS: &str = "renders";
/// Where the records live, under the exports folder.
pub(super) const RECORDS_DIR: &str = ".renders";
const NEWEST: usize = 20;
pub(super) const RECORD_VERSION: u32 = 1;

/// The box a turntable's pixels change in, top down, on its square canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MovingBox {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub canvas: u32,
}

/// Why a file came out wrong, so the app can offer the next step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ProblemKind {
    /// Toon on a model with nothing to colour it with: left out.
    NoTexture,
    /// A GIF over the size target: written, but look at it.
    OverTarget,
    /// The line pass ran out of memory.
    OutOfMemory,
    /// ffmpeg is missing or cannot write this format.
    MissingTool,
    /// Compressed so that Blender cannot read it.
    Unreadable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderProblem {
    pub kind: ProblemKind,
    pub message: String,
    /// What a WebP loop of the same turn would weigh, about.
    pub webp_bytes: Option<f64>,
    /// The last of what Blender said, for Show the log, when it failed.
    #[serde(default)]
    #[specta(optional)]
    pub log: Option<String>,
}

/// Where one file stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RenderFileState {
    Queued,
    Running,
    Done,
    /// Written, with a problem to look at.
    Warn,
    Failed,
    /// Left out, and the problem says why.
    Skipped,
}

impl RenderFileState {
    pub fn is_settled(self) -> bool {
        !matches!(self, Self::Queued | Self::Running)
    }
}

/// One file a render makes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderFile {
    pub style: RenderStyle,
    /// Where it goes, an absolute path.
    pub file: String,
    pub state: RenderFileState,
    pub bytes: f64,
    pub seconds: f64,
    pub problem: Option<RenderProblem>,
    pub moving: Option<MovingBox>,
}

impl RenderFile {
    pub(super) fn new(style: RenderStyle, file: &Path) -> Self {
        Self {
            style,
            file: file.display().to_string(),
            state: RenderFileState::Queued,
            bytes: 0.0,
            seconds: 0.0,
            problem: None,
            moving: None,
        }
    }
}

/// One model of a render.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderItem {
    pub target: RenderTarget,
    pub files: Vec<RenderFile>,
}

/// How a whole render stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RenderState {
    Running,
    /// Stopped by a person, or the app closed: it can go on.
    Stopped,
    Done,
}

/// A render, as the app and agents read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderRecord {
    pub version: u32,
    pub id: String,
    pub source: PackSource,
    /// "of Winter Spire", "from the library".
    pub label: String,
    /// The `exports/renders` folder of its files.
    pub dir: String,
    pub settings: RenderSettings,
    pub state: RenderState,
    pub items: Vec<RenderItem>,
    /// The one contact sheet of all the models, when it was asked for.
    pub sheet: Option<RenderFile>,
    pub created_at: String,
    pub finished_at: Option<String>,
    pub message: Option<String>,
}

impl RenderRecord {
    fn every_file(&self) -> impl Iterator<Item = &RenderFile> {
        self.items
            .iter()
            .flat_map(|i| i.files.iter())
            .chain(self.sheet.iter())
    }

    /// Files settled so far.
    pub fn done(&self) -> usize {
        self.every_file().filter(|f| f.state.is_settled()).count()
    }

    /// Files in all.
    pub fn total(&self) -> usize {
        self.every_file().count()
    }

    /// Files that came out with a problem to look at.
    pub fn problems(&self) -> usize {
        self.every_file().filter(|f| f.problem.is_some()).count()
    }
}

fn records_dir(root: &Path) -> PathBuf {
    root.join(EXPORTS_DIR).join(RECORDS_DIR)
}

/// Scratch space for one render, deleted when its files are done.
pub(super) fn work_dir(root: &Path, id: &str) -> PathBuf {
    records_dir(root).join(id)
}

fn record_path(root: &Path, id: &str) -> CoreResult<PathBuf> {
    let safe = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !safe {
        return Err(CoreError::NotFound(format!("the render {id}")));
    }
    Ok(records_dir(root).join(format!("{id}.json")))
}

pub(super) fn save(root: &Path, record: &RenderRecord) -> CoreResult<()> {
    let path = record_path(root, &record.id)?;
    let dir = records_dir(root);
    fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    crate::json::write_json(&path, record)
}

/// A saved render. One the app closed on reads as stopped.
///
/// # Errors
///
/// Fails for an unknown render.
pub(super) fn load(root: &Path, id: &str) -> CoreResult<RenderRecord> {
    let path = record_path(root, id)?;
    if !path.is_file() {
        return Err(CoreError::NotFound(format!("the render {id}")));
    }
    let text = fs::read_to_string(&path).map_err(|e| CoreError::io(&path, e))?;
    let mut record: RenderRecord =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&path, e))?;
    if record.state == RenderState::Running {
        record.state = RenderState::Stopped;
    }
    for file in record.items.iter_mut().flat_map(|i| i.files.iter_mut()) {
        if file.state == RenderFileState::Running {
            file.state = RenderFileState::Queued;
        }
    }
    Ok(record)
}

/// The newest renders first.
pub(super) fn recent(root: &Path) -> Vec<RenderRecord> {
    let Ok(read) = fs::read_dir(records_dir(root)) else {
        return Vec::new();
    };
    let mut all: Vec<RenderRecord> = read
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name();
            let stem = name.to_str()?.strip_suffix(".json")?.to_string();
            load(root, &stem).ok()
        })
        .collect();
    all.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
    all.truncate(NEWEST);
    all
}

/// The first of `stem.ext`, `stem-2.ext` and so on nobody has written, or
/// `stem.ext` itself when earlier renders may go.
pub(super) fn file_in(dir: &Path, stem: &str, extension: &str, replace: bool) -> PathBuf {
    let plain = dir.join(format!("{stem}.{extension}"));
    if replace || !plain.exists() {
        return plain;
    }
    let mut n = 2;
    loop {
        let next = dir.join(format!("{stem}-{n}.{extension}"));
        if !next.exists() {
            return next;
        }
        n += 1;
    }
}

/// A new render's id: the time it started, and a little noise so two renders
/// in one second get their own.
pub(super) fn new_id() -> String {
    let stamp: String = crate::project::now_rfc3339()
        .chars()
        .filter(char::is_ascii_digit)
        .take(14)
        .collect();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    format!("render-{stamp}-{:04x}", nanos & 0xffff)
}
