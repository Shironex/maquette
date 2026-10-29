//! Object states and clip choices: the motion part of the layout.
//!
//! A prop's states are poses of its parts (a cabinet closed, its door open,
//! its breakers on), each part moved and turned from where the model file
//! has it. Transitions time the move from one state to another and
//! triggers say what starts it on the page: arriving at a camera stop,
//! scrolling through a range, a click or a hover. They live in
//! `layout.json` under `states`, keyed by asset id like the placements:
//!
//! ```json
//! "states": {
//!   "switch-cabinet": {
//!     "states": [
//!       { "id": "closed", "name": "Closed", "parts": {} },
//!       { "id": "door-open", "name": "Door open",
//!         "parts": { "Door": { "position": [0, 0, 0], "rotation": [0, -105, 0] } } }
//!     ],
//!     "transitions": [{ "from": "closed", "to": "door-open", "duration": 0.6,
//!       "delay": 0, "easing": "ease-out", "stagger": 0 }],
//!     "triggers": [{ "kind": "stop", "to": "door-open", "stop": "switch-room",
//!       "part": null, "start": null, "end": null }]
//!   }
//! }
//! ```
//!
//! The first state is the rest pose. `clips` keeps what the Motion view
//! changed about a model's animation clips, keyed by asset id and clip
//! name; today that is whether the clip ships (`{ "ship": false }` makes
//! Build scene leave it out). Clips without an entry ship.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{MAX_REACH, SceneLayout, invalid};
use crate::error::CoreResult;
use crate::project::slugify;

pub const MAX_STATES: usize = 16;
pub const MAX_TRIGGERS: usize = 32;
pub const STATE_NAME_MAX_CHARS: usize = 60;
/// The longest a transition, its delay or its stagger may take, in seconds.
pub const MAX_SECONDS: f32 = 60.0;
/// How far a part may turn from its rest pose, in degrees.
const MAX_TURN: f32 = 3600.0;

/// How far one part moves from its rest pose in a state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PartPose {
    /// Metres added to the part's own position, glTF axes (y up).
    pub position: [f32; 3],
    /// Degrees added to its own rotation, X, then Y, then Z.
    pub rotation: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ObjectState {
    /// Stays the same when the state is renamed.
    pub id: String,
    pub name: String,
    /// Part (the node name in the model file) to how it moves. Parts left
    /// out stay at rest.
    pub parts: BTreeMap<String, PartPose>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Easing {
    Linear,
    /// Cubic, slow at the start.
    EaseIn,
    /// Cubic, slow at the end.
    EaseOut,
    EaseInOut,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    /// Seconds.
    pub duration: f32,
    /// Seconds before the move starts.
    pub delay: f32,
    pub easing: Easing,
    /// Seconds between one part starting and the next; 0 moves them together.
    pub stagger: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum TriggerKind {
    /// Arriving at a camera stop.
    Stop,
    /// Scrolling through a share of the page, scrubbed with the scroll.
    Scroll,
    Click,
    Hover,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StateTrigger {
    pub kind: TriggerKind,
    /// The state it goes to.
    pub to: String,
    /// For a stop trigger: the camera stop's id.
    pub stop: Option<String>,
    /// For a click or hover: the part it listens on; `None` is the whole model.
    pub part: Option<String>,
    /// For a scroll trigger: where the move starts and ends, percent of the page.
    pub start: Option<f32>,
    pub end: Option<f32>,
}

/// One asset's states, transitions and triggers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStates {
    /// The first is the rest pose.
    pub states: Vec<ObjectState>,
    pub transitions: Vec<StateTransition>,
    pub triggers: Vec<StateTrigger>,
}

/// What the Motion view changed about one clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipSettings {
    /// False leaves the clip out of the shipped model.
    pub ship: bool,
}

fn finite(values: &[f32]) -> bool {
    values.iter().all(|v| v.is_finite())
}

fn seconds(value: f32) -> bool {
    value.is_finite() && (0.0..=MAX_SECONDS).contains(&value)
}

fn check_state(asset: &str, state: &ObjectState) -> CoreResult<()> {
    let name = state.name.trim();
    if name.is_empty() {
        return Err(invalid(format!("Give every state of {asset} a name.")));
    }
    if name.chars().count() > STATE_NAME_MAX_CHARS {
        return Err(invalid(format!(
            "Keep the state name {name} to {STATE_NAME_MAX_CHARS} characters or fewer."
        )));
    }
    if state.id.is_empty() || slugify(&state.id) != state.id {
        return Err(invalid(format!(
            "The state {name} needs an id of lowercase letters, digits and dashes, like {}.",
            slugify(name)
        )));
    }
    for (part, pose) in &state.parts {
        if part.trim().is_empty() {
            return Err(invalid(format!(
                "The state {name} moves a part without a name."
            )));
        }
        if !finite(&pose.position) || !finite(&pose.rotation) {
            return Err(invalid(format!(
                "The state {name} moves {part} by a value that is not a number."
            )));
        }
        if pose.position.iter().any(|v| v.abs() > MAX_REACH) {
            return Err(invalid(format!(
                "The state {name} moves {part} more than {MAX_REACH} m."
            )));
        }
        if pose.rotation.iter().any(|v| v.abs() > MAX_TURN) {
            return Err(invalid(format!(
                "The state {name} turns {part} more than {MAX_TURN} degrees."
            )));
        }
    }
    Ok(())
}

fn check_transition(ids: &BTreeSet<&str>, t: &StateTransition) -> CoreResult<()> {
    for end in [&t.from, &t.to] {
        if !ids.contains(end.as_str()) {
            return Err(invalid(format!(
                "A transition names the state {end}, which is not there."
            )));
        }
    }
    if t.from == t.to {
        return Err(invalid(format!(
            "A transition goes from {} to itself.",
            t.from
        )));
    }
    if !seconds(t.duration) || !seconds(t.delay) || !seconds(t.stagger) {
        return Err(invalid(format!(
            "The move from {} to {} needs a duration, delay and stagger of 0 to {MAX_SECONDS} s.",
            t.from, t.to
        )));
    }
    Ok(())
}

fn check_trigger(ids: &BTreeSet<&str>, stops: &BTreeSet<&str>, t: &StateTrigger) -> CoreResult<()> {
    if !ids.contains(t.to.as_str()) {
        return Err(invalid(format!(
            "A trigger goes to the state {}, which is not there.",
            t.to
        )));
    }
    match t.kind {
        TriggerKind::Stop => match t.stop.as_deref() {
            Some(stop) if stops.contains(stop) => Ok(()),
            Some(stop) => Err(invalid(format!(
                "A trigger waits for the camera stop {stop}, which is not in the layout."
            ))),
            None => Err(invalid(
                "A stop trigger needs the camera stop it waits for.",
            )),
        },
        TriggerKind::Scroll => match (t.start, t.end) {
            (Some(start), Some(end))
                if start.is_finite()
                    && end.is_finite()
                    && 0.0 <= start
                    && start < end
                    && end <= 100.0 =>
            {
                Ok(())
            }
            _ => Err(invalid(
                "A scroll trigger needs a start below its end, both from 0 to 100 percent of the page.",
            )),
        },
        TriggerKind::Click | TriggerKind::Hover => match t.part.as_deref() {
            Some(part) if part.trim().is_empty() => Err(invalid(
                "A click or hover trigger names a part without a name.",
            )),
            _ => Ok(()),
        },
    }
}

/// Checks one asset's states against the layout's camera stops.
fn check_object(asset: &str, object: &ObjectStates, stops: &BTreeSet<&str>) -> CoreResult<()> {
    if object.states.is_empty() {
        return Err(invalid(format!(
            "{asset} has transitions or triggers but no states. Give it a rest state first."
        )));
    }
    if object.states.len() > MAX_STATES {
        return Err(invalid(format!(
            "Keep the states of {asset} to {MAX_STATES} or fewer."
        )));
    }
    let mut ids = BTreeSet::new();
    for state in &object.states {
        check_state(asset, state)?;
        if !ids.insert(state.id.as_str()) {
            return Err(invalid(format!(
                "Two states of {asset} share the id {}.",
                state.id
            )));
        }
    }
    let mut pairs = BTreeSet::new();
    for t in &object.transitions {
        check_transition(&ids, t)?;
        if !pairs.insert((t.from.as_str(), t.to.as_str())) {
            return Err(invalid(format!(
                "{asset} times the move from {} to {} twice.",
                t.from, t.to
            )));
        }
    }
    if object.triggers.len() > MAX_TRIGGERS {
        return Err(invalid(format!(
            "Keep the triggers of {asset} to {MAX_TRIGGERS} or fewer."
        )));
    }
    for t in &object.triggers {
        check_trigger(&ids, stops, t)?;
    }
    Ok(())
}

/// Checks the states and clip choices of `layout`.
pub(super) fn validate(layout: &SceneLayout) -> CoreResult<()> {
    let stops: BTreeSet<&str> = layout.stops.iter().map(|s| s.id.as_str()).collect();
    for (asset, object) in &layout.states {
        check_object(asset, object, &stops)?;
    }
    for (asset, clips) in &layout.clips {
        if clips.keys().any(|name| name.trim().is_empty()) {
            return Err(invalid(format!(
                "{asset} has a clip choice without a clip name."
            )));
        }
    }
    Ok(())
}

/// Drops the triggers that wait for a camera stop the layout no longer
/// has, so removing a stop never leaves a state pointing at nothing.
/// Returns how many went.
pub fn prune_stops(layout: &mut SceneLayout) -> usize {
    let stops: BTreeSet<String> = layout.stops.iter().map(|s| s.id.clone()).collect();
    let mut dropped = 0;
    for object in layout.states.values_mut() {
        let before = object.triggers.len();
        object.triggers.retain(|t| {
            t.kind != TriggerKind::Stop || t.stop.as_ref().is_some_and(|s| stops.contains(s))
        });
        dropped += before - object.triggers.len();
    }
    dropped
}

/// The clip choices without the ones that say what a clip does anyway
/// (ship), so `layout.json` only keeps what differs.
pub fn tidy_clips(
    clips: &BTreeMap<String, BTreeMap<String, ClipSettings>>,
) -> BTreeMap<String, BTreeMap<String, ClipSettings>> {
    clips
        .iter()
        .filter_map(|(asset, list)| {
            let kept: BTreeMap<String, ClipSettings> = list
                .iter()
                .filter(|(_, s)| !s.ship)
                .map(|(name, s)| (name.clone(), *s))
                .collect();
            (!kept.is_empty()).then(|| (asset.clone(), kept))
        })
        .collect()
}

/// The clips of `asset` that stay off the site.
pub fn clips_off<'a>(layout: &'a SceneLayout, asset: &str) -> Vec<&'a str> {
    layout
        .clips
        .get(asset)
        .map(|list| {
            list.iter()
                .filter(|(_, s)| !s.ship)
                .map(|(name, _)| name.as_str())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
