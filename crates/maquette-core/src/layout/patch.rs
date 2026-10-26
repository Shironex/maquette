//! Editing the layout: patches for placements, stops and the desk, applied
//! and validated as a whole.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use super::*;

/// One change to a model's placement. Fields left out keep their value; a
/// new placement starts at the origin, unrotated, at scale 1.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementPatch {
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub scale: Option<f32>,
}

/// One change to a stop: an existing id updates it, a new one (or none)
/// adds a stop, which then needs a name, a position and a target.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopPatch {
    pub id: Option<String>,
    pub name: Option<String>,
    pub position: Option<[f32; 3]>,
    pub target: Option<[f32; 3]>,
    pub fov: Option<f32>,
    /// `Some(None)` (an explicit null) goes back to measuring the view.
    #[serde(default, deserialize_with = "explicit")]
    pub models: Option<Option<Vec<String>>>,
    #[serde(default)]
    pub remove: bool,
}

/// A partial update: only what it names changes.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutPatch {
    /// Asset id to its change; `None` drops the placement, so the model
    /// stands side by side again.
    #[serde(default)]
    pub models: BTreeMap<String, Option<PlacementPatch>>,
    #[serde(default)]
    pub stops: Vec<StopPatch>,
    /// Every stop id in the new order.
    pub stop_order: Option<Vec<String>>,
    /// `Some(None)` (an explicit null) removes the desk.
    #[serde(default, deserialize_with = "explicit")]
    pub desk: Option<Option<Desk>>,
    /// Asset id to its whole set of states; `None` removes them.
    #[serde(default)]
    pub states: BTreeMap<String, Option<ObjectStates>>,
    /// Asset id to clip name to whether it ships.
    #[serde(default)]
    pub clips: BTreeMap<String, BTreeMap<String, ClipSettings>>,
    /// Asset id to its whole explode track; `None` removes it.
    #[serde(default)]
    pub explode: BTreeMap<String, Option<ExplodeTrack>>,
}

/// Applies `patch` to `layout`. `known` are the asset ids a placement, a
/// stop's model list, states or clip choices may name (the assets with a
/// model). Triggers and explode keys that wait for a stop the patch
/// removes go with it.
/// Nothing changes when any part is refused.
///
/// # Errors
///
/// Fails on an unknown asset or stop, a new stop without a name, position
/// or target, a stop order that is not every stop once, or a result that
/// [`validate`] refuses.
pub fn apply(
    layout: &SceneLayout,
    patch: &LayoutPatch,
    known: &[String],
) -> CoreResult<SceneLayout> {
    let mut next = layout.clone();
    let check_asset = |id: &str| {
        if known.iter().any(|k| k == id) {
            Ok(())
        } else {
            Err(invalid(format!(
                "There is no model for the asset {id}. Use one of: {}.",
                known.join(", ")
            )))
        }
    };
    for (id, change) in &patch.models {
        match change {
            None => {
                next.models.remove(id);
            }
            Some(change) => {
                check_asset(id)?;
                let placement = next.models.entry(id.clone()).or_default();
                if let Some(position) = change.position {
                    placement.position = position;
                }
                if let Some(rotation) = change.rotation {
                    placement.rotation = rotation;
                }
                if let Some(scale) = change.scale {
                    placement.scale = scale;
                }
            }
        }
    }
    for change in &patch.stops {
        let found = change
            .id
            .as_deref()
            .and_then(|id| next.stops.iter().position(|s| s.id == id));
        if change.remove {
            let index = found.ok_or_else(|| {
                invalid(format!(
                    "There is no camera stop {} to remove.",
                    change.id.as_deref().unwrap_or("without an id")
                ))
            })?;
            next.stops.remove(index);
            continue;
        }
        if let Some(Some(models)) = &change.models {
            for id in models {
                check_asset(id)?;
            }
        }
        match found {
            Some(index) => {
                let stop = &mut next.stops[index];
                if let Some(name) = &change.name {
                    stop.name = name.trim().to_string();
                }
                if let Some(position) = change.position {
                    stop.position = position;
                }
                if let Some(target) = change.target {
                    stop.target = target;
                }
                if let Some(fov) = change.fov {
                    stop.fov = fov;
                }
                if let Some(models) = &change.models {
                    stop.models.clone_from(models);
                }
            }
            None => {
                let name = change
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .ok_or_else(|| invalid("A new camera stop needs a name."))?;
                let (Some(position), Some(target)) = (change.position, change.target) else {
                    return Err(invalid(format!(
                        "The new stop {name} needs a position and a target."
                    )));
                };
                let id = match &change.id {
                    Some(id) => id.clone(),
                    None => stop_id(&next, name),
                };
                next.stops.push(LayoutStop {
                    id,
                    name: name.to_string(),
                    position,
                    target,
                    fov: change.fov.unwrap_or(DEFAULT_FOV),
                    models: change.models.clone().flatten(),
                });
            }
        }
    }
    if let Some(order) = &patch.stop_order {
        let wanted: BTreeSet<&String> = order.iter().collect();
        let have: BTreeSet<&String> = next.stops.iter().map(|s| &s.id).collect();
        if wanted != have || order.len() != next.stops.len() {
            return Err(invalid(format!(
                "stopOrder must name every stop once: {}.",
                next.stops
                    .iter()
                    .map(|s| s.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        next.stops
            .sort_by_key(|s| order.iter().position(|id| *id == s.id));
    }
    if let Some(desk) = patch.desk {
        next.desk = desk;
    }
    prune_stops(&mut next);
    for (id, change) in &patch.states {
        match change {
            None => {
                next.states.remove(id);
            }
            Some(object) => {
                check_asset(id)?;
                next.states.insert(id.clone(), object.clone());
            }
        }
    }
    for (id, clips) in &patch.clips {
        check_asset(id)?;
        let list = next.clips.entry(id.clone()).or_default();
        for (name, settings) in clips {
            list.insert(name.clone(), *settings);
        }
    }
    next.clips = states::tidy_clips(&next.clips);
    for (id, change) in &patch.explode {
        match change {
            None => {
                next.timeline.explode.remove(id);
            }
            Some(track) => {
                check_asset(id)?;
                next.timeline.explode.insert(id.clone(), track.clone());
            }
        }
    }
    validate(&next)?;
    Ok(next)
}
