//! What a website's Ship needs before it can copy anything, checked before
//! anyone is asked to approve it. The Ship starter (`starter/ship.mjs`)
//! refuses the same things, but only once it runs, which is after the
//! approval card; asking a person to approve a Ship that is bound to fail
//! wastes their time and hides the reason behind an exit code.

use std::path::Path;

use crate::layout;

/// The start of the reason a site's data file is refused without camera
/// stops. Agents get advice by this phrase, so keep it in step with
/// the MCP error table.
pub const NO_STOPS: &str = "No camera stops:";
/// The start of the reason a site's data file is refused without placed models.
pub const NO_PLACED: &str = "No placed models:";
/// The start of the reason the site repo folder is gone.
pub const NO_SITE_REPO: &str = "The site repo is not there:";
/// The start of the reason the site's `maquette.site.json` cannot be used.
pub const BAD_SITE_CONFIG: &str = "The site's maquette.site.json cannot be used:";
/// The start of the reason the project's `layout.json` cannot be read.
pub const BAD_LAYOUT: &str = "The layout cannot be read:";

/// Why a website Ship from `project_dir` into the site repo `repo` would
/// fail before it copies anything, as a sentence that says what to do, or
/// None when it can go ahead. Checks that the repo folder is there, that
/// its `maquette.site.json` (when it has one) is one this Maquette reads,
/// and, when the site takes scene data, that the layout places at least one
/// model and has at least one camera stop, since shipping an empty layout
/// would wipe the site's scene.
pub fn ship_blocker(project_dir: &Path, repo: &Path) -> Option<String> {
    if !repo.is_dir() {
        return Some(format!(
            "{NO_SITE_REPO} {} is missing or not a folder. Pick the site repo again in the Ship view, then ship again.",
            repo.display()
        ));
    }
    let config = match super::load_config(repo) {
        Ok(config) => config,
        Err(err) => {
            return Some(format!(
                "{BAD_SITE_CONFIG} {err} Fix the file, or save the site folders again in the Ship view, then ship again."
            ));
        }
    };
    // Without a data file Ship only copies files, whatever the layout holds.
    config.data.as_ref()?;
    let scene = match layout::load(project_dir) {
        Ok(scene) => scene,
        Err(err) => {
            return Some(format!(
                "{BAD_LAYOUT} {err} Fix layout.json in the Layout editor, then ship again."
            ));
        }
    };
    if scene.models.is_empty() {
        return Some(format!(
            "{NO_PLACED} the site takes scene data, and shipping a layout without models would wipe its scene. Place the models in the layout, then ship again; nothing was copied."
        ));
    }
    if scene.stops.is_empty() {
        return Some(format!(
            "{NO_STOPS} the site takes scene data, and its page opens on the first camera stop. Add one in the layout, then ship again; nothing was copied."
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::*;

    struct Scratch {
        _dir: tempfile::TempDir,
        project: std::path::PathBuf,
        site: std::path::PathBuf,
    }

    fn scratch() -> Scratch {
        let dir = tempfile::tempdir().expect("dir");
        let project = dir.path().join("project");
        let site = dir.path().join("site");
        fs::create_dir_all(&project).expect("project");
        fs::create_dir_all(&site).expect("site");
        Scratch {
            _dir: dir,
            project,
            site,
        }
    }

    fn with_data(site: &Path) {
        fs::write(
            site.join(super::super::CONFIG_FILE),
            json!({ "version": 1, "models": "public/models", "data": "src/shipped.json" })
                .to_string(),
        )
        .expect("config");
    }

    fn write_layout(project: &Path, placed: bool, stops: bool) {
        let models = if placed {
            json!({ "lamp": { "position": [0, 0, 0], "rotation": [0, 0, 0], "scale": 1 } })
        } else {
            json!({})
        };
        let stops = if stops {
            json!([{ "id": "hero", "name": "Hero", "position": [0, 1, 3], "target": [0, 0, 0],
                     "fov": 40, "models": null }])
        } else {
            json!([])
        };
        let value = json!({ "version": 1, "models": models, "stops": stops, "desk": null });
        fs::write(project.join("layout.json"), value.to_string()).expect("layout");
    }

    #[test]
    fn a_site_without_scene_data_ships_whatever_the_layout_holds() {
        let s = scratch();
        assert_eq!(ship_blocker(&s.project, &s.site), None, "no config");
        fs::write(
            s.site.join(super::super::CONFIG_FILE),
            r#"{ "version": 1, "models": "public/scene" }"#,
        )
        .expect("config");
        assert_eq!(ship_blocker(&s.project, &s.site), None, "no data file");
    }

    #[test]
    fn scene_data_needs_placed_models_and_a_camera_stop() {
        let s = scratch();
        with_data(&s.site);
        let none = ship_blocker(&s.project, &s.site).expect("no layout file");
        assert!(none.starts_with(NO_PLACED), "{none}");
        write_layout(&s.project, true, false);
        let stops = ship_blocker(&s.project, &s.site).expect("no stops");
        assert!(stops.starts_with(NO_STOPS), "{stops}");
        assert!(stops.contains("Add one in the layout"), "{stops}");
        write_layout(&s.project, false, true);
        let placed = ship_blocker(&s.project, &s.site).expect("no models");
        assert!(placed.starts_with(NO_PLACED), "{placed}");
        write_layout(&s.project, true, true);
        assert_eq!(ship_blocker(&s.project, &s.site), None);
    }

    #[test]
    fn a_missing_repo_a_bad_site_file_or_a_broken_layout_is_named() {
        let s = scratch();
        let gone = ship_blocker(&s.project, &s.site.join("nope")).expect("gone");
        assert!(gone.starts_with(NO_SITE_REPO), "{gone}");
        assert!(gone.contains("nope"), "{gone}");

        fs::write(s.site.join(super::super::CONFIG_FILE), "{ not json").expect("bad");
        let bad = ship_blocker(&s.project, &s.site).expect("bad config");
        assert!(bad.starts_with(BAD_SITE_CONFIG), "{bad}");
        fs::write(
            s.site.join(super::super::CONFIG_FILE),
            r#"{ "version": 1, "models": "../outside" }"#,
        )
        .expect("outside");
        let outside = ship_blocker(&s.project, &s.site).expect("outside the site");
        assert!(outside.starts_with(BAD_SITE_CONFIG), "{outside}");

        with_data(&s.site);
        fs::write(s.project.join("layout.json"), "{ broken").expect("layout");
        let broken = ship_blocker(&s.project, &s.site).expect("broken layout");
        assert!(broken.starts_with(BAD_LAYOUT), "{broken}");
    }
}
