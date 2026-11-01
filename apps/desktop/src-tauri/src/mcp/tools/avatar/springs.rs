//! The springs part of the avatar tools: confirming candidates by id or
//! none at all, the springs mode word, and the short answers of a build the
//! springs stop.

use maquette_core::avatar::SpringSpec;
use maquette_core::avatar::export::{NoSprings, SpringsChange, SpringsView};
use maquette_core::avatar::springs::{Chained, FoundSprings, SpringsMode};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::{Value, json};

use super::to_short_value;

/// One spring to confirm. Name a candidate get_avatar found by its id and
/// the rest comes from it; give kind, rootBone and line to confirm a part
/// by hand.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpringArg {
    /// The candidate's id, such as hair-1, or a new id (lowercase letters, digits, dashes).
    pub id: String,
    /// "hair", "cloth" or "accessory"; the candidate's when left out.
    pub kind: Option<String>,
    /// The VRM humanoid bone it hangs from; the candidate's when left out.
    pub root_bone: Option<String>,
    /// Its centre line in glTF metres from where it hangs, 2 to 64 points; the candidate's
    /// when left out.
    pub line: Option<Vec<[f64; 3]>>,
    /// Joints along the chain, 1 to 16; by its length when left out (a small piece, such as
    /// charm-1, keeps its 2 to 4).
    pub joints: Option<u32>,
    /// "soft" (hair), "heavy" (cloth) or "stiff" (accessory); by kind when left out.
    pub preset: Option<String>,
    /// 0 to 4: how strongly each joint returns to its rest direction.
    pub stiffness: Option<f64>,
    /// 0 to 2: how strongly gravity pulls it down.
    pub gravity_power: Option<f64>,
    /// 0 to 1: how quickly a swing dies down.
    pub drag_force: Option<f64>,
    /// 0 to 0.5 metres: each joint's radius against the colliders.
    pub hit_radius: Option<f64>,
}

/// What `set_avatar` confirms: the springs, or none at all.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum SpringsArg {
    /// "none": the VRM carries no springs, whatever the finder proposes.
    Word(String),
    /// The springs to confirm, replacing the ones confirmed before. An empty list clears the
    /// picks: nobody picked yet, so a build waits for a pick when the last export had springs.
    List(Vec<SpringArg>),
}

/// The change `arg` makes to the confirmed springs.
pub(super) fn change(view: &SpringsView, arg: &SpringsArg) -> Result<SpringsChange, String> {
    match arg {
        SpringsArg::Word(word) if word.eq_ignore_ascii_case("none") => {
            Ok(SpringsChange::None(NoSprings::None))
        }
        SpringsArg::Word(word) => Err(format!(
            "springs is a list of springs to confirm, or \"none\" for no springs, not {word:?}."
        )),
        SpringsArg::List(list) => confirmed(view, list).map(SpringsChange::Picked),
    }
}

pub(super) fn parse_springs(word: Option<&str>) -> Result<SpringsMode, String> {
    word.map_or(Ok(SpringsMode::default()), |w| {
        serde_json::from_value(json!(w.to_lowercase()))
            .map_err(|_| format!("springs must be auto or off, not {w:?}."))
    })
}

/// The springs `args` confirm, each filled in from the candidate of the
/// same id where it leaves something out.
pub(super) fn confirmed(view: &SpringsView, args: &[SpringArg]) -> Result<Vec<SpringSpec>, String> {
    let candidates = view
        .found
        .as_ref()
        .map(|f| f.candidates.as_slice())
        .unwrap_or_default();
    args.iter()
        .map(|arg| {
            let found = candidates.iter().find(|c| c.id == arg.id);
            let kind = match (&arg.kind, found) {
                (Some(k), _) => json!(k.to_lowercase()),
                (None, Some(c)) => json!(c.kind),
                (None, None) => {
                    return Err(format!(
                        "No candidate is called {}. Name one get_avatar lists under springs.found, or give kind, rootBone and line.",
                        arg.id
                    ));
                }
            };
            let root_bone = arg.root_bone.clone().or_else(|| found.map(|c| c.root_bone.clone()));
            let line = arg.line.clone().or_else(|| found.map(|c| c.line.clone()));
            // A small piece keeps its short chain unless the joints are given.
            let joints = arg
                .joints
                .or_else(|| found.filter(|c| c.small).map(|c| c.joints));
            let spec = json!({
                "id": arg.id,
                "kind": kind,
                "rootBone": root_bone,
                "line": line,
                "joints": joints,
                "preset": arg.preset.as_deref().map(str::to_lowercase),
                "stiffness": arg.stiffness,
                "gravityPower": arg.gravity_power,
                "dragForce": arg.drag_force,
                "hitRadius": arg.hit_radius,
            });
            serde_json::from_value(spec).map_err(|e| format!("Spring {}: {e}.", arg.id))
        })
        .collect()
}

/// A chain step, short: the sheets to judge and what it added.
pub(super) fn chain_summary(found: &Chained) -> Value {
    let report = &found.report;
    json!({
        "sourceVersion": found.source_version,
        "state": to_short_value(found.state),
        "sheets": to_short_value(&found.sheets),
        "chains": report.chains.iter().map(|c| json!({
            "id": c.id,
            "rootBone": c.root_bone,
            "joints": c.joints.len(),
            "length": c.length,
            "vertices": c.vertices,
            "colliderGroups": c.collider_groups,
        })).collect::<Vec<_>>(),
        "colliders": report.colliders.len(),
        "changed": report.changed,
    })
}

/// The candidates, short: what a person picks from.
pub(super) fn candidates_summary(found: &FoundSprings) -> Value {
    json!({
        "sheet": found.sheet,
        "candidates": found.candidates.iter().map(|c| json!({
            "id": c.id,
            "kind": to_short_value(c.kind),
            "rootBone": c.root_bone,
            "length": c.length,
            "vertices": c.vertices,
            "confidence": c.confidence,
            "joints": c.joints,
            "color": c.color,
            "small": c.small,
        })).collect::<Vec<_>>(),
    })
}
