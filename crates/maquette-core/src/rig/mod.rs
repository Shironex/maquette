//! The rig doctor: a rigged model from the Inbox (or an accepted version) to
//! a clean next version with correct skin weights.
//!
//! It is the character rig skill run by the app: the same scripts, bundled
//! from the skill's folder so there is one copy of them ([`scripts`]). Reading
//! needs no Blender and happens on arrival (`maquette_glb`'s rig probe). The
//! pose test and the fix run headless on a click or an agent's call, always
//! on a copy under `<workspace>/<cache>/rig/<id>/`, and take minutes.
//!
//! Only `JOINTS_0` and `WEIGHTS_0` change: mesh, UVs, texture, bone names,
//! hierarchy and rest pose stay. A misplaced joint chain is only ever moved
//! through [`RigDoctors::decide_joints`], which no agent can reach; adding the
//! draft as the next version is a person's call too. It never starts a Tripo
//! or Meshy send.

use serde::{Deserialize, Serialize};

mod chains;
mod clips;
mod creature;
mod defects;
mod humanoid;
mod land;
mod read;
mod record;
mod run;
pub mod scripts;
mod service;
mod steps;
mod verdict;

pub use chains::{RigChainEdit, waits_for_chains};
pub use clips::clips_on_joints;
pub use land::got_worse;
pub use read::{read_model, route_of};
pub use record::{load, recent, register, save, work_dir};
pub use service::RigDoctors;
pub use verdict::{SEAM_MM, TEAR_MM};

/// The workspace cache folder records and drafts live in.
pub const WORK_DIR: &str = "rig";
/// Rounds of fixing before the doctor stops instead of guessing.
pub const MAX_ROUNDS: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RigRoute {
    /// Mixamo names and one skinned primitive.
    Humanoid,
    /// Its own bone names: chains read from the bone tree.
    Creature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RigState {
    /// The scripts cannot work on this model; nothing was written.
    Refused,
    /// Read; the pose test waits for a click or an agent's call.
    Ready,
    Testing,
    /// The before sheet is there; the fix waits for a click or a call.
    Tested,
    Fixing,
    /// A draft and its checks are there; adding it is a person's call.
    Fixed,
    /// A stop: the draft (if any) is kept and not added.
    Stopped,
    /// The draft became the next version.
    Landed,
    Discarded,
    Cancelled,
    Failed,
}

impl RigState {
    pub fn running(self) -> bool {
        matches!(self, Self::Testing | Self::Fixing)
    }
}

/// The six steps on the rig doctor's left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RigStep {
    Read,
    Accept,
    Before,
    Fix,
    Verify,
    Land,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RigStarter {
    Person,
    Agent,
}

/// What a run does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RigRun {
    /// The pose test before, with the probe.
    PoseTest,
    /// The fix, its checks and the pose test after.
    Fix,
    /// The fix again with a wider seam; the last round.
    Round2,
    /// A creature's draft walking on a copy, to see the rig move. The clip
    /// never goes into the draft.
    WalkProof,
}

/// Where the model comes from: an Inbox file until it is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigSource {
    /// The Inbox file; null once accepted.
    pub inbox: Option<String>,
    pub project: Option<String>,
    pub asset: Option<String>,
    /// The accepted version the doctor starts from.
    pub version: Option<u32>,
}

/// One line of why the model is refused, or what was fine about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigIssue {
    pub ok: bool,
    pub title: String,
    pub detail: String,
}

/// What the read says about the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigModel {
    /// "Mixamo, 65 joints".
    pub rig: String,
    pub joints: u32,
    pub skinned_meshes: u32,
    pub clips: Vec<String>,
    pub triangles: u32,
    pub height: f32,
    pub textures: u32,
    pub hand_far: u32,
    pub arm_far: u32,
    pub islands: u32,
    /// One continuous shell: the per island fix would tear it.
    pub one_island: bool,
    /// Mixamo names, so the humanoid route can run on it.
    #[serde(default)]
    pub humanoid_ok: bool,
    /// The Inbox check that sends a person here, in words.
    pub finding: Option<String>,
}

/// One row of a pose sheet: a pose or clip frame, or a creature chain bent.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigRow {
    pub id: String,
    pub label: String,
    /// "cape swings", or null for a row that looks fine. On an after row, how
    /// its numbers moved.
    pub note: Option<String>,
    /// The defects (by number) this row shows.
    pub defects: Vec<u32>,
    /// The worst edge's change in length, mm at a one metre tall model. Any
    /// rig stretches an edge across a bent joint, so it says little alone.
    pub stretch_mm: f64,
    /// How far an edge stretches past what a healthy skin does, held over half
    /// a percent of the surface: the part the weights answer for. A record
    /// older than the share has its single worst edge here. Null on a record
    /// older than this number.
    #[serde(default)]
    pub beyond_body_mm: Option<f64>,
    /// The single worst edge of that reading. Null on an older record.
    #[serde(default)]
    pub worst_edge_mm: Option<f64>,
    /// A few edges pull hard on too little of the surface to call a tear, so
    /// the row is not flagged: where to look, in words.
    #[serde(default)]
    pub look: Option<String>,
    /// How far seam copies that share a position at rest came apart: a crack.
    pub seam_gap_mm: f64,
    pub front: Option<String>,
    pub side: Option<String>,
    pub back: Option<String>,
    pub xray: Option<String>,
    /// The file before, under the same camera as this row. Null when the
    /// row's own sheet holds only one file.
    #[serde(default)]
    pub before_front: Option<String>,
    #[serde(default)]
    pub before_side: Option<String>,
    /// How an after row moved from the before sheet.
    #[serde(default)]
    pub change: Option<RigRowChange>,
}

/// Which way a number went after the fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RigTrend {
    Better,
    Worse,
    Same,
}

/// An after row's before numbers, and which way each went.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigRowChange {
    pub stretch_before: f64,
    pub beyond_body_before: Option<f64>,
    pub seam_gap_before: f64,
    pub stretch: RigTrend,
    pub seam_gap: RigTrend,
}

/// A pose sheet: one PNG of everything, and its rows.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigSheet {
    pub file: String,
    pub rows: Vec<RigRow>,
    pub seconds: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum DefectKind {
    /// Weights alone answer it.
    WeightsOnly,
    /// A joint move would be needed, and waits for a person's yes.
    NeedsYes,
    /// Nothing the doctor changes; a note for the person.
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigDefect {
    pub number: u32,
    pub title: String,
    pub place: String,
    /// Bones and counts: "LeftArm 3,912 v".
    pub tags: Vec<String>,
    pub kind: DefectKind,
    pub detail: String,
    /// Set by the fix: "fixed", or what stays open.
    pub outcome: Option<String>,
    pub fixed: bool,
}

/// One row of a creature's chains table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigChain {
    pub chain: String,
    /// The chain's first bone, which an edit names; empty on a row of several
    /// chains, which cannot be edited.
    #[serde(default)]
    pub key: String,
    pub bones: u32,
    pub side: String,
    /// chain, stray, swapped or junk.
    pub read_as: String,
    pub note: Option<String>,
}

/// What the creature route read from the bones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigCreature {
    pub front: String,
    pub legs: u32,
    pub swapped: u32,
    pub junk: Vec<String>,
    /// Bones with no or trivial weight: in no chain, and left in the file.
    #[serde(default)]
    pub ignored: Vec<String>,
    pub rigid_pieces: u32,
    pub one_shell: bool,
    pub chains: Vec<RigChain>,
    /// The weight map render.
    pub weight_map: Option<String>,
}

/// One step the fix took, with its number ("5,330 v").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigFixStep {
    pub title: String,
    pub detail: String,
    pub value: String,
}

/// One check of the verify step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigCheck {
    pub ok: bool,
    pub title: String,
    pub detail: String,
    pub value: String,
}

/// A number before and after the fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigStat {
    pub label: String,
    pub before: Option<String>,
    pub after: String,
    /// A number that is still bad.
    pub bad: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum JointMoveState {
    /// Waits for a person. No agent can answer it.
    Asked,
    Approved,
    Declined,
}

/// The offer to move a misplaced joint chain onto its mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct JointMove {
    pub state: JointMoveState,
    /// "left arm".
    pub limb: String,
    /// How far the chain sits off the mesh.
    pub off_cm: f64,
    pub joints: u32,
    /// The clips with translation tracks on those joints: they would pull
    /// the joints back.
    pub breaks_clips: Vec<String>,
    /// The file with the joints moved, ready if the answer is yes.
    pub draft: String,
    /// The same poses with the joints moved.
    pub sheet: Option<RigSheet>,
}

/// One run step, for the Headless Blender list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigStage {
    pub name: String,
    pub detail: String,
    /// Null while it runs.
    pub seconds: Option<f64>,
}

/// One log line under a running pose test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigLogLine {
    pub at: String,
    pub script: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigProgress {
    pub label: String,
    pub done: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RigStopKind {
    /// It still tears after the last round.
    Rounds,
    /// A check got worse after the fix.
    Worse,
    /// The fix moved no weight: there is no draft, and the defects stay open.
    NoChange,
}

/// Why the doctor stopped, with the row that shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigStop {
    pub kind: RigStopKind,
    pub title: String,
    pub detail: String,
    /// The pose row that shows it.
    pub row: Option<String>,
}

/// One rig doctor case, on disk in its work folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigRecord {
    pub id: String,
    pub name: String,
    pub started_by: RigStarter,
    /// An agent opened this case too, so its next draft asks a person.
    #[serde(default)]
    #[specta(skip)]
    pub agent_opened: bool,
    pub route: Option<RigRoute>,
    pub state: RigState,
    pub step: RigStep,
    pub source: RigSource,
    /// The file the doctor reads (the Inbox file or the accepted version).
    pub input: String,
    /// The work folder.
    pub dir: String,
    pub model: RigModel,
    pub issues: Vec<RigIssue>,
    pub creature: Option<RigCreature>,
    pub before: Option<RigSheet>,
    pub after: Option<RigSheet>,
    pub defects: Vec<RigDefect>,
    pub stats: Vec<RigStat>,
    pub fix_steps: Vec<RigFixStep>,
    pub checks: Vec<RigCheck>,
    pub joint_move: Option<JointMove>,
    pub round: u32,
    pub draft: Option<String>,
    pub progress: Option<RigProgress>,
    pub stages: Vec<RigStage>,
    pub log: Vec<RigLogLine>,
    pub stop: Option<RigStop>,
    /// A person looked at the chains read from the bone tree. A creature's
    /// draft cannot be added before that.
    #[serde(default)]
    pub chains_confirmed: bool,
    /// What a person corrected in those chains; every read applies it.
    #[serde(default)]
    pub chain_edits: Vec<RigChainEdit>,
    /// The strip of a creature's walk clip proof.
    #[serde(default)]
    pub walk_proof: Option<String>,
    /// The version the draft became.
    pub landed: Option<u32>,
    pub error: Option<String>,
    pub updated_ms: f64,
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_blender;
#[cfg(test)]
mod tests_blender_person;
#[cfg(test)]
mod tests_blender_readings;
#[cfg(test)]
mod tests_rounds;
