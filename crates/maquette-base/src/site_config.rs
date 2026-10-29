//! A site's `maquette.site.json`: where Ship writes the models, posters
//! and scene data in the site, and how the site's own preview server
//! starts. Writing it and the index of shipped models are maquette-core's
//! `site` module.

use std::fs;
use std::io;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

pub const CONFIG_FILE: &str = "maquette.site.json";
pub const CONFIG_VERSION: u32 = 1;
/// Where Ship copies the models of a site without a config.
pub const LEGACY_MODELS: &str = "public/models";
pub const LEGACY_POSTERS: &str = "public/posters";

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
