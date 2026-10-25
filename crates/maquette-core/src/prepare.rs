//! Model preparation: what happens to an asset's model before the scene is
//! built. A real size (a generator sends every prop at about a metre) and
//! fix-ups, one click repairs such as a decimate or putting the origin at
//! the bottom. They live in `prepare.json` next to `project.json`, keyed by
//! asset id like the layout and the material tweaks, so every new version
//! of the asset gets them again.
//!
//! ```json
//! {
//!   "version": 1,
//!   "models": {
//!     "desk-lamp": {
//!       "size": { "axis": "height", "value": 45, "unit": "cm" },
//!       "fixups": [
//!         { "op": "decimate", "triangles": 60000 },
//!         { "op": "origin-bottom" }
//!       ]
//!     }
//!   }
//! }
//! ```
//!
//! The Build scene starter applies them in Blender (`tools/fixups.py`):
//! the fix-ups in a fixed order ([`FixupKind`]), then a uniform scale that
//! gives the model its real size, then the layout's placement on top, so a
//! layout scale stays the person's own tweak. The source model in
//! `models/` is never written. Both files are inputs of that step, so a
//! change reruns it. The Inbox and the Inspector run the same script on its
//! own to show the numbers before and after ([`preview`]).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;

mod frame;
mod patch;
pub mod preview;
mod suggest;

pub use frame::{PrepFrame, extent, frame_of, measured_along, scale_for};
pub use patch::{PrepPatch, apply_patch, save_entry, update};
pub use suggest::{Suggestion, suggest};

pub const PREPARE_FILE: &str = "prepare.json";
pub const PREPARE_VERSION: u32 = 1;
/// The fix-up script, a starter in `tools/` that Build scene imports.
pub const SCRIPT_PATH: &str = "tools/fixups.py";
pub const SCRIPT: &str = include_str!("../starter/fixups.py");
/// The smallest and largest real size, in metres: a millimetre to a
/// kilometre.
pub const MIN_METRES: f32 = 0.001;
pub const MAX_METRES: f32 = 1000.0;
pub const MIN_TRIANGLES: u32 = 100;
pub const MAX_TRIANGLES: u32 = 10_000_000;
pub const MIN_TEXTURE: u32 = 64;
pub const MAX_TEXTURE: u32 = 16_384;
/// A loose part this share of the model's diagonal or smaller goes.
pub const DEFAULT_LOOSE_SHARE: f32 = 0.02;
pub const MAX_LOOSE_SHARE: f32 = 0.5;

/// Which side of the model the real size measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SizeAxis {
    /// Y, up.
    Height,
    /// X.
    Width,
    /// Z.
    Depth,
    /// Whichever is the largest.
    Longest,
}

impl SizeAxis {
    pub fn label(self) -> &'static str {
        match self {
            SizeAxis::Height => "high",
            SizeAxis::Width => "wide",
            SizeAxis::Depth => "deep",
            SizeAxis::Longest => "on its longest side",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SizeUnit {
    Cm,
    M,
}

impl SizeUnit {
    pub fn metres(self) -> f32 {
        match self {
            SizeUnit::Cm => 0.01,
            SizeUnit::M => 1.0,
        }
    }
}

/// How big the real object is along one side.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RealSize {
    pub axis: SizeAxis,
    /// In `unit`.
    pub value: f32,
    pub unit: SizeUnit,
}

impl RealSize {
    pub fn metres(&self) -> f32 {
        self.value * self.unit.metres()
    }

    /// "45 cm high", "1.2 m wide".
    pub fn describe(&self) -> String {
        let unit = match self.unit {
            SizeUnit::Cm => "cm",
            SizeUnit::M => "m",
        };
        format!("{} {unit} {}", trim_number(self.value), self.axis.label())
    }
}

/// 45, 0.5, 1.25: no trailing zeros.
fn trim_number(v: f32) -> String {
    let text = format!("{:.3}", v);
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The kinds of fix-up, in the order they are applied.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum FixupKind {
    ApplyTransforms,
    RemoveLoose,
    Merge,
    Decimate,
    ShrinkTextures,
    OriginBottom,
}

impl FixupKind {
    pub const ALL: [FixupKind; 6] = [
        FixupKind::ApplyTransforms,
        FixupKind::RemoveLoose,
        FixupKind::Merge,
        FixupKind::Decimate,
        FixupKind::ShrinkTextures,
        FixupKind::OriginBottom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FixupKind::ApplyTransforms => "Apply transforms",
            FixupKind::RemoveLoose => "Remove loose parts",
            FixupKind::Merge => "Merge objects",
            FixupKind::Decimate => "Decimate",
            FixupKind::ShrinkTextures => "Shrink textures",
            FixupKind::OriginBottom => "Origin to bottom centre",
        }
    }
}

/// One repair. Each kind appears at most once per asset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum Fixup {
    /// Bakes object transforms into the meshes (rigged meshes keep theirs).
    ApplyTransforms,
    /// Drops loose parts whose size is at most `minShare` of the model's
    /// diagonal, 0 to 0.5.
    RemoveLoose {
        #[serde(rename = "minShare")]
        min_share: f32,
    },
    /// Joins the meshes into one, or one per material.
    Merge {
        #[serde(rename = "byMaterial")]
        by_material: bool,
    },
    /// Collapses the model down to about this many triangles.
    Decimate { triangles: u32 },
    /// Scales every image down to this many pixels on its longest edge.
    ShrinkTextures {
        #[serde(rename = "maxSize")]
        max_size: u32,
    },
    /// Moves the model so its bottom centre is the origin, on the floor.
    OriginBottom,
}

impl Fixup {
    pub fn kind(&self) -> FixupKind {
        match self {
            Fixup::ApplyTransforms => FixupKind::ApplyTransforms,
            Fixup::RemoveLoose { .. } => FixupKind::RemoveLoose,
            Fixup::Merge { .. } => FixupKind::Merge,
            Fixup::Decimate { .. } => FixupKind::Decimate,
            Fixup::ShrinkTextures { .. } => FixupKind::ShrinkTextures,
            Fixup::OriginBottom => FixupKind::OriginBottom,
        }
    }

    /// "Decimate to 60k", "Shrink textures to 2048 px".
    pub fn describe(&self) -> String {
        match self {
            Fixup::Decimate { triangles } => {
                format!(
                    "Decimate to {}",
                    crate::inbox::checks::short_count(*triangles)
                )
            }
            Fixup::ShrinkTextures { max_size } => format!("Shrink textures to {max_size} px"),
            Fixup::RemoveLoose { min_share } => format!(
                "Remove loose parts under {}%",
                trim_number(min_share * 100.0)
            ),
            Fixup::Merge { by_material: true } => "Merge objects by material".to_string(),
            other => other.kind().label().to_string(),
        }
    }
}

/// One asset's preparation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelPrep {
    pub size: Option<RealSize>,
    /// In the order they apply.
    pub fixups: Vec<Fixup>,
}

impl ModelPrep {
    pub fn is_empty(&self) -> bool {
        self.size.is_none() && self.fixups.is_empty()
    }

    pub fn fixup(&self, kind: FixupKind) -> Option<&Fixup> {
        self.fixups.iter().find(|f| f.kind() == kind)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScenePrep {
    pub version: u32,
    /// Asset id to its preparation.
    pub models: BTreeMap<String, ModelPrep>,
}

impl Default for ScenePrep {
    fn default() -> Self {
        Self {
            version: PREPARE_VERSION,
            models: BTreeMap::new(),
        }
    }
}

pub fn path(project_dir: &Path) -> PathBuf {
    project_dir.join(PREPARE_FILE)
}

/// Reads `prepare.json`, or nothing prepared when the project has none.
///
/// # Errors
///
/// Fails when the file exists but cannot be read, comes from a newer
/// Maquette or holds values [`validate`] refuses.
pub fn load(project_dir: &Path) -> CoreResult<ScenePrep> {
    let file = path(project_dir);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(ScenePrep::default()),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    // The version first, so a newer file whose shape changed says so.
    let version = value.get("version").and_then(serde_json::Value::as_u64);
    if let Some(found) = version.filter(|v| *v > u64::from(PREPARE_VERSION)) {
        return Err(CoreError::UnsupportedVersion {
            found: u32::try_from(found).unwrap_or(u32::MAX),
            supported: PREPARE_VERSION,
        });
    }
    let found: ScenePrep = serde_json::from_value(value).map_err(|e| CoreError::json(&file, e))?;
    validate(&found)?;
    Ok(found)
}

/// Checks, tidies and writes the whole file atomically. Callers go through
/// [`update`], which reads the file on disk first.
fn save(project_dir: &Path, prep: &ScenePrep) -> CoreResult<ScenePrep> {
    validate(prep)?;
    let stored = normalized(prep);
    write_json(&path(project_dir), &stored)?;
    Ok(stored)
}

/// The fix-ups in their order, no empty entries and the current version.
pub fn normalized(prep: &ScenePrep) -> ScenePrep {
    let models = prep
        .models
        .iter()
        .filter(|(_, entry)| !entry.is_empty())
        .map(|(id, entry)| {
            let mut fixups = entry.fixups.clone();
            fixups.sort_by_key(Fixup::kind);
            (
                id.clone(),
                ModelPrep {
                    size: entry.size,
                    fixups,
                },
            )
        })
        .collect();
    ScenePrep {
        version: PREPARE_VERSION,
        models,
    }
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

/// Checks one real size: a number between a millimetre and a kilometre.
///
/// # Errors
///
/// Returns the problem as a sentence.
pub fn check_size(what: &str, size: &RealSize) -> CoreResult<()> {
    let metres = size.metres();
    if !size.value.is_finite() || !(MIN_METRES..=MAX_METRES).contains(&metres) {
        return Err(invalid(format!(
            "{what}: a real size must be between 1 mm and 1000 m, not {} {}.",
            size.value,
            match size.unit {
                SizeUnit::Cm => "cm",
                SizeUnit::M => "m",
            }
        )));
    }
    Ok(())
}

/// Checks one fix-up's numbers.
///
/// # Errors
///
/// Returns the problem as a sentence.
pub fn check_fixup(what: &str, fixup: &Fixup) -> CoreResult<()> {
    match *fixup {
        Fixup::RemoveLoose { min_share }
            if !(min_share.is_finite() && min_share > 0.0 && min_share <= MAX_LOOSE_SHARE) =>
        {
            Err(invalid(format!(
                "{what}: remove-loose takes a minShare above 0 and at most {MAX_LOOSE_SHARE}, not {min_share}."
            )))
        }
        Fixup::Decimate { triangles } if !(MIN_TRIANGLES..=MAX_TRIANGLES).contains(&triangles) => {
            Err(invalid(format!(
                "{what}: decimate takes {MIN_TRIANGLES} to {MAX_TRIANGLES} triangles, not {triangles}."
            )))
        }
        Fixup::ShrinkTextures { max_size } if !(MIN_TEXTURE..=MAX_TEXTURE).contains(&max_size) => {
            Err(invalid(format!(
                "{what}: shrink-textures takes a maxSize of {MIN_TEXTURE} to {MAX_TEXTURE} px, not {max_size}."
            )))
        }
        _ => Ok(()),
    }
}

/// Checks every entry: sizes in range, fix-up numbers in range and each
/// kind of fix-up at most once per asset.
///
/// # Errors
///
/// Returns the first problem as a sentence for the person or agent.
pub fn validate(prep: &ScenePrep) -> CoreResult<()> {
    for (asset, entry) in &prep.models {
        if let Some(size) = &entry.size {
            check_size(asset, size)?;
        }
        for (i, fixup) in entry.fixups.iter().enumerate() {
            check_fixup(asset, fixup)?;
            if entry.fixups[..i].iter().any(|f| f.kind() == fixup.kind()) {
                return Err(invalid(format!(
                    "{asset} lists {} twice. Keep one of each kind.",
                    fixup.kind().label().to_lowercase()
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
