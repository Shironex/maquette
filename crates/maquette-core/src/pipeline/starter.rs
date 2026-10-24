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
        "tools/build_scene.py",
        "27e68367ae83272ee3f0e4336a1480163e5be43beb437e70f36e601086c6f604",
    ),
    (
        "tools/build_scene.py",
        "b8c2b636ab7cab77f608d05d1698bff31af8d583822bb27e05f078d989af2b6c",
    ),
    (
        "tools/export_character.py",
        "a450a138b62489b9bb6ffc848392de75bc3539460cd275a181f88e986b20e054",
    ),
    (
        "tools/export_web.py",
        "e67655bf5b32f480d84a53f0c7b4657c891576993dedbc1eff1585696a59d486",
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
    format!("{:x}", Sha256::digest(folded.as_bytes()))
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

/// The scripts a template's steps reference. Imported projects bring their
/// own.
pub fn scripts(template: ProjectTemplate) -> &'static [StarterScript] {
    match template {
        ProjectTemplate::FloatingDiorama | ProjectTemplate::Room => {
            &[BUILD_SCENE, BAKE_LIGHT, EXPORT_WEB, RENDER_STOPS, SHIP]
        }
        ProjectTemplate::CharacterShowcase => {
            &[BUILD_CHARACTER, EXPORT_CHARACTER, RENDER_STOPS, SHIP]
        }
        ProjectTemplate::Imported => &[],
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
/// to `<name>.bak`. Current ones and paths that are not starters of the
/// template are left alone. Returns the status after the update.
///
/// # Errors
///
/// Fails when a script or its backup cannot be written.
pub fn update(
    project_dir: &Path,
    template: ProjectTemplate,
    paths: &[String],
    replace_edited: bool,
) -> CoreResult<Vec<StarterStatus>> {
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
    for entry in &mut after {
        entry.backup = backups.get(entry.path.as_str()).cloned();
    }
    Ok(after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::template::for_template;

    #[test]
    fn every_template_script_has_a_starter() {
        for template in [
            ProjectTemplate::FloatingDiorama,
            ProjectTemplate::Room,
            ProjectTemplate::CharacterShowcase,
        ] {
            let def = for_template(template);
            for step in &def.steps {
                if let Some(script) = &step.script {
                    assert!(
                        scripts(template).iter().any(|s| s.path == script),
                        "{template:?} has no starter for {script}"
                    );
                }
            }
        }
        assert!(scripts(ProjectTemplate::Imported).is_empty());
    }

    #[test]
    fn starters_stay_plain_and_headless() {
        for script in scripts(ProjectTemplate::FloatingDiorama)
            .iter()
            .chain(scripts(ProjectTemplate::CharacterShowcase))
        {
            assert!(
                !script.text.contains('\u{2014}'),
                "{} has an em dash",
                script.path
            );
            assert!(
                !script.text.contains('\u{2013}'),
                "{} has an en dash",
                script.path
            );
            // The window is never opened from a pipeline script.
            assert!(!script.text.contains("wm.window"), "{}", script.path);
        }
    }

    #[test]
    fn posters_frame_the_scene_when_it_has_no_stops_and_never_save_it() {
        let text = RENDER_STOPS.text;
        assert!(text.contains("def overview_camera(scene)"));
        assert!(text.contains("one framed three quarter poster of the whole scene"));
        assert!(text.contains("remove_camera(camera)"));
        // The temporary camera must not end up in the user's blend.
        assert!(!text.contains("save_mainfile") && !text.contains("save_as_mainfile"));
    }

    #[test]
    fn the_build_places_models_from_the_layout_and_posters_keep_its_names() {
        let build = BUILD_SCENE.text;
        assert!(build.contains("--layout"));
        assert!(build.contains("layout.json"));
        assert!(build.contains("def place(collection, placement)"));
        assert!(build.contains("def add_stops(scene, stops)"));
        assert!(build.contains("\"maquette_stop_name\""));
        // Cameras the user placed in the base blend win over the layout's.
        assert!(build.contains("own_stops(scene)"));
        let posters = RENDER_STOPS.text;
        assert!(posters.contains("LABEL = \"maquette_stop_name\""));
        assert!(posters.contains("entry[\"label\"]"));
    }

    #[test]
    fn write_skips_files_that_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = write(dir.path(), ProjectTemplate::FloatingDiorama).expect("write");
        assert_eq!(first.len(), 5);
        let character = tempfile::tempdir().expect("tempdir");
        let written = write(character.path(), ProjectTemplate::CharacterShowcase).expect("write");
        assert_eq!(written.len(), 4);
        assert!(written.contains(&"tools/build_character.py".to_string()));
        let edited = dir.path().join("tools/build_scene.py");
        fs::write(&edited, "print('mine')\n").expect("edit");
        let again = write(dir.path(), ProjectTemplate::FloatingDiorama).expect("again");
        assert!(again.is_empty());
        assert_eq!(
            fs::read_to_string(&edited).expect("read"),
            "print('mine')\n"
        );
    }

    fn state(project: &Path, path: &str) -> StarterState {
        status(project, ProjectTemplate::FloatingDiorama)
            .into_iter()
            .find(|s| s.path == path)
            .expect("listed")
            .state
    }

    #[test]
    fn fresh_starters_are_recorded_and_current() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), ProjectTemplate::FloatingDiorama).expect("write");
        assert!(dir.path().join(RECORD_FILE).is_file());
        assert!(
            status(dir.path(), ProjectTemplate::FloatingDiorama)
                .iter()
                .all(|s| s.state == StarterState::Current)
        );
        assert!(status(dir.path(), ProjectTemplate::Imported).is_empty());
    }

    #[test]
    fn a_recorded_copy_of_an_older_starter_updates_and_an_edited_one_waits() {
        let dir = tempfile::tempdir().expect("tempdir");
        let project = dir.path();
        // As if an older Maquette had written these two and recorded them.
        let old_bake = "print('stub: nothing baked')\n";
        let old_export = "print('old export')\n";
        fs::create_dir_all(project.join("tools")).expect("tools");
        fs::write(project.join(BAKE_LIGHT.path), old_bake).expect("bake");
        fs::write(project.join(EXPORT_WEB.path), old_export).expect("export");
        let mut record = Record::default();
        record
            .scripts
            .insert(BAKE_LIGHT.path.into(), text_hash(old_bake));
        record
            .scripts
            .insert(EXPORT_WEB.path.into(), text_hash(old_export));
        save_record(project, &record).expect("record");
        // The user changed the export afterwards.
        fs::write(project.join(EXPORT_WEB.path), "print('mine')\n").expect("edit");

        assert_eq!(state(project, BAKE_LIGHT.path), StarterState::Outdated);
        assert_eq!(state(project, EXPORT_WEB.path), StarterState::Edited);
        assert_eq!(state(project, BUILD_SCENE.path), StarterState::Missing);

        let all: Vec<String> = scripts(ProjectTemplate::FloatingDiorama)
            .iter()
            .map(|s| s.path.to_string())
            .collect();
        let after = update(project, ProjectTemplate::FloatingDiorama, &all, false).expect("update");
        let of = |path: &str| {
            after
                .iter()
                .find(|s| s.path == path)
                .expect("listed")
                .clone()
        };
        assert_eq!(of(BAKE_LIGHT.path).state, StarterState::Current);
        assert_eq!(of(BUILD_SCENE.path).state, StarterState::Current);
        assert_eq!(of(EXPORT_WEB.path).state, StarterState::Edited);
        assert_eq!(
            fs::read_to_string(project.join(EXPORT_WEB.path)).expect("read"),
            "print('mine')\n",
            "an edited script is never overwritten silently"
        );
        assert_eq!(
            fs::read_to_string(project.join(BAKE_LIGHT.path)).expect("read"),
            BAKE_LIGHT.text
        );

        let replaced = update(
            project,
            ProjectTemplate::FloatingDiorama,
            &[EXPORT_WEB.path.to_string()],
            true,
        )
        .expect("replace");
        let export = replaced
            .iter()
            .find(|s| s.path == EXPORT_WEB.path)
            .expect("listed");
        assert_eq!(export.state, StarterState::Current);
        assert_eq!(export.backup.as_deref(), Some("tools/export_web.py.bak"));
        assert_eq!(
            fs::read_to_string(project.join("tools/export_web.py.bak")).expect("backup"),
            "print('mine')\n"
        );
        // A second replace keeps the first backup.
        fs::write(project.join(EXPORT_WEB.path), "print('mine again')\n").expect("edit");
        let again = update(
            project,
            ProjectTemplate::FloatingDiorama,
            &[EXPORT_WEB.path.to_string()],
            true,
        )
        .expect("again");
        assert_eq!(
            again
                .iter()
                .find(|s| s.path == EXPORT_WEB.path)
                .and_then(|s| s.backup.clone())
                .as_deref(),
            Some("tools/export_web.py.bak2")
        );
    }

    #[test]
    fn unrecorded_scripts_match_the_known_older_starters() {
        let dir = tempfile::tempdir().expect("tempdir");
        let project = dir.path();
        write(project, ProjectTemplate::FloatingDiorama).expect("write");
        // Before the record existed: no tools/.starter.json.
        fs::remove_file(project.join(RECORD_FILE)).expect("no record");
        // The first bake starter, a stub, as projects from then have it.
        let first = include_str!("fixtures/bake_light_first.py");
        fs::write(project.join(BAKE_LIGHT.path), first).expect("first");
        assert_eq!(state(project, BAKE_LIGHT.path), StarterState::Outdated);
        let older = "print('a starter from long ago')\n";
        fs::write(project.join(BAKE_LIGHT.path), older).expect("older");
        assert_eq!(
            state(project, BAKE_LIGHT.path),
            StarterState::Edited,
            "unknown text without a record may be edited"
        );
        assert!(
            OLDER.iter().all(|(path, hash)| {
                scripts(ProjectTemplate::FloatingDiorama)
                    .iter()
                    .chain(scripts(ProjectTemplate::CharacterShowcase))
                    .any(|s| s.path == *path && text_hash(s.text) != *hash)
            }),
            "a known older hash is never the current starter"
        );
        // A CRLF copy of the shipped starter still counts as current.
        fs::write(
            project.join(BUILD_SCENE.path),
            BUILD_SCENE.text.replace('\n', "\r\n"),
        )
        .expect("crlf");
        assert_eq!(state(project, BUILD_SCENE.path), StarterState::Current);
    }
}
