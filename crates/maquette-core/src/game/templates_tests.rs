//! A game template fills the settings a person can edit, shows what it will
//! change first, and leaves every file where it is.

use std::fs;

use super::tests::{game_project, glb_game};
use super::*;
use crate::assets::AssetKind;
use crate::project;

/// Every file under `dir`, with its bytes.
fn tree(dir: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(folder) = stack.pop() {
        for entry in fs::read_dir(&folder).expect("read").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let bytes = fs::read(&path).expect("bytes");
                out.push((path, bytes));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_template_passes_the_checks_a_hand_edit_passes() {
    let (_ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let base = settings_of(&manifest).expect("game");
    let list = game_templates();
    assert_eq!(list.len(), 4);
    for template in list {
        let mut next = base.clone();
        next.format = template.format;
        next.budgets = crate::settings::game::merge_budgets(&base.budgets, &template.budgets);
        validate(&next).expect(&template.name);
        assert!(!template.budgets.is_empty(), "{}", template.name);
    }
}

#[test]
fn a_preview_changes_nothing_and_lists_only_what_differs() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");

    let props = preview_template(&dir, &manifest, GameTemplateId::PropSet).expect("preview");
    assert!(!props.changes, "a new game already has the prop numbers");
    assert!(props.budgets.is_empty());

    let kit = preview_template(&dir, &manifest, GameTemplateId::LevelKitObj).expect("preview");
    assert!(kit.changes);
    assert_eq!(
        (kit.format_before, kit.format_after),
        (GameFormat::Glb, GameFormat::Obj)
    );
    assert_eq!(kit.budgets.len(), 1);
    assert_eq!(kit.budgets[0].folder, "kit");
    assert_eq!(kit.budgets[0].before, None);
    assert_eq!(kit.budgets[0].after.triangles, 5_000);
    assert!(
        kit.notes.iter().any(|n| n.contains("are not .obj")),
        "the game's own files are .glb: {:?}",
        kit.notes
    );
    assert!(kit.notes.iter().any(|n| n.contains("props/")));

    let again = project::load(&ws.root, &ws.config.layout, &manifest.id).expect("load");
    assert_eq!(again.game, manifest.game, "a preview saves nothing");
}

#[test]
fn applying_fills_the_settings_keeps_other_rows_and_touches_no_file() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let before = tree(&ws.repo);

    let saved = apply_template(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        GameTemplateId::LevelKitObj,
    )
    .expect("apply");

    let settings = settings_of(&saved).expect("game");
    let old = settings_of(&manifest).expect("game");
    assert_eq!(settings.format, GameFormat::Obj);
    let folders: Vec<&str> = settings.budgets.iter().map(|b| b.folder.as_str()).collect();
    assert_eq!(folders, ["props", "characters", "kit"]);
    assert_eq!(settings.export, old.export, "textures and cap stay");
    assert_eq!(settings.bake, old.bake);
    assert_eq!(settings.models_dir, old.models_dir);
    assert_eq!(
        saved.assets.len(),
        manifest.assets.len(),
        "no starter assets"
    );
    assert_eq!(tree(&ws.repo), before, "the game repo is untouched");

    // Still a plain setting a person changes afterwards.
    let edited = save_settings(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        GameSettingsChange {
            format: Some(GameFormat::Glb),
            ..GameSettingsChange::default()
        },
    )
    .expect("edit");
    assert_eq!(settings_of(&edited).expect("game").format, GameFormat::Glb);
}

#[test]
fn a_shipped_model_keeps_its_file_and_an_unshipped_one_follows() {
    let (ws, manifest) = game_project(
        |d| {
            let repo = d.join("empty-game");
            fs::create_dir_all(repo.join("assets/models")).expect("models");
            repo
        },
        GameFormat::Glb,
    );
    let current = project::modify(&ws.root, &ws.config.layout, &manifest.id, |m| {
        for name in ["Shipped crate", "Fresh crate"] {
            let asset = crate::assets::add(
                m,
                crate::NewAsset {
                    name: name.into(),
                    kind: AssetKind::Prop,
                    stage: crate::Stage::Web,
                },
            )?;
            crate::assets::add_version(m, &asset.id, "models/crate_v1.glb".to_string(), None)?;
        }
        Ok(())
    })
    .expect("assets");
    let dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    ship::add_to_record(&dir, "assets/models", &["shipped_crate.glb".to_string()]).expect("record");

    let preview = preview_template(&dir, &current, GameTemplateId::LevelKitObj).expect("preview");
    assert_eq!(preview.kept_files, ["shipped_crate.glb"]);
    assert_eq!(preview.new_paths.len(), 1);
    assert_eq!(preview.new_paths[0].to, "kit/fresh_crate.obj");

    let saved = apply_template(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        GameTemplateId::LevelKitObj,
    )
    .expect("apply");
    let settings = settings_of(&saved).expect("game");
    let file = |id: &str| {
        let asset = saved.assets.iter().find(|a| a.id == id).expect("asset");
        file_of(settings, asset)
    };
    assert_eq!(file("shipped-crate"), "shipped_crate.glb");
    assert_eq!(file("fresh-crate"), "kit/fresh_crate.obj");
    let record = ship::read_record(&dir);
    assert!(
        record.remove.is_empty(),
        "Ship removes nothing after a template"
    );
}

#[test]
fn a_website_has_no_templates() {
    let (ws, mut manifest) = game_project(glb_game, GameFormat::Glb);
    let dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    manifest.target = ProjectTarget::Website;
    assert!(preview_template(&dir, &manifest, GameTemplateId::PropSet).is_err());
}

#[test]
fn a_new_game_can_start_from_a_template() {
    let repo = tempfile::tempdir().expect("dir");
    let settings = new_settings(
        &NewGame {
            format: GameFormat::Glb,
            models_dir: "assets/models".into(),
            template: Some(GameTemplateId::Character),
        },
        Some(repo.path()),
    )
    .expect("settings");
    let folders: Vec<&str> = settings.budgets.iter().map(|b| b.folder.as_str()).collect();
    assert_eq!(folders, ["", "characters"]);
    assert!(settings.models.is_empty());
}
