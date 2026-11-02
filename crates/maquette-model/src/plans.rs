//! Planning canvases: node boards inside a project for planning with
//! agents (mechanics and story for a game, brainstorming for a website).
//! Each canvas is one plain JSON file, `plans/<id>.plan.json` in the
//! project folder, written whole and atomically with its nodes and edges
//! sorted by id and positions in whole pixels, so the same canvas
//! always writes the same bytes and a sync tool sees only real changes.
//!
//! ```json
//! {
//!   "version": 2,
//!   "name": "Combat ideas",
//!   "preset": "brainstorm",
//!   "created": "2026-11-02T10:00:00.000Z",
//!   "edited": "2026-11-02T10:12:00.000Z",
//!   "nodes": [
//!     { "id": "n-1", "kind": "note", "x": 40, "y": 80, "width": null,
//!       "title": "", "text": "Frost veil slows time", "image": null, "url": null,
//!       "asset": null, "direction": null, "by": null },
//!     { "id": "n-2", "kind": "image", "x": 320, "y": 80, "width": 240,
//!       "title": "Porch at dusk", "text": "", "image": "plans/images/porch-1a2b3c4d.png",
//!       "url": null, "asset": null, "direction": null, "by": null },
//!     { "id": "n-3", "kind": "asset", "x": 640, "y": 80, "width": null,
//!       "title": "", "text": "", "image": null, "url": null,
//!       "asset": "frost-warden", "direction": null, "by": null }
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
//!
//! An asset node keeps only the asset's id and a Lab direction node only
//! the direction's folder name: their name, picture and state are read
//! from the project each time they show, never copied into the file, so a
//! canvas never goes stale and a removed asset only shows as gone. Files
//! of an older version are brought up to this one as they are read
//! ([`migrate`]) and written as this version only when a save changes them.

use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

pub mod agent;
mod change;
mod check;
mod files;
pub mod images;
pub mod log;
pub mod migrate;
pub mod ops;
mod place;
pub mod summary;
pub mod text;

pub use change::{Changed, Writer, change, undo};
pub use check::{check_name, is_link_id, is_project_relative, validate};
pub use files::{PlanDoc, PlanSummary, create, create_by, get, list, remove, rename, save};
pub use text::trimmed;

pub const PLANS_DIR: &str = "plans";
pub const PLAN_SUFFIX: &str = ".plan.json";
pub const PLAN_VERSION: u32 = 2;
pub const NAME_MAX_CHARS: usize = 120;
pub const TITLE_MAX_CHARS: usize = 300;
pub const TEXT_MAX_CHARS: usize = 20_000;
pub const URL_MAX_CHARS: usize = 2_048;
/// A picture's project relative path.
pub const IMAGE_PATH_MAX_CHARS: usize = 1_024;
/// Who made a node or edge: an agent's name.
pub const BY_MAX_CHARS: usize = 64;
/// The id of an asset or a Lab direction a node links.
pub const LINK_ID_MAX_CHARS: usize = 128;
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
    /// An asset of the project, by its id. Its name, picture and state are
    /// read from the project each time.
    Asset,
    /// A direction in the project's Lab, by its folder name. Read live too.
    Direction,
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
    /// An asset node's asset id. Nothing else about the asset is stored.
    pub asset: Option<String>,
    /// A Lab direction node's direction id, its folder under `concepts/`.
    pub direction: Option<String>,
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
            name: trimmed(name).to_string(),
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

/// Reads and checks a canvas file, normalised ([`normalized`]) so the
/// canvas a view gets is the one a save writes back.
///
/// # Errors
///
/// Fails when the file is missing, unreadable or over [`MAX_FILE_BYTES`]
/// (checked before reading), comes from a newer Maquette or holds values
/// [`validate`] refuses once normalised.
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
    let value = migrate::to_current(value)?;
    let plan: PlanFile = serde_json::from_value(value).map_err(|e| CoreError::json(file, e))?;
    // Checked as it would be written, so a title padded past the limit
    // with spaces still opens.
    let plan = normalized(&plan);
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

/// The canvas as it is written: the current version, trimmed name, node
/// titles and addresses, empty edge labels as none, and nodes and edges
/// sorted by id. Trimming takes the [`text::TRIMMED`] set. The view does the same (`apps/web/src/state/plans.ts`,
/// `planOf`), so a canvas it sends back unchanged is the canvas saved.
pub fn normalized(plan: &PlanFile) -> PlanFile {
    let mut nodes: Vec<PlanNode> = plan
        .nodes
        .iter()
        .map(|n| PlanNode {
            title: trimmed(&n.title).to_string(),
            url: n.url.as_deref().map(|u| trimmed(u).to_string()),
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
                .map(|l| trimmed(l).to_string())
                .filter(|l| !l.is_empty()),
            ..e.clone()
        })
        .collect();
    edges.sort_by(|a, b| a.id.cmp(&b.id));
    PlanFile {
        version: PLAN_VERSION,
        name: trimmed(&plan.name).to_string(),
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
        .register::<ops::PlanOp>()
        .register::<log::PlanLogEntry>()
        .register::<log::PlanLogLine>()
}

#[cfg(test)]
mod tests;
