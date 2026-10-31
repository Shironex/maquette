//! What a pack would hold before anything runs: its models with their
//! numbers, the kinds with the biggest model of each, a size estimate and
//! the models whose clips stay out. Reads only.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::licence;
use super::{MAX_MODELS, PackSettings, PackSource, kind_label};
use crate::assets::{Asset, AssetKind, ModelStats};
use crate::error::{CoreError, CoreResult};
use crate::library::{self, LibraryEntry, LibrarySource};
use crate::prepare::{ModelPrep, SizeAxis};
use crate::project;
use crate::workspace::FolderLayout;

/// What made a project asset's version and what it cost, as the library
/// reads it from the project's sends. The shell hands it in, since the
/// sends live above this layer.
pub type SourceOf<'a> = dyn Fn(&Path, &Asset, u32) -> (LibrarySource, Option<f64>) + 'a;

/// Bytes a zipped triangle costs, about, as OBJ text and as GLB.
const OBJ_TRIANGLE_BYTES: f64 = 40.0;
const GLB_TRIANGLE_BYTES: f64 = 30.0;
/// Bytes a texture pixel costs as PNG, zipped, about.
const PIXEL_BYTES: f64 = 0.9;

/// One model of a pack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackModel {
    /// Its file stem and folder in the pack: `cast_iron_stove`.
    pub key: String,
    pub name: String,
    pub kind: AssetKind,
    /// The library entry it comes from.
    pub entry: Option<String>,
    /// The project asset it comes from.
    pub project: Option<String>,
    pub asset: Option<String>,
    pub version: u32,
    /// The model file, an absolute path.
    pub file: String,
    pub stats: Option<ModelStats>,
    pub rigged: bool,
    /// Its height in metres when its real size gives one.
    pub height_m: Option<f32>,
    /// Its clips may be Mixamo's, so they stay out.
    pub mixamo: bool,
    pub source: Option<LibrarySource>,
    pub licence: String,
    /// Its picture, an absolute path.
    pub thumbnail: Option<String>,
    /// Its concept and refs, absolute paths.
    pub refs: Vec<String>,
    /// The real size and fix-ups it has where it came from.
    pub prep: Option<ModelPrep>,
}

impl PackModel {
    pub fn triangles(&self) -> u32 {
        self.stats.as_ref().map_or(0, |s| s.triangles)
    }
}

/// One kind's row in the budget table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KindRow {
    pub kind: AssetKind,
    pub label: String,
    pub count: u32,
    /// The most triangles one model of the kind has now.
    pub biggest: u32,
    pub budget: Option<u32>,
}

/// What a pack would hold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackPlan {
    /// "12 models from the library", "8 models of Frostveil".
    pub label: String,
    pub models: Vec<PackModel>,
    pub kinds: Vec<KindRow>,
    /// About what the zip weighs.
    pub estimate_bytes: f64,
    /// Models whose clips stay out.
    pub mixamo: u32,
    /// Models left out, each with why.
    pub skipped: Vec<String>,
}

/// `cast_iron_stove` from "Cast iron stove", once in `taken`.
fn key_for(name: &str, taken: &mut BTreeSet<String>) -> String {
    let stem = if name.chars().any(char::is_alphanumeric) {
        project::slugify(name).replace('-', "_")
    } else {
        "model".to_string()
    };
    let mut key = stem.clone();
    let mut n = 2;
    while !taken.insert(key.clone()) {
        key = format!("{stem}_{n}");
        n += 1;
    }
    key
}

fn height_of(prep: Option<&ModelPrep>) -> Option<f32> {
    prep.and_then(|p| p.size)
        .filter(|s| s.axis == SizeAxis::Height)
        .map(|s| s.metres())
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

struct Draft {
    name: String,
    kind: AssetKind,
    entry: Option<String>,
    project: Option<String>,
    asset: Option<String>,
    version: u32,
    file: PathBuf,
    stats: Option<ModelStats>,
    source: Option<LibrarySource>,
    baked_mixamo: bool,
    thumbnail: Option<PathBuf>,
    refs: Vec<PathBuf>,
    prep: Option<ModelPrep>,
}

fn model(draft: Draft, taken: &mut BTreeSet<String>) -> PackModel {
    let read = licence::rig_of(&draft.file);
    let joints = draft.stats.as_ref().map_or(0, |s| s.joints);
    let rigged = joints > 0 || read.is_some_and(|(rig, _)| rig.is_some());
    let clips = read.map_or_else(|| draft.stats.as_ref().map_or(0, |s| s.animations), |r| r.1);
    let mixamo = clips > 0
        && licence::may_be_mixamo(draft.source, read.and_then(|r| r.0), draft.baked_mixamo);
    PackModel {
        key: key_for(&draft.name, taken),
        height_m: height_of(draft.prep.as_ref()),
        licence: licence::note(draft.source),
        file: path_string(&draft.file),
        thumbnail: draft
            .thumbnail
            .filter(|t| t.is_file())
            .map(|t| path_string(&t)),
        refs: draft
            .refs
            .iter()
            .filter(|r| r.is_file())
            .map(|r| path_string(r))
            .collect(),
        name: draft.name,
        kind: draft.kind,
        entry: draft.entry,
        project: draft.project,
        asset: draft.asset,
        version: draft.version,
        stats: draft.stats,
        rigged,
        mixamo,
        source: draft.source,
        prep: draft.prep,
    }
}

/// The draft of a library entry's newest version, or why it has none.
fn from_entry(root: &Path, layout: &FolderLayout, entry: &LibraryEntry) -> Result<Draft, String> {
    let latest = entry
        .latest()
        .ok_or_else(|| format!("{} has no model.", entry.name))?;
    let folder = library::entry_dir(root, &entry.id).map_err(|e| e.to_string())?;
    let file = folder.join(&latest.file);
    if !file.is_file() {
        return Err(format!("{}: its model file is missing.", entry.name));
    }
    let from = &latest.from;
    let baked_mixamo = project::dir(root, layout, &from.project)
        .map(|dir| licence::baked_mixamo(root, &dir, &from.asset))
        .unwrap_or(false);
    Ok(Draft {
        name: entry.name.clone(),
        kind: entry.kind,
        entry: Some(entry.id.clone()),
        project: None,
        asset: None,
        version: latest.version,
        stats: latest.stats.clone(),
        source: Some(latest.source),
        baked_mixamo,
        thumbnail: latest.thumbnail.as_ref().map(|t| folder.join(t)),
        refs: entry.refs.iter().map(|r| folder.join(r)).collect(),
        prep: latest.prep.clone(),
        file,
    })
}

fn entries_of(root: &Path, source: &PackSource) -> CoreResult<Vec<String>> {
    match source {
        PackSource::Entries { entries } => Ok(entries.clone()),
        PackSource::Collection { collection } => library::collections(root)?
            .into_iter()
            .find(|c| &c.id == collection)
            .map(|c| c.entries)
            .ok_or_else(|| CoreError::NotFound(format!("the collection {collection}"))),
        PackSource::Project { .. } => Ok(Vec::new()),
    }
}

fn project_drafts(
    root: &Path,
    layout: &FolderLayout,
    id: &str,
    source_of: &SourceOf<'_>,
) -> CoreResult<(String, Vec<Result<Draft, String>>)> {
    let manifest = project::load(root, layout, id)?;
    let dir = project::dir(root, layout, id)?;
    let prep = crate::prepare::load(&dir)?;
    let drafts = manifest
        .assets
        .iter()
        .filter_map(|asset| asset.latest().map(|v| (asset, v)))
        .map(|(asset, latest)| {
            let file = dir.join(&latest.file);
            if !file.is_file() {
                return Err(format!("{}: its model file is missing.", asset.name));
            }
            let (source, _) = source_of(&dir, asset, latest.version);
            Ok(Draft {
                name: asset.name.clone(),
                kind: asset.kind,
                entry: None,
                project: Some(id.to_string()),
                asset: Some(asset.id.clone()),
                version: latest.version,
                stats: latest.stats.clone(),
                source: Some(source),
                baked_mixamo: licence::baked_mixamo(root, &dir, &asset.id),
                thumbnail: asset.thumbnail.as_ref().map(|t| dir.join(t)),
                refs: asset.refs.iter().map(|r| dir.join(&r.file)).collect(),
                prep: prep.models.get(&asset.id).cloned(),
                file,
            })
        })
        .collect();
    Ok((manifest.name, drafts))
}

fn kinds(models: &[PackModel], settings: &PackSettings) -> Vec<KindRow> {
    [AssetKind::Prop, AssetKind::Character, AssetKind::Scene]
        .into_iter()
        .filter_map(|kind| {
            let of: Vec<&PackModel> = models.iter().filter(|m| m.kind == kind).collect();
            (!of.is_empty()).then(|| KindRow {
                kind,
                label: kind_label(kind).to_string(),
                count: u32::try_from(of.len()).unwrap_or(u32::MAX),
                biggest: of.iter().map(|m| m.triangles()).max().unwrap_or(0),
                budget: settings.budget(kind),
            })
        })
        .collect()
}

/// About what one model weighs in the zip.
pub(super) fn estimate(model: &PackModel, settings: &PackSettings) -> f64 {
    let triangles = match settings.budget(model.kind) {
        Some(budget) if !model.rigged => model.triangles().min(budget),
        _ => model.triangles(),
    };
    let per_triangle = match settings.format {
        super::PackFormat::Obj => OBJ_TRIANGLE_BYTES,
        super::PackFormat::Glb => GLB_TRIANGLE_BYTES,
        super::PackFormat::Both => OBJ_TRIANGLE_BYTES + GLB_TRIANGLE_BYTES,
    };
    let (count, edge) = model
        .stats
        .as_ref()
        .map_or((1, settings.texture), |s| (s.textures, s.max_texture));
    let edge = if settings.texture == 0 {
        edge
    } else {
        edge.min(settings.texture)
    };
    f64::from(triangles) * per_triangle + f64::from(count) * f64::from(edge).powi(2) * PIXEL_BYTES
}

/// What `source` would put in a pack made with `settings`.
///
/// # Errors
///
/// Fails for an unknown collection or project, and for a pack of no
/// models or of more than [`MAX_MODELS`].
pub fn plan(
    root: &Path,
    layout: &FolderLayout,
    source: &PackSource,
    settings: &PackSettings,
    source_of: &SourceOf<'_>,
) -> CoreResult<PackPlan> {
    let (label, drafts) = match source {
        PackSource::Project { project } => {
            let (name, drafts) = project_drafts(root, layout, project, source_of)?;
            (format!("of {name}"), drafts)
        }
        _ => {
            let ids = entries_of(root, source)?;
            let drafts = ids
                .iter()
                .map(|id| match library::load(root, id) {
                    Ok(entry) => from_entry(root, layout, &entry),
                    Err(err) => Err(format!("{id}: {err}")),
                })
                .collect();
            ("from the library".to_string(), drafts)
        }
    };
    let mut taken = BTreeSet::new();
    let mut models = Vec::new();
    let mut skipped = Vec::new();
    for draft in drafts {
        match draft {
            Ok(draft) => models.push(model(draft, &mut taken)),
            Err(why) => skipped.push(why),
        }
    }
    if models.is_empty() {
        return Err(CoreError::Invalid(
            "There is no model to pack. Pick models with a model file.".to_string(),
        ));
    }
    if models.len() > MAX_MODELS {
        return Err(CoreError::Invalid(format!(
            "A pack takes up to {MAX_MODELS} models. Pick fewer."
        )));
    }
    let n = models.len();
    Ok(PackPlan {
        label: format!("{n} model{} {label}", if n == 1 { "" } else { "s" }),
        kinds: kinds(&models, settings),
        estimate_bytes: models.iter().map(|m| estimate(m, settings)).sum(),
        mixamo: u32::try_from(models.iter().filter(|m| m.mixamo).count()).unwrap_or(u32::MAX),
        models,
        skipped,
    })
}
