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

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

mod files;
pub mod images;

pub use files::{PlanDoc, PlanSummary, create, get, list, remove, rename, save};

pub const PLANS_DIR: &str = "plans";
pub const PLAN_SUFFIX: &str = ".plan.json";
pub const PLAN_VERSION: u32 = 1;
pub const NAME_MAX_CHARS: usize = 120;
pub const TITLE_MAX_CHARS: usize = 300;
pub const TEXT_MAX_CHARS: usize = 20_000;
pub const URL_MAX_CHARS: usize = 2_048;
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
/// and single dashes, as [`crate::project::folder_slug`] makes them.
pub fn is_plan_id(id: &str) -> bool {
    !id.is_empty()
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
/// Fails when the file is missing or unreadable, comes from a newer
/// Maquette or holds values [`validate`] refuses.
pub fn load_file(file: &Path) -> CoreResult<PlanFile> {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!(
                "the canvas file {}",
                file.display()
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

fn invalid<T>(message: String) -> CoreResult<T> {
    Err(CoreError::Invalid(message))
}

fn is_item_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= ID_MAX_CHARS
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Checks a canvas name: one line, not blank, not too long.
pub fn check_name(name: &str) -> CoreResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return invalid("Give the canvas a name.".into());
    }
    if name.chars().count() > NAME_MAX_CHARS {
        return invalid(format!(
            "Keep the canvas name under {NAME_MAX_CHARS} characters."
        ));
    }
    if name.chars().any(char::is_control) {
        return invalid("Keep the canvas name on one line.".into());
    }
    Ok(())
}

/// A project relative picture path: `/` separators, no `..`, not absolute.
pub fn is_project_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn is_web_address(url: &str) -> bool {
    let url = url.trim();
    (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() > "https://".len()
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn check_node(node: &PlanNode) -> CoreResult<()> {
    let id = &node.id;
    if !is_item_id(id) {
        return invalid(format!("The node id {id} is not a plain id."));
    }
    let within = |v: i32| v.unsigned_abs() <= MAX_COORD.unsigned_abs();
    if !within(node.x) || !within(node.y) {
        return invalid(format!("The node {id} sits too far out on the canvas."));
    }
    if let Some(w) = node.width
        && !(MIN_WIDTH..=MAX_WIDTH).contains(&w)
    {
        return invalid(format!(
            "The node {id} must be {MIN_WIDTH} to {MAX_WIDTH} pixels wide."
        ));
    }
    if node.title.chars().count() > TITLE_MAX_CHARS {
        return invalid(format!(
            "The node {id} has a title over {TITLE_MAX_CHARS} characters."
        ));
    }
    if node.text.chars().count() > TEXT_MAX_CHARS {
        return invalid(format!(
            "The node {id} holds over {TEXT_MAX_CHARS} characters."
        ));
    }
    match node.kind {
        NodeKind::Image => match node.image.as_deref() {
            Some(path) if is_project_relative(path) => {}
            _ => {
                return invalid(format!(
                    "The picture node {id} needs a picture in the project."
                ));
            }
        },
        NodeKind::Link => match node.url.as_deref() {
            Some(url) if url.len() <= URL_MAX_CHARS && is_web_address(url) => {}
            _ => {
                return invalid(format!(
                    "The link node {id} needs an http or https address."
                ));
            }
        },
        NodeKind::Note => {}
    }
    Ok(())
}

/// Checks a whole canvas: its name, the limits, every node, and edges that
/// join two nodes it has.
///
/// # Errors
///
/// [`CoreError::Invalid`] saying what to fix.
pub fn validate(plan: &PlanFile) -> CoreResult<()> {
    check_name(&plan.name)?;
    if plan.nodes.len() > MAX_NODES {
        return invalid(format!("A canvas holds at most {MAX_NODES} nodes."));
    }
    if plan.edges.len() > MAX_EDGES {
        return invalid(format!(
            "A canvas holds at most {MAX_EDGES} links between nodes."
        ));
    }
    let mut ids = BTreeSet::new();
    for node in &plan.nodes {
        check_node(node)?;
        if !ids.insert(node.id.as_str()) {
            return invalid(format!("Two nodes share the id {}.", node.id));
        }
    }
    let mut edge_ids = BTreeSet::new();
    for edge in &plan.edges {
        if !is_item_id(&edge.id) || !edge_ids.insert(edge.id.as_str()) {
            return invalid(format!(
                "The edge id {} is not a plain, unique id.",
                edge.id
            ));
        }
        if !ids.contains(edge.from.as_str()) || !ids.contains(edge.to.as_str()) {
            return invalid(format!(
                "The edge {} joins a node the canvas does not have.",
                edge.id
            ));
        }
        if edge
            .label
            .as_ref()
            .is_some_and(|l| l.chars().count() > TITLE_MAX_CHARS)
        {
            return invalid(format!(
                "The edge {} has a label that is too long.",
                edge.id
            ));
        }
    }
    Ok(())
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
