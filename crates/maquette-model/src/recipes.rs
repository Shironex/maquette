//! Animation recipes: a base clip from the animation library with changes
//! laid over it, which a later step solves and bakes headless in Blender
//! into the model's next version. Each recipe is one plain JSON file,
//! `recipes/<id>.recipe.json` in the project folder, written whole and
//! atomically.
//!
//! ```json
//! {
//!   "version": 1,
//!   "name": "Reach for the lantern",
//!   "revision": 1,
//!   "created": "2026-11-05T10:00:00.000Z",
//!   "edited": "2026-11-05T10:12:00.000Z",
//!   "base": { "name": "Standing Idle", "frames": 120, "length": 4.0,
//!             "hash": "9f1c...", "mixamo": true },
//!   "blanks": [
//!     { "id": "lantern", "label": "The prop the hand reaches", "kind": "prop",
//!       "prop": null, "point": null }
//!   ],
//!   "layers": [
//!     { "id": "l-1", "kind": "reach", "bone": "rightHand",
//!       "settings": { "weight": 1.0, "start": 0.2, "end": 0.6, "target": "lantern",
//!                     "rotation": null, "offset": null, "cycles": null, "amount": null } }
//!   ]
//! }
//! ```
//!
//! A recipe never holds keys, frames, an FBX, a mesh, a skeleton or a
//! render: only the base clip's name, frame count, length and hash, the
//! blanks (targets filled per project, such as which prop a hand reaches)
//! and the changes, each on a VRM humanoid bone name (`rightHand`, `head`),
//! so the file can be shared. `mixamo` marks a base clip read from a Mixamo
//! rig: a bake of the recipe counts as Mixamo's (left out of export packs,
//! never saved to the clip library), while the recipe itself stays
//! shareable. `revision` starts at 1 and goes up only when a recipe is
//! saved back to a library of recipes, never on an edit in the project.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::plans::trimmed;

mod check;
mod files;

pub use check::{check_name, check_props, settle_props, validate};
pub use files::{
    RecipeChange, RecipeRead, RecipeSummary, create, get, list, remove, rename, update,
};

pub const RECIPES_DIR: &str = "recipes";
pub const RECIPE_SUFFIX: &str = ".recipe.json";
pub const RECIPE_VERSION: u32 = 1;
pub const NAME_MAX_CHARS: usize = 120;
/// A blank's label, saying what fills it.
pub const LABEL_MAX_CHARS: usize = 120;
pub const ID_MAX_CHARS: usize = 64;
/// The base clip's key hash, as the clip library writes it.
pub const HASH_MAX_CHARS: usize = 128;
pub const MAX_LAYERS: usize = 32;
pub const MAX_BLANKS: usize = 16;
/// The longest base clip a recipe takes, in frames.
pub const MAX_FRAMES: u32 = 100_000;
/// How far a nudge may move a bone, in metres along each axis.
pub const MAX_OFFSET_METRES: f32 = 2.0;
/// How far from the origin a point blank may sit, in metres.
pub const MAX_POINT_METRES: f32 = 1_000.0;
/// How far a wave swings, in degrees.
pub const MAX_WAVE_DEGREES: f32 = 90.0;
/// How many waves fit in the part of the clip a wave layer covers.
pub const MAX_CYCLES: f32 = 50.0;
/// The biggest recipe file read. A recipe at every limit is far under it.
pub const MAX_FILE_BYTES: u64 = 256 * 1024;

/// What a change does to the base clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum LayerKind {
    /// A hand or foot reaches a target (a prop or a point) with IK.
    Reach,
    /// A bone's pose is turned or moved by the same amount across the clip.
    Nudge,
    /// The head or neck turns to follow a target.
    LookAt,
    /// An arm waves, laid over the clip.
    Wave,
}

/// What fills a blank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum BlankKind {
    /// An asset of the project, by its id.
    Prop,
    /// A point in the scene, in metres with Y up.
    Point,
}

/// The library clip a recipe starts from, by what matches it on another
/// computer: its name and frame count. The hash tells keys that changed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBase {
    pub name: String,
    pub frames: u32,
    /// Seconds.
    pub length: f32,
    pub hash: String,
    /// Read from a Mixamo rig: a bake counts as Mixamo's.
    pub mixamo: bool,
}

/// A target the person or an agent fills per project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBlank {
    pub id: String,
    pub label: String,
    pub kind: BlankKind,
    /// A prop blank's asset id, once filled.
    pub prop: Option<String>,
    /// A point blank's point, once filled.
    pub point: Option<[f32; 3]>,
}

impl RecipeBlank {
    /// Whether something fills it.
    pub fn filled(&self) -> bool {
        match self.kind {
            BlankKind::Prop => self.prop.is_some(),
            BlankKind::Point => self.point.is_some(),
        }
    }
}

/// A change's settings. Each kind reads its own fields; the rest are none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LayerSettings {
    /// How much of the change shows, 0 to 1.
    pub weight: f32,
    /// Where in the clip it starts and ends, as shares of its length, 0 to 1.
    pub start: f32,
    pub end: f32,
    /// Reach and look at: the blank that holds the target.
    pub target: Option<String>,
    /// Nudge: degrees added to the bone's rotation, X then Y then Z.
    pub rotation: Option<[f32; 3]>,
    /// Nudge: metres added to the bone's position, Y up.
    pub offset: Option<[f32; 3]>,
    /// Wave: how many waves between start and end.
    pub cycles: Option<f32>,
    /// Wave: how far each wave swings, in degrees.
    pub amount: Option<f32>,
}

impl Default for LayerSettings {
    fn default() -> Self {
        Self {
            weight: 1.0,
            start: 0.0,
            end: 1.0,
            target: None,
            rotation: None,
            offset: None,
            cycles: None,
            amount: None,
        }
    }
}

/// One change laid over the base clip, on one humanoid bone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeLayer {
    pub id: String,
    pub kind: LayerKind,
    /// A VRM humanoid bone name: `rightHand`, `head`, `leftUpperArm`.
    pub bone: String,
    pub settings: LayerSettings,
}

/// `recipes/<id>.recipe.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeFile {
    pub version: u32,
    pub name: String,
    pub revision: u32,
    /// RFC 3339, when the recipe was made.
    pub created: String,
    /// RFC 3339, the last write that changed something.
    pub edited: String,
    pub base: RecipeBase,
    pub blanks: Vec<RecipeBlank>,
    /// In the order they are laid over the clip.
    pub layers: Vec<RecipeLayer>,
}

impl RecipeFile {
    /// A recipe called `name` on `base` with no changes yet, made now.
    pub fn new(name: &str, base: RecipeBase) -> Self {
        let now = crate::project::now_rfc3339();
        Self {
            version: RECIPE_VERSION,
            name: trimmed(name).to_string(),
            revision: 1,
            created: now.clone(),
            edited: now,
            base,
            blanks: Vec::new(),
            layers: Vec::new(),
        }
    }
}

/// `<project>/recipes`.
pub fn dir(project_dir: &Path) -> PathBuf {
    project_dir.join(RECIPES_DIR)
}

/// True for an id a recipe file may have, the same shape as a canvas id.
pub fn is_recipe_id(id: &str) -> bool {
    id.len() <= ID_MAX_CHARS && crate::plans::is_plan_id(id)
}

/// The file of the recipe `id`, which must be a recipe id.
///
/// # Errors
///
/// [`CoreError::NotFound`] for anything that is not a recipe id.
pub fn path(project_dir: &Path, id: &str) -> CoreResult<PathBuf> {
    if !is_recipe_id(id) {
        return Err(CoreError::NotFound(format!("the recipe {id}")));
    }
    Ok(dir(project_dir).join(format!("{id}{RECIPE_SUFFIX}")))
}

/// Reads and checks a recipe file, normalised ([`normalized`]) so what a
/// view gets is what a write puts back.
///
/// # Errors
///
/// Fails when the file is missing, unreadable or over [`MAX_FILE_BYTES`],
/// comes from a newer Maquette or holds values [`validate`] refuses.
pub fn load_file(file: &Path) -> CoreResult<RecipeFile> {
    let text = match crate::plans::read_capped(file, MAX_FILE_BYTES) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!(
                "the recipe file {}",
                file.display()
            )));
        }
        Err(err) if err.kind() == io::ErrorKind::FileTooLarge => {
            return Err(CoreError::Invalid(format!(
                "The recipe file is over {} KB, too big to be a recipe.",
                MAX_FILE_BYTES / 1024
            )));
        }
        Err(err) => return Err(CoreError::io(file, err)),
    };
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| CoreError::json(file, e))?;
    // The version first, so a newer file whose shape changed says so.
    let found = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if found > u64::from(RECIPE_VERSION) {
        return Err(CoreError::UnsupportedVersion {
            found: u32::try_from(found).unwrap_or(u32::MAX),
            supported: RECIPE_VERSION,
        });
    }
    let recipe: RecipeFile = serde_json::from_value(value).map_err(|e| CoreError::json(file, e))?;
    let recipe = normalized(&recipe);
    validate(&recipe)?;
    Ok(recipe)
}

/// The recipe as it is written: this version, a trimmed name, base name
/// and labels, and each change holding only the settings its kind reads.
pub fn normalized(recipe: &RecipeFile) -> RecipeFile {
    let blanks = recipe
        .blanks
        .iter()
        .map(|b| RecipeBlank {
            id: trimmed(&b.id).to_string(),
            label: trimmed(&b.label).to_string(),
            prop: if b.kind == BlankKind::Prop {
                b.prop.as_deref().map(|p| trimmed(p).to_string())
            } else {
                None
            },
            point: if b.kind == BlankKind::Point {
                b.point
            } else {
                None
            },
            kind: b.kind,
        })
        .collect();
    let layers = recipe
        .layers
        .iter()
        .map(|l| RecipeLayer {
            id: trimmed(&l.id).to_string(),
            bone: trimmed(&l.bone).to_string(),
            settings: settings_for(l.kind, &l.settings),
            kind: l.kind,
        })
        .collect();
    RecipeFile {
        version: RECIPE_VERSION,
        name: trimmed(&recipe.name).to_string(),
        base: RecipeBase {
            name: trimmed(&recipe.base.name).to_string(),
            ..recipe.base.clone()
        },
        blanks,
        layers,
        ..recipe.clone()
    }
}

/// `settings` with only the fields `kind` reads.
fn settings_for(kind: LayerKind, settings: &LayerSettings) -> LayerSettings {
    let targeted = matches!(kind, LayerKind::Reach | LayerKind::LookAt);
    let nudge = kind == LayerKind::Nudge;
    let wave = kind == LayerKind::Wave;
    LayerSettings {
        weight: settings.weight,
        start: settings.start,
        end: settings.end,
        target: settings
            .target
            .as_deref()
            .map(|t| trimmed(t).to_string())
            .filter(|_| targeted),
        rotation: settings.rotation.filter(|_| nudge),
        offset: settings.offset.filter(|_| nudge),
        cycles: settings.cycles.filter(|_| wave),
        amount: settings.amount.filter(|_| wave),
    }
}

pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<RecipeFile>()
        .register::<RecipeSummary>()
        .register::<RecipeChange>()
}

#[cfg(test)]
mod tests;
