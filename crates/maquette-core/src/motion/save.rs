//! Saving the Motion view: a project's object states and the clips that
//! stay off the site, written into its `layout.json` next to the models and
//! camera stops, which stay as they are.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{CoreError, CoreResult};
use crate::layout::{self, ClipSettings, ObjectStates, SceneLayout};
use crate::project::ProjectManifest;

/// Refuses states that move a part the asset's model does not have.
///
/// # Errors
///
/// `Invalid`, naming the first unknown part and its asset.
pub fn check_parts(
    project_dir: &Path,
    manifest: &ProjectManifest,
    states: &BTreeMap<String, ObjectStates>,
) -> CoreResult<()> {
    match super::unknown_parts(project_dir, manifest, states).first() {
        Some((asset, part)) => Err(CoreError::Invalid(format!(
            "The model of {asset} has no part named {part}. Pick one of the parts its model has."
        ))),
        None => Ok(()),
    }
}

/// The layout with `states` and the clips in `clips_off` (asset to clip
/// names) replacing the ones it had.
///
/// # Errors
///
/// `Invalid` for an asset the project has no model for, a part its model
/// lacks, or states the layout refuses.
pub fn with_motion(
    project_dir: &Path,
    manifest: &ProjectManifest,
    current: &SceneLayout,
    states: BTreeMap<String, ObjectStates>,
    clips_off: &BTreeMap<String, Vec<String>>,
) -> CoreResult<SceneLayout> {
    let known = layout::known_assets(manifest);
    for asset in states.keys().chain(clips_off.keys()) {
        if !known.contains(asset) {
            return Err(CoreError::Invalid(format!(
                "There is no model for the asset {asset}."
            )));
        }
    }
    check_parts(project_dir, manifest, &states)?;
    let clips: BTreeMap<String, BTreeMap<String, ClipSettings>> = clips_off
        .iter()
        .filter(|(_, names)| !names.is_empty())
        .map(|(asset, names)| {
            let off = names
                .iter()
                .map(|n| (n.clone(), ClipSettings { ship: false }))
                .collect();
            (asset.clone(), off)
        })
        .collect();
    let next = SceneLayout {
        states: states
            .into_iter()
            .filter(|(_, o)| !o.states.is_empty())
            .collect(),
        clips,
        ..current.clone()
    };
    layout::validate(&next)?;
    Ok(next)
}

/// Writes the Motion view's states and clip choices into the layout.
///
/// # Errors
///
/// As [`with_motion`], or when `layout.json` cannot be read or written.
pub fn save(
    project_dir: &Path,
    manifest: &ProjectManifest,
    states: BTreeMap<String, ObjectStates>,
    clips_off: &BTreeMap<String, Vec<String>>,
) -> CoreResult<()> {
    let current = layout::load(project_dir)?;
    let next = with_motion(project_dir, manifest, &current, states, clips_off)?;
    layout::save(project_dir, &next)
}
