use std::path::Path;

use super::*;
use crate::inbox::{analyse::analyse_bytes, fixture};

fn checks(bytes: &[u8], kind: AssetKind, config: &WorkspaceConfig) -> Vec<Check> {
    run(
        &analyse_bytes(bytes, Path::new(".")).expect("analyse"),
        kind,
        config,
        SendNotes::default(),
        None,
    )
}

fn status(list: &[Check], id: &str) -> Option<CheckStatus> {
    list.iter().find(|c| c.id == id).map(|c| c.status)
}

#[test]
fn a_clean_prop_passes_everything() {
    let list = checks(
        &fixture::clean_prop(),
        AssetKind::Prop,
        &WorkspaceConfig::default(),
    );
    assert!(
        list.iter().all(|c| c.status == CheckStatus::Pass),
        "{list:#?}"
    );
    assert_eq!(worst(&list), CheckStatus::Pass);
}

#[test]
fn budgets_drive_the_triangle_and_texture_rows() {
    let mut config = WorkspaceConfig::default();
    config.budgets.tris_per_model = 10;
    let list = checks(&fixture::clean_prop(), AssetKind::Prop, &config);
    assert_eq!(status(&list, "triangles"), Some(CheckStatus::Warning));
    config.budgets.tris_per_model = 5;
    let list = checks(&fixture::clean_prop(), AssetKind::Prop, &config);
    assert_eq!(status(&list, "triangles"), Some(CheckStatus::Fail));
    let mut report = analyse_bytes(&fixture::clean_prop(), Path::new(".")).expect("analyse");
    report.textures[0].width = Some(4096);
    let list = run(
        &report,
        AssetKind::Prop,
        &WorkspaceConfig::default(),
        SendNotes::default(),
        None,
    );
    let row = list.iter().find(|c| c.id == "textures").expect("row");
    assert_eq!(row.status, CheckStatus::Warning);
    assert!(
        row.detail
            .as_deref()
            .unwrap_or_default()
            .contains("over the 2048 px default")
    );
}

#[test]
fn character_rows_cover_rig_and_finger_weights() {
    let config = WorkspaceConfig::default();
    let list = checks(&fixture::character(true), AssetKind::Character, &config);
    assert_eq!(status(&list, "rig"), Some(CheckStatus::Pass));
    let finger = list.iter().find(|c| c.id == "finger-weights").expect("row");
    assert_eq!(finger.status, CheckStatus::Fail);
    let detail = finger.detail.as_deref().unwrap_or_default();
    assert!(detail.starts_with("48 vertices"), "{detail}");
    assert!(detail.contains("LeftHandIndex1 (24)"));
    assert_eq!(status(&list, "animations"), Some(CheckStatus::Pass));
    assert_eq!(worst(&list), CheckStatus::Fail);

    let clean = checks(&fixture::character(false), AssetKind::Character, &config);
    assert_eq!(status(&clean, "finger-weights"), Some(CheckStatus::Pass));
    // Sleeve ends and chains hanging below the wrists belong to the hands.
    let cuffs = checks(
        &fixture::character_with_cuffs(),
        AssetKind::Character,
        &config,
    );
    assert_eq!(status(&cuffs, "finger-weights"), Some(CheckStatus::Pass));
    let mut still = analyse_bytes(&fixture::character(false), Path::new(".")).expect("analyse");
    still.animations.clear();
    let list = run(
        &still,
        AssetKind::Character,
        &config,
        SendNotes::default(),
        None,
    );
    assert_eq!(status(&list, "animations"), Some(CheckStatus::Warning));
    // The same rig on a prop is suspicious; no rig on a character fails.
    let prop = checks(&fixture::character(false), AssetKind::Prop, &config);
    assert_eq!(status(&prop, "rig"), Some(CheckStatus::Warning));
    let unrigged = checks(&fixture::clean_prop(), AssetKind::Character, &config);
    assert_eq!(status(&unrigged, "rig"), Some(CheckStatus::Fail));
}

#[test]
fn merged_objects_warn_except_in_scenes() {
    let config = WorkspaceConfig::default();
    let list = checks(&fixture::merged_islands(), AssetKind::Prop, &config);
    assert_eq!(status(&list, "merged"), Some(CheckStatus::Warning));
    assert_eq!(status(&list, "pbr"), Some(CheckStatus::Warning));
    let scene = checks(&fixture::merged_islands(), AssetKind::Scene, &config);
    assert_eq!(status(&scene, "merged"), Some(CheckStatus::Pass));
}

#[test]
fn a_prop_that_is_one_mesh_offers_a_split_into_parts() {
    let config = WorkspaceConfig::default();
    let list = checks(&fixture::merged_islands(), AssetKind::Prop, &config);
    let row = list.iter().find(|c| c.id == "merged").expect("merged");
    assert_eq!(row.title, "One mesh, no parts");
    assert!(
        row.detail
            .as_deref()
            .is_some_and(|d| d.starts_with("Motion states and explode tracks move named parts.")),
        "{row:?}"
    );
    assert_eq!(
        row.fixes[0],
        Fixup::SplitParts {
            by: crate::prepare::SplitBy::Loose,
            merge_under: crate::prepare::DEFAULT_MERGE_UNDER,
        },
        "by loose pieces with bits under 1% merged, first"
    );
    assert_eq!(row.fixes[0].describe(), "Split into parts");
    // A character keeps the older wording: its rig keeps the mesh whole.
    let character = checks(&fixture::merged_islands(), AssetKind::Character, &config);
    let row = character.iter().find(|c| c.id == "merged").expect("merged");
    assert!(row.title.starts_with("Looks like"), "{row:?}");
}

#[test]
fn a_send_without_pbr_does_not_warn_about_missing_maps() {
    let config = WorkspaceConfig::default();
    let mut report = analyse_bytes(&fixture::merged_islands(), Path::new(".")).expect("analyse");
    report.pbr.base_color = true;
    let sent = |maps| {
        let notes = SendNotes {
            maps,
            ..SendNotes::default()
        };
        run(&report, AssetKind::Scene, &config, notes, None)
    };
    assert_eq!(
        status(&sent(SentMaps::Any), "pbr"),
        Some(CheckStatus::Warning)
    );
    let list = sent(SentMaps::NoPbr);
    assert_eq!(status(&list, "pbr"), Some(CheckStatus::Pass));
    let row = list.iter().find(|c| c.id == "pbr").expect("row");
    assert_eq!(row.title, "PBR off, as sent");
    assert_eq!(worst(&list), CheckStatus::Pass);
    assert_eq!(
        status(&sent(SentMaps::NoTexture), "pbr"),
        Some(CheckStatus::Pass)
    );
}

#[test]
fn a_by_hand_tripo_character_with_base_color_only_gets_a_note() {
    let config = WorkspaceConfig::default();
    let mut report = analyse_bytes(&fixture::character(false), Path::new(".")).expect("analyse");
    report.pbr.metallic_roughness = false;
    report.pbr.normal = false;
    let by_hand = SendNotes {
        by_hand_tripo: true,
        ..SendNotes::default()
    };
    let list = run(&report, AssetKind::Character, &config, by_hand, None);
    let row = list.iter().find(|c| c.id == "pbr").expect("row");
    assert_eq!(row.status, CheckStatus::Pass);
    assert_eq!(
        row.detail.as_deref(),
        Some("Base color only, as Tripo's Smart Mesh exports it.")
    );
    // A prop by hand, or a character over the API, keeps the advice.
    for (kind, notes) in [
        (AssetKind::Prop, by_hand),
        (AssetKind::Character, SendNotes::default()),
    ] {
        let list = run(&report, kind, &config, notes, None);
        let row = list.iter().find(|c| c.id == "pbr").expect("row");
        assert_eq!(row.status, CheckStatus::Warning);
        assert!(
            row.detail
                .as_deref()
                .unwrap_or_default()
                .contains("Turn on PBR in the Tripo settings")
        );
    }
    // Without even a base color map it still warns.
    report.pbr.base_color = false;
    let list = run(&report, AssetKind::Character, &config, by_hand, None);
    assert_eq!(status(&list, "pbr"), Some(CheckStatus::Warning));
}

#[test]
fn detailed_textures_keep_the_warning_and_say_why() {
    let mut report = analyse_bytes(&fixture::clean_prop(), Path::new(".")).expect("analyse");
    report.textures[0].width = Some(4096);
    let notes = SendNotes {
        large_textures: true,
        ..SendNotes::default()
    };
    let list = run(
        &report,
        AssetKind::Prop,
        &WorkspaceConfig::default(),
        notes,
        None,
    );
    let row = list.iter().find(|c| c.id == "textures").expect("row");
    assert_eq!(row.status, CheckStatus::Warning);
    let detail = row.detail.as_deref().unwrap_or_default();
    assert!(detail.contains("sent with detailed textures"), "{detail}");
    assert!(detail.contains("Compress resizes them"));
}

fn fixes(list: &[Check], id: &str) -> Vec<Fixup> {
    list.iter()
        .find(|c| c.id == id)
        .map(|c| c.fixes.clone())
        .unwrap_or_default()
}

#[test]
fn checks_offer_the_fixups_that_answer_them() {
    let mut config = WorkspaceConfig::default();
    config.budgets.tris_per_model = 5000;
    let list = checks(&fixture::messy_prop(), AssetKind::Prop, &config);
    assert_eq!(status(&list, "triangles"), Some(CheckStatus::Fail));
    assert_eq!(
        fixes(&list, "triangles"),
        [Fixup::Decimate { triangles: 5000 }],
        "decimate to the budget"
    );
    assert!(
        fixes(&list, "textures").is_empty(),
        "a 1024 px texture is under the 2K default, so no fix"
    );
    assert_eq!(status(&list, "origin"), Some(CheckStatus::Warning));
    assert_eq!(fixes(&list, "origin"), [Fixup::OriginBottom]);
    assert_eq!(
        fixes(&list, "objects"),
        [Fixup::Merge { by_material: false }, Fixup::ApplyTransforms],
        "three objects under a scaled root"
    );

    config.compression.texture_size = crate::workspace::TextureSize::K1;
    let mut report = analyse_bytes(&fixture::messy_prop(), Path::new(".")).expect("analyse");
    report.textures[0].width = Some(2048);
    let list = run(
        &report,
        AssetKind::Prop,
        &config,
        SendNotes::default(),
        None,
    );
    assert_eq!(
        fixes(&list, "textures"),
        [Fixup::ShrinkTextures { max_size: 1024 }]
    );

    let clean = checks(
        &fixture::clean_prop(),
        AssetKind::Prop,
        &WorkspaceConfig::default(),
    );
    assert!(clean.iter().all(|c| c.fixes.is_empty()), "{clean:#?}");
    assert_eq!(status(&clean, "origin"), Some(CheckStatus::Pass));
    assert!(status(&clean, "objects").is_none(), "one object, no row");

    let merged = checks(
        &fixture::merged_islands(),
        AssetKind::Prop,
        &WorkspaceConfig::default(),
    );
    assert!(matches!(
        fixes(&merged, "merged").as_slice(),
        [
            Fixup::SplitParts {
                by: crate::prepare::SplitBy::Loose,
                ..
            },
            Fixup::RemoveLoose { .. }
        ]
    ));
    let rigged = checks(
        &fixture::character(false),
        AssetKind::Character,
        &WorkspaceConfig::default(),
    );
    assert!(
        status(&rigged, "objects").is_none(),
        "a rig keeps its meshes"
    );
    let scene = checks(
        &fixture::messy_prop(),
        AssetKind::Scene,
        &WorkspaceConfig::default(),
    );
    assert!(
        status(&scene, "origin").is_none(),
        "scenes are left as they are"
    );
}

#[test]
fn a_game_folder_budget_replaces_the_flat_one_and_is_named() {
    let mut report = analyse_bytes(&fixture::clean_prop(), Path::new(".")).expect("analyse");
    let config = WorkspaceConfig::default();
    let props = FolderBudget {
        folder: "props".into(),
        triangles: 12_000,
        texture: 1024,
    };
    let list = run(
        &report,
        AssetKind::Prop,
        &config,
        SendNotes::default(),
        Some(&props),
    );
    let tris = list.iter().find(|c| c.id == "triangles").expect("tris");
    assert_eq!(tris.status, CheckStatus::Pass);
    assert_eq!(
        tris.detail.as_deref(),
        Some("Within the props budget, 12k tris.")
    );

    // 14.6k triangles: inside the flat 60k, over the props 12k.
    report.triangles = 14_600;
    report.textures[0].width = Some(2048);
    let flat = run(
        &report,
        AssetKind::Prop,
        &config,
        SendNotes::default(),
        None,
    );
    assert_eq!(status(&flat, "triangles"), Some(CheckStatus::Pass));
    assert_eq!(status(&flat, "textures"), Some(CheckStatus::Pass));
    let list = run(
        &report,
        AssetKind::Prop,
        &config,
        SendNotes::default(),
        Some(&props),
    );
    let tris = list.iter().find(|c| c.id == "triangles").expect("tris");
    assert_eq!(tris.status, CheckStatus::Warning);
    assert!(
        tris.detail
            .as_deref()
            .is_some_and(|d| d.starts_with("Over the props budget, 12k tris.")),
        "{tris:#?}"
    );
    assert_eq!(tris.fixes, vec![Fixup::Decimate { triangles: 12_000 }]);
    let tex = list.iter().find(|c| c.id == "textures").expect("textures");
    assert_eq!(tex.status, CheckStatus::Warning);
    assert!(
        tex.detail
            .as_deref()
            .is_some_and(|d| d.contains("over the props budget, 1024 px")),
        "{tex:#?}"
    );
    assert_eq!(tex.fixes, vec![Fixup::ShrinkTextures { max_size: 1024 }]);

    // The model folder's top level reads as such.
    let top = FolderBudget {
        folder: String::new(),
        triangles: 20_000,
        texture: 1024,
    };
    let list = run(
        &report,
        AssetKind::Prop,
        &config,
        SendNotes::default(),
        Some(&top),
    );
    let tris = list.iter().find(|c| c.id == "triangles").expect("tris");
    assert_eq!(
        tris.detail.as_deref(),
        Some("Within the model folder's budget, 20k tris.")
    );
}

#[test]
fn a_rigged_character_gets_a_vrm_row_that_never_fails() {
    let config = WorkspaceConfig::default();
    let clean = checks(&fixture::character(false), AssetKind::Character, &config);
    let row = clean.iter().find(|c| c.id == "vrm").expect("vrm row");
    assert_eq!(row.status, CheckStatus::Pass);
    assert_eq!(row.title, "VRM ready, 52 of 55 bones");
    // It sits right after the rig row.
    let at = |id: &str| clean.iter().position(|c| c.id == id);
    assert_eq!(at("vrm"), at("rig").map(|i| i + 1));

    let posed = fixture::humanoid(&fixture::HumanoidSpec {
        arm_drop: 50.0,
        yaw: 90.0,
        ..fixture::HumanoidSpec::default()
    });
    let list = checks(&posed, AssetKind::Character, &config);
    let row = list.iter().find(|c| c.id == "vrm").expect("vrm row");
    assert_eq!(row.status, CheckStatus::Warning);
    assert_eq!(row.title, "Not VRM ready: 2 things to fix");
    let detail = row.detail.as_deref().unwrap_or_default();
    assert!(
        detail.starts_with("Faces +X, VRM wants +Z. Arms hang below horizontal"),
        "{detail}"
    );
    assert!(
        detail.contains("52 of 55 humanoid bones mapped."),
        "{detail}"
    );

    // A prop, rigged or not, has no VRM row.
    let prop = checks(&fixture::character(false), AssetKind::Prop, &config);
    assert!(prop.iter().all(|c| c.id != "vrm"));
}
