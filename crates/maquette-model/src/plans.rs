//! Planning canvases: node boards inside a project for planning with
//! agents (mechanics and story for a game, brainstorming for a website).
//! Each canvas is one plain JSON file, `plans/<id>.plan.json` in the
//! project folder, written whole and atomically with its nodes and edges
//! sorted by id and positions in whole pixels, so the same canvas
//! always writes the same bytes and a sync tool sees only real changes.
//!
//! ```json
//! {
//!   "version": 1,
//!   "name": "Combat ideas",
//!   "preset": "brainstorm",
//!   "created": "2026-11-02T10:00:00.000Z",
//!   "edited": "2026-11-02T10:12:00.000Z",
//!   "nodes": [
//!     { "id": "n-1", "kind": "note", "x": 40, "y": 80, "width": null,
//!       "title": "", "text": "Frost veil slows time", "image": null, "url": null,
//!       "by": null },
//!     { "id": "n-2", "kind": "image", "x": 320, "y": 80, "width": 240,
//!       "title": "Porch at dusk", "text": "", "image": "plans/images/porch-1a2b3c4d.png",
//!       "url": null, "by": null }
//!   ],
//!   "edges": [
//!     { "id": "e-1", "from": "n-1", "to": "n-2", "fromSide": "right", "toSide": "left",
//!       "kind": "relates", "label": null, "by": null }
//!   ]
//! }
//! ```
//!
//! The view (pan and zoom) is not stored: it changes on every scroll and
//! would make the file churn between computers. `by` is null for a person;
//! an agent's writes will carry its name. Pictures are project relative
//! paths: one from inside the project is linked where it is, one from
//! elsewhere is copied into `plans/images/` first ([`images`]).

use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

mod check;
mod files;
pub mod images;

pub use check::{check_name, is_project_relative, validate};
pub use files::{PlanDoc, PlanSummary, create, get, list, remove, rename, save};

pub const PLANS_DIR: &str = "plans";
pub const PLAN_SUFFIX: &str = ".plan.json";
pub const PLAN_VERSION: u32 = 1;
pub const NAME_MAX_CHARS: usize = 120;
pub const TITLE_MAX_CHARS: usize = 300;
pub const TEXT_MAX_CHARS: usize = 20_000;
pub const URL_MAX_CHARS: usize = 2_048;
/// A picture's project relative path.
pub const IMAGE_PATH_MAX_CHARS: usize = 1_024;
/// Who made a node or edge: an agent's name.
pub const BY_MAX_CHARS: usize = 64;
/// The biggest canvas file read. A canvas at every limit is far under it; a
/// bigger file (a broken sync, a wrong file) lists as broken unread.
pub const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
pub const ID_MAX_CHARS: usize = 64;
pub const MAX_NODES: usize = 2_000;
pub const MAX_EDGES: usize = 4_000;
/// How far from the origin a node may sit, in canvas pixels.
pub const MAX_COORD: i32 = 1_000_000;
pub const MIN_WIDTH: u32 = 120;
pub const MAX_WIDTH: u32 = 2_000;

/// What a canvas started from. Presets only set the starting content;
/// every canvas holds every kind of node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PlanPreset {
    Brainstorm,
    Story,
    Mechanics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum NodeKind {
    /// Plain text.
    Note,
    /// A picture inside the project, with an optional caption.
    Image,
    /// A web address with a title and a short note.
    Link,
}

/// The side of a node an edge leaves or arrives at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeKind {
    /// Two ideas belong together.
    Relates,
    /// The order of stops or beats.
    LeadsTo,
    /// Where research came from.
    Source,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanNode {
    pub id: String,
    pub kind: NodeKind,
    /// The top left corner on the canvas, in whole pixels.
    pub x: i32,
    pub y: i32,
    /// `None` keeps the kind's own width.
    pub width: Option<u32>,
    /// A link's title or a picture's caption; a note may leave it empty.
    pub title: String,
    /// A note's text or a link's short note.
    pub text: String,
    /// An image node's picture, relative to the project with `/` between
    /// folders.
    pub image: Option<String>,
    /// A link node's `http` or `https` address.
    pub url: Option<String>,
    /// Who made it: `None` for a person, else the agent's name.
    pub by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub from_side: Option<Side>,
    pub to_side: Option<Side>,
    pub kind: EdgeKind,
    pub label: Option<String>,
    pub by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanFile {
    pub version: u32,
    pub name: String,
    pub preset: PlanPreset,
    /// RFC 3339, when the canvas was made.
    pub created: String,
    /// RFC 3339, the last save that changed something.
    pub edited: String,
    pub nodes: Vec<PlanNode>,
    pub edges: Vec<PlanEdge>,
}

impl PlanFile {
    /// An empty canvas called `name`, made now.
    pub fn new(name: &str, preset: PlanPreset) -> Self {
        let now = crate::project::now_rfc3339();
        Self {
            version: PLAN_VERSION,
            name: name.trim().to_string(),
            preset,
            created: now.clone(),
            edited: now,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

/// `<project>/plans`.
pub fn dir(project_dir: &Path) -> PathBuf {
    project_dir.join(PLANS_DIR)
}

/// True for an id a canvas file may have: lower case ASCII letters, digits
/// and dashes, as [`crate::project::folder_slug`] makes them, and never a
/// name Windows keeps for a device (`con`, `nul`, `com1`).
pub fn is_plan_id(id: &str) -> bool {
    !id.is_empty()
        && !crate::project::is_windows_reserved(id)
        && id.len() <= ID_MAX_CHARS
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}

/// The file of the canvas `id`, which must be a plan id.
pub fn path(project_dir: &Path, id: &str) -> CoreResult<PathBuf> {
    if !is_plan_id(id) {
        return Err(CoreError::NotFound(format!("the canvas {id}")));
    }
    Ok(dir(project_dir).join(format!("{id}{PLAN_SUFFIX}")))
}

/// Reads and checks a canvas file.
///
/// # Errors
///
/// Fails when the file is missing, unreadable or over [`MAX_FILE_BYTES`]
/// (checked before reading), comes from a newer Maquette or holds values
/// [`validate`] refuses.
pub fn load_file(file: &Path) -> CoreResult<PlanFile> {
    let text = match read_capped(file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!(
                "the canvas file {}",
                file.display()
            )));
        }
        Err(err) if err.kind() == io::ErrorKind::FileTooLarge => {
            return Err(CoreError::Invalid(format!(
                "The canvas file is over {} MB, too big to be a canvas.",
                MAX_FILE_BYTES / (1024 * 1024)
            )));
        }
        Err(err) => return Err(CoreError::io(file, err)),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| CoreError::json(file, e))?;
    // The version first, so a newer file whose shape changed says so.
    let version = value.get("version").and_then(serde_json::Value::as_u64);
    if let Some(found) = version.filter(|v| *v > u64::from(PLAN_VERSION)) {
        return Err(CoreError::UnsupportedVersion {
            found: u32::try_from(found).unwrap_or(u32::MAX),
            supported: PLAN_VERSION,
        });
    }
    let plan: PlanFile = serde_json::from_value(value).map_err(|e| CoreError::json(file, e))?;
    validate(&plan)?;
    Ok(plan)
}

/// The file's text, refused unread when its size is over [`MAX_FILE_BYTES`].
/// The read stops at the cap too, for a file that grows meanwhile.
fn read_capped(file: &Path) -> io::Result<String> {
    let handle = fs::File::open(file)?;
    if handle.metadata()?.len() > MAX_FILE_BYTES {
        return Err(io::ErrorKind::FileTooLarge.into());
    }
    let mut text = String::new();
    handle.take(MAX_FILE_BYTES + 1).read_to_string(&mut text)?;
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        return Err(io::ErrorKind::FileTooLarge.into());
    }
    Ok(text)
}

/// The canvas as it is written: the current version, trimmed text, and
/// nodes and edges sorted by id.
pub fn normalized(plan: &PlanFile) -> PlanFile {
    let mut nodes: Vec<PlanNode> = plan
        .nodes
        .iter()
        .map(|n| PlanNode {
            title: n.title.trim().to_string(),
            url: n.url.as_ref().map(|u| u.trim().to_string()),
            ..n.clone()
        })
        .collect();
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    let mut edges: Vec<PlanEdge> = plan
        .edges
        .iter()
        .map(|e| PlanEdge {
            label: e
                .label
                .as_ref()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty()),
            ..e.clone()
        })
        .collect();
    edges.sort_by(|a, b| a.id.cmp(&b.id));
    PlanFile {
        version: PLAN_VERSION,
        name: plan.name.trim().to_string(),
        nodes,
        edges,
        ..plan.clone()
    }
}

pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<PlanFile>()
        .register::<PlanDoc>()
        .register::<PlanSummary>()
        .register::<images::PlanImage>()
}

#[cfg(test)]
mod tests;
