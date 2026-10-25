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
}

/// Asset id to its newest model as shipped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipIndex {
    pub version: u32,
    pub project: String,
    pub models: BTreeMap<String, IndexEntry>,
}

/// The newest model of every asset that has one, under the name the
/// Compress step ships it as (see [`shipped_names`]).
pub fn ship_index(manifest: &ProjectManifest) -> ShipIndex {
    let renamed = shipped_names(manifest);
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
            };
            Some((asset.id.clone(), entry))
        })
        .collect();
    ShipIndex {
        version: INDEX_VERSION,
        project: manifest.id.clone(),
        models,
    }
}

/// Writes [`ship_index`] into `cache_dir` and returns its path, for the
/// `{shipIndex}` placeholder.
///
/// # Errors
///
/// Fails when the file cannot be written.
pub fn write_ship_index(cache_dir: &Path, manifest: &ProjectManifest) -> CoreResult<PathBuf> {
    fs::create_dir_all(cache_dir).map_err(|e| CoreError::io(cache_dir, e))?;
    let path = cache_dir.join(INDEX_FILE);
    write_json(&path, &ship_index(manifest))?;
    Ok(path)
}

#[cfg(test)]
mod ship_script;
#[cfg(test)]
mod tests;
