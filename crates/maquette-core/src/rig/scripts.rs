//! The character rig skill's scripts, bundled from the skill's own folder:
//! the app and the skill run one copy. [`install`] writes them into a run's
//! work folder keeping the skill's layout, since the creature scripts reach
//! `../humanoid/glbio.py`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, CoreResult};

macro_rules! bundled {
    ($route:literal, $($name:literal),+ $(,)?) => {
        &[$(($name, include_str!(concat!(
            "../../../../plugins/maquette/skills/maquette-character-rig/scripts/",
            $route, "/", $name
        )))),+]
    };
}

/// The humanoid route's files.
pub const HUMANOID: &[(&str, &str)] = bundled!(
    "humanoid",
    "glbio.py",
    "probe.py",
    "fixweights.py",
    "narrow.py",
    "rigmove.py",
    "verify.py",
    "rendertest.py",
);

/// The creature route's files. `rigconfig.py` is the skill's example; a run
/// writes its own over it with `autoconfig.py`.
pub const CREATURE: &[(&str, &str)] = bundled!(
    "creature",
    "autoconfig.py",
    "rigconfig.py",
    "shlib.py",
    "groups.py",
    "posing.py",
    "probe.py",
    "pieces.py",
    "weightmap.py",
    "posetest.py",
    "fix.py",
    "verify.py",
    "sheet.py",
    "walkclip.py",
    "clipsheet.py",
);

/// Writes both routes under `<dir>/scripts`, and answers that folder.
///
/// # Errors
///
/// Fails when the folder cannot be written.
pub fn install(dir: &Path) -> CoreResult<PathBuf> {
    let root = dir.join("scripts");
    for (route, files) in [("humanoid", HUMANOID), ("creature", CREATURE)] {
        let folder = root.join(route);
        fs::create_dir_all(&folder).map_err(|e| CoreError::io(&folder, e))?;
        for (name, text) in files {
            let file = folder.join(name);
            // Keep the rigconfig.py a run wrote.
            if *name == "rigconfig.py" && file.is_file() {
                continue;
            }
            fs::write(&file, text).map_err(|e| CoreError::io(&file, e))?;
        }
    }
    Ok(root)
}

/// The text of a bundled script of `route`.
pub fn text(route: &[(&'static str, &'static str)], name: &str) -> CoreResult<&'static str> {
    route
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, t)| *t)
        .ok_or_else(|| CoreError::Invalid(format!("No bundled script {name}.")))
}
