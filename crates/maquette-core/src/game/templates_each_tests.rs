//! Every built-in template: its numbers, its preview and what apply saves, and
//! where a scene lands once a game has an environment or kit folder.

use super::templates_tests::tree;
use super::tests::{game_project, glb_game};
use super::*;
use crate::assets::AssetKind;
use crate::project;
use crate::settings::game::game_template;

/// id, texture format, cap, drop maps, then the rows (folder, triangles, texture).
type Expected = (
    GameTemplateId,
    GameTextures,
    u32,
    bool,
    &'static [(&'static str, u32, u32)],
);

const NEW_TEMPLATES: [Expected; 7] = [
    (
        GameTemplateId::WebGame,
        GameTextures::Jpg,
        1024,
        false,
        &[
            ("props", 10_000, 1024),
            ("characters", 25_000, 1024),
            ("kit", 3_000, 512),
        ],
    ),
    (
        GameTemplateId::Godot,
        GameTextures::Png,
        2048,
        false,
        &[
            ("props", 20_000, 2048),
            ("characters", 50_000, 2048),
            ("environment", 30_000, 2048),
        ],
    ),
    (
        GameTemplateId::Unity,
        GameTextures::Png,
        2048,
        false,
        &[
            ("props", 15_000, 1024),
            ("characters", 40_000, 2048),
            ("kit", 5_000, 1024),
        ],
    ),
    (
        GameTemplateId::Unreal,
        GameTextures::Png,
        4096,
        false,
        &[
            ("props", 40_000, 2048),
            ("characters", 80_000, 4096),
            ("environment", 60_000, 2048),
        ],
    ),
    (
        GameTemplateId::Mobile,
        GameTextures::Jpg,
        1024,
        false,
        &[
            ("props", 4_000, 512),
            ("characters", 15_000, 1024),
            ("kit", 2_000, 512),
        ],
    ),
    (
        GameTemplateId::VrStandalone,
        GameTextures::Jpg,
        1024,
        false,
        &[
            ("props", 5_000, 1024),
            ("characters", 20_000, 1024),
            ("kit", 2_000, 512),
        ],
    ),
    (
        GameTemplateId::GameJam,
        GameTextures::Jpg,
        1024,
        true,
        &[("", 20_000, 1024)],
    ),
];

/// Every new id carries the numbers it was agreed with, in glb, never a new
/// format, and the preview of it says what apply then saves.
#[test]
fn each_new_template_has_its_numbers_and_apply_matches_its_preview() {
    for (id, textures, cap, drop_maps, rows) in NEW_TEMPLATES {
        let t = game_template(id);
        assert_eq!(t.format, GameFormat::Glb, "{id:?}");
        assert_eq!(t.textures, Some(textures), "{id:?}");
        assert_eq!(t.texture_cap, Some(cap), "{id:?}");
        assert_eq!(t.drop_maps, Some(drop_maps), "{id:?}");
        let got: Vec<(&str, u32, u32)> = t
            .budgets
            .iter()
            .map(|b| (b.folder.as_str(), b.triangles, b.texture))
            .collect();
        assert_eq!(got, rows, "{id:?}");
        assert!(!t.why.is_empty(), "{id:?}");
        assert!(!t.why.contains(['\u{2014}', '\u{2013}']), "{id:?}");

        let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
        let dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
        let before = tree(&ws.repo);
        let preview = preview_template(&dir, &manifest, id).expect("preview");
        assert!(preview.changes, "{id:?}");
        assert_eq!(
            preview.export_before,
            settings_of(&manifest).expect("game").export
        );
        assert_eq!(preview.export_after.textures, textures, "{id:?}");
        assert_eq!(preview.export_after.texture_cap, cap, "{id:?}");
        assert_eq!(preview.export_after.drop_maps, drop_maps, "{id:?}");

        let saved = apply_template(&ws.root, &ws.config.layout, &manifest.id, id).expect("apply");
        let settings = settings_of(&saved).expect("game");
        assert_eq!(
            settings.export, preview.export_after,
            "{id:?}: apply matches the preview"
        );
        assert_eq!(settings.format, preview.format_after, "{id:?}");
        for change in &preview.budgets {
            let row = settings.budgets.iter().find(|b| b.folder == change.folder);
            assert_eq!(row, Some(&change.after), "{id:?} {}", change.folder);
        }
        for row in &manifest.game.as_ref().expect("game").budgets {
            let kept = settings.budgets.iter().any(|b| b.folder == row.folder);
            assert!(kept, "{id:?}: the {} row stays", row.folder);
        }
        assert_eq!(
            saved.assets.len(),
            manifest.assets.len(),
            "{id:?}: no assets"
        );
        assert_eq!(tree(&ws.repo), before, "{id:?}: no file moves");
    }
}

#[test]
fn a_shape_template_leaves_textures_alone_and_a_second_apply_changes_nothing() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let kit = preview_template(&dir, &manifest, GameTemplateId::LevelKit).expect("preview");
    assert_eq!(kit.export_before, kit.export_after);

    let saved = apply_template(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        GameTemplateId::Unreal,
    )
    .expect("apply");
    let again = preview_template(&dir, &saved, GameTemplateId::Unreal).expect("preview");
    assert!(!again.changes, "{again:?}");
    assert_eq!(again.export_before, again.export_after);
}

#[test]
fn the_notes_name_the_maps_and_the_jpg_and_top_level_effects() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let jam = preview_template(&dir, &manifest, GameTemplateId::GameJam).expect("preview");
    assert!(
        jam.notes.iter().any(|n| n.contains("metal-roughness")),
        "{:?}",
        jam.notes
    );
    assert!(
        jam.notes.iter().any(|n| n.contains("JPG")),
        "{:?}",
        jam.notes
    );
    assert!(
        jam.notes.iter().any(|n| n.contains("top level")),
        "{:?}",
        jam.notes
    );
    let godot = preview_template(&dir, &manifest, GameTemplateId::Godot).expect("preview");
    assert!(
        godot.notes.iter().all(|n| !n.contains("JPG")),
        "{:?}",
        godot.notes
    );
}

/// A scene goes to an environment or kit folder when the game has one, and
/// to props as before when it has neither.
#[test]
fn a_scene_goes_to_an_environment_or_kit_folder_not_props() {
    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let with_scene = project::modify(&ws.root, &ws.config.layout, &manifest.id, |m| {
        let asset = crate::assets::add(
            m,
            crate::NewAsset {
                name: "Crypt hall".into(),
                kind: AssetKind::Scene,
                stage: crate::Stage::Web,
            },
        )?;
        crate::assets::add_version(m, &asset.id, "models/hall_v1.glb".to_string(), None)?;
        Ok(())
    })
    .expect("scene");
    let file = |m: &crate::project::ProjectManifest| {
        let settings = settings_of(m).expect("game");
        file_of(settings, &m.assets[m.assets.len() - 1])
    };
    assert_eq!(
        file(&with_scene),
        "props/crypt_hall.glb",
        "no environment or kit row yet"
    );

    let dir = project::dir(&ws.root, &ws.config.layout, &manifest.id).expect("dir");
    let preview = preview_template(&dir, &with_scene, GameTemplateId::Godot).expect("preview");
    assert_eq!(
        preview.new_paths.len(),
        0,
        "a model keeps the folder it was going to"
    );

    let godot = apply_template(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        GameTemplateId::Godot,
    )
    .expect("apply");
    assert_eq!(
        file(&godot),
        "props/crypt_hall.glb",
        "its path is kept once it has one"
    );

    let fresh = project::modify(&ws.root, &ws.config.layout, &manifest.id, |m| {
        crate::assets::add(
            m,
            crate::NewAsset {
                name: "Moon yard".into(),
                kind: AssetKind::Scene,
                stage: crate::Stage::Web,
            },
        )?;
        Ok(())
    })
    .expect("fresh");
    assert_eq!(file(&fresh), "environment/moon_yard.glb");

    let (ws, manifest) = game_project(glb_game, GameFormat::Glb);
    let unity = apply_template(
        &ws.root,
        &ws.config.layout,
        &manifest.id,
        GameTemplateId::Unity,
    )
    .expect("apply");
    let kit_scene = project::modify(&ws.root, &ws.config.layout, &unity.id, |m| {
        crate::assets::add(
            m,
            crate::NewAsset {
                name: "Moon yard".into(),
                kind: AssetKind::Scene,
                stage: crate::Stage::Web,
            },
        )?;
        Ok(())
    })
    .expect("kit scene");
    assert_eq!(file(&kit_scene), "kit/moon_yard.glb");
}
