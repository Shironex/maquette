use std::fs;

use super::*;
use crate::assets::{AssetKind, NewAsset, Stage, add, add_version};
use crate::game::ship::ShipChange;
use crate::pipeline::template::for_template;
use crate::project::{PROJECT_VERSION, ProjectManifest, ProjectTemplate};

fn manifest(site: Option<&Path>) -> ProjectManifest {
    let mut m = ProjectManifest {
        version: PROJECT_VERSION,
        id: "study".into(),
        name: "Study".into(),
        template: ProjectTemplate::Room,
        target: Default::default(),
        game: None,
        site_repo: site.map(|s| s.display().to_string()),
        site_repo_name: None,
        site_repo_state: Default::default(),
        source_folder: None,
        created_at: "2026-10-27T09:30:00.000Z".into(),
        assets: Vec::new(),
    };
    for name in ["Desk", "Reading armchair"] {
        let new = NewAsset {
            name: name.into(),
            kind: AssetKind::Prop,
            stage: Stage::Blender,
        };
        let id = add(&mut m, new).expect("asset").id;
        let file = format!("models/{}_v1.glb", id.replace('-', "_"));
        add_version(&mut m, &id, file, None).expect("version");
    }
    m
}

#[test]
fn a_project_without_ship_json_ships_as_it_always_did() {
    let dir = tempfile::tempdir().expect("dir");
    let settings = load(dir.path()).expect("load");
    assert_eq!(settings, ShipSettings::default());
    assert_eq!(settings.style, ShipStyle::Scene);
    assert_eq!(settings.textures, TextureFormat::Ktx2);
    assert!(!settings.stable_names() && !settings.own_origin() && !settings.size_guard());
    assert_eq!(settings.origin_word(), "scene");
    // An older or partial file loads with the same defaults.
    fs::write(path(dir.path()), r#"{ "version": 1 }"#).expect("write");
    assert_eq!(load(dir.path()).expect("load"), ShipSettings::default());
}

#[test]
fn update_saves_only_real_choices_and_refuses_bad_names() {
    let dir = tempfile::tempdir().expect("dir");
    let mut change = ShipSettingsChange {
        style: Some(ShipStyle::Models),
        textures: Some(TextureFormat::Webp),
        ..ShipSettingsChange::default()
    };
    change.assets.insert(
        "desk".into(),
        AssetShip {
            choice: ShipChoice::Never,
            file: Some(" writing_desk.glb ".into()),
        },
    );
    change.assets.insert("lamp".into(), AssetShip::default());
    let saved = update(dir.path(), change).expect("save");
    assert_eq!(
        saved.file("desk"),
        Some("writing_desk"),
        "trimmed, without .glb"
    );
    assert!(
        !saved.assets.contains_key("lamp"),
        "defaults are not stored"
    );
    assert!(saved.own_origin() && saved.stable_names() && saved.size_guard());
    assert_eq!(load(dir.path()).expect("load"), saved);

    for (asset, file) in [
        ("desk", "Writing Desk"),
        ("desk", "../up"),
        ("lamp", "writing_desk"),
    ] {
        let mut bad = ShipSettingsChange::default();
        bad.assets.insert(
            asset.into(),
            AssetShip {
                choice: ShipChoice::Auto,
                file: Some(file.into()),
            },
        );
        if asset == "lamp" {
            // Two assets on one file name.
            assert!(update(dir.path(), bad).is_err(), "{file}");
        } else {
            assert_eq!(update(dir.path(), bad).expect_err(file).code(), "invalid");
        }
    }
    let quality = ShipSettingsChange {
        quality: Some(0),
        ..ShipSettingsChange::default()
    };
    assert!(update(dir.path(), quality).is_err());
    assert_eq!(
        load(dir.path()).expect("load"),
        saved,
        "refused changes write nothing"
    );
}

#[test]
fn stable_names_keep_one_file_per_asset_across_versions() {
    let mut m = manifest(None);
    add_version(&mut m, "desk", "models/desk_v2.glb".into(), None).expect("v2");
    let scene = crate::assets::shipped_names(&m, &ShipSettings::default());
    assert!(
        scene.is_empty(),
        "versioned files already carry their names"
    );
    let models = ShipSettings {
        style: ShipStyle::Models,
        ..ShipSettings::default()
    };
    let stable = crate::assets::shipped_names(&m, &models);
    assert_eq!(stable.get("desk_v2").map(String::as_str), Some("desk"));
    assert_eq!(
        stable.get("reading_armchair_v1").map(String::as_str),
        Some("reading_armchair")
    );
    assert!(
        !stable.contains_key("desk_v1"),
        "only the newest version takes the name"
    );
}

/// A site with a hand made desk and an exported desk and armchair.
fn scene(root: &Path) -> (ProjectManifest, std::path::PathBuf, std::path::PathBuf) {
    let project = root.join("project");
    let site = root.join("site");
    let models = site.join("apps/landing/public/room/models");
    fs::create_dir_all(project.join("web/dist")).expect("dist");
    fs::create_dir_all(&models).expect("models");
    crate::site::save_target(
        &site,
        &crate::site::SiteTarget {
            models: "apps/landing/public/room/models".into(),
            posters: None,
            data: None,
        },
    )
    .expect("target");
    fs::write(models.join("desk.glb"), b"hand made").expect("desk");
    fs::write(models.join("lamp.glb"), b"the site's own lamp").expect("lamp");
    fs::write(project.join("web/dist/desk.glb"), b"maquette desk, larger").expect("desk");
    fs::write(project.join("web/dist/reading_armchair.glb"), b"chair").expect("chair");
    let mut change = ShipSettingsChange {
        style: Some(ShipStyle::Models),
        ..ShipSettingsChange::default()
    };
    change.assets.clear();
    update(&project, change).expect("ship");
    (manifest(Some(&site)), project, models)
}

#[test]
fn the_plan_holds_back_site_files_maquette_did_not_make() {
    let dir = tempfile::tempdir().expect("dir");
    let (m, project, _) = scene(dir.path());
    let def = for_template(ProjectTemplate::Room);
    let plan = plan(&project, &m, &def).expect("plan");
    assert!(plan.exported);
    assert_eq!(plan.target.models, "apps/landing/public/room/models");
    assert!(plan.target.configured && plan.target.models_exist);
    let by = |f: &str| plan.files.iter().find(|p| p.file == f).expect(f);
    assert_eq!(by("reading_armchair.glb").change, ShipChange::New);
    assert!(by("reading_armchair.glb").ships);
    let desk = by("desk.glb");
    assert_eq!(desk.change, ShipChange::HandMade);
    assert!(!desk.ships);
    assert!(
        desk.held
            .as_deref()
            .is_some_and(|h| h.contains("not made by Maquette"))
    );
    assert_eq!(desk.before_bytes, Some(9.0));
    assert_eq!(desk.asset.as_deref(), Some("desk"));
    assert_eq!(plan.left.len(), 1);
    assert_eq!(plan.left[0].file, "lamp.glb");
    assert!(!plan.left[0].removed);
}

#[test]
fn always_replaces_and_the_size_guard_holds_a_larger_remake() {
    let dir = tempfile::tempdir().expect("dir");
    let (m, project, models) = scene(dir.path());
    let def = for_template(ProjectTemplate::Room);
    let mut always = ShipSettingsChange::default();
    always.assets.insert(
        "desk".into(),
        AssetShip {
            choice: ShipChoice::Always,
            file: None,
        },
    );
    update(&project, always).expect("always");
    let desk = plan(&project, &m, &def).expect("plan").files.remove(0);
    assert_eq!(desk.file, "desk.glb");
    assert!(desk.ships && desk.held.is_none());

    // Once Maquette shipped it, a larger remake is held only by the guard.
    let key = models.display().to_string();
    crate::game::ship::add_to_record(&project, &key, &["desk.glb".to_string()]).expect("record");
    let mut auto = ShipSettingsChange::default();
    auto.assets.insert("desk".into(), AssetShip::default());
    update(&project, auto).expect("auto");
    let desk = plan(&project, &m, &def).expect("plan").files.remove(0);
    assert_eq!(desk.change, ShipChange::Replaced);
    assert!(desk.held.as_deref().is_some_and(|h| h.contains("smaller")));
    let scene_style = ShipSettingsChange {
        style: Some(ShipStyle::Scene),
        ..ShipSettingsChange::default()
    };
    update(&project, scene_style).expect("scene");
    // A scene has no size guard, so the same file ships.
    let plan = plan(&project, &m, &def).expect("plan");
    assert!(
        plan.files.iter().any(|f| f.file == "desk.glb" && f.ships),
        "no guard for a scene"
    );
}

#[test]
fn the_size_report_compares_source_and_shipped() {
    let dir = tempfile::tempdir().expect("dir");
    let (m, project, _) = scene(dir.path());
    fs::create_dir_all(project.join("models")).expect("models");
    fs::write(project.join("models/desk_v1.glb"), vec![0u8; 4096]).expect("source");
    let def = for_template(ProjectTemplate::Room);
    let report = size_report(&project, &m, &def);
    assert_eq!(report.rows.len(), 2);
    let desk = &report.rows[0];
    assert_eq!(desk.shipped_file, "desk.glb");
    assert_eq!(desk.source_bytes, Some(4096.0));
    assert_eq!(desk.shipped_bytes, Some(21.0));
    assert_eq!(desk.source_triangles, None, "not a real model");
    assert!(
        report
            .csv
            .lines()
            .nth(1)
            .is_some_and(|l| l.starts_with("desk,Desk,1,models/desk_v1.glb,4096,"))
    );
    assert!(
        report.markdown.contains("| Desk | 4 KB | 0 KB |"),
        "{}",
        report.markdown
    );
    assert_eq!(
        report.totals.models, 0,
        "rows without triangles stay out of the totals"
    );
}
