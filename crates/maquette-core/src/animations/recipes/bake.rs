//! Baking a recipe: its base clip goes onto a rigged model through the
//! retarget, and its changes are solved over it exactly, in one headless
//! Blender run (`bake_recipe.py`), as one new clip named after the recipe.
//!
//! The result is staged under the workspace cache, never added to the
//! project: adding it as the model's next version after a review is a later
//! step. It records the recipe and the version (a hash of its file) it was
//! baked from, the base clip and its hash, whether it counts as Mixamo's,
//! the timings and what the review reads: the planted feet's slide, the rig's
//! stretch past a healthy skin, each reach's gap to its target at its end and
//! an elbow or knee that bends the other way than the clip has it.
//!
//! A bake of a Mixamo base counts as Mixamo's. The staged file lives in
//! the cache, which neither the clip library nor an export pack reads, and
//! adding a model file refuses it there. It also carries a mark in its own
//! GLB (`asset.extras`), which packs and Save clips to the library honour
//! while the file keeps it; accepting a marked file records the bake on the
//! asset, which outlives the mark (see `crate::recipes::baked`).

use serde::{Deserialize, Serialize};

mod add;
mod job;
mod result;
mod review;
mod service;
mod stale;

pub use crate::recipes::baked::{
    MARK_KEY, RecipeBakeMark, RecipeBakeUse, mark_of, marked_mixamo, took_mixamo, write_mark,
};
pub use add::{AddClaim, BAKE_CHANGED, OUT_OF_DATE, ready_to_add};
pub use job::{Prepared, open_blanks, prepare};
pub use result::GAP_LIMIT_CM;
pub use result::mark_for;
pub use review::{
    RecipeBakeReview, RecipeReviewLine, RecipeReviewStatus, SLIDE_OVER_BASE_CM,
    STRETCH_OVER_BASE_MM, review_of,
};
pub use service::{RecipeBakes, record_of};
pub use stale::{RECIPE_CHANGED, stale_of};

/// The bake script.
pub const BAKE_SCRIPT: &str = include_str!("../bake_recipe.py");
/// The workspace cache folder recipe bakes are staged in, per project and
/// recipe: `<cache>/recipes/<project>/<recipe>/`.
pub const WORK_DIR: &str = crate::workspace::RECIPE_BAKES_DIR;
/// The staged model in a bake's folder.
pub const STAGED_FILE: &str = "staged.glb";

/// Bake recipe `recipe` of `project` onto the rigged model `asset`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBakeRequest {
    pub project: String,
    pub recipe: String,
    pub asset: String,
    /// The recipe's `version` as it was read. Given, a recipe that changed
    /// since is refused (code `moved`), so the bake is the recipe on show.
    #[serde(default)]
    pub expected: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RecipeBakeState {
    Running,
    /// The staged result is there, waiting for its review.
    Done,
    Failed,
    Cancelled,
    /// A person added it as the model's next version; the staged model is
    /// gone.
    Added,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RecipeBakeStarter {
    Person,
    Agent,
}

/// How far a running bake is: "Solving frame 12 of 48".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBakeProgress {
    pub label: String,
    pub done: u32,
    pub total: u32,
}

/// One reach as the bake measured it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeReachResult {
    /// The change's id.
    pub layer: String,
    /// The hand or foot, by VRM name.
    pub bone: String,
    /// How far it ends from its target at the reach's end, cm; none when
    /// that frame was not measured.
    pub gap_cm: Option<f32>,
    /// The frame that was measured on.
    pub frame: u32,
    /// Within [`GAP_LIMIT_CM`].
    pub reached: bool,
    /// The first frame of the reach its elbow or knee bends the other way
    /// than the clip has it there; none when it never does.
    pub flip_frame: Option<u32>,
}

/// A finished bake: what it was baked from and what the review reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBakeResult {
    pub recipe: String,
    pub recipe_name: String,
    /// The recipe file's version (a hash of its bytes) it was baked from.
    pub recipe_version: String,
    /// The recipe's motion version it was baked from: the file's hash
    /// without its library link and other bookkeeping, which is what
    /// "out of date" reads. Empty in a record from before, which reads by
    /// `recipe_version` instead.
    #[serde(default)]
    pub motion_version: String,
    /// The library clip it started from, and the hash of its keys; both
    /// empty for a bake on the rest pose.
    pub base_clip: String,
    pub base_hash: String,
    /// The hash the recipe holds for its base, when the library's differs.
    pub recipe_base_hash: Option<String>,
    /// Counts as Mixamo's: left out of export packs, never saved to the
    /// clip library.
    pub mixamo: bool,
    pub asset: String,
    pub asset_name: String,
    /// The version it was baked onto.
    pub from_version: u32,
    /// The new clip's name in the model.
    pub clip: String,
    /// The model's clips earlier bakes of this recipe added, whatever they
    /// were named, which this one replaces. A same named clip from anywhere
    /// else stays, and the new clip takes `name_2` and on. (Records from
    /// before held one name as `replaces`, which is no longer read.)
    #[serde(default)]
    pub replaced: Vec<String>,
    /// What a name clash with the model's clips came to, in a sentence.
    #[serde(default)]
    pub clip_note: Option<String>,
    pub frames: u32,
    /// The staged model, absolute.
    pub file: String,
    /// The way the model faces as read from its rig, glTF axes (Y up).
    pub facing: [f32; 3],
    /// The worst slide of a planted foot, cm, the frames of that plant and
    /// the foot; and the base clip's own slide on this model.
    pub slide_cm: f32,
    pub slide_frames: Option<[u32; 2]>,
    pub slide_foot: Option<String>,
    pub base_slide_cm: f32,
    /// The worst stretch past a healthy skin over the measured frames, mm at
    /// a model one metre tall, with its frame; and the base clip's own.
    /// None when it could not be measured (`stretch_note` says why).
    pub stretch_mm: Option<f32>,
    pub stretch_frame: Option<u32>,
    pub base_stretch_mm: Option<f32>,
    pub stretch_note: Option<String>,
    pub reaches: Vec<RecipeReachResult>,
    /// How long Blender took, and the whole bake, seconds.
    pub blender_seconds: f32,
    pub seconds: f32,
    /// What to know about this bake (a base clip with other keys).
    pub warnings: Vec<String>,
}

/// A recipe's bake, as the app and agents read it. One per recipe of a
/// project: a new bake replaces the one before.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeBakeRecord {
    pub project: String,
    pub recipe: String,
    pub recipe_name: String,
    pub asset: String,
    pub state: RecipeBakeState,
    pub started_by: RecipeBakeStarter,
    pub progress: Option<RecipeBakeProgress>,
    pub error: Option<String>,
    pub started_ms: f64,
    pub updated_ms: f64,
    pub finished_ms: Option<f64>,
    pub result: Option<RecipeBakeResult>,
    /// Why a finished bake is out of date: the recipe changed since, or the
    /// model has another newest version than the one it went onto. Worked
    /// out on each read, never saved.
    #[serde(default)]
    pub stale: Option<String>,
    /// A finished bake's numbers against their limits, worked out again on
    /// each read.
    #[serde(default)]
    pub review: Option<RecipeBakeReview>,
    /// The model version it was added as.
    #[serde(default)]
    pub added_version: Option<u32>,
}

impl RecipeBakeRecord {
    pub fn running(&self) -> bool {
        self.state == RecipeBakeState::Running
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_add;
#[cfg(test)]
mod tests_blender;
#[cfg(test)]
mod tests_mark;
#[cfg(test)]
mod tests_rest;
