//! What the springs steps leave: the candidates the finder proposed, the
//! chain step's report and its result with an approval state, and the
//! summary a VRM report records.

use serde::{Deserialize, Serialize};

use crate::avatar::normalize::{NormalizeSheets, NormalizeState};
use crate::avatar::reweight::ReweightChecks;
use crate::inbox::vrm::{SpringCollider, SpringKind, SpringSettings};

/// A part hanging off the body the finder proposes for a spring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpringCandidate {
    /// `hair-1`, `cloth-2`: the kind and its rank, most sure first.
    pub id: String,
    pub kind: SpringKind,
    /// The VRM humanoid bone it hangs from.
    pub root_bone: String,
    /// Its centre line from where it hangs, glTF metres.
    pub line: Vec<[f64; 3]>,
    /// Metres along the line.
    pub length: f64,
    /// Metres it drops below where it hangs from.
    pub drop: f64,
    pub vertices: u32,
    /// 0 to 1: how sure the finder is this part hangs free.
    pub confidence: f64,
    /// The joints a chain gets by its length.
    pub joints: u32,
    /// Its colour on the candidates sheet, `#rrggbb`.
    pub color: String,
}

/// The finder's `candidates.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct FoundFile {
    pub(crate) height: f64,
    pub(crate) vertices: u32,
    pub(crate) candidates: Vec<SpringCandidate>,
}

/// What the finder found on the model a VRM is written from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FoundSprings {
    /// Most sure first.
    pub candidates: Vec<SpringCandidate>,
    /// The model with each candidate in its colour, front, side and back;
    /// an absolute path.
    pub sheet: String,
    /// The model the finder read, an absolute path.
    pub input: String,
    /// The model's height in metres.
    pub height: f64,
}

/// One chain the chain step added.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ChainBuilt {
    pub id: String,
    pub kind: Option<SpringKind>,
    pub root_bone: String,
    /// The root bone's joint (node name).
    pub root_joint: String,
    /// The chain's joint nodes, root to tip.
    pub joints: Vec<String>,
    pub length: f64,
    /// Vertices weighted to the chain.
    pub vertices: u32,
    /// Of those, the ones that keep some of their old weight where the part
    /// meets the rest.
    pub blended: u32,
    /// Each joint's head, glTF metres.
    pub heads: Vec<[f64; 3]>,
    /// How far the part's vertices lie from the line, the median.
    pub radius: f64,
    /// The collider groups it touches.
    pub collider_groups: Vec<String>,
}

/// The chain step's `report.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ChainReport {
    pub height: f64,
    pub chains: Vec<ChainBuilt>,
    pub colliders: Vec<SpringCollider>,
    pub weights: ReweightChecks,
    /// Vertices whose weights changed.
    pub changed: u32,
}

/// A model with chains added and what it takes to judge it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Chained {
    pub project: String,
    pub asset: String,
    pub source_version: u32,
    pub state: NormalizeState,
    /// The model with the chains, an absolute path.
    pub glb: String,
    /// Before: the parts and the joints to come. After: the sway test,
    /// rows rest, left, right and back.
    pub sheets: NormalizeSheets,
    pub report: ChainReport,
    /// The run's cache key; an approval holds for this key only.
    pub key: String,
    pub approved_at: Option<String>,
}

/// One spring as a written VRM has it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpringWritten {
    pub id: String,
    pub kind: Option<SpringKind>,
    pub root_bone: String,
    pub joints: Vec<String>,
    pub length: f64,
    pub vertices: u32,
    pub settings: SpringSettings,
    pub collider_groups: Vec<String>,
}

/// The springs a written VRM carries, for its report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpringsSummary {
    pub springs: Vec<SpringWritten>,
    pub colliders: u32,
    /// The collider groups, by name.
    pub collider_groups: Vec<String>,
}
