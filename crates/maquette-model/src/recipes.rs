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
//! saved back to the workspace's recipe library, never on an edit in the
//! project. `from` names the library recipe and revision a project's recipe
//! was copied from or last saved as; it is left out of every exported file.
//!
//! Version 2 adds what a recipe uses to make new motion, each written only
//! when used, so a recipe that uses none of it is saved as version 1 and a
//! Maquette from before still opens it: more clips `joined` after the base,
//! each across a blend window, a `trim` on any clip, `mirror`, `time` edits
//! (retime ranges, freezes and holds over) on a clip's own frames, and the
//! pin, pose key and grip changes ([`LayerKind`]). Keys and time edits sit on
//! the source clip's frames (a clip index and a frame), so a trim, a retime
//! or a new blend carries them with the motion they belong to. A trim keeps
//! what it hides: a key or edit outside the kept frames stays in the file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::plans::trimmed;

pub mod baked;
mod change;
mod check;
mod check_edits;
mod clip_moves;
mod copies;
mod edits;
mod files;
mod keys;
mod normal;
pub mod pins;
pub mod timeline;

pub use change::{RecipeChange, update};
pub use check::{check_name, check_props, gone_props, settle_props, validate};
pub use clip_moves::CLIP_IN_USE;
pub use copies::{create_whole, link};
pub use edits::{BlendCurve, BlendWindow, LineUp, TimeEdit, TimeKind, bake_unsupported, uses_v2};
pub use files::{
    RecipeRead, RecipeSummary, TOO_BIG, create, create_joined, get, list, list_with,
    motion_version_of, names, remove, rename, rename_with, version,
};
pub use keys::{BonePose, GripFrom, KeyEase, LayerMode, PinFlat, PoseKey};
pub use normal::{load_versioned, normalized};

pub const RECIPES_DIR: &str = "recipes";
pub const RECIPE_SUFFIX: &str = ".recipe.json";
/// The newest file version this Maquette reads and writes.
pub const RECIPE_VERSION: u32 = 2;
/// The version a recipe that uses nothing of version 2 is written as, so
/// a Maquette from before still opens it.
pub const RECIPE_VERSION_1: u32 = 1;
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
/// The longest a whole recipe runs once its clips are joined and its time
/// edits played, in frames: no longer than the longest clip it takes, so a
/// file of eight long clips, slow retimes and long freezes cannot make a
/// timeline too big to preview or bake.
pub const MAX_TIMELINE_FRAMES: u32 = MAX_FRAMES;
/// How far a nudge may move a bone, in metres along each axis.
pub const MAX_OFFSET_METRES: f32 = 2.0;
/// How far from the origin a point blank may sit, in metres.
pub const MAX_POINT_METRES: f32 = 1_000.0;
/// How far a wave swings, in degrees.
pub const MAX_WAVE_DEGREES: f32 = 90.0;
/// How many waves fit in the part of the clip a wave layer covers.
pub const MAX_CYCLES: f32 = 50.0;
/// What a recipe on the rest pose calls its base.
pub const REST_POSE_NAME: &str = "Rest pose";
/// The rest pose's frame rate, and how many frames it holds unless told.
pub const REST_FPS: f32 = 30.0;
pub const REST_FRAMES: u32 = 60;
/// The longest a rest pose recipe runs, in frames (20 seconds).
pub const MAX_REST_FRAMES: u32 = 600;
/// The biggest recipe file read or written. A recipe at every limit fits
/// under it (the recipe-wide [`MAX_RECIPE_KEYS`] and [`MAX_RECIPE_TURNS`]
/// keep pose keys and grips from filling it), and a write that would still
/// go over is refused, since such a file would not open again.
pub const MAX_FILE_BYTES: u64 = 256 * 1024;
/// The most clips a recipe joins, the base included.
pub const MAX_CLIPS: usize = 8;
/// The most keys a pose key change holds.
pub const MAX_KEYS: usize = 64;
/// The most bones one pose key turns.
pub const MAX_KEY_BONES: usize = 32;
/// The most pose keys a recipe holds across all its pose key changes.
pub const MAX_RECIPE_KEYS: usize = 256;
/// The most bone turns a recipe holds across all its pose keys and grips.
pub const MAX_RECIPE_TURNS: usize = 1024;
/// The most bones a grip's pose turns: both hands, every finger, and the
/// forearms.
pub const MAX_GRIP_BONES: usize = 64;
/// The most time edits a recipe holds.
pub const MAX_TIME_EDITS: usize = 32;
/// How far a retime slows down or speeds up a range.
pub const MIN_SPEED: f32 = 0.25;
pub const MAX_SPEED: f32 = 4.0;
/// The longest a freeze or a hold over lasts, in frames (20 seconds at 30 fps).
pub const MAX_HOLD_FRAMES: u32 = 600;
/// How far a pose key or a grip turns a bone, in degrees each way.
pub const MAX_TURN_DEGREES: f32 = 180.0;
/// How long a blend window lasts unless told, in frames.
pub const BLEND_FRAMES: u32 = 8;

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
    /// A hand or foot stays put across part of the clip. Version 2.
    Pin,
    /// Sparse pose keys on the bones and frames a person set. Version 2.
    Pose,
    /// A hand pose from the grip library, a copy plus a link back. Version 2.
    Grip,
}

impl LayerKind {
    /// A change of version 2, which a file of version 1 cannot hold.
    pub fn is_v2(self) -> bool {
        matches!(self, Self::Pin | Self::Pose | Self::Grip)
    }
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
///
/// Two other shapes keep the same fields, so a version 1 reader that does
/// not know them still reads the file (as a base clip it cannot find):
///
/// - `rest`: the recipe runs on the model's rest pose for `frames` frames
///   at [`REST_FPS`], with no base clip. Its name is [`REST_POSE_NAME`],
///   its hash empty and it is never Mixamo's.
/// - `byName` (with an empty `hash`, not rest): a base known by its name
///   only, as a starter recipe names it when the clip library had no such
///   clip. It matches the first library clip of that name, whatever its
///   frames. Any other base, an empty hash included, matches by name and
///   frame count.
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
    /// The model's rest pose, no base clip. A file from before reads false.
    #[serde(default)]
    #[specta(optional)]
    pub rest: bool,
    /// Known by its clip's name only, as a starter names it. A file from
    /// before reads false.
    #[serde(default)]
    #[specta(optional)]
    pub by_name: bool,
    /// The frames of the clip kept, in then out (`[6, 58]` keeps 52), cut
    /// before the clips are joined. None keeps it whole. Version 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub trim: Option<[u32; 2]>,
    /// A joined clip's blend from the clip before it. None on the base.
    /// Version 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub blend: Option<BlendWindow>,
}

impl RecipeBase {
    /// The rest pose held for `frames` frames at [`REST_FPS`].
    pub fn rest_pose(frames: u32) -> Self {
        let frames = frames.clamp(2, MAX_REST_FRAMES);
        Self {
            name: REST_POSE_NAME.to_string(),
            frames,
            length: rest_length(frames),
            hash: String::new(),
            mixamo: false,
            rest: true,
            by_name: false,
            trim: None,
            blend: None,
        }
    }

    /// A base known by its clip's name only (an empty hash), until the
    /// clip library has a clip of that name.
    pub fn named(name: &str) -> Self {
        Self {
            name: trimmed(name).to_string(),
            frames: 1,
            length: 1.0 / REST_FPS,
            hash: String::new(),
            mixamo: false,
            rest: false,
            by_name: true,
            trim: None,
            blend: None,
        }
    }

    /// Known by name only: marked so (as a starter's base is), not the rest
    /// pose, and no hash.
    pub fn name_only(&self) -> bool {
        self.by_name && !self.rest && self.hash.is_empty()
    }

    /// How many of its frames the clip keeps after its trim.
    pub fn kept(&self) -> u32 {
        self.trim
            .map_or(self.frames, |[from, to]| to.saturating_sub(from))
    }
}

/// How long the rest pose lasts over `frames` frames, seconds.
pub fn rest_length(frames: u32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let frames = frames as f32;
    frames / REST_FPS
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
    /// Pin and grip: frames it eases in over. Version 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub ease_in: Option<u32>,
    /// Pin and grip: frames it eases out over. Version 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub ease_out: Option<u32>,
    /// A foot pin: what of the foot stays flat. Version 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub flat: Option<PinFlat>,
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
            ease_in: None,
            ease_out: None,
            flat: None,
        }
    }
}

/// One change laid over the base clip, on one humanoid bone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeLayer {
    pub id: String,
    pub kind: LayerKind,
    /// A VRM humanoid bone name: `rightHand`, `head`, `leftUpperArm`. A
    /// grip names the hand that holds; pose keys name their bones in each
    /// key, so theirs is empty.
    pub bone: String,
    pub settings: LayerSettings,
    /// Pose keys and grips: laid over the clip (add) or in its place
    /// (replace). Pose keys add unless told, grips replace. Version 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub mode: Option<LayerMode>,
    /// Pose keys: only the bones and frames a person set, in clip and frame
    /// order. Version 2.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[specta(optional)]
    pub keys: Vec<PoseKey>,
    /// A grip: the recipe's own copy of the grip's pose, degrees per bone,
    /// no frames. Version 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub pose: Option<BonePose>,
    /// A grip: the library grip and revision it was copied from. Like the
    /// recipe's own `from`, it stays out of the motion version. Version 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub from: Option<GripFrom>,
}

impl RecipeLayer {
    /// A change of `kind` on `bone` with `settings` and nothing of version 2.
    pub fn new(id: &str, kind: LayerKind, bone: &str, settings: LayerSettings) -> Self {
        Self {
            id: id.to_string(),
            kind,
            bone: bone.to_string(),
            settings,
            mode: None,
            keys: Vec::new(),
            pose: None,
            from: None,
        }
    }
}

/// A prop blank whose asset the project no longer has: it reads as open,
/// and the file keeps the prop until the blank is cleared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GoneProp {
    /// The blank's id.
    pub blank: String,
    /// The asset id it still names.
    pub prop: String,
}

/// The library recipe and revision a project's recipe came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeFrom {
    /// The library recipe's id.
    pub entry: String,
    /// The library recipe's uid, made once when it was made. An entry of
    /// the same id with another uid is another recipe (the first was
    /// removed and its id taken again), so the link reads as gone.
    pub uid: String,
    /// Its revision, from 1.
    pub revision: u32,
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
    /// The first clip; in version 1 the only one.
    pub base: RecipeBase,
    /// The clips joined after the base, in order, each across its blend
    /// window. Clip 1 is the first of these, clip 0 the base. Version 2.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[specta(optional)]
    pub joined: Vec<RecipeBase>,
    /// Left and right swapped across the whole recipe, right after the
    /// clips are joined. Version 2.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[specta(optional)]
    pub mirror: bool,
    /// Retime ranges, freezes and holds over, each on a clip's own frames.
    /// Version 2.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[specta(optional)]
    pub time: Vec<TimeEdit>,
    pub blanks: Vec<RecipeBlank>,
    /// In the order they are laid over the clip.
    pub layers: Vec<RecipeLayer>,
    /// The library recipe and revision it was copied from or last saved
    /// as; none for a recipe made here or imported. Never exported.
    #[serde(default)]
    pub from: Option<RecipeFrom>,
}

impl RecipeFile {
    /// A recipe called `name` on `base` with no changes yet, made now.
    pub fn new(name: &str, base: RecipeBase) -> Self {
        let now = crate::project::now_rfc3339();
        Self {
            version: RECIPE_VERSION_1,
            name: trimmed(name).to_string(),
            revision: 1,
            created: now.clone(),
            edited: now,
            base,
            joined: Vec::new(),
            mirror: false,
            time: Vec::new(),
            blanks: Vec::new(),
            layers: Vec::new(),
            from: None,
        }
    }

    /// Every clip in order: the base, then the joined ones.
    pub fn clips(&self) -> impl Iterator<Item = &RecipeBase> {
        std::iter::once(&self.base).chain(&self.joined)
    }

    /// Clip `index`: 0 the base, then the joined ones.
    pub fn clip(&self, index: u32) -> Option<&RecipeBase> {
        match index {
            0 => Some(&self.base),
            n => self.joined.get(usize::try_from(n - 1).ok()?),
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
    Ok(load_versioned(file)?.0)
}

/// True for a library recipe uid: 8 to 64 lowercase hex digits.
pub fn is_entry_uid(uid: &str) -> bool {
    (8..=64).contains(&uid.len()) && uid.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A recipe file's version: the first 16 hex digits of the SHA-256 of its
/// bytes, so any change on disk (a write here, a hand edit, a sync from
/// another computer) gives a new one, whatever `edited` says.
pub fn version_of(text: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(text.as_bytes()))[..16].to_string()
}

/// A recipe's motion version: the [`version_of`] its file would have with
/// the bookkeeping left out (`from`, `revision`, `edited`, `created`, and a
/// grip's library link), so saving it to the recipe library, which writes
/// the link into the file, or linking a grip again, does not read as a
/// change to what a bake was made from. Everything of version 2 that moves
/// the model counts: clips, trims, blends, mirror, time edits, pins, keys
/// and grip poses. A recipe that uses nothing of version 2 serializes as it
/// did before, so its motion version stays the same.
pub fn motion_version(recipe: &RecipeFile) -> String {
    let mut bare = recipe.clone();
    bare.from = None;
    for layer in &mut bare.layers {
        layer.from = None;
    }
    bare.revision = 0;
    bare.edited = String::new();
    bare.created = String::new();
    version_of(&serde_json::to_string(&bare).unwrap_or_default())
}

/// `name`, or `name (2)`, `name (3)` and on when another of `taken` has it
/// (ignoring case and edge spaces), cut to fit [`NAME_MAX_CHARS`]. Ids are
/// not touched: only the name a person reads.
pub fn unique_name<S: AsRef<str>>(name: &str, taken: &[S]) -> String {
    let name = trimmed(name);
    let used = |candidate: &str| {
        taken
            .iter()
            .any(|t| trimmed(t.as_ref()).to_lowercase() == candidate.to_lowercase())
    };
    if !used(name) {
        return name.to_string();
    }
    for n in 2..10_000u32 {
        let suffix = format!(" ({n})");
        let room = NAME_MAX_CHARS.saturating_sub(suffix.chars().count());
        let short: String = name.chars().take(room).collect();
        let candidate = format!("{}{suffix}", short.trim_end());
        if !used(&candidate) {
            return candidate;
        }
    }
    name.to_string()
}

pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<RecipeFile>()
        .register::<RecipeSummary>()
        .register::<RecipeChange>()
        .register::<RecipeFrom>()
        .register::<TimeEdit>()
        .register::<PoseKey>()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_v2;

#[cfg(test)]
mod tests_v2_clips;

#[cfg(test)]
mod tests_v2_limits;

#[cfg(test)]
mod tests_v2_motion;

#[cfg(test)]
mod tests_v2_size;

#[cfg(test)]
mod tests_timeline;

#[cfg(test)]
mod tests_clip_shares;

#[cfg(test)]
mod tests_pins;
