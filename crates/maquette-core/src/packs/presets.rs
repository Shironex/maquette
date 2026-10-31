//! Pack presets: settings saved under a name, in
//! `<workspace>/exports/presets.json`, with how often each was used. One
//! built in preset, "Plain OBJ pack", is always there.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use super::{EXPORTS_DIR, PackSettings};
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::project::{now_rfc3339, slugify};

const PRESETS_FILE: &str = "presets.json";
const PRESETS_VERSION: u32 = 1;
/// The built in preset's id.
pub(super) const PLAIN_OBJ: &str = "plain-obj";
const MAX_PRESETS: usize = 32;

/// Settings saved under a name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackPreset {
    pub id: String,
    pub name: String,
    /// The settings, the pack name aside: each pack names its own.
    pub settings: PackSettings,
    /// True for the one that comes with Maquette.
    pub built_in: bool,
    pub saved_at: Option<String>,
    pub used: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresetsFile {
    version: u32,
    presets: Vec<PackPreset>,
    /// How often the built in preset was used.
    #[serde(default)]
    built_in_used: u32,
}

static WRITES: Mutex<()> = Mutex::new(());

fn path(root: &Path) -> PathBuf {
    root.join(EXPORTS_DIR).join(PRESETS_FILE)
}

fn read(root: &Path) -> CoreResult<PresetsFile> {
    let file = path(root);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(PresetsFile::default()),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let found: PresetsFile = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if found.version > PRESETS_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: found.version,
            supported: PRESETS_VERSION,
        });
    }
    Ok(found)
}

fn write(root: &Path, mut file: PresetsFile) -> CoreResult<()> {
    file.version = PRESETS_VERSION;
    let dir = root.join(EXPORTS_DIR);
    fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    write_json(&path(root), &file)
}

fn plain_obj(used: u32) -> PackPreset {
    PackPreset {
        id: PLAIN_OBJ.to_string(),
        name: "Plain OBJ pack".to_string(),
        settings: PackSettings::default(),
        built_in: true,
        saved_at: None,
        used,
    }
}

/// Every preset, the built in one first, then the saved ones by name.
///
/// # Errors
///
/// Fails when the file cannot be read or comes from a newer Maquette.
pub fn presets(root: &Path) -> CoreResult<Vec<PackPreset>> {
    let file = read(root)?;
    let mut saved = file.presets;
    saved.sort_by_key(|p| p.name.to_lowercase());
    let mut out = vec![plain_obj(file.built_in_used)];
    out.extend(saved);
    Ok(out)
}

/// Saves `settings` under `name`, over a saved preset of the same name.
/// Answers every preset.
///
/// # Errors
///
/// Refuses an empty name, the built in preset's name and too many presets.
pub fn save_preset(
    root: &Path,
    name: &str,
    settings: &PackSettings,
) -> CoreResult<Vec<PackPreset>> {
    let name = name.trim();
    let id = slugify(name);
    if !name.chars().any(char::is_alphanumeric) {
        return Err(CoreError::Invalid("Give the preset a name.".to_string()));
    }
    if id == PLAIN_OBJ || name.eq_ignore_ascii_case("plain obj pack") {
        return Err(CoreError::Invalid(
            "Plain OBJ pack comes with Maquette. Save yours under another name.".to_string(),
        ));
    }
    let _turn = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let mut file = read(root)?;
    let used = file
        .presets
        .iter()
        .find(|p| p.id == id)
        .map_or(0, |p| p.used);
    file.presets.retain(|p| p.id != id);
    if file.presets.len() >= MAX_PRESETS {
        return Err(CoreError::Invalid(format!(
            "Keep {MAX_PRESETS} presets or fewer. Delete one in exports/presets.json first."
        )));
    }
    file.presets.push(PackPreset {
        id,
        name: name.to_string(),
        settings: PackSettings {
            name: String::new(),
            ..settings.clone()
        },
        built_in: false,
        saved_at: Some(now_rfc3339()),
        used,
    });
    write(root, file)?;
    presets(root)
}

/// Counts a use of the preset `id`; an unknown id is left alone.
pub(super) fn mark_used(root: &Path, id: &str) {
    let _turn = WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let Ok(mut file) = read(root) else {
        return;
    };
    if id == PLAIN_OBJ {
        file.built_in_used += 1;
    } else if let Some(preset) = file.presets.iter_mut().find(|p| p.id == id) {
        preset.used += 1;
    } else {
        return;
    }
    if let Err(err) = write(root, file) {
        tracing::warn!(error = %err, "could not count a pack preset's use");
    }
}
