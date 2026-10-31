//! What a project's other files say about one asset: its placement, the
//! camera stops that count it, the path ends that follow it, its object
//! states, clips and explode track in `layout.json`, its real size and
//! fix-ups, its material tweaks, its avatar spec, its built source and its
//! game model. A
//! remove takes them out and keeps them in the trash record; a restore puts
//! back what still fits.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::avatar::{self, AvatarSpec};
use crate::built::{self, BuiltSource};
use crate::error::CoreResult;
use crate::layout::{self, ClipSettings, ExplodeTrack, ObjectStates, Placement, SceneLayout};
use crate::materials::{self, ModelMaterials};
use crate::prepare::{self, ModelPrep};
use crate::project::ProjectManifest;
use crate::settings::game::GameModel;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct AssetLinks {
    pub placement: Option<Placement>,
    /// The camera stops whose hand set models listed it.
    pub stops: Vec<String>,
    /// The paths whose first point followed it.
    pub path_starts: Vec<String>,
    /// The paths whose last point followed it.
    pub path_ends: Vec<String>,
    pub states: Option<ObjectStates>,
    pub clips: Option<BTreeMap<String, ClipSettings>>,
    pub explode: Option<ExplodeTrack>,
    pub prep: Option<ModelPrep>,
    pub materials: Option<ModelMaterials>,
    pub avatar: Option<AvatarSpec>,
    pub built: Option<BuiltSource>,
    pub game: Option<GameModel>,
}

fn layout_links(layout: &SceneLayout, id: &str, links: &mut AssetLinks) {
    links.placement = layout.models.get(id).copied();
    links.states = layout.states.get(id).cloned();
    links.clips = layout.clips.get(id).cloned();
    links.explode = layout.timeline.explode.get(id).cloned();
    links.stops = layout
        .stops
        .iter()
        .filter(|s| s.models.as_ref().is_some_and(|m| m.iter().any(|a| a == id)))
        .map(|s| s.id.clone())
        .collect();
    let ends = |end: fn(&layout::ScenePath) -> &Option<String>| {
        layout
            .paths
            .iter()
            .filter(|p| end(p).as_deref() == Some(id))
            .map(|p| p.id.clone())
            .collect::<Vec<_>>()
    };
    links.path_starts = ends(|p| &p.start);
    links.path_ends = ends(|p| &p.end);
}

/// Reads what the project's files hold for `id`. A file that cannot be
/// read is skipped with a warning; the remove goes on without it.
pub(super) fn take(
    project_dir: &Path,
    manifest: &ProjectManifest,
    id: &str,
) -> (AssetLinks, Vec<String>) {
    let mut links = AssetLinks {
        game: manifest
            .game
            .as_ref()
            .and_then(|g| g.models.get(id).cloned()),
        ..AssetLinks::default()
    };
    let mut warnings = Vec::new();
    let mut warn = |what: &str, err: crate::error::CoreError| {
        warnings.push(format!(
            "Could not read {what}, so it keeps what it says. {err}"
        ));
    };
    match layout::load(project_dir) {
        Ok(layout) => layout_links(&layout, id, &mut links),
        Err(err) => warn(layout::LAYOUT_FILE, err),
    }
    match prepare::load(project_dir) {
        Ok(prep) => links.prep = prep.models.get(id).cloned(),
        Err(err) => warn(prepare::PREPARE_FILE, err),
    }
    match materials::load(project_dir) {
        Ok(found) => links.materials = found.models.get(id).cloned(),
        Err(err) => warn(materials::MATERIALS_FILE, err),
    }
    match avatar::load(project_dir) {
        Ok(found) => links.avatar = found.avatars.get(id).cloned(),
        Err(err) => warn(avatar::AVATAR_FILE, err),
    }
    match built::load(project_dir) {
        Ok(found) => links.built = found.assets.get(id).cloned(),
        Err(err) => warn(built::BUILT_FILE, err),
    }
    (links, warnings)
}

fn clear_layout(layout: &mut SceneLayout, id: &str) {
    layout.models.remove(id);
    layout.states.remove(id);
    layout.clips.remove(id);
    layout.timeline.explode.remove(id);
    for stop in &mut layout.stops {
        if let Some(models) = stop.models.as_mut() {
            models.retain(|a| a != id);
        }
    }
    for path in &mut layout.paths {
        if path.start.as_deref() == Some(id) {
            path.start = None;
        }
        if path.end.as_deref() == Some(id) {
            path.end = None;
        }
    }
}

/// Writes `file` only when `links` says it held something of the asset.
fn when(needed: bool, file: &str, write: impl FnOnce() -> CoreResult<()>) -> Option<String> {
    if !needed {
        return None;
    }
    write()
        .err()
        .map(|err| format!("Could not update {file}. {err}"))
}

fn has_layout(links: &AssetLinks) -> bool {
    links.placement.is_some()
        || links.states.is_some()
        || links.clips.is_some()
        || links.explode.is_some()
        || !links.stops.is_empty()
        || !links.path_starts.is_empty()
        || !links.path_ends.is_empty()
}

/// Takes the asset `id` out of the layout, prep, materials, avatar and built
/// files, wherever `links` found it. Returns what could not be written.
pub(super) fn clear(project_dir: &Path, id: &str, links: &AssetLinks) -> Vec<String> {
    let steps = [
        when(has_layout(links), layout::LAYOUT_FILE, || {
            layout::update(project_dir, |current| {
                let mut layout = current.clone();
                clear_layout(&mut layout, id);
                Ok(layout)
            })
            .map(|_| ())
        }),
        when(links.prep.is_some(), prepare::PREPARE_FILE, || {
            prepare::save_entry(project_dir, id, &ModelPrep::default()).map(|_| ())
        }),
        when(links.materials.is_some(), materials::MATERIALS_FILE, || {
            let empty = BTreeMap::from([(id.to_string(), ModelMaterials::default())]);
            materials::save_looks(project_dir, &empty).map(|_| ())
        }),
        when(links.avatar.is_some(), avatar::AVATAR_FILE, || {
            avatar::set(project_dir, id, None).map(|_| ())
        }),
        when(links.built.is_some(), built::BUILT_FILE, || {
            built::update(project_dir, |file| {
                file.assets.remove(id);
                Ok(())
            })
        }),
    ];
    steps.into_iter().flatten().collect()
}

fn restore_layout(layout: &mut SceneLayout, id: &str, links: &AssetLinks) {
    if let Some(placement) = links.placement {
        layout.models.entry(id.to_string()).or_insert(placement);
    }
    if let Some(states) = &links.states {
        layout
            .states
            .entry(id.to_string())
            .or_insert_with(|| states.clone());
    }
    if let Some(clips) = &links.clips {
        layout
            .clips
            .entry(id.to_string())
            .or_insert_with(|| clips.clone());
    }
    if let Some(track) = &links.explode {
        layout
            .timeline
            .explode
            .entry(id.to_string())
            .or_insert_with(|| track.clone());
    }
    for stop in &mut layout.stops {
        if links.stops.contains(&stop.id)
            && let Some(models) = stop.models.as_mut()
            && !models.iter().any(|a| a == id)
        {
            models.push(id.to_string());
        }
    }
    for path in &mut layout.paths {
        if path.start.is_none() && links.path_starts.contains(&path.id) {
            path.start = Some(id.to_string());
        }
        if path.end.is_none() && links.path_ends.contains(&path.id) {
            path.end = Some(id.to_string());
        }
    }
    // A stop removed since then takes its triggers and keys with it.
    layout::prune_stops(layout);
}

/// Puts back what `links` kept of the asset `id`, where nothing has taken
/// its place since. Returns what could not be written.
pub(super) fn put_back(project_dir: &Path, id: &str, links: &AssetLinks) -> Vec<String> {
    let steps = [
        when(has_layout(links), layout::LAYOUT_FILE, || {
            layout::update(project_dir, |current| {
                let mut layout = current.clone();
                restore_layout(&mut layout, id, links);
                Ok(layout)
            })
            .map(|_| ())
        }),
        when(links.prep.is_some(), prepare::PREPARE_FILE, || {
            prepare::update(project_dir, |current| {
                let mut next = current.clone();
                if let Some(prep) = &links.prep {
                    next.models
                        .entry(id.to_string())
                        .or_insert_with(|| prep.clone());
                }
                Ok(next)
            })
            .map(|_| ())
        }),
        when(links.materials.is_some(), materials::MATERIALS_FILE, || {
            materials::update(project_dir, |current| {
                let mut next = current.clone();
                if let Some(look) = &links.materials {
                    next.models
                        .entry(id.to_string())
                        .or_insert_with(|| look.clone());
                }
                Ok(next)
            })
            .map(|_| ())
        }),
        when(links.avatar.is_some(), avatar::AVATAR_FILE, || {
            avatar::update(project_dir, |current| {
                let mut next = current.clone();
                if let Some(spec) = &links.avatar {
                    next.avatars
                        .entry(id.to_string())
                        .or_insert_with(|| spec.clone());
                }
                Ok(next)
            })
            .map(|_| ())
        }),
        when(links.built.is_some(), built::BUILT_FILE, || {
            built::update(project_dir, |file| {
                if let Some(source) = &links.built {
                    file.assets
                        .entry(id.to_string())
                        .or_insert_with(|| source.clone());
                }
                Ok(())
            })
        }),
    ];
    steps.into_iter().flatten().collect()
}
