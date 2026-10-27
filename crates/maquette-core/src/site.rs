//! The site a project ships to: where the site keeps what Ship writes, the
//! index of shipped models the Ship step reads, and the site's preview
//! server ([`preview`]).
//!
//! A site opts in to Ship's data with `maquette.site.json` at its root:
//!
//! ```json
//! {
//!   "version": 1,
//!   "models": "public/scene/models",
//!   "posters": null,
//!   "data": "src/scene/shipped.json",
//!   "preview": { "command": ["pnpm", "dev"], "url": "http://localhost:15292/" }
//! }
//! ```
//!
//! Every folder is relative to the site and stays inside it. `posters: null`
//! leaves the posters out; without `data` Ship only copies files. A site
//! without the file gets the folders Ship always used: models in
//! `public/models`, posters in `public/posters`, no data. The Ship starter
//! (`tools/ship.mjs`) reads the same file, so the two stay in step.

pub mod preview;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::assets::shipped_names;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::ProjectManifest;
use crate::ship::{ShipChoice, ShipSettings};

pub const CONFIG_FILE: &str = "maquette.site.json";
pub const CONFIG_VERSION: u32 = 1;
/// Where Ship copies the models of a site without a config.
pub const LEGACY_MODELS: &str = "public/models";
pub const LEGACY_POSTERS: &str = "public/posters";
/// The index of shipped models, in the project's pipeline cache folder.
pub const INDEX_FILE: &str = "ship-index.json";
pub const INDEX_VERSION: u32 = 1;

/// How the site's own preview server starts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewConfig {
    /// A program on PATH and its args, run in the site folder. Empty uses
    /// the site's package manager: `<pnpm|bun|yarn|npm> run dev`.
    #[serde(default)]
    pub command: Vec<String>,
    /// The address the server answers on. Without it Maquette reads it
    /// from the server's output.
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteConfig {
    pub version: u32,
    /// The folder the models are copied into.
    #[serde(default = "legacy_models")]
    pub models: String,
    /// The folder the posters are copied into; `None` copies none.
    #[serde(default)]
    pub posters: Option<String>,
    /// The file Ship writes the site's data to; `None` writes none.
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub preview: Option<PreviewConfig>,
}

fn legacy_models() -> String {
    LEGACY_MODELS.to_string()
}

impl SiteConfig {
    /// What a site without `maquette.site.json` gets.
    pub fn legacy() -> Self {
        Self {
            version: CONFIG_VERSION,
            models: legacy_models(),
            posters: Some(LEGACY_POSTERS.to_string()),
            data: None,
            preview: None,
        }
    }
}

/// True for a relative path that cannot leave the folder it is joined to.
fn stays_inside(path: &str) -> bool {
    let p = Path::new(path);
    !path.trim().is_empty()
        && !path.starts_with(['/', '\\'])
        && !p.is_absolute()
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

/// Checks the version and that every folder stays inside the site.
///
/// # Errors
///
/// Returns the first problem as a sentence.
pub fn validate(config: &SiteConfig) -> CoreResult<()> {
    if config.version > CONFIG_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: config.version,
            supported: CONFIG_VERSION,
        });
    }
    let folders = [
        ("models", Some(&config.models)),
        ("posters", config.posters.as_ref()),
        ("data", config.data.as_ref()),
    ];
    for (what, path) in folders {
        if let Some(path) = path
            && !stays_inside(path)
        {
            return Err(invalid(format!(
                "{CONFIG_FILE}: {what} {path:?} must be a path inside the site, without .."
            )));
        }
    }
    Ok(())
}

/// The site's `maquette.site.json`, or [`SiteConfig::legacy`] without one.
///
/// # Errors
///
/// Fails when the file exists but cannot be read or [`validate`] refuses it.
pub fn load_config(site: &Path) -> CoreResult<SiteConfig> {
    let file = site.join(CONFIG_FILE);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(SiteConfig::legacy()),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let config: SiteConfig = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    validate(&config)?;
    Ok(config)
}

/// Where Ship writes in a site, as the app and agents set it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SiteTarget {
    /// Relative to the site, for example `apps/landing/public/room/models`.
    pub models: String,
    /// `None` copies no posters.
    pub posters: Option<String>,
    /// `None` writes no scene data.
    pub data: Option<String>,
}

/// Writes `target` into the site's `maquette.site.json`, creating it when
/// the site has none. Every other key the file has (`preview`, anything a
/// person added) is kept as it is.
///
/// # Errors
///
/// Fails when a folder leaves the site, or the file cannot be read or
/// written.
pub fn save_target(site: &Path, target: &SiteTarget) -> CoreResult<SiteConfig> {
    let clean = |p: &str| p.trim().trim_matches('/').replace('\\', "/");
    let models = clean(&target.models);
    let posters = target
        .posters
        .as_deref()
        .map(clean)
        .filter(|p| !p.is_empty());
    let data = target.data.as_deref().map(clean).filter(|p| !p.is_empty());
    let file = site.join(CONFIG_FILE);
    let mut value = match fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str::<serde_json::Value>(&text)
            .map_err(|e| CoreError::json(&file, e))?,
        Err(err) if err.kind() == io::ErrorKind::NotFound => serde_json::json!({}),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let Some(map) = value.as_object_mut() else {
        return Err(invalid(format!("{CONFIG_FILE} is not a JSON object.")));
    };
    map.entry("version")
        .or_insert(serde_json::json!(CONFIG_VERSION));
    map.insert("models".into(), serde_json::json!(models));
    map.insert("posters".into(), serde_json::json!(posters));
    map.insert("data".into(), serde_json::json!(data));
    let config: SiteConfig =
        serde_json::from_value(value.clone()).map_err(|e| CoreError::json(&file, e))?;
    validate(&config)?;
    write_json(&file, &value)?;
    Ok(config)
}

/// The folder the site keeps its shipped models in. An unreadable config
/// falls back to the legacy folder, so the Inspector still shows something.
pub fn models_dir(site: &Path) -> PathBuf {
    let models = load_config(site).map_or_else(
        |err| {
            tracing::warn!(error = %err, "unreadable site config");
            LEGACY_MODELS.to_string()
        },
        |c| c.models,
    );
    site.join(models)
}

/// One shipped model, as the Ship step finds it in the compressed folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub name: String,
    /// The compressed file name, for example `desk_lamp_v1.glb`.
    pub file: String,
    pub version: u32,
    /// Whether Ship writes it ([`ShipChoice`]); left out for `auto`.
    #[serde(default, skip_serializing_if = "is_auto")]
    pub choice: ShipChoice,
}

fn is_auto(choice: &ShipChoice) -> bool {
    *choice == ShipChoice::Auto
}

/// Asset id to its newest model as shipped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipIndex {
    pub version: u32,
    pub project: String,
    pub models: BTreeMap<String, IndexEntry>,
    /// `own` when the models are exported at their own origin, so the scene
    /// data says they are not in place.
    #[serde(default = "scene_origin")]
    pub origin: String,
    /// True when Ship keeps a site file that is smaller than the new one.
    #[serde(default)]
    pub size_guard: bool,
}

fn scene_origin() -> String {
    "scene".to_string()
}

/// The newest model of every asset that has one, under the name the
/// Compress step ships it as (see [`shipped_names`]).
pub fn ship_index(manifest: &ProjectManifest, ship: &ShipSettings) -> ShipIndex {
    let renamed = shipped_names(manifest, ship);
    let models = crate::layout::placeable(manifest)
        .into_iter()
        .filter_map(|asset| {
            let latest = asset.latest()?;
            let stem = Path::new(&latest.file)
                .file_stem()?
                .to_string_lossy()
                .to_lowercase();
            let shipped = renamed.get(&stem).cloned().unwrap_or(stem);
            let entry = IndexEntry {
                name: asset.name.clone(),
                file: format!("{shipped}.glb"),
                version: latest.version,
                choice: ship.choice(&asset.id),
            };
            Some((asset.id.clone(), entry))
        })
        .collect();
    ShipIndex {
        version: INDEX_VERSION,
        project: manifest.id.clone(),
        models,
        origin: ship.origin_word().to_string(),
        size_guard: ship.size_guard(),
    }
}

/// Writes [`ship_index`] into `cache_dir` and returns its path, for the
/// `{shipIndex}` placeholder.
///
/// # Errors
///
/// Fails when the file cannot be written.
pub fn write_ship_index(
    cache_dir: &Path,
    manifest: &ProjectManifest,
    ship: &ShipSettings,
) -> CoreResult<PathBuf> {
    fs::create_dir_all(cache_dir).map_err(|e| CoreError::io(cache_dir, e))?;
    let path = cache_dir.join(INDEX_FILE);
    write_json(&path, &ship_index(manifest, ship))?;
    Ok(path)
}

#[cfg(test)]
mod ship_script;
#[cfg(test)]
mod tests;
