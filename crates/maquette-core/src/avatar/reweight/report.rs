//! What a reweight leaves to judge it by: the script's counts and the
//! result with its approval state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
    /// The run's cache key; an approval holds for this key only.
    pub key: String,
    pub approved_at: Option<String>,
}
