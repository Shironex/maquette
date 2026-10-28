//! Assembling one prop from parts made as separate models: each part its
//! own asset (made from an object ref, say) with its own versions. The
//! parts join in headless Blender (`assemble.py`) into one GLB, a node per
//! part under one node named after the prop, so object states and explode
//! tracks pick them by name. It lands as the next version of a target
//! asset, new or existing.
//!
//! The recipe is saved per prop in `assemblies.json` next to
//! `project.json`, so assembling again picks up a part's newer version:
//!
//! ```json
//! {
//!   "version": 1,
//!   "props": {
//!     "sigil-gate": {
//!       "name": "sigil_gate",
//!       "parts": [
//!         { "asset": "sigil-gate-door", "name": "door_leaf", "pivot": "left-hinge",
//!           "at": [-0.6, 0, 0], "turn": 0 }
//!       ]
//!     }
//!   }
//! }
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::blender::{self, Script};
use super::versions::{self, Target};
use crate::assets::{self, AssetKind, AssetVersion};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::pipeline::tools::Toolbox;
use crate::prepare::MAX_PART_NAME;
use crate::project::{self, ProjectManifest};
use crate::workspace::FolderLayout;

pub const SCRIPT: &str = include_str!("assemble.py");
pub const ASSEMBLIES_FILE: &str = "assemblies.json";
const ASSEMBLIES_VERSION: u32 = 1;
const SCRIPT_FILE: &str = "assemble.py";
const PLAN_FILE: &str = "plan.json";
const OUTPUT_FILE: &str = "prop.glb";
const STATS_FILE: &str = "stats.json";
pub const MIN_PARTS: usize = 2;
pub const MAX_PARTS: usize = 16;
/// How far a part may sit from the prop's origin, in metres.
pub const MAX_OFFSET: f32 = 1000.0;

/// Where a part turns about.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Pivot {
    /// The origin the model was made with.
    Origin,
    #[default]
    BottomCentre,
    Centre,
    /// The left edge at the bottom, seen from the front: a door's hinge.
    LeftHinge,
    RightHinge,
}

/// One part of a prop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssemblePart {
    /// The asset whose model it is.
    pub asset: String,
    /// Its version; `None` takes the current one, so a redo flows in.
    #[serde(default)]
    pub version: Option<u32>,
    /// The node name it ships as.
    pub name: String,
    pub pivot: Pivot,
    /// Where its pivot goes, in metres (Y up); `None` keeps it where its
    /// model put it.
    #[serde(default)]
    pub at: Option<[f32; 3]>,
    /// Degrees about the up axis.
    #[serde(default)]
    pub turn: f32,
}

/// A prop to assemble, in node order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssembleRequest {
    pub project: String,
    /// The asset it becomes a version of; `None` makes (or finds) one
    /// named after the prop.
    #[serde(default)]
    pub asset: Option<String>,
    /// The prop's name: its root node and the new asset's name.
    pub name: String,
    pub parts: Vec<AssemblePart>,
}

/// One node of the assembled prop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssembledNode {
    pub name: String,
    /// Where it sits under the root, in metres (Y up).
    pub at: [f32; 3],
}

/// What an assembly made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Assembled {
    pub project: ProjectManifest,
    pub asset: String,
    pub version: AssetVersion,
    pub root: String,
    pub nodes: Vec<AssembledNode>,
}

/// The recipe kept for a prop: the request without its project and asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub name: String,
    pub parts: Vec<AssemblePart>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct AssembliesFile {
    version: u32,
    #[serde(default)]
    props: BTreeMap<String, Recipe>,
}

#[derive(Debug, Deserialize)]
struct Stats {
    root: String,
    parts: Vec<AssembledNode>,
}

fn invalid(message: String) -> CoreError {
    CoreError::Invalid(message)
}

/// Checks a request against the project: two to sixteen parts, each with
/// its own name and a model, none the target itself, offsets in range.
/// Returns each part's model file, relative to the project folder.
///
/// # Errors
///
/// Returns the first problem as a sentence.
pub fn check(manifest: &ProjectManifest, request: &AssembleRequest) -> CoreResult<Vec<String>> {
    let name = request.name.trim();
    if name.is_empty() || name.chars().count() > MAX_PART_NAME {
        return Err(invalid(format!(
            "Give the prop a name of 1 to {MAX_PART_NAME} characters."
        )));
    }
    let count = request.parts.len();
    if !(MIN_PARTS..=MAX_PARTS).contains(&count) {
        return Err(invalid(format!(
            "Assemble {MIN_PARTS} to {MAX_PARTS} parts, not {count}."
        )));
    }
    let target = match &request.asset {
        Some(id) => Some(assets::find(manifest, id)?.id.clone()),
        None => assets::find_by_name(manifest, name).map(|a| a.id.clone()),
    };
    let mut files = Vec::new();
    for (n, part) in request.parts.iter().enumerate() {
        let part_name = part.name.trim();
        if part_name.is_empty() || part_name.chars().count() > MAX_PART_NAME {
            return Err(invalid(format!(
                "Part {}: give it a name of 1 to {MAX_PART_NAME} characters.",
                n + 1
            )));
        }
        if part_name == name {
            return Err(invalid(format!(
                "{part_name} is the prop's own name. Give the part another."
            )));
        }
        if request.parts[..n]
            .iter()
            .any(|p| p.name.trim() == part_name)
        {
            return Err(invalid(format!(
                "Two parts are named {part_name}. Give each its own name."
            )));
        }
        let asset = assets::find(manifest, &part.asset)?;
        if target.as_deref() == Some(asset.id.as_str()) {
            return Err(invalid(format!(
                "{} is the prop itself, so it cannot be one of its parts.",
                asset.name
            )));
        }
        let version = match part.version {
            Some(v) => asset.versions.iter().find(|x| x.version == v),
            None => asset.versions.last(),
        }
        .ok_or_else(|| {
            invalid(format!(
                "{} has no model{} yet.",
                asset.name,
                part.version.map(|v| format!(" v{v}")).unwrap_or_default()
            ))
        })?;
        let numbers = part.at.unwrap_or_default();
        if !part.turn.is_finite()
            || numbers
                .iter()
                .any(|v| !v.is_finite() || v.abs() > MAX_OFFSET)
        {
            return Err(invalid(format!(
                "{part_name}: keep it within {MAX_OFFSET} m of the prop's origin."
            )));
        }
        files.push(version.file.clone());
    }
    Ok(files)
}

/// The script's plan: each part's model as an absolute path.
fn plan(request: &AssembleRequest, project_dir: &Path, files: &[String]) -> serde_json::Value {
    let parts: Vec<serde_json::Value> = request
        .parts
        .iter()
        .zip(files)
        .map(|(part, file)| {
            serde_json::json!({
                "file": project_dir.join(file).display().to_string(),
                "name": part.name.trim(),
                "pivot": part.pivot,
                "at": part.at,
                "turn": part.turn,
            })
        })
        .collect();
    serde_json::json!({ "name": request.name.trim(), "parts": parts })
}

fn script() -> Script<'static> {
    Script {
        file: SCRIPT_FILE,
        text: SCRIPT,
        what: "the assembly",
        args: [
            "--plan",
            PLAN_FILE,
            "--output",
            OUTPUT_FILE,
            "--stats",
            STATS_FILE,
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect(),
        outputs: vec![OUTPUT_FILE.to_string(), STATS_FILE.to_string()],
    }
}

/// Assembles the prop in `work` (a folder of its own) and adds it to the
/// project as the next version of its target. Blocks until Blender is done.
///
/// # Errors
///
/// Fails for a request [`check`] refuses, without Blender, or when the
/// assembly fails.
pub fn assemble(
    root: &Path,
    layout: &FolderLayout,
    work: &Path,
    request: &AssembleRequest,
    tools: &Toolbox,
) -> CoreResult<Assembled> {
    let manifest = project::load(root, layout, &request.project)?;
    let files = check(&manifest, request)?;
    let project_dir = project::dir(root, layout, &request.project)?;
    fs::create_dir_all(work).map_err(|e| CoreError::io(work, e))?;
    write_json(&work.join(PLAN_FILE), &plan(request, &project_dir, &files))?;
    blender::run(work, &script(), tools)?;
    let file = work.join(STATS_FILE);
    let text = fs::read_to_string(&file).map_err(|e| CoreError::io(&file, e))?;
    let stats: Stats = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    let name = request.name.trim();
    let target = match &request.asset {
        Some(id) => Target::Asset(id),
        None => Target::Named {
            name,
            kind: AssetKind::Prop,
        },
    };
    let landed = versions::add(
        root,
        layout,
        &request.project,
        target,
        &work.join(OUTPUT_FILE),
    )?;
    save_recipe(
        &project_dir,
        &landed.asset.id,
        Recipe {
            name: name.to_string(),
            parts: request.parts.clone(),
        },
    )?;
    Ok(Assembled {
        project: landed.project,
        asset: landed.asset.id,
        version: landed.version,
        root: stats.root,
        nodes: stats.parts,
    })
}

fn read_file(project_dir: &Path) -> AssembliesFile {
    fs::read_to_string(project_dir.join(ASSEMBLIES_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<AssembliesFile>(&text).ok())
        .filter(|f| f.version <= ASSEMBLIES_VERSION)
        .unwrap_or_default()
}

/// The recipe a prop was last assembled with, if any.
pub fn recipe(project_dir: &Path, asset: &str) -> Option<Recipe> {
    read_file(project_dir).props.remove(asset)
}

fn save_recipe(project_dir: &Path, asset: &str, recipe: Recipe) -> CoreResult<()> {
    let mut file = read_file(project_dir);
    file.version = ASSEMBLIES_VERSION;
    file.props.insert(asset.to_string(), recipe);
    write_json(&project_dir.join(ASSEMBLIES_FILE), &file)
}

#[cfg(test)]
#[path = "assemble_tests.rs"]
mod tests;
