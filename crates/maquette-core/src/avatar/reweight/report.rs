//! What a reweight leaves to judge it by: the script's counts and the
//! result with its approval state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::avatar::BridgeFaces;
use crate::avatar::normalize::{NormalizeSheets, NormalizeState};

/// The skin weights after the reweight.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ReweightChecks {
    pub vertices: u32,
    pub max_influences: u32,
    /// Vertices left without weight; a good run leaves none.
    pub unweighted: u32,
    /// Vertices whose weights do not sum to 1.
    pub off_sum: u32,
}

/// One arm's share of the counts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ReweightSide {
    pub moved: u32,
    pub kept_as_sleeve: u32,
    /// The limb radius along the arm, shoulder to finger tips, as a share
    /// of the height.
    pub limb_radius: Vec<f64>,
    /// Faces of this arm's bridges.
    pub bridge_faces: u32,
}

/// Faces that join a part following an arm to a part staying with the
/// body, far from the shoulder: lifting the arm stretches them whatever the
/// weights (a mesh fused where a sleeve rested on the hip).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ReweightBridge {
    /// `left` or `right`.
    pub side: String,
    pub faces: u32,
    /// Their middle in the model's own space, metres.
    pub at: [f64; 3],
    /// The longest stretched edge once the arm is level, metres.
    pub length: f64,
}

/// The script's `report.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ReweightReport {
    /// Skinned vertices read.
    pub vertices: u32,
    /// Vertices the arms held any weight of, before.
    pub arm_weighted: u32,
    /// Vertices that gave arm weight to the trunk.
    pub moved: u32,
    /// The arm weight moved, summed over those vertices.
    pub weight_moved: f64,
    /// Arm vertices kept on the arm: skin, sleeves, gloves.
    pub kept_as_sleeve: u32,
    /// Vertices in the smooth band between the two, keeping some.
    pub band: u32,
    /// Arm vertices that gave their body or leg weight to the arm, to
    /// follow it wholly where the lift moves them far.
    pub follow_arm: u32,
    /// Faces of every bridge.
    pub bridge_faces: u32,
    /// The largest bridges, largest first.
    pub bridges: Vec<ReweightBridge>,
    /// Bridge faces taken out of the model so they are not drawn; 0 when
    /// they were kept.
    pub hidden_faces: u32,
    /// The arm bones the weight came from, with how many vertices each.
    pub from_bones: BTreeMap<String, u32>,
    /// The trunk bones it went to, with how many vertices each.
    pub to_bones: BTreeMap<String, u32>,
    pub weights: ReweightChecks,
    /// Per arm, `left` and `right`.
    pub sides: BTreeMap<String, ReweightSide>,
}

/// A reweighted model and what it takes to judge it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Reweighted {
    pub project: String,
    pub asset: String,
    pub source_version: u32,
    pub state: NormalizeState,
    /// The reweighted GLB, an absolute path.
    pub glb: String,
    /// Rows rest, arms level and arms raised; columns front, side, back and
    /// the shoulders.
    pub sheets: NormalizeSheets,
    pub report: ReweightReport,
    /// Whether the bridge faces were hidden or kept.
    #[serde(default)]
    pub bridges: BridgeFaces,
    /// The run's cache key; an approval holds for this key only.
    pub key: String,
    pub approved_at: Option<String>,
}
