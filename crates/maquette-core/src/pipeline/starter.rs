//! Starter scripts: the small, generic tool scripts a new project's template
//! steps run, so its pipeline works on day one. They are written into the
//! project's `tools/` once, when the template is first written, and never
//! over a file that exists: after that they belong to the project.

use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::Path;

use crate::error::{CoreError, CoreResult};
use crate::project::ProjectTemplate;

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
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(CoreError::io(&path, err)),
        }
    }
    Ok(written)
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
}
