//! Runs the Ship starter (`starter/ship.mjs`) with node against a scratch
//! project and site. Skipped where node is not on PATH.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

use crate::toolchain::{ProbeEnv, find_on_path};

const SCRIPT: &str = include_str!("../../starter/ship.mjs");

struct Scratch {
    _dir: tempfile::TempDir,
    project: PathBuf,
    site: PathBuf,
}

fn scratch() -> Scratch {
    let dir = tempfile::tempdir().expect("dir");
    let project = dir.path().join("project");
    let site = dir.path().join("site");
    fs::create_dir_all(project.join("web/dist")).expect("dist");
    fs::create_dir_all(project.join("web/posters")).expect("posters");
    fs::create_dir_all(project.join("tools")).expect("tools");
    fs::create_dir_all(&site).expect("site");
    fs::write(project.join("tools/ship.mjs"), SCRIPT).expect("script");
    fs::write(project.join("web/dist/desk_lamp_v1.glb"), b"glb").expect("glb");
    fs::write(project.join("web/posters/hero.png"), b"png").expect("png");
    fs::write(
        project.join("web/posters/stops.json"),
        r#"{ "stops": [{ "file": "hero.png", "label": "Hero" }] }"#,
    )
    .expect("stops");
    let index = json!({
        "version": 1,
        "project": "bench",
        "models": {
            "desk-lamp": { "name": "Desk lamp", "file": "desk_lamp_v1.glb", "version": 1 },
            "rock": { "name": "Rock", "file": "rock_v1.glb", "version": 1 }
        }
    });
    fs::write(project.join("index.json"), index.to_string()).expect("index");
    Scratch {
        _dir: dir,
        project,
        site,
    }
}

fn layout(placed: bool) -> Value {
    let models = if placed {
        json!({ "desk-lamp": { "position": [0.4, 0, 0], "rotation": [0, 30, 0], "scale": 1 } })
    } else {
        json!({})
    };
    json!({
        "version": 1,
        "models": models,
        "stops": [{ "id": "hero", "name": "Hero", "position": [0, 1, 2], "target": [0, 0, 0],
                    "fov": 35, "models": null }],
        "desk": null,
        "timeline": { "explode": { "desk-lamp": { "distance": 0.2, "easing": "ease-in-out",
                      "keys": [{ "stop": "hero", "amount": 1 }] } },
                      "paths": { "cord": { "easing": "linear",
                      "keys": [{ "stop": "hero", "amount": 0.5 }] } } },
        "paths": [{ "id": "cord", "name": "Cord", "curve": "polyline",
                    "points": [[0, 0, 0], [0.4, 0, 0]], "start": null, "end": "desk-lamp",
                    "width": 0.01, "color": "#3a3936", "glow": "#ffe08a" }]
    })
}

fn ship(s: &Scratch) -> Option<Output> {
    let node = find_on_path(&["node"], &ProbeEnv::from_system())?;
    let args = [
        "tools/ship.mjs",
        "--models",
        "web/dist",
        "--posters",
        "web/posters",
        "--site",
        &s.site.display().to_string(),
        "--index",
        "index.json",
        "--layout",
        "layout.json",
        "--materials",
        "materials.json",
    ]
    .map(str::to_string);
    Some(
        Command::new(node)
            .args(args)
            .current_dir(&s.project)
            .output()
            .expect("node runs"),
    )
}

fn site_config(site: &Path, config: &Value) {
    fs::write(site.join(super::CONFIG_FILE), config.to_string()).expect("config");
}

fn read(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read")).expect("json")
}

#[test]
fn ship_writes_the_scene_data_the_site_asks_for() {
    let s = scratch();
    fs::write(s.project.join("layout.json"), layout(true).to_string()).expect("layout");
    let tweaks =
        json!({ "version": 1, "models": { "desk-lamp": { "all": { "tint": "#3a3936" } } } });
    fs::write(s.project.join("materials.json"), tweaks.to_string()).expect("tweaks");
    site_config(
        &s.site,
        &json!({ "version": 1, "models": "public/scene/models", "posters": null,
                 "data": "src/scene/shipped.json" }),
    );
    let Some(out) = ship(&s) else { return };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        s.site
            .join("public/scene/models/desk_lamp_v1.glb")
            .is_file()
    );
    assert!(
        !s.site.join("public/posters").exists(),
        "posters: null copies none"
    );
    let data = read(&s.site.join("src/scene/shipped.json"));
    assert_eq!(data["version"], 1);
    assert_eq!(data["project"], "bench");
    assert_eq!(
        data["models"],
        json!({ "desk-lamp": { "name": "Desk lamp", "file": "desk_lamp_v1.glb", "inPlace": true } }),
        "a model not in the build is left out"
    );
    assert_eq!(data["layout"], layout(true));
    assert_eq!(data["materials"], tweaks);
    assert_eq!(data["posters"], Value::Null);
}

#[test]
fn ship_lists_the_posters_it_copies() {
    let s = scratch();
    fs::write(s.project.join("layout.json"), layout(true).to_string()).expect("layout");
    site_config(
        &s.site,
        &json!({ "version": 1, "posters": "public/stills", "data": "data/scene.json" }),
    );
    let Some(out) = ship(&s) else { return };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(s.site.join("public/models/desk_lamp_v1.glb").is_file());
    assert!(s.site.join("public/stills/hero.png").is_file());
    let data = read(&s.site.join("data/scene.json"));
    assert_eq!(
        data["posters"],
        json!({ "folder": "public/stills", "stops": [{ "file": "hero.png", "label": "Hero" }] })
    );
    assert_eq!(data["materials"], json!({ "version": 1, "models": {} }));
}

#[test]
fn ship_stops_before_an_empty_layout_wipes_the_site() {
    let s = scratch();
    fs::write(s.project.join("layout.json"), layout(false).to_string()).expect("layout");
    site_config(&s.site, &json!({ "version": 1, "data": "shipped.json" }));
    let Some(out) = ship(&s) else { return };
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("0 placed models"));
    assert!(!s.site.join("public").exists(), "nothing was copied");
    assert!(!s.site.join("shipped.json").exists());
}

#[test]
fn a_site_without_a_config_gets_the_files_as_before() {
    let s = scratch();
    let Some(out) = ship(&s) else { return };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(s.site.join("public/models/desk_lamp_v1.glb").is_file());
    assert!(s.site.join("public/posters/stops.json").is_file());
    assert_eq!(
        fs::read_dir(&s.site)
            .expect("site")
            .filter_map(Result::ok)
            .count(),
        1,
        "only public/ was written"
    );
}

#[test]
fn ship_refuses_a_folder_outside_the_site() {
    let s = scratch();
    fs::write(s.project.join("layout.json"), layout(true).to_string()).expect("layout");
    site_config(&s.site, &json!({ "version": 1, "models": "../elsewhere" }));
    let Some(out) = ship(&s) else { return };
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("outside the site repo"));
}

#[test]
fn ship_copies_only_the_models_its_data_lists_and_drops_the_ones_it_shipped_before() {
    let s = scratch();
    fs::write(s.project.join("layout.json"), layout(true).to_string()).expect("layout");
    site_config(
        &s.site,
        &json!({ "version": 1, "models": "public/scene/models", "data": "src/scene/shipped.json" }),
    );
    // An old export of a layout entry that is gone, still in the build.
    fs::write(s.project.join("web/dist/desk.glb"), b"stale").expect("stale");
    let models = s.site.join("public/scene/models");
    fs::create_dir_all(&models).expect("models");
    fs::write(models.join("logo.glb"), b"the site's own").expect("site file");
    let Some(out) = ship(&s) else { return };
    let log = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(models.join("desk_lamp_v1.glb").is_file());
    assert!(
        !models.join("desk.glb").exists(),
        "only the models in the data ship"
    );
    assert!(
        log.contains("desk.glb is in no shipped asset, not copied"),
        "{log}"
    );
    assert!(log.contains("shipped 1 models"), "{log}");
    let record = read(&s.project.join("web/ship-record.json"));
    assert_eq!(record["version"], 1);

    // The lamp leaves the scene: Ship removes the file it shipped, and only it.
    let index = json!({ "version": 1, "project": "bench", "models": {
        "rock": { "name": "Rock", "file": "rock_v1.glb", "version": 1 } } });
    fs::write(s.project.join("index.json"), index.to_string()).expect("index");
    fs::write(s.project.join("web/dist/rock_v1.glb"), b"glb").expect("rock");
    let Some(out) = ship(&s) else { return };
    let log = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(models.join("rock_v1.glb").is_file());
    assert!(!models.join("desk_lamp_v1.glb").exists(), "{log}");
    assert!(
        log.contains("removed") && log.contains("desk_lamp_v1.glb"),
        "{log}"
    );
    assert!(
        models.join("logo.glb").is_file(),
        "a file Ship never shipped stays"
    );
    let data = read(&s.site.join("src/scene/shipped.json"));
    assert_eq!(data["models"].as_object().map(|m| m.len()), Some(1));
}
