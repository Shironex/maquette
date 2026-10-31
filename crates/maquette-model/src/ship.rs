//! How a website project ships: `ship.json` next to `project.json`.
//!
//! ```json
//! {
//!   "version": 1,
//!   "style": "models",
//!   "textures": "webp",
//!   "quality": 80,
//!   "assets": {
//!     "desk": { "choice": "never" },
//!     "reading-armchair": { "file": "armchair" }
//!   }
//! }
//! ```
//!
//! There are two styles. `scene` is the scene Maquette builds: each model is
//! exported where the layout puts it, files are named after the asset and
//! its version (`desk_lamp_v2.glb`), and the site places them from the
//! scene data. `models` is for a hand-built site that loads one file per
//! model and places them in its own code: each model is exported at its own
//! origin, files keep one name for every version (`desk_lamp.glb`, or the
//! file name chosen for the asset), and Ship never replaces a site file
//! with a larger one unless the person chose to. A project without the file
//! ships as a scene with KTX2 textures, as every project did before.
//!
//! `textures` is the format Compress encodes every map in: `ktx2` (ETC1S or
//! UASTC from the presets, needs a KTX2 decoder on the site), `webp` or
//! `avif` (smaller to send, no decoder, more GPU memory). `quality` is the
//! WebP and AVIF quality, 1 to 100.
//!
//! Per asset, `choice` says whether Ship writes its model: `auto` (the
//! default) writes it unless the site's file was not made by Maquette (or,
//! for the models style, is smaller), `never` leaves the site's file alone
//! and `always` writes it over whatever is there. `file` is the stem the
//! model ships as, without `.glb`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;

pub const SHIP_FILE: &str = "ship.json";
pub const SHIP_VERSION: u32 = 1;
pub const DEFAULT_QUALITY: u8 = 80;
/// The longest file stem a person may choose.
pub const MAX_FILE_CHARS: usize = 64;

/// What the site does with the models.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ShipStyle {
    /// Maquette builds the scene: placed exports, versioned names.
    #[default]
    Scene,
    /// A hand-built site: one file per model, own origin, stable names.
    Models,
}

/// The format Compress encodes textures in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum TextureFormat {
    #[default]
    Ktx2,
    Webp,
    Avif,
}

impl TextureFormat {
    /// The gltf-transform command, and how the log names it.
    pub fn word(self) -> &'static str {
        match self {
            TextureFormat::Ktx2 => "ktx2",
            TextureFormat::Webp => "webp",
            TextureFormat::Avif => "avif",
        }
    }

    pub fn is_ktx2(self) -> bool {
        self == TextureFormat::Ktx2
    }
}

/// Whether Ship writes one asset's model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ShipChoice {
    /// Written unless the site's file is not Maquette's (or is smaller, for
    /// the models style).
    #[default]
    Auto,
    /// Never written; the site's file is left alone.
    Never,
    /// Written over whatever the site has.
    Always,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetShip {
    #[serde(default)]
    pub choice: ShipChoice,
    /// The stem the model ships as, without `.glb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

impl AssetShip {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShipSettings {
    pub version: u32,
    #[serde(default)]
    pub style: ShipStyle,
    #[serde(default)]
    pub textures: TextureFormat,
    #[serde(default = "default_quality")]
    pub quality: u8,
    /// Asset id to its choice and file name.
    #[serde(default)]
    pub assets: BTreeMap<String, AssetShip>,
}

fn default_quality() -> u8 {
    DEFAULT_QUALITY
}

impl Default for ShipSettings {
    fn default() -> Self {
        Self {
            version: SHIP_VERSION,
            style: ShipStyle::Scene,
            textures: TextureFormat::Ktx2,
            quality: DEFAULT_QUALITY,
            assets: BTreeMap::new(),
        }
    }
}

impl ShipSettings {
    /// True when every version ships under one file name.
    pub fn stable_names(&self) -> bool {
        self.style == ShipStyle::Models
    }

    /// True when each model is exported at its own origin.
    pub fn own_origin(&self) -> bool {
        self.style == ShipStyle::Models
    }

    /// True when Ship holds back a replacement larger than the site's file.
    pub fn size_guard(&self) -> bool {
        self.style == ShipStyle::Models
    }

    pub fn choice(&self, asset: &str) -> ShipChoice {
        self.assets.get(asset).map(|a| a.choice).unwrap_or_default()
    }

    /// The file stem chosen for `asset`, if any.
    pub fn file(&self, asset: &str) -> Option<&str> {
        self.assets.get(asset).and_then(|a| a.file.as_deref())
    }

    /// The `{exportOrigin}` value the Export GLB step gets.
    pub fn origin_word(&self) -> &'static str {
        if self.own_origin() { "own" } else { "scene" }
    }
}

/// A change from the app or an agent. Every field left out stays.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ShipSettingsChange {
    #[serde(default)]
    pub style: Option<ShipStyle>,
    #[serde(default)]
    pub textures: Option<TextureFormat>,
    #[serde(default)]
    pub quality: Option<u8>,
    /// Asset id to its new entry. An entry replaces the old one whole.
    #[serde(default)]
    pub assets: BTreeMap<String, AssetShip>,
}

pub fn path(project_dir: &Path) -> PathBuf {
    project_dir.join(SHIP_FILE)
}

/// True for a stem a site can load: lowercase letters, digits, `_` and `-`.
pub fn is_file_stem(stem: &str) -> bool {
    !stem.is_empty()
        && stem.chars().count() <= MAX_FILE_CHARS
        && !stem.starts_with(['-', '.'])
        && stem
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Checks the quality and every chosen file name.
///
/// # Errors
///
/// Returns the first problem as a sentence.
pub fn validate(settings: &ShipSettings) -> CoreResult<()> {
    let invalid = |m: String| Err(CoreError::Invalid(m));
    if settings.version > SHIP_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: settings.version,
            supported: SHIP_VERSION,
        });
    }
    if !(1..=100).contains(&settings.quality) {
        return invalid(format!(
            "The texture quality must be between 1 and 100, not {}.",
            settings.quality
        ));
    }
    let mut seen = BTreeSet::new();
    for (asset, entry) in &settings.assets {
        let Some(file) = &entry.file else { continue };
        if !is_file_stem(file) {
            return invalid(format!(
                "{asset}: {file:?} is not a file name. Use lowercase letters, digits, _ and -, without .glb."
            ));
        }
        if !seen.insert(file.as_str()) {
            return invalid(format!(
                "Two assets ship as {file}.glb. Give each its own name."
            ));
        }
    }
    Ok(())
}

/// The project's `ship.json`, or the scene defaults without one.
///
/// # Errors
///
/// Fails when the file exists but cannot be read or [`validate`] refuses it.
pub fn load(project_dir: &Path) -> CoreResult<ShipSettings> {
    let file = path(project_dir);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(ShipSettings::default()),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let settings: ShipSettings =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    validate(&settings)?;
    Ok(settings)
}

/// [`load`], falling back to the defaults (and a warning) for a broken file,
/// for the views that must still show something.
pub fn load_or_default(project_dir: &Path) -> ShipSettings {
    load(project_dir).unwrap_or_else(|err| {
        tracing::warn!(error = %err, "unreadable ship.json, shipping as a scene");
        ShipSettings::default()
    })
}

/// One read, change and write at a time, so overlapping saves both land.
static WRITES: Mutex<()> = Mutex::new(());

/// Applies `change` to the stored settings and saves them. Entries set back
/// to the defaults are dropped, so the file only lists real choices.
///
/// # Errors
///
/// Fails when the result does not validate or cannot be written.
pub fn update(project_dir: &Path, change: ShipSettingsChange) -> CoreResult<ShipSettings> {
    let _one = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let mut settings = load(project_dir)?;
    if let Some(style) = change.style {
        settings.style = style;
    }
    if let Some(textures) = change.textures {
        settings.textures = textures;
    }
    if let Some(quality) = change.quality {
        settings.quality = quality;
    }
    for (asset, mut entry) in change.assets {
        entry.file = entry
            .file
            .map(|f| f.trim().trim_end_matches(".glb").to_string())
            .filter(|f| !f.is_empty());
        if entry.is_default() {
            settings.assets.remove(&asset);
        } else {
            settings.assets.insert(asset, entry);
        }
    }
    settings.version = SHIP_VERSION;
    validate(&settings)?;
    write_json(&path(project_dir), &settings)?;
    Ok(settings)
}
