//! The asset registry: every model, prop and scene a project tracks, with the
//! stage it sits in on the Board and its versions. Lives in `project.json`
//! next to the project manifest.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::project::{ProjectManifest, now_rfc3339, slugify};
use crate::ship::ShipSettings;

pub mod copy;
pub mod remove;

pub const ASSET_NAME_MAX_CHARS: usize = 80;
/// The folder of a project the By hand sends keep their handoff folders in.
pub const HANDOFF_DIR: &str = "handoff";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum AssetKind {
    Prop,
    Character,
    Scene,
}

/// The Board columns, in pipeline order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    Concept,
    Refs,
    Tripo,
    Blender,
    Web,
    Ship,
}

impl Stage {
    pub const ALL: [Stage; 6] = [
        Stage::Concept,
        Stage::Refs,
        Stage::Tripo,
        Stage::Blender,
        Stage::Web,
        Stage::Ship,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Stage::Concept => "Concept",
            Stage::Refs => "Refs",
            Stage::Tripo => "3D model",
            Stage::Blender => "Blender",
            Stage::Web => "Web",
            Stage::Ship => "Ship",
        }
    }
}

/// Mirrors the kit's status chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum AssetStatus {
    Done,
    Cached,
    Running,
    Failed,
    Review,
    Queued,
}

/// The numbers a card shows, taken from the model analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelStats {
    pub triangles: u32,
    /// Bytes as a JS safe number.
    pub bytes: f64,
    pub textures: u32,
    /// The longest texture edge in pixels, 0 without textures.
    pub max_texture: u32,
    pub joints: u32,
    pub animations: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetVersion {
    pub version: u32,
    /// Relative to the project folder, with `/` separators.
    pub file: String,
    pub created_at: String,
    pub stats: Option<ModelStats>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    /// Unique within the project, derived from the name.
    pub id: String,
    pub name: String,
    pub kind: AssetKind,
    pub stage: Stage,
    pub status: AssetStatus,
    /// Oldest first.
    pub versions: Vec<AssetVersion>,
    /// Relative to the project folder, with `/` separators.
    pub thumbnail: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// The library entry and version this asset was copied from, when it
    /// came from the library. The project owns its copy either way; the
    /// origin only says where to look for an update. Older files have none.
    #[serde(default)]
    pub origin: Option<AssetOrigin>,
    /// Its refs and the view each shows, once someone tagged them. Empty
    /// for an asset nobody tagged: its views follow its refs' order then
    /// (see [`crate::refs`]).
    #[serde(default)]
    pub refs: Vec<crate::refs::AssetRef>,
}

/// Where a Board asset came from in the library: the entry id and the
/// library version its newest model is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetOrigin {
    pub entry: String,
    pub version: u32,
}

/// The asset an exported model comes from, by its GLB file stem: the asset
/// with a version file of that stem (`clay_v1` is the clay's v1), with the
/// version. The pipeline names an export after the model file it built, so
/// this is how "clay_v1.glb" reads as "clay" in the app. `None` for a model
/// no asset version made.
pub fn for_model_stem<'a>(
    manifest: &'a crate::ProjectManifest,
    stem: &str,
) -> Option<(&'a Asset, u32)> {
    let stem_of = |file: &str| {
        std::path::Path::new(file)
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
    };
    let wanted = stem.to_lowercase();
    manifest.assets.iter().find_map(|asset| {
        asset
            .versions
            .iter()
            .rev()
            .find(|v| stem_of(&v.file).as_deref() == Some(wanted.as_str()))
            .map(|v| (asset, v.version))
    })
}

fn model_stem(file: &str) -> Option<String> {
    std::path::Path::new(file)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
}

/// What the compressed models ship as: the model file stem to the stem it
/// ships under. For a scene ([`ShipSettings::stable_names`] false) that is
/// the asset's current name plus the version (`lantern_v1` of an asset now
/// called "Paper lantern" ships as `paper_lantern_v1`). With stable names
/// the newest version ships as the asset id in snake case (`paper_lantern`),
/// so a new version keeps the file the site loads. A file name chosen for
/// the asset (`ship.json`) is used for its newest version either way. Only
/// versions whose shipped stem differs from their file stem appear, so a
/// project nobody renamed ships exactly as before. The version files, the
/// asset id and the pipeline's raw exports keep their names: cache keys,
/// the presets and the inbox matches are built on them. A name that would
/// land on another model's stem keeps the file stem.
pub fn shipped_names(manifest: &ProjectManifest, ship: &ShipSettings) -> BTreeMap<String, String> {
    let versions: Vec<(&Asset, &AssetVersion, String)> = manifest
        .assets
        .iter()
        .flat_map(|a| a.versions.iter().map(move |v| (a, v)))
        .filter_map(|(a, v)| model_stem(&v.file).map(|stem| (a, v, stem)))
        .collect();
    let mut taken: BTreeSet<String> = versions.iter().map(|(_, _, stem)| stem.clone()).collect();
    let mut out = BTreeMap::new();
    // Chosen and stable names first, so a versioned name never takes one.
    let newest = |asset: &Asset, version: &AssetVersion| {
        asset.latest().is_some_and(|l| l.version == version.version)
    };
    let fixed = |asset: &Asset, version: &AssetVersion| -> Option<String> {
        if !newest(asset, version) {
            return None;
        }
        match ship.file(&asset.id) {
            Some(file) => Some(file.to_string()),
            None if ship.stable_names() => Some(asset.id.replace('-', "_")),
            None => None,
        }
    };
    for (asset, version, stem) in &versions {
        if let Some(shipped) = fixed(asset, version)
            && &shipped != stem
            && taken.insert(shipped.clone())
        {
            out.insert(stem.clone(), shipped);
        }
    }
    for (asset, version, stem) in &versions {
        if out.contains_key(stem) || fixed(asset, version).is_some() {
            continue;
        }
        // The same text the asset id and its version files are made of.
        let base = slugify(&asset.name).replace('-', "_");
        let shipped = format!("{base}_v{}", version.version);
        if &shipped != stem && taken.insert(shipped.clone()) {
            out.insert(stem.clone(), shipped);
        }
    }
    out
}

/// The asset and version behind a shipped file stem, with the stem of the
/// model file it was built from: the reverse of [`shipped_names`].
pub fn for_shipped_stem<'a>(
    manifest: &'a ProjectManifest,
    ship: &ShipSettings,
    stem: &str,
) -> Option<(&'a Asset, u32, String)> {
    let wanted = stem.to_lowercase();
    let (model, _) = shipped_names(manifest, ship)
        .into_iter()
        .find(|(_, shipped)| *shipped == wanted)?;
    for_model_stem(manifest, &model).map(|(asset, version)| (asset, version, model))
}

impl Asset {
    pub fn latest(&self) -> Option<&AssetVersion> {
        self.versions.last()
    }

    pub fn next_version(&self) -> u32 {
        self.versions.iter().map(|v| v.version).max().unwrap_or(0) + 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewAsset {
    pub name: String,
    pub kind: AssetKind,
    pub stage: Stage,
}

pub(crate) fn clean_name(name: &str) -> CoreResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("Give the asset a name.".to_string()));
    }
    if name.chars().count() > ASSET_NAME_MAX_CHARS {
        return Err(CoreError::Invalid(format!(
            "Keep the asset name to {ASSET_NAME_MAX_CHARS} characters or fewer."
        )));
    }
    Ok(name.to_string())
}

fn unique_id(manifest: &ProjectManifest, name: &str) -> String {
    let base = slugify(name);
    let taken = |id: &str| manifest.assets.iter().any(|a| a.id == id);
    if !taken(&base) {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !taken(id))
        .unwrap_or(base)
}

pub fn find<'a>(manifest: &'a ProjectManifest, id: &str) -> CoreResult<&'a Asset> {
    manifest
        .assets
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| CoreError::NotFound(format!("the asset {id}")))
}

pub(crate) fn find_mut<'a>(
    manifest: &'a mut ProjectManifest,
    id: &str,
) -> CoreResult<&'a mut Asset> {
    manifest
        .assets
        .iter_mut()
        .find(|a| a.id == id)
        .ok_or_else(|| CoreError::NotFound(format!("the asset {id}")))
}

/// Adds an asset with no versions yet. A new asset waits for work, so it
/// starts as queued.
pub fn add(manifest: &mut ProjectManifest, new: NewAsset) -> CoreResult<Asset> {
    let name = clean_name(&new.name)?;
    let now = now_rfc3339();
    let asset = Asset {
        id: unique_id(manifest, &name),
        name,
        kind: new.kind,
        stage: new.stage,
        status: AssetStatus::Queued,
        versions: Vec::new(),
        thumbnail: None,
        created_at: now.clone(),
        updated_at: now,
        origin: None,
        refs: Vec::new(),
    };
    manifest.assets.push(asset.clone());
    Ok(asset)
}

pub fn move_to(manifest: &mut ProjectManifest, id: &str, stage: Stage) -> CoreResult<Asset> {
    let asset = find_mut(manifest, id)?;
    if asset.stage != stage {
        asset.stage = stage;
        asset.updated_at = now_rfc3339();
    }
    Ok(asset.clone())
}

/// Changes the display name. The id, the files and the version records stay
/// as they are, so refs, inbox matches and pipeline outputs keep working.
/// A name another asset already answers to is refused, since a model accepted
/// under that name would not know which asset it belongs to.
pub fn rename(manifest: &mut ProjectManifest, id: &str, name: &str) -> CoreResult<Asset> {
    let name = clean_name(name)?;
    if find_by_name(manifest, &name).is_some_and(|a| a.id != id) {
        return Err(CoreError::Invalid(format!(
            "Another asset is already called {name}."
        )));
    }
    let asset = find_mut(manifest, id)?;
    if asset.name != name {
        asset.name = name;
        asset.updated_at = now_rfc3339();
    }
    Ok(asset.clone())
}

pub fn set_status(manifest: &mut ProjectManifest, id: &str, status: AssetStatus) -> CoreResult<()> {
    let asset = find_mut(manifest, id)?;
    asset.status = status;
    asset.updated_at = now_rfc3339();
    Ok(())
}

/// The status an asset shows when no job drives it: Review once it has a
/// model (what a fresh accept sets), else Queued like a new refs-only card.
pub fn resting_status(asset: &Asset) -> AssetStatus {
    if asset.versions.is_empty() {
        AssetStatus::Queued
    } else {
        AssetStatus::Review
    }
}

/// The asset whose id or current name slugs to the same text, so a second
/// Accept of "Sakura tree" becomes v2 of the same asset, also after the
/// asset was renamed (its id keeps the first name).
pub fn find_by_name<'a>(manifest: &'a ProjectManifest, name: &str) -> Option<&'a Asset> {
    let slug = slugify(name.trim());
    manifest
        .assets
        .iter()
        .find(|a| a.id == slug)
        .or_else(|| manifest.assets.iter().find(|a| slugify(&a.name) == slug))
}

/// Points the asset at `thumbnail` (relative to the project folder) when
/// `version` is still its latest, so a slow render of an older version never
/// replaces a newer one. Returns whether it changed.
pub fn set_thumbnail(
    manifest: &mut ProjectManifest,
    id: &str,
    version: u32,
    thumbnail: String,
) -> CoreResult<bool> {
    let asset = find_mut(manifest, id)?;
    if asset.latest().map(|v| v.version) != Some(version)
        || asset.thumbnail.as_deref() == Some(thumbnail.as_str())
    {
        return Ok(false);
    }
    asset.thumbnail = Some(thumbnail);
    Ok(true)
}

/// True when a ref image may stand in as the card's picture: there is no
/// thumbnail yet, or it is an older ref. A model render or a poster stays.
pub fn ref_may_replace(thumbnail: Option<&str>) -> bool {
    thumbnail.is_none_or(|t| t.starts_with("refs/"))
}

/// Shows the ref image `file` (relative to the project folder) on the
/// asset's card unless it already shows a render or a poster. Returns
/// whether it changed.
pub fn set_ref_thumbnail(manifest: &mut ProjectManifest, id: &str, file: &str) -> CoreResult<bool> {
    let asset = find_mut(manifest, id)?;
    if !ref_may_replace(asset.thumbnail.as_deref()) || asset.thumbnail.as_deref() == Some(file) {
        return Ok(false);
    }
    asset.thumbnail = Some(file.to_string());
    asset.updated_at = now_rfc3339();
    Ok(true)
}

/// Takes in an asset from another project. An asset with the same name
/// that is already here keeps its id and versions, moves up to the later
/// stage and gains the newcomer's picture when a ref may replace its own.
/// Otherwise the asset is added under an id that is free here.
pub fn adopt(manifest: &mut ProjectManifest, asset: Asset) -> Asset {
    let existing = find_by_name(manifest, &asset.name).map(|a| a.id.clone());
    if let Some(existing) = existing.and_then(|id| find_mut(manifest, &id).ok()) {
        if asset.stage > existing.stage {
            existing.stage = asset.stage;
        }
        if asset.thumbnail.is_some() && ref_may_replace(existing.thumbnail.as_deref()) {
            existing.thumbnail = asset.thumbnail;
        }
        existing.updated_at = now_rfc3339();
        return existing.clone();
    }
    let placed = Asset {
        id: unique_id(manifest, &asset.name),
        updated_at: now_rfc3339(),
        ..asset
    };
    manifest.assets.push(placed.clone());
    placed
}

/// Records a new version on an existing asset and returns it.
pub fn add_version(
    manifest: &mut ProjectManifest,
    id: &str,
    file: String,
    stats: Option<ModelStats>,
) -> CoreResult<AssetVersion> {
    let asset = find_mut(manifest, id)?;
    let version = AssetVersion {
        version: asset.next_version(),
        file,
        created_at: now_rfc3339(),
        stats,
    };
    asset.versions.push(version.clone());
    asset.updated_at = version.created_at.clone();
    Ok(version)
}

#[cfg(test)]
mod tests;

/// Registers this module's IPC types (an asset, its versions and its stage) with the shell's
/// TypeScript export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<Asset>()
        .register::<AssetKind>()
        .register::<AssetStatus>()
        .register::<AssetVersion>()
        .register::<AssetOrigin>()
        .register::<ModelStats>()
        .register::<NewAsset>()
        .register::<Stage>()
}
