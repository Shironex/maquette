//! What a canvas's asset and Lab direction nodes show. A node keeps only
//! an id ([`crate::plans`]); its name, picture, versions and state are read
//! here from the project each time the canvas asks, so a node is never out
//! of date and an asset removed from the Board only shows as gone.
//!
//! Each state offers its next step in the view: an asset with a model to
//! check opens the Inbox, one with no model yet the Lab, a ready one the
//! Inspector; a direction opens the Lab.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;

use crate::assets::{Asset, AssetKind, ModelStats, Stage};
use crate::concepts::{self, ConceptJob, ConceptJobState, Direction};
use crate::inbox::InboxItem;
use crate::plans::{MAX_NODES, is_link_id};
use crate::project::ProjectManifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum AssetLinkState {
    /// No asset with this id in the project: removed, or from elsewhere.
    Gone,
    /// On the Board without a model yet.
    NoModel,
    /// A model for it waits in the Inbox to be checked.
    Check,
    /// Its newest version's file is not on disk.
    FileMissing,
    /// Its newest model is there.
    Ready,
}

/// An asset node's live view of its asset.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetLink {
    pub id: String,
    pub state: AssetLinkState,
    /// The rest is `None` for a gone asset.
    pub name: Option<String>,
    pub kind: Option<AssetKind>,
    pub stage: Option<Stage>,
    /// The asset's picture on this computer, when the file is there.
    pub thumbnail: Option<String>,
    /// The newest version, and its file relative to the project.
    pub version: Option<u32>,
    pub file: Option<String>,
    pub stats: Option<ModelStats>,
    /// Models in the Inbox made for this asset, waiting to be checked.
    pub checks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum DirectionLinkState {
    /// No direction with this id in the Lab.
    Gone,
    /// The direction has no images yet.
    Empty,
    /// Images for it are queued or being made.
    Running,
    /// It has images and none is picked.
    Pick,
    /// At least one image is picked.
    Picked,
}

/// A Lab direction node's live view of its direction.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DirectionLink {
    pub id: String,
    pub state: DirectionLinkState,
    pub name: Option<String>,
    pub images: u32,
    pub picked: u32,
    /// Its images still on their way, and those of the same jobs done.
    pub pending: u32,
    pub done: u32,
    /// The first picked image, else the newest, on this computer.
    pub thumbnail: Option<String>,
}

/// The live view of every asset and direction a canvas links.
#[derive(Debug, Clone, Default, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanLinks {
    pub assets: Vec<AssetLink>,
    pub directions: Vec<DirectionLink>,
}

/// What the links are read from: the project, the workspace Inbox and the
/// Lab's jobs.
pub struct LinkFacts<'a> {
    pub project_dir: &'a Path,
    pub manifest: &'a ProjectManifest,
    pub inbox: &'a [InboxItem],
    pub jobs: &'a [ConceptJob],
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The ids asked for, each once, in order, at most as many as a canvas
/// holds nodes.
fn unique(ids: &[String]) -> Vec<&str> {
    let mut seen = BTreeSet::new();
    ids.iter()
        .map(String::as_str)
        .filter(|id| seen.insert(*id))
        .take(MAX_NODES)
        .collect()
}

/// `relative` inside the project as an absolute path, when the file is there.
fn present(project_dir: &Path, relative: &str) -> Option<String> {
    let path = project_dir.join(relative);
    path.is_file().then(|| path.display().to_string())
}

/// The live view of `assets` and `directions` (ids from a canvas). An id
/// that names nothing, or is not an id at all, reads as gone.
pub fn read(facts: &LinkFacts<'_>, assets: &[String], directions: &[String]) -> PlanLinks {
    PlanLinks {
        assets: unique(assets)
            .into_iter()
            .map(|id| asset_link(facts, id))
            .collect(),
        directions: unique(directions)
            .into_iter()
            .map(|id| direction_link(facts, id))
            .collect(),
    }
}

fn gone_asset(id: &str) -> AssetLink {
    AssetLink {
        id: id.to_string(),
        state: AssetLinkState::Gone,
        name: None,
        kind: None,
        stage: None,
        thumbnail: None,
        version: None,
        file: None,
        stats: None,
        checks: 0,
    }
}

/// Inbox models made for `asset` of this project that nothing replaced.
fn checks_for(facts: &LinkFacts<'_>, asset: &str) -> u32 {
    let project = facts.manifest.id.as_str();
    let mine = |item: &&InboxItem| {
        let sent = item
            .link
            .as_ref()
            .is_some_and(|l| l.project == project && l.asset == asset);
        let aimed = item
            .aim
            .as_ref()
            .is_some_and(|a| a.project == project && a.asset == asset);
        (sent || aimed) && item.replaced_by.is_none()
    };
    count(facts.inbox.iter().filter(mine).count())
}

fn asset_link(facts: &LinkFacts<'_>, id: &str) -> AssetLink {
    let found = is_link_id(id)
        .then(|| facts.manifest.assets.iter().find(|a| a.id == id))
        .flatten();
    let Some(asset) = found else {
        return gone_asset(id);
    };
    let latest = asset.latest();
    let checks = checks_for(facts, id);
    let file_there = latest.is_some_and(|v| present(facts.project_dir, &v.file).is_some());
    let state = match latest {
        _ if checks > 0 => AssetLinkState::Check,
        None => AssetLinkState::NoModel,
        Some(_) if !file_there => AssetLinkState::FileMissing,
        Some(_) => AssetLinkState::Ready,
    };
    live_asset(facts.project_dir, asset, state, checks)
}

fn live_asset(project_dir: &Path, asset: &Asset, state: AssetLinkState, checks: u32) -> AssetLink {
    let latest = asset.latest();
    AssetLink {
        id: asset.id.clone(),
        state,
        name: Some(asset.name.clone()),
        kind: Some(asset.kind),
        stage: Some(asset.stage),
        thumbnail: asset
            .thumbnail
            .as_deref()
            .and_then(|t| present(project_dir, t)),
        version: latest.map(|v| v.version),
        file: latest.map(|v| v.file.clone()),
        stats: latest.and_then(|v| v.stats.clone()),
        checks,
    }
}

fn direction_link(facts: &LinkFacts<'_>, id: &str) -> DirectionLink {
    let found = is_link_id(id)
        .then(|| {
            concepts::load_direction(facts.project_dir, id)
                .ok()
                .flatten()
        })
        .flatten();
    let Some(direction) = found else {
        return DirectionLink {
            id: id.to_string(),
            state: DirectionLinkState::Gone,
            name: None,
            images: 0,
            picked: 0,
            pending: 0,
            done: 0,
            thumbnail: None,
        };
    };
    live_direction(facts, &direction)
}

fn live_direction(facts: &LinkFacts<'_>, direction: &Direction) -> DirectionLink {
    let jobs: Vec<&ConceptJob> = facts
        .jobs
        .iter()
        .filter(|j| j.project == facts.manifest.id && j.direction == direction.id)
        .collect();
    let pending = count(jobs.iter().filter(|j| !j.state.is_finished()).count());
    let done = count(
        jobs.iter()
            .filter(|j| j.state == ConceptJobState::Done)
            .count(),
    );
    let picked = count(direction.images.iter().filter(|i| i.picked).count());
    let images = count(direction.images.len());
    let state = if pending > 0 {
        DirectionLinkState::Running
    } else if picked > 0 {
        DirectionLinkState::Picked
    } else if images > 0 {
        DirectionLinkState::Pick
    } else {
        DirectionLinkState::Empty
    };
    let cover = direction
        .images
        .iter()
        .find(|i| i.picked)
        .or_else(|| direction.images.iter().max_by_key(|i| i.index));
    DirectionLink {
        id: direction.id.clone(),
        state,
        name: Some(direction.name.clone()),
        images,
        picked,
        pending,
        done: if pending > 0 { done } else { 0 },
        thumbnail: cover.and_then(|i| present(facts.project_dir, &i.file)),
    }
}

pub fn register(types: specta::Types) -> specta::Types {
    types.register::<PlanLinks>()
}

#[cfg(test)]
mod tests;
