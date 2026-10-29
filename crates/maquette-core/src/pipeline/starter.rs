//! Starter scripts: the small, generic tool scripts a new project's template
//! steps run, so its pipeline works on day one. They are written into the
//! project's `tools/` once, when the template is first written, and never
//! over a file that exists: after that they belong to the project.
//!
//! Each write is recorded in `tools/.starter.json` (the script's path and
//! the hash of the text written), so a later Maquette can tell an untouched
//! copy of an older starter from one the user edited. Scripts written before
//! that record existed are matched against the hashes of every earlier
//! starter ([`OLDER`]). Untouched ones update in one go; an edited one is
//! only replaced when asked, with the old file kept as `<name>.bak`.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{CoreError, CoreResult};
use crate::project::ProjectTemplate;

/// Where the record of written starters lives, relative to the project.
pub const RECORD_FILE: &str = "tools/.starter.json";

/// Hashes of earlier starter versions, for projects made before
/// [`RECORD_FILE`] existed. A script whose text hashes to one of these is an
/// untouched copy of that version. Add the old hash here whenever a starter
/// changes.
const OLDER: &[(&str, &str)] = &[
    (
        "tools/bake_light.py",
        "0395baa6ce1fd08a97f15c47bc3f6676c6fa763d8b2886d46790f380ecd37b55",
    ),
    (
        "tools/bake_light.py",
        "861b25a56504ec254b2bafa4a0d5080431ebd9b4be52eb80afaac09f807ebf98",
    ),
    (
        "tools/build_scene.py",
        "27e68367ae83272ee3f0e4336a1480163e5be43beb437e70f36e601086c6f604",
    ),
    (
        "tools/build_scene.py",
        "b8c2b636ab7cab77f608d05d1698bff31af8d583822bb27e05f078d989af2b6c",
    ),
    (
        "tools/build_scene.py",
        "beb41cd4353d00079f562545667c56cd2e3f267f6a04d087f430b96900657dd1",
    ),
    (
        "tools/build_scene.py",
        "378d3527366f7c6a366a070e6e88d1a5d3738e5da9bf7456cd3aa3620593ff99",
    ),
    (
        "tools/build_scene.py",
        "6673c062e2592802c5a62268bdf2e3f456a63103958edf1509662930f4f9bfca",
    ),
    (
        "tools/build_scene.py",
        "e4bcebdf223d63af804879b3a1f96b233328d0271ec0722e8c2296d38d522323",
    ),
    (
        "tools/export_character.py",
        "a450a138b62489b9bb6ffc848392de75bc3539460cd275a181f88e986b20e054",
    ),
    (
        "tools/export_game.py",
        "177147974d12310bda284f7c62b5cff1b2460a9414659147239014174713f79c",
    ),
    (
        "tools/export_game.py",
        "337b4e824ee1fcd32e9bba20cea28d321cb4cd088dfd2ffe1148419f5de6495e",
    ),
    (
        "tools/export_game.py",
        "b61ebb6be9b04459f2cd69f0b376eaf7433739f2218d09d00a293176c31089fa",
    ),
    (
        "tools/export_web.py",
        "e67655bf5b32f480d84a53f0c7b4657c891576993dedbc1eff1585696a59d486",
    ),
    (
        "tools/export_web.py",
        "2fa35e389871a4dd8cac8a19d264aa20052bedb1de74aac2e739388dac943c22",
    ),
    (
        "tools/export_web.py",
        "fd77207200db5c9daa23784af02a2e18a315a325304f0b7756c54839a8c6f851",
    ),
    (
        "tools/export_web.py",
        "983da9e6089c66ecc51e6e92e7069be7f0c5d9bafb0d6903f1c3c9117f05a838",
    ),
    (
        "tools/export_web.py",
        "8d436335faf4548e381c56149d6d11bff814dd48a997305cd17c2ef6abf50106",
    ),
    (
        "tools/fixups.py",
        "625c3b8886d307542425b36fd7436bfbe50472c57eda897d8f532a91087e5a12",
    ),
    (
        "tools/fixups.py",
        "e69bc384ee9b4e5029fc0b31a6c7f76bd790dea59b0d5cb8191125be75a4d91a",
    ),
    (
        "tools/fixups.py",
        "08a72ebe268acc9fd9bb941225cb5b0b278abae33443e8fc9d175fe9edfc212e",
    ),
    (
        "tools/game_export.mjs",
        "8e4448c0a766841b4ec03e0216cee85f863920f2836a1df10d941b702e2747ed",
    ),
    (
        "tools/render_stops.py",
        "25e9477fc998ed5aa290bf095d91b9faf3e667cadc5a11145231290ea04fd153",
    ),
    (
        "tools/render_stops.py",
        "3a2b36c2f21d269a6d106570f05accb637a07ebe69e42092258e76457f6e220e",
    ),
    (
        "tools/render_stops.py",
        "ecadad4a5df2499ade7cb3c5eb50a2f18669cc95a389cc171cb74ef4b7c396d1",
    ),
    (
        "tools/ship.mjs",
        "d92fddb54514a13dd7a8c2f6daa4f5e3cd4389b00180162eace6d8aeaec5ea23",
    ),
    (
        "tools/ship.mjs",
        "b08f2221e09c8081f582cddd22bf094d1fdabbf56ce84a1726525b1f860e9e11",
    ),
];

/// How a project's copy of a starter compares with the one this build ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum StarterState {
    /// The same as the shipped starter.
    Current,
    /// An untouched copy of an older starter: safe to update.
    Outdated,
    /// Changed by hand (or of unknown origin): only replaced when asked.
    Edited,
    /// Not in the project: an update writes it.
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StarterStatus {
    /// Relative to the project folder, for example `tools/bake_light.py`.
    pub path: String,
    pub state: StarterState,
    /// Where the old file went when this update replaced an edited one.
    pub backup: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Record {
    /// Script path to the hash of the text Maquette wrote there.
    scripts: BTreeMap<String, String>,
}

/// The hash a starter is compared by. Line endings are folded, so a copy
/// that went through a CRLF checkout or editor still matches.
fn text_hash(text: &str) -> String {
    let folded = text.replace("\r\n", "\n");
    hex::encode(Sha256::digest(folded.as_bytes()))
}

fn load_record(project_dir: &Path) -> Record {
    fs::read_to_string(project_dir.join(RECORD_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_record(project_dir: &Path, record: &Record) -> CoreResult<()> {
    let path = project_dir.join(RECORD_FILE);
    let text =
        serde_json::to_string_pretty(record).map_err(|e| CoreError::Invalid(e.to_string()))?;
    crate::fs::write_atomic(&path, format!("{text}\n").as_bytes())
        .map_err(|e| CoreError::io(&path, e))
}

/// One script and where it goes, relative to the project folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarterScript {
    pub path: &'static str,
    pub text: &'static str,
}

const BUILD_SCENE: StarterScript = StarterScript {
    path: "tools/build_scene.py",
    text: include_str!("../../starter/build_scene.py"),
};
const FIXUPS: StarterScript = StarterScript {
    path: crate::prepare::SCRIPT_PATH,
    text: crate::prepare::SCRIPT,
};
const BAKE_LIGHT: StarterScript = StarterScript {
    path: "tools/bake_light.py",
    text: include_str!("../../starter/bake_light.py"),
};
const EXPORT_WEB: StarterScript = StarterScript {
    path: "tools/export_web.py",
    text: include_str!("../../starter/export_web.py"),
};
const BUILD_CHARACTER: StarterScript = StarterScript {
    path: "tools/build_character.py",
    text: include_str!("../../starter/build_character.py"),
};
const EXPORT_CHARACTER: StarterScript = StarterScript {
    path: "tools/export_character.py",
    text: include_str!("../../starter/export_character.py"),
};
const RENDER_STOPS: StarterScript = StarterScript {
    path: "tools/render_stops.py",
    text: include_str!("../../starter/render_stops.py"),
};
const SHIP: StarterScript = StarterScript {
    path: "tools/ship.mjs",
    text: include_str!("../../starter/ship.mjs"),
};
const BUILD_MODELS: StarterScript = StarterScript {
    path: super::template::game::BUILD_SCRIPT,
    text: include_str!("../../starter/build_models.py"),
};
const EXPORT_GAME: StarterScript = StarterScript {
    path: super::template::game::EXPORT_SCRIPT,
    text: include_str!("../../starter/export_game.py"),
};
const GAME_EXPORT: StarterScript = StarterScript {
    path: super::template::game::GAME_EXPORT_SCRIPT,
    text: include_str!("../../starter/game_export.mjs"),
};
const SHIP_GAME: StarterScript = StarterScript {
    path: super::template::game::SHIP_SCRIPT,
    text: include_str!("../../starter/ship_game.mjs"),
};

/// The scripts a template's steps reference. Imported projects bring their
/// own. A game gets the bake starter too, so turning Bake light on later
/// finds it.
pub fn scripts(template: ProjectTemplate) -> &'static [StarterScript] {
    match template {
        ProjectTemplate::FloatingDiorama | ProjectTemplate::Room => &[
            BUILD_SCENE,
            FIXUPS,
            BAKE_LIGHT,
            EXPORT_WEB,
            RENDER_STOPS,
            SHIP,
        ],
        ProjectTemplate::CharacterShowcase => {
            &[BUILD_CHARACTER, EXPORT_CHARACTER, RENDER_STOPS, SHIP]
        }
        ProjectTemplate::Imported => &[],
        ProjectTemplate::GameImport => &[
            BUILD_MODELS,
            FIXUPS,
            BAKE_LIGHT,
            EXPORT_GAME,
            GAME_EXPORT,
            SHIP_GAME,
        ],
    }
}

/// Writes the template's scripts under `project_dir`, skipping any that
/// exist, and returns the paths it wrote.
///
/// # Errors
///
/// Fails when a folder or file cannot be created.
pub fn write(project_dir: &Path, template: ProjectTemplate) -> CoreResult<Vec<String>> {
    let mut written = Vec::new();
    let mut record = load_record(project_dir);
    for script in scripts(template) {
        let path = project_dir.join(script.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        // create_new: an edited script is never replaced, even in a race.
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(script.text.as_bytes())
                    .map_err(|e| CoreError::io(&path, e))?;
                written.push(script.path.to_string());
                record
                    .scripts
                    .insert(script.path.to_string(), text_hash(script.text));
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(CoreError::io(&path, err)),
        }
    }
    if !written.is_empty() {
        save_record(project_dir, &record)?;
    }
    Ok(written)
}

fn state_of(project_dir: &Path, script: &StarterScript, record: &Record) -> StarterState {
    let Ok(text) = fs::read_to_string(project_dir.join(script.path)) else {
        return StarterState::Missing;
    };
    let hash = text_hash(&text);
    if hash == text_hash(script.text) {
        return StarterState::Current;
    }
    let untouched = match record.scripts.get(script.path) {
        Some(written) => *written == hash,
        None => OLDER
            .iter()
            .any(|(path, old)| *path == script.path && *old == hash),
    };
    if untouched {
        StarterState::Outdated
    } else {
        StarterState::Edited
    }
}

/// How each of the template's starters in the project compares with the
/// shipped ones. Empty for an imported project.
pub fn status(project_dir: &Path, template: ProjectTemplate) -> Vec<StarterStatus> {
    let record = load_record(project_dir);
    scripts(template)
        .iter()
        .map(|script| StarterStatus {
            path: script.path.to_string(),
            state: state_of(project_dir, script, &record),
            backup: None,
        })
        .collect()
}

/// `tools/bake_light.py.bak`, or `.bak2`, `.bak3` when that is taken.
fn backup_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    (1u32..)
        .map(|n| {
            let suffix = if n == 1 { String::new() } else { n.to_string() };
            path.with_file_name(format!("{name}.bak{suffix}"))
        })
        .find(|p| !p.exists())
        .unwrap_or_else(|| path.with_file_name(format!("{name}.bak")))
}

/// Brings the starters at `paths` up to date. Outdated and missing ones are
/// written; an edited one only when `replace_edited` is set, after moving it
/// to `<name>.bak`. Current ones are left alone. Returns the status after
/// the update.
///
/// # Errors
///
/// Refuses, writing nothing, a path that is not a starter of the template.
/// Fails when a script or its backup cannot be written, and when a script
/// it should have written is still not current afterwards.
pub fn update(
    project_dir: &Path,
    template: ProjectTemplate,
    paths: &[String],
    replace_edited: bool,
) -> CoreResult<Vec<StarterStatus>> {
    let known = scripts(template);
    if let Some(unknown) = paths.iter().find(|p| !known.iter().any(|s| s.path == *p)) {
        return Err(CoreError::Invalid(format!(
            "{unknown} is not a starter script of this project's template."
        )));
    }
    let mut record = load_record(project_dir);
    let mut backups = BTreeMap::new();
    for script in scripts(template)
        .iter()
        .filter(|s| paths.iter().any(|p| p == s.path))
    {
        let path = project_dir.join(script.path);
        match state_of(project_dir, script, &record) {
            StarterState::Current => continue,
            StarterState::Edited if !replace_edited => continue,
            StarterState::Edited => {
                let backup = backup_path(&path);
                fs::copy(&path, &backup).map_err(|e| CoreError::io(&backup, e))?;
                let relative = backup
                    .strip_prefix(project_dir)
                    .unwrap_or(&backup)
                    .to_string_lossy()
                    .replace('\\', "/");
                backups.insert(script.path, relative);
            }
            StarterState::Outdated | StarterState::Missing => {}
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        crate::fs::write_atomic(&path, script.text.as_bytes())
            .map_err(|e| CoreError::io(&path, e))?;
        record
            .scripts
            .insert(script.path.to_string(), text_hash(script.text));
    }
    save_record(project_dir, &record)?;
    let mut after = status(project_dir, template);
    let stuck: Vec<&str> = after
        .iter()
        .filter(|s| paths.contains(&s.path) && s.state != StarterState::Current)
        .filter(|s| replace_edited || s.state != StarterState::Edited)
        .map(|s| s.path.as_str())
        .collect();
    if !stuck.is_empty() {
        return Err(CoreError::Invalid(format!(
            "{} did not update: the file on disk still differs from the starter.",
            stuck.join(", ")
        )));
    }
    for entry in &mut after {
        entry.backup = backups.get(entry.path.as_str()).cloned();
    }
    Ok(after)
}

#[cfg(test)]
mod export_tests;
#[cfg(test)]
mod tests;
