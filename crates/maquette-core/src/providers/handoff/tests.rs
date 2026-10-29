use super::options::{StudioMesh, StudioRig};
use super::*;
use crate::providers::presets::{TripoGeometryQuality, TripoTextureQuality};

fn value(rows: &[CardRow], label: &str) -> Option<String> {
    rows.iter()
        .find(|r| r.label == label)
        .map(|r| r.value.clone())
}

fn studio(mesh: StudioMesh, rig: StudioRig) -> StudioSettings {
    StudioSettings { mesh, rig }
}

#[test]
fn the_tripo_card_speaks_studio_words() {
    let preset = TripoPreset::character();
    let defaults = StudioSettings::defaults(&preset, PresetVariant::Character);
    let rows = studio::tripo_rows(&preset, defaults, "maq_hero_v1_tripo");
    let get = |label: &str| value(&rows, label);
    assert_eq!(get("Tab").as_deref(), Some("HD Model"));
    assert_eq!(get("AI Model").as_deref(), Some("H3.1"));
    assert_eq!(get("Ultra Mesh Quality").as_deref(), Some("Off"));
    assert_eq!(get("Texture Quality").as_deref(), Some("2K"));
    assert_eq!(get("Topology").as_deref(), Some("Triangle"));
    assert_eq!(get("Polycount").as_deref(), Some("50000"));
    assert_eq!(get("AI Complete").as_deref(), Some("On"));
    assert_eq!(get("Remove Lighting").as_deref(), Some("On"));
    assert_eq!(get("Generate in Parts").as_deref(), Some("Off"));
    assert_eq!(get("Rig").as_deref(), Some("Biped (Mixamo)"));
    assert_eq!(get("Animations").as_deref(), Some("Idle, Walk"));
    assert_eq!(get("Export").as_deref(), Some("maq_hero_v1_tripo"));
    for api in [
        "Model",
        "Face limit",
        "Geometry",
        "Smart low poly",
        "Quad mesh",
    ] {
        assert_eq!(get(api), None, "{api} is an API name");
    }
    for gone in ["Real-world size", "Fix the image first"] {
        assert_eq!(get(gone), None, "Studio has no {gone}");
    }

    let mut prop = TripoPreset::prop();
    prop.geometry_quality = TripoGeometryQuality::Detailed;
    prop.texture_quality = TripoTextureQuality::Detailed;
    let rows = studio::tripo_rows(
        &prop,
        StudioSettings::defaults(&prop, PresetVariant::Prop),
        "maq_cup_v2_tripo",
    );
    let get = |label: &str| value(&rows, label);
    assert_eq!(get("Ultra Mesh Quality").as_deref(), Some("On"));
    assert_eq!(get("Texture Quality").as_deref(), Some("4K"));
    assert_eq!(get("Polycount").as_deref(), Some("20000"), "adaptive");
    assert_eq!(get("Rig").as_deref(), Some("None"), "a prop has no rig");
}

#[test]
fn smart_mesh_is_the_p2_tab_without_the_hd_rows() {
    let rows = studio::tripo_rows(
        &TripoPreset::prop(),
        studio(StudioMesh::SmartMesh, StudioRig::None),
        "maq_cup_v1_tripo",
    );
    let get = |label: &str| value(&rows, label);
    assert_eq!(get("Tab").as_deref(), Some("Smart Mesh (P2.0)"));
    assert_eq!(get("AI Model"), None, "the tab is the model");
    assert_eq!(get("Ultra Mesh Quality"), None);
    assert_eq!(get("Polycount").as_deref(), Some("20000"));

    // HD Model with a P series preset model takes the default H model.
    let mut p2 = TripoPreset::prop();
    p2.model = "P2-20260801".into();
    let rows = studio::tripo_rows(
        &p2,
        studio(StudioMesh::HdModel, StudioRig::None),
        "maq_cup_v2_tripo",
    );
    assert_eq!(value(&rows, "AI Model").as_deref(), Some("H3.1"));
}

#[test]
fn every_rig_choice_has_its_row() {
    let preset = TripoPreset::character();
    let rig = |choice: StudioRig| {
        let rows = studio::tripo_rows(
            &preset,
            studio(StudioMesh::HdModel, choice),
            "maq_wolf_v1_tripo",
        );
        (value(&rows, "Rig"), value(&rows, "Animations"))
    };
    assert_eq!(
        rig(StudioRig::Biped),
        (Some("Biped (Mixamo)".into()), Some("Idle, Walk".into()))
    );
    assert_eq!(rig(StudioRig::Quadruped), (Some("Quadruped".into()), None));
    assert_eq!(
        rig(StudioRig::MultiLegged),
        (Some("Multi-legged".into()), None)
    );
    assert_eq!(rig(StudioRig::None), (Some("None".into()), None));
}

fn lamp_handoff<'a>(
    images: &'a [PathBuf],
    views: &'a [String],
    preset: &'a TripoPreset,
    settings: Option<StudioSettings>,
) -> Handoff<'a> {
    Handoff {
        id: "iron-street-lamp-1",
        project: "p",
        asset: "iron-street-lamp",
        asset_name: "Iron street lamp",
        service: ProviderId::Tripo,
        variant: PresetVariant::Prop,
        images,
        views,
        tripo: Some(preset),
        meshy: None,
        studio: settings,
        prompt: None,
    }
}

#[test]
fn settings_md_lists_the_studio_rows_and_the_export_name() {
    let dir = tempfile::tempdir().expect("dir");
    let image = dir.path().join("lamp.png");
    fs::write(&image, b"png").expect("image");
    let preset = TripoPreset::prop();
    let handoff = lamp_handoff(std::slice::from_ref(&image), &[], &preset, None);
    let (folder, card) = handoff.write(dir.path()).expect("write");
    assert_eq!(card.export_name, "maq_iron-street-lamp_v1_tripo");
    assert_eq!(card.mode, "Image to 3D");
    let md = fs::read_to_string(folder.join("settings.md")).expect("md");
    for line in [
        "2. Upload front.png.",
        "| Tab | HD Model |  |",
        "| Ultra Mesh Quality | Off |  |",
        "| Texture Quality | 2K |  |",
        "| Polycount | 20000 | Studio has no adaptive |",
        "| Topology | Triangle | quads export FBX only |",
        "| Generate in Parts | Off | recommended |",
        "| Rig | None | skip Rig, export the model as is |",
        "| Export | maq_iron-street-lamp_v1_tripo | GLB, with this file name |",
    ] {
        assert!(md.contains(line), "{line} in {md}");
    }
    assert!(!md.contains("Face limit") && !md.contains("Real-world size"));
}

#[test]
fn settings_md_puts_each_view_in_its_multiview_slot() {
    let dir = tempfile::tempdir().expect("dir");
    let images: Vec<PathBuf> = ["a.png", "b.png", "c.png"]
        .iter()
        .map(|name| {
            let path = dir.path().join(name);
            fs::write(&path, b"png").expect("image");
            path
        })
        .collect();
    let views: Vec<String> = ["front", "left", "back"]
        .iter()
        .map(|v| (*v).to_string())
        .collect();
    // Multi-view follows the refs even with the preset's switch off.
    let preset = TripoPreset::prop();
    let settings = studio(StudioMesh::SmartMesh, StudioRig::Quadruped);
    let handoff = lamp_handoff(&images, &views, &preset, Some(settings));
    let (folder, card) = handoff.write(dir.path()).expect("write");
    assert_eq!(card.mode, "Multiview to 3D");
    assert_eq!(card.images, vec!["front.png", "left.png", "back.png"]);
    let md = fs::read_to_string(folder.join("settings.md")).expect("md");
    for line in [
        "start a new Multiview to 3D",
        "2. Upload each image into the slot of its view: front.png in Front, left.png in Left, back.png in Back.",
        "| Tab | Smart Mesh (P2.0) |  |",
        "| Rig | Quadruped | export GLB (or FBX) with Export Skeleton on, Texture Resolution 2K |",
    ] {
        assert!(md.contains(line), "{line} in {md}");
    }
}

#[test]
fn the_meshy_card_keeps_its_words() {
    let rows = meshy_rows(&MeshyPreset::character(), PresetVariant::Character);
    let get = |label: &str| value(&rows, label);
    assert_eq!(get("AI model").as_deref(), Some("Meshy 7.1"));
    assert_eq!(get("Remesh").as_deref(), Some("50,000 triangles"));
    assert_eq!(get("Pose").as_deref(), Some("A-pose"));
    assert_eq!(get("Rig").as_deref(), Some("1.70 m tall"));
    assert_eq!(get("Remove lighting"), None);
}
