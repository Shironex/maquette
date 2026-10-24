//! The asset registry: every model, prop and scene a project tracks, with the
//! stage it sits in on the Board and its versions. Lives in `project.json`
//! next to the project manifest.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::project::{ProjectManifest, now_rfc3339, slugify};

pub const ASSET_NAME_MAX_CHARS: usize = 80;

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
            Stage::Tripo => "Tripo",
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

fn clean_name(name: &str) -> CoreResult<String> {
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

fn find_mut<'a>(manifest: &'a mut ProjectManifest, id: &str) -> CoreResult<&'a mut Asset> {
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

pub fn set_status(manifest: &mut ProjectManifest, id: &str, status: AssetStatus) -> CoreResult<()> {
    let asset = find_mut(manifest, id)?;
    asset.status = status;
    asset.updated_at = now_rfc3339();
    Ok(())
}

/// The asset whose name slugs to the same id, so a second Accept of
/// "Sakura tree" becomes v2 of the same asset.
pub fn find_by_name<'a>(manifest: &'a ProjectManifest, name: &str) -> Option<&'a Asset> {
    let slug = slugify(name.trim());
    manifest.assets.iter().find(|a| a.id == slug)
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
    let slug = slugify(asset.name.trim());
    if let Some(existing) = manifest.assets.iter_mut().find(|a| a.id == slug) {
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
mod tests {
    use super::*;
    use crate::project::{PROJECT_VERSION, ProjectTemplate};

    fn manifest() -> ProjectManifest {
        ProjectManifest {
            version: PROJECT_VERSION,
            id: "moon-island".into(),
            name: "Moon Island".into(),
            template: ProjectTemplate::FloatingDiorama,
            site_repo: None,
            source_folder: None,
            created_at: "2026-10-24T09:30:00.000Z".into(),
            assets: Vec::new(),
        }
    }

    fn new_asset(name: &str) -> NewAsset {
        NewAsset {
            name: name.into(),
            kind: AssetKind::Prop,
            stage: Stage::Tripo,
        }
    }

    #[test]
    fn add_gives_unique_ids_and_rejects_bad_names() {
        let mut m = manifest();
        let a = add(&mut m, new_asset(" Sakura tree ")).expect("a");
        let b = add(&mut m, new_asset("Sakura Tree")).expect("b");
        assert_eq!(a.id, "sakura-tree");
        assert_eq!(a.name, "Sakura tree");
        assert_eq!(b.id, "sakura-tree-2");
        assert_eq!(a.status, AssetStatus::Queued);
        assert!(add(&mut m, new_asset("  ")).is_err());
        assert!(add(&mut m, new_asset(&"x".repeat(81))).is_err());
        assert_eq!(m.assets.len(), 2);
    }

    #[test]
    fn move_and_versions() {
        let mut m = manifest();
        let a = add(&mut m, new_asset("Vending machine")).expect("add");
        let moved = move_to(&mut m, &a.id, Stage::Web).expect("move");
        assert_eq!(moved.stage, Stage::Web);
        assert_eq!(
            move_to(&mut m, "nope", Stage::Web)
                .expect_err("missing")
                .code(),
            "not-found"
        );
        let v1 =
            add_version(&mut m, &a.id, "models/vending_machine_v1.glb".into(), None).expect("v1");
        let v2 =
            add_version(&mut m, &a.id, "models/vending_machine_v2.glb".into(), None).expect("v2");
        assert_eq!((v1.version, v2.version), (1, 2));
        assert_eq!(find(&m, &a.id).expect("find").latest(), Some(&v2));
        assert_eq!(
            find_by_name(&m, "vending MACHINE").map(|a| a.id.as_str()),
            Some("vending-machine")
        );
        set_status(&mut m, &a.id, AssetStatus::Done).expect("status");
        assert_eq!(find(&m, &a.id).expect("find").status, AssetStatus::Done);
    }

    #[test]
    fn thumbnails_follow_the_latest_version_only() {
        let mut m = manifest();
        let a = add(&mut m, new_asset("Crate")).expect("add");
        add_version(&mut m, &a.id, "models/crate_v1.glb".into(), None).expect("v1");
        add_version(&mut m, &a.id, "models/crate_v2.glb".into(), None).expect("v2");
        assert!(!set_thumbnail(&mut m, &a.id, 1, "thumbs/crate_v1.png".into()).expect("old"));
        assert_eq!(find(&m, &a.id).expect("find").thumbnail, None);
        assert!(set_thumbnail(&mut m, &a.id, 2, "thumbs/crate_v2.png".into()).expect("new"));
        assert!(!set_thumbnail(&mut m, &a.id, 2, "thumbs/crate_v2.png".into()).expect("same"));
        assert_eq!(
            find(&m, &a.id).expect("find").thumbnail.as_deref(),
            Some("thumbs/crate_v2.png")
        );
    }

    #[test]
    fn json_shape() {
        assert_eq!(
            serde_json::to_string(&Stage::Blender).expect("json"),
            "\"blender\""
        );
        assert_eq!(
            serde_json::to_string(&AssetKind::Character).expect("json"),
            "\"character\""
        );
        assert!(Stage::Concept < Stage::Ship);
    }
}
