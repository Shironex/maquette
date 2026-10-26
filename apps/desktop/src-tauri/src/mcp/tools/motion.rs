//! Motion for agents: `get_motion` reads every model's rig, clips and parts
//! with what the clips weigh inside the scene budget; `set_layout` takes
//! the object states and clip choices through the edits here.

use std::collections::BTreeMap;

use maquette_core::layout::{
    ClipSettings, Easing, ObjectState, ObjectStates, PartPose, StateTransition, StateTrigger,
    TriggerKind,
};
use maquette_core::project::slugify;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;

use super::{McpTools, Out, ProjectArgs, respond, to_short_value};
use crate::commands::motion;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PoseEdit {
    /// Metres added to the part's own position, [x, y, z] with y up. Zero when left out.
    pub position: Option<[f32; 3]>,
    /// Degrees added to its own rotation, X then Y then Z. Zero when left out.
    pub rotation: Option<[f32; 3]>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StateEdit {
    /// Made from the name when left out; keep it to rename a state.
    pub id: Option<String>,
    pub name: String,
    /// Part name (as get_motion lists the model's parts) to how it moves from rest.
    #[serde(default)]
    pub parts: BTreeMap<String, PoseEdit>,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EasingEdit {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl EasingEdit {
    pub(super) fn easing(self) -> Easing {
        match self {
            Self::Linear => Easing::Linear,
            Self::EaseIn => Easing::EaseIn,
            Self::EaseOut => Easing::EaseOut,
            Self::EaseInOut => Easing::EaseInOut,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransitionEdit {
    /// A state id or name.
    pub from: String,
    /// A state id or name.
    pub to: String,
    /// Seconds, 0.6 when left out.
    pub duration: Option<f32>,
    /// Seconds before the move starts, 0 when left out.
    pub delay: Option<f32>,
    /// ease-in-out when left out.
    pub easing: Option<EasingEdit>,
    /// Seconds between one part starting and the next, 0 when left out.
    pub stagger: Option<f32>,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TriggerEdit {
    Stop,
    Scroll,
    Click,
    Hover,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TriggerEditArgs {
    /// stop (arriving at a camera stop), scroll (a range of the page), click or hover.
    pub kind: TriggerEdit,
    /// The state id or name it goes to.
    pub to: String,
    /// For stop: the camera stop id, as get_layout lists it.
    pub stop: Option<String>,
    /// For click or hover: the part it listens on; leave it out for the whole model.
    pub part: Option<String>,
    /// For scroll: where the move starts, percent of the page.
    pub start: Option<f32>,
    /// For scroll: where it ends, percent of the page.
    pub end: Option<f32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatesEdit {
    /// The asset id.
    pub asset: String,
    /// Every state in order, the first is the rest pose. Replaces the asset's states.
    #[serde(default)]
    pub states: Vec<StateEdit>,
    #[serde(default)]
    pub transitions: Vec<TransitionEdit>,
    #[serde(default)]
    pub triggers: Vec<TriggerEditArgs>,
    /// Drop every state of the asset.
    #[serde(default)]
    pub remove: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClipEdit {
    /// The asset id.
    pub asset: String,
    /// The clip name, as get_motion lists it.
    pub clip: String,
    /// false keeps the clip off the site (Build scene leaves it out), true ships it again.
    pub ship: bool,
}

/// A state id from an id or a name the edit gave.
fn state_id(states: &[ObjectState], named: &str) -> String {
    let named = named.trim();
    states
        .iter()
        .find(|s| s.id == named || s.name.eq_ignore_ascii_case(named))
        .map_or_else(|| named.to_string(), |s| s.id.clone())
}

fn object_states(edit: &StatesEdit) -> ObjectStates {
    let states: Vec<ObjectState> = edit
        .states
        .iter()
        .map(|s| ObjectState {
            id: s
                .id
                .as_deref()
                .map_or_else(|| slugify(&s.name), |id| id.trim().to_string()),
            name: s.name.trim().to_string(),
            parts: s
                .parts
                .iter()
                .map(|(part, pose)| {
                    let pose = PartPose {
                        position: pose.position.unwrap_or_default(),
                        rotation: pose.rotation.unwrap_or_default(),
                    };
                    (part.clone(), pose)
                })
                .collect(),
        })
        .collect();
    let transitions = edit
        .transitions
        .iter()
        .map(|t| StateTransition {
            from: state_id(&states, &t.from),
            to: state_id(&states, &t.to),
            duration: t.duration.unwrap_or(0.6),
            delay: t.delay.unwrap_or(0.0),
            easing: t.easing.unwrap_or(EasingEdit::EaseInOut).easing(),
            stagger: t.stagger.unwrap_or(0.0),
        })
        .collect();
    let triggers = edit
        .triggers
        .iter()
        .map(|t| StateTrigger {
            kind: match t.kind {
                TriggerEdit::Stop => TriggerKind::Stop,
                TriggerEdit::Scroll => TriggerKind::Scroll,
                TriggerEdit::Click => TriggerKind::Click,
                TriggerEdit::Hover => TriggerKind::Hover,
            },
            to: state_id(&states, &t.to),
            stop: t.stop.as_ref().map(|s| s.trim().to_string()),
            part: t.part.clone(),
            start: t.start,
            end: t.end,
        })
        .collect();
    ObjectStates {
        states,
        transitions,
        triggers,
    }
}

/// The layout patch's states: each asset's whole set, or `None` to drop it.
pub(super) fn states_patch(edits: &[StatesEdit]) -> BTreeMap<String, Option<ObjectStates>> {
    edits
        .iter()
        .map(|e| {
            let set = (!e.remove).then(|| object_states(e));
            (e.asset.trim().to_string(), set)
        })
        .collect()
}

/// The layout patch's clip choices, asset to clip to whether it ships.
pub(super) fn clips_patch(edits: &[ClipEdit]) -> BTreeMap<String, BTreeMap<String, ClipSettings>> {
    let mut out: BTreeMap<String, BTreeMap<String, ClipSettings>> = BTreeMap::new();
    for e in edits {
        out.entry(e.asset.trim().to_string())
            .or_default()
            .insert(e.clip.clone(), ClipSettings { ship: e.ship });
    }
    out
}

#[tool_router(router = motion_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get a project's motion: every model with its rig (joint count, whether it maps onto a humanoid skeleton, the bone map and the bones a humanoid clip would miss, most bones on one vertex), its animation clips (name, seconds, frames, key rate, keyframe bytes in the source and as shipped, root travel in metres, whether it loops) and its parts (the mesh nodes object states move), the object states, transitions and triggers, the clips kept off the site, and the budget: clips are bytes inside the models, so they count in the scene budget and the first view, with the motion share of each. Reads only.",
        annotations(read_only_hint = true)
    )]
    async fn get_motion(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let view = motion::get(host.session(), &args.project).map_err(|e| e.message)?;
                Ok(to_short_value(&view))
            })
            .await;
        respond(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_name_states_by_name_and_fill_the_defaults() {
        let edit: StatesEdit = serde_json::from_value(serde_json::json!({
            "asset": " cabinet ",
            "states": [
                { "name": "Closed" },
                { "name": "Door open", "parts": { "Door": { "rotation": [0, -105, 0] } } }
            ],
            "transitions": [{ "from": "Closed", "to": "door open" }],
            "triggers": [{ "kind": "stop", "to": "Door open", "stop": "switch-room" }]
        }))
        .expect("edit");
        let patch = states_patch(&[edit]);
        let set = patch["cabinet"].as_ref().expect("set");
        assert_eq!(set.states[1].id, "door-open");
        assert_eq!(set.states[1].parts["Door"].position, [0.0; 3]);
        let t = &set.transitions[0];
        assert_eq!((t.from.as_str(), t.to.as_str()), ("closed", "door-open"));
        assert_eq!(t.easing, Easing::EaseInOut);
        assert_eq!(set.triggers[0].to, "door-open");

        let gone: StatesEdit =
            serde_json::from_value(serde_json::json!({ "asset": "cabinet", "remove": true }))
                .expect("edit");
        assert!(states_patch(&[gone])["cabinet"].is_none());

        let clips: Vec<ClipEdit> = serde_json::from_value(serde_json::json!([
            { "asset": "figure", "clip": "Wave", "ship": false },
            { "asset": "figure", "clip": "Walk loop", "ship": true }
        ]))
        .expect("clips");
        let clips = clips_patch(&clips);
        assert!(!clips["figure"]["Wave"].ship && clips["figure"]["Walk loop"].ship);
    }
}
