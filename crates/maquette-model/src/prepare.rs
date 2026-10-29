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
//!       ],
//!       "skipBake": true,
//!       "padEdges": true,
//!       "parts": { "names": { "part_1": "lid" }, "merge": [], "hidden": [] }
//!     }
//!   }
//! }
//! ```
//!
//! `skipBake` true leaves the asset out of the Bake light step: Build scene
//! tags its objects and the bake gives them no lightmap and no second UV
//! set, so the model ships with its own material lighting. A generated scan
//! wants that (its many tiny UV islands bake into flat lit facets), so a
//! model accepted from the Inbox skips the bake unless the person chose
//! otherwise ([`SKIP_BAKE_FROM_INBOX`]). Left out or null, the asset bakes.
//!
//! `padEdges` true pads the asset's textures before they are shrunk: every
//! texel no UV island covers takes the colour of the islands around it, so
//! the light gaps of a scan's atlas cannot bleed over its seams as thin
//! white lines once the texture is smaller and the browser mipmaps it. It
//! runs before a decimate, on the model's own UVs. A model accepted from
//! the Inbox pads unless the person chose otherwise
//! ([`PAD_EDGES_FROM_INBOX`]). Left out or null, the textures stay as they are.
//!
//! `parts` names the model's parts for the export (see [`PartsPlan`]): a
//! split or a provider gives them plain names, and object states and
//! explode tracks pick them by the names set here.
//!
//! The Build scene starter applies them in Blender (`tools/fixups.py`):
//! the fix-ups in a fixed order ([`FixupKind`]), then a uniform scale that
//! gives the model its real size, then the layout's placement on top, so a
//! layout scale stays the person's own tweak. The source model in
//! `models/` is never written. Both files are inputs of that step, so a
//! change reruns it. The Inbox and the Inspector run the same script on its
//! own to show the numbers before and after (`prepare::preview` in core).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;

mod fixup;
mod frame;
mod parts;
mod patch;
mod suggest;

pub use fixup::{Fixup, FixupKind, SplitBy};
pub use frame::{PrepFrame, extent, frame_of, measured_along, scale_for};
pub use parts::{MAX_PART_NAME, PartsPlan, check_parts};
pub use patch::{PartsPatch, PrepPatch, apply_patch, save_entry, update};
pub use suggest::{Suggestion, suggest};

pub const PREPARE_FILE: &str = "prepare.json";
pub const PREPARE_VERSION: u32 = 1;
/// The smallest and largest real size, in metres: a millimetre to a
/// kilometre.
pub const MIN_METRES: f32 = 0.001;
pub const MAX_METRES: f32 = 1000.0;
pub const MIN_TRIANGLES: u32 = 100;
pub const MAX_TRIANGLES: u32 = 10_000_000;
pub const MIN_TEXTURE: u32 = 64;
pub const MAX_TEXTURE: u32 = 16_384;
/// What an asset accepted from the Inbox gets when nobody chose whether it
/// bakes: Inbox models are provider scans, which the bake does not suit.
pub const SKIP_BAKE_FROM_INBOX: bool = true;
/// What an asset accepted from the Inbox gets when nobody chose whether its
/// texture edges are padded: a scan's atlas has gaps that bleed when shrunk.
pub const PAD_EDGES_FROM_INBOX: bool = true;
/// A loose part this share of the model's diagonal or smaller goes.
pub const DEFAULT_LOOSE_SHARE: f32 = 0.02;
pub const MAX_LOOSE_SHARE: f32 = 0.5;
/// A split piece this share of the model's diagonal or smaller joins its
/// nearest bigger neighbour instead of becoming a part of its own.
pub const DEFAULT_MERGE_UNDER: f32 = 0.01;
pub const MAX_MERGE_UNDER: f32 = 0.2;
/// A rotate fix-up turns at most this far about each axis, in degrees.
pub const MAX_TURN_DEGREES: f32 = 360.0;

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
pub fn trim_number(v: f32) -> String {
    let text = format!("{:.3}", v);
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// One asset's preparation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelPrep {
    pub size: Option<RealSize>,
    /// In the order they apply.
    pub fixups: Vec<Fixup>,
    /// True leaves the asset out of the bake; false or null bakes it. Null
    /// means nobody chose, so an Inbox accept may fill in its default.
    #[serde(default)]
    pub skip_bake: Option<bool>,
    /// True pads the UV islands of its textures into the gaps around them
    /// before they shrink; false or null leaves them. Null means nobody
    /// chose, so an Inbox accept may fill in its default.
    #[serde(default)]
    pub pad_edges: Option<bool>,
    /// The names its parts ship as, parts joined and parts drawn as a
    /// ghost in the Inspector. Null names none.
    #[serde(default)]
    pub parts: Option<PartsPlan>,
}

impl ModelPrep {
    pub fn is_empty(&self) -> bool {
        self.size.is_none()
            && self.fixups.is_empty()
            && self.skip_bake.is_none()
            && self.pad_edges.is_none()
            && self.parts.as_ref().is_none_or(PartsPlan::is_empty)
    }

    /// Whether Build scene pads its texture edges.
    pub fn pads_edges(&self) -> bool {
        self.pad_edges == Some(true)
    }

    /// Whether Bake light leaves this asset out.
    pub fn skips_bake(&self) -> bool {
        self.skip_bake == Some(true)
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
                    skip_bake: entry.skip_bake,
                    pad_edges: entry.pad_edges,
                    parts: entry
                        .parts
                        .as_ref()
                        .filter(|p| !p.is_empty())
                        .map(PartsPlan::normalized),
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
        Fixup::SplitParts { merge_under, .. }
            if !(merge_under.is_finite() && (0.0..=MAX_MERGE_UNDER).contains(&merge_under)) =>
        {
            Err(invalid(format!(
                "{what}: split-parts takes a mergeUnder of 0 to {MAX_MERGE_UNDER}, not {merge_under}."
            )))
        }
        Fixup::Rotate { x, y, z }
            if ![x, y, z]
                .iter()
                .all(|d| d.is_finite() && d.abs() <= MAX_TURN_DEGREES) =>
        {
            Err(invalid(format!(
                "{what}: rotate takes x, y and z in degrees from -{MAX_TURN_DEGREES} to {MAX_TURN_DEGREES}, not {x}, {y}, {z}."
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
        if let Some(parts) = &entry.parts {
            check_parts(asset, parts)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

/// Registers this module's IPC types (the scene prep, its suggestions and the fix-up report) with
/// the shell's TypeScript export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<ScenePrep>()
        .register::<Suggestion>()
        .register::<PrepFrame>()
}
