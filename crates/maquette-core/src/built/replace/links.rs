//! What a project's other files say about the asset that keeps its place:
//! everything keyed by its id stays as it is (its placement, the camera
//! stops and path ends that name it, its object states, explode track and
//! clip choices, its ship choice and file name, its game file), and what
//! names a part or a clip of the old model is checked against the new one.
//! Nothing is taken out of the layout: a part the new model lacks is
//! reported, and works again once the script names a part that way.

use std::path::Path;

use crate::assets::Asset;
use crate::layout::{ObjectStates, SceneLayout};
use crate::prepare::trim_number;
use crate::project::ProjectManifest;
use crate::ship::{ShipChoice, ShipSettings, ShipStyle};

use super::report::{SwapAction, SwapSetting};

/// What stays by the asset's id, what no longer resolves, and what to know.
#[derive(Default)]
pub(super) struct Links {
    pub kept: Vec<SwapSetting>,
    pub unresolved: Vec<String>,
    pub warnings: Vec<String>,
}

fn kept(what: impl Into<String>, why: &str) -> SwapSetting {
    SwapSetting::new(what, SwapAction::Kept, why)
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn triple(v: [f32; 3]) -> String {
    format!(
        "{}, {}, {}",
        trim_number(v[0]),
        trim_number(v[1]),
        trim_number(v[2])
    )
}

const BY_ID: &str = "It is keyed by the asset's id, which stays.";

/// The parts and clips the asset's states, triggers and clip choices name
/// that the new model does not have.
fn unresolved_states(
    states: &ObjectStates,
    parts: &[String],
    has_parts: bool,
    out: &mut Vec<String>,
) {
    if !has_parts {
        return;
    }
    let missing = |part: &String| !parts.contains(part);
    for state in &states.states {
        for part in state.parts.keys().filter(|p| missing(p)) {
            out.push(format!(
                "The state {} moves the part {part}, which the built model does not have.",
                state.name
            ));
        }
    }
    for trigger in &states.triggers {
        if let Some(part) = trigger.part.as_ref().filter(|p| missing(p)) {
            out.push(format!(
                "A trigger to the state {} listens on the part {part}, which the built model does not have.",
                trigger.to
            ));
        }
    }
}

fn layout_links(
    layout: &SceneLayout,
    id: &str,
    parts: Option<&[String]>,
    clips: Option<&[String]>,
    out: &mut Links,
) {
    if let Some(p) = layout.models.get(id) {
        out.kept.push(kept(
            format!(
                "Layout placement: position {}, turn {}, scale {}",
                triple(p.position),
                triple(p.rotation),
                trim_number(p.scale)
            ),
            BY_ID,
        ));
    }
    let stops: Vec<&str> = layout
        .stops
        .iter()
        .filter(|s| s.models.as_ref().is_some_and(|m| m.iter().any(|a| a == id)))
        .map(|s| s.name.as_str())
        .collect();
    if !stops.is_empty() {
        out.kept.push(kept(
            format!("Camera stops that list it: {}", stops.join(", ")),
            BY_ID,
        ));
    }
    let ends = layout
        .paths
        .iter()
        .filter(|p| p.start.as_deref() == Some(id) || p.end.as_deref() == Some(id))
        .count();
    if ends > 0 {
        out.kept
            .push(kept(format!("Path ends that follow it: {ends}"), BY_ID));
    }
    if let Some(states) = layout.states.get(id) {
        out.kept.push(kept(
            format!(
                "Object states: {}, {}",
                count(states.states.len(), "state", "states"),
                count(states.triggers.len(), "trigger", "triggers")
            ),
            "They stay in the layout and move the parts the built model has by those names.",
        ));
        unresolved_states(
            states,
            parts.unwrap_or_default(),
            parts.is_some(),
            &mut out.unresolved,
        );
    }
    if let Some(track) = layout.timeline.explode.get(id) {
        out.kept.push(kept(
            format!("Explode track, {}", count(track.keys.len(), "key", "keys")),
            "It stays in the layout and spreads the built model's parts.",
        ));
        if let Some(parts) = parts.filter(|p| p.len() < 2) {
            out.unresolved.push(format!(
                "The explode track spreads parts apart, and the built model has {}.",
                count(parts.len(), "part", "parts")
            ));
        }
    }
    if let Some(choices) = layout.clips.get(id) {
        out.kept.push(kept(
            format!("Clip choices, {}", count(choices.len(), "clip", "clips")),
            "They stay in the layout and apply to clips of those names.",
        ));
        if let Some(clips) = clips {
            for name in choices.keys().filter(|n| !clips.contains(*n)) {
                out.unresolved.push(format!(
                    "The clip choice for {name} names a clip the built model does not have."
                ));
            }
        }
    }
}

fn ship_links(manifest: &ProjectManifest, ship: &ShipSettings, asset: &Asset, out: &mut Links) {
    if let Ok(game) = crate::game::settings_of(manifest) {
        out.kept.push(kept(
            format!("Game file {}", crate::game::file_of(game, asset)),
            "The built model exports to the same file, and Ship, which always asks in a game, shows it as replaced.",
        ));
        return;
    }
    let choice = ship.choice(&asset.id);
    if let Some(file) = ship.file(&asset.id) {
        out.kept
            .push(kept(format!("Ship file name {file}.glb"), BY_ID));
    }
    match choice {
        ShipChoice::Never => {
            out.kept.push(kept("Ship choice: never", BY_ID));
            out.warnings.push(format!(
                "{} is set to never ship, so Ship leaves the site's file alone. Set its choice to auto or always to ship the built model.",
                asset.name
            ));
        }
        ShipChoice::Always => out.kept.push(kept("Ship choice: always", BY_ID)),
        ShipChoice::Auto => {}
    }
    if ship.style == ShipStyle::Models && choice == ShipChoice::Auto {
        out.warnings.push(format!(
            "This site is hand-built: the built model ships under the same file name, and Ship holds it back when the site's file is smaller or was not made by Maquette. The Ship plan says so; set {} to always ship to replace it.",
            asset.name
        ));
    }
}

/// Reads what the project's files hold for `asset`, against the parts and
/// clips of the new model (`None` when its file could not be read).
pub(super) fn of_asset(
    project_dir: &Path,
    manifest: &ProjectManifest,
    layout: &SceneLayout,
    ship: &ShipSettings,
    asset: &Asset,
    parts: Option<&[String]>,
    clips: Option<&[String]>,
) -> Links {
    let mut out = Links::default();
    layout_links(layout, &asset.id, parts, clips, &mut out);
    ship_links(manifest, ship, asset, &mut out);
    let bakes = crate::recipes::baked::of_asset(project_dir, &asset.id).unwrap_or_default();
    if !bakes.is_empty() {
        let mixamo = bakes.iter().any(|b| b.mixamo);
        out.kept.push(kept(
            format!(
                "Recipe bake records, {}",
                count(bakes.len(), "bake", "bakes")
            ),
            if mixamo {
                "They stay on the asset, so its clips still count as coming from a Mixamo clip."
            } else {
                "They stay on the asset: a clip of one of these names still counts as a recipe bake."
            },
        ));
    }
    let avatar = crate::avatar::load(project_dir)
        .map(|file| file.avatars.contains_key(&asset.id))
        .unwrap_or(false);
    if avatar {
        out.kept.push(kept("Avatar spec", BY_ID));
        out.warnings.push(format!(
            "{} has an avatar spec, which stays. It only fits a model with the same rig, so check the Avatar view.",
            asset.name
        ));
    }
    out
}
