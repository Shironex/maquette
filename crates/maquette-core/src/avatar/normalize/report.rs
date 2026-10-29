//! What a normalising run leaves to judge it by: the script's report
//! (measures before and after, the arm lifts, the weight checks) and the
//! result with its approval state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::Elbows;
use crate::inbox::vrm::VrmReadiness;

/// The two pose sheets, absolute paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NormalizeSheets {
    pub before: String,
    pub after: String,
}

/// The rest pose as the script measured it, in glTF metres and degrees.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PoseMeasures {
    pub facing_deg: f64,
    pub arm_left_deg: f64,
    pub arm_right_deg: f64,
    /// Degrees each elbow bends at rest.
    pub elbow_left_deg: Option<f64>,
    pub elbow_right_deg: Option<f64>,
    pub ground_y: f64,
    pub height: f64,
    pub hips: [f64; 3],
    pub hips_offset: f64,
    pub vertices: u32,
}

/// How far one arm turned.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ArmLift {
    pub shoulder_deg: f64,
    /// Only when the elbows were straightened.
    pub elbow_deg: Option<f64>,
}

/// The skin weights over the bone groups.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct WeightChecks {
    pub vertices: u32,
    /// Vertices with more than 4 bone influences.
    pub over_influences: u32,
    /// Vertices whose weights do not sum to 1.
    pub off_sum: u32,
    pub unweighted: u32,
    pub max_influences: u32,
    pub min_sum: Option<f64>,
    pub max_sum: Option<f64>,
    /// Vertices trimmed to 4 influences or normalised.
    pub fixed: u32,
}

/// The script's `report.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct NormalizeReport {
    pub before: PoseMeasures,
    pub after: PoseMeasures,
    /// Degrees the model turned about the up axis.
    pub turned_deg: f64,
    /// Metres it moved, in glTF axes.
    pub moved: [f64; 3],
    pub elbows: Elbows,
    pub lifted: BTreeMap<String, ArmLift>,
    pub weights_before: WeightChecks,
    pub weights: WeightChecks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum NormalizeState {
    /// Made and waiting for a person's yes.
    Pending,
    /// A person said yes to this exact output.
    Approved,
}

/// A normalised model and what it takes to judge it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Normalized {
    pub project: String,
    pub asset: String,
    pub source_version: u32,
    pub state: NormalizeState,
    /// The normalised GLB, an absolute path.
    pub glb: String,
    pub sheets: NormalizeSheets,
    pub report: NormalizeReport,
    /// The readiness of the normalised GLB.
    pub readiness: VrmReadiness,
    /// The run's cache key; an approval holds for this key only.
    pub key: String,
    pub approved_at: Option<String>,
}
