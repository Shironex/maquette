//! The Timeline view: the page as it scrolls, one row per camera stop.
//!
//! [`view`] gathers what the view lays out: the camera stops in tour
//! order, every model with its parts, the object states (whose stop and
//! scroll triggers the view places on the rows), the paths and the explode
//! and energize tracks of [`crate::layout::timeline`]. [`save`] writes back the stop and scroll
//! triggers of the assets the view changed and the whole timeline; the
//! states themselves, their transitions and the click and hover triggers
//! stay as the Motion view saved them.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::error::{CoreError, CoreResult};
use crate::layout::{
    self, LayoutStop, ObjectStates, SceneLayout, ScenePath, StateTrigger, Timeline, TriggerKind,
};
use crate::project::ProjectManifest;

/// A model the timeline can move.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TimelineModel {
    pub asset: String,
    pub name: String,
    pub thumbnail: Option<String>,
    /// The nodes that draw a mesh, in file order: what an explode track
    /// spreads apart. Empty when the model cannot be read.
    pub parts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TimelineView {
    pub project: String,
    /// In tour order: the columns of the timeline.
    pub stops: Vec<LayoutStop>,
    /// Every model, in Build scene order.
    pub models: Vec<TimelineModel>,
    pub states: BTreeMap<String, ObjectStates>,
    /// The layout's paths, whose energize tracks get a lane each.
    pub paths: Vec<ScenePath>,
    pub timeline: Timeline,
}

/// True for the triggers the timeline places: the ones the scroll drives.
pub fn on_scroll(trigger: &StateTrigger) -> bool {
    matches!(trigger.kind, TriggerKind::Stop | TriggerKind::Scroll)
}

/// Everything the Timeline view shows for a project.
///
/// # Errors
///
/// Fails when `layout.json` cannot be read.
pub fn view(project_dir: &Path, manifest: &ProjectManifest) -> CoreResult<TimelineView> {
    let layout = layout::load(project_dir)?;
    Ok(view_of(project_dir, manifest, layout))
}

fn view_of(project_dir: &Path, manifest: &ProjectManifest, layout: SceneLayout) -> TimelineView {
    let mut parts = crate::motion::parts(project_dir, manifest);
    let models = layout::placeable(manifest)
        .into_iter()
        .map(|asset| TimelineModel {
            asset: asset.id.clone(),
            name: asset.name.clone(),
            thumbnail: asset.thumbnail.clone(),
            parts: parts.remove(&asset.id).unwrap_or_default(),
        })
        .collect();
    TimelineView {
        project: manifest.id.clone(),
        stops: layout.stops,
        models,
        states: layout.states,
        paths: layout.paths,
        timeline: layout.timeline,
    }
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

/// `current` with the stop and scroll triggers of each asset in `triggers`
/// replaced and the timeline replaced whole.
///
/// # Errors
///
/// `Invalid` for an asset without states or without a model, a trigger
/// that is not a stop or scroll trigger, or a result the layout refuses.
pub fn with_timeline(
    current: &SceneLayout,
    known: &[String],
    triggers: &BTreeMap<String, Vec<StateTrigger>>,
    timeline: Timeline,
) -> CoreResult<SceneLayout> {
    let mut next = current.clone();
    for (asset, list) in triggers {
        let object = next.states.get_mut(asset).ok_or_else(|| {
            invalid(format!(
                "{asset} has no states yet. Give it states in the Motion view first."
            ))
        })?;
        if list.iter().any(|t| !on_scroll(t)) {
            return Err(invalid(
                "The timeline sets stop and scroll triggers only. Click and hover belong to the Motion view.",
            ));
        }
        object.triggers.retain(|t| !on_scroll(t));
        object.triggers.extend(list.iter().cloned());
    }
    for asset in timeline.explode.keys() {
        if !known.contains(asset) {
            return Err(invalid(format!("There is no model for the asset {asset}.")));
        }
    }
    next.timeline = timeline;
    layout::validate(&next)?;
    Ok(next)
}

/// Writes the timeline's triggers and tracks into the layout and returns
/// the view of it.
///
/// # Errors
///
/// As [`with_timeline`], or when `layout.json` cannot be read or written.
pub fn save(
    project_dir: &Path,
    manifest: &ProjectManifest,
    triggers: &BTreeMap<String, Vec<StateTrigger>>,
    timeline: Timeline,
) -> CoreResult<TimelineView> {
    let current = layout::load(project_dir)?;
    let known = layout::known_assets(manifest);
    let next = with_timeline(&current, &known, triggers, timeline)?;
    layout::save(project_dir, &next)?;
    Ok(view_of(project_dir, manifest, next))
}

#[cfg(test)]
mod tests;

/// Registers this module's IPC types (the Timeline view) with the shell's TypeScript export;
/// `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types.register::<TimelineView>()
}
