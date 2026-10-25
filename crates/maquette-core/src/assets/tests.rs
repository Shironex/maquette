use super::*;
use crate::project::{PROJECT_VERSION, ProjectTemplate};

fn manifest() -> ProjectManifest {
    ProjectManifest {
        version: PROJECT_VERSION,
        id: "moon-island".into(),
        name: "Moon Island".into(),
        template: ProjectTemplate::FloatingDiorama,
        site_repo: None,
        source_folder: None,
        created_at: "2026-10-24T09:30:00.000Z".into(),
        assets: Vec::new(),
    }
}

fn new_asset(name: &str) -> NewAsset {
    NewAsset {
        name: name.into(),
        kind: AssetKind::Prop,
        stage: Stage::Tripo,
    }
}

#[test]
fn add_gives_unique_ids_and_rejects_bad_names() {
    let mut m = manifest();
    let a = add(&mut m, new_asset(" Sakura tree ")).expect("a");
    let b = add(&mut m, new_asset("Sakura Tree")).expect("b");
    assert_eq!(a.id, "sakura-tree");
    assert_eq!(a.name, "Sakura tree");
    assert_eq!(b.id, "sakura-tree-2");
    assert_eq!(a.status, AssetStatus::Queued);
    assert!(add(&mut m, new_asset("  ")).is_err());
    assert!(add(&mut m, new_asset(&"x".repeat(81))).is_err());
    assert_eq!(m.assets.len(), 2);
}

fn with_version(m: &mut ProjectManifest, id: &str, n: u32) {
    let file = format!("models/{}_v{n}.glb", id.replace('-', "_"));
    add_version(m, id, file, None).expect("version");
}

#[test]
fn a_renamed_asset_ships_under_its_current_name() {
    let mut m = manifest();
    let lantern = add(&mut m, new_asset("Lantern")).expect("lantern").id;
    let rock = add(&mut m, new_asset("Rock")).expect("rock").id;
    for id in [&lantern, &rock] {
        with_version(&mut m, id, 1);
    }
    with_version(&mut m, &lantern, 2);
    // Nobody renamed anything: every file ships as it is.
    assert!(shipped_names(&m).is_empty());

    rename(&mut m, &lantern, "Paper lantern").expect("rename");
    let names = shipped_names(&m);
    assert_eq!(
        names.get("lantern_v1").map(String::as_str),
        Some("paper_lantern_v1")
    );
    assert_eq!(
        names.get("lantern_v2").map(String::as_str),
        Some("paper_lantern_v2")
    );
    assert!(!names.contains_key("rock_v1"));
    let (asset, version, raw) = for_shipped_stem(&m, "Paper_Lantern_v2").expect("reverse");
    assert_eq!(
        (asset.id.as_str(), version, raw.as_str()),
        ("lantern", 2, "lantern_v2")
    );
    assert!(for_shipped_stem(&m, "lantern_v2").is_none());

    // A name that lands on another model's file keeps its own file name.
    rename(&mut m, &rock, "Lantern").expect_err("the name is taken");
    rename(&mut m, &lantern, "Rock").expect_err("the name is taken");
}

#[test]
fn a_new_name_never_takes_another_models_stem() {
    let mut m = manifest();
    let a = add(&mut m, new_asset("Plinth")).expect("a").id;
    let b = add(&mut m, new_asset("Plinth base")).expect("b").id;
    with_version(&mut m, &a, 1);
    with_version(&mut m, &b, 1);
    // "Plinth" stays "plinth_v1"; "Plinth base" already is "plinth_base_v1".
    assert!(shipped_names(&m).is_empty());
    // The second asset takes a name whose stem is the first one's file.
    m.assets[1].name = "Plinth".into();
    assert!(!shipped_names(&m).contains_key("plinth_base_v1"));
}

#[test]
fn move_and_versions() {
    let mut m = manifest();
    let a = add(&mut m, new_asset("Vending machine")).expect("add");
    let moved = move_to(&mut m, &a.id, Stage::Web).expect("move");
    assert_eq!(moved.stage, Stage::Web);
    assert_eq!(
        move_to(&mut m, "nope", Stage::Web)
            .expect_err("missing")
            .code(),
        "not-found"
    );
    let v1 = add_version(&mut m, &a.id, "models/vending_machine_v1.glb".into(), None).expect("v1");
    let v2 = add_version(&mut m, &a.id, "models/vending_machine_v2.glb".into(), None).expect("v2");
    assert_eq!((v1.version, v2.version), (1, 2));
    assert_eq!(find(&m, &a.id).expect("find").latest(), Some(&v2));
    assert_eq!(
        find_by_name(&m, "vending MACHINE").map(|a| a.id.as_str()),
        Some("vending-machine")
    );
    set_status(&mut m, &a.id, AssetStatus::Done).expect("status");
    assert_eq!(find(&m, &a.id).expect("find").status, AssetStatus::Done);
}

#[test]
fn rename_changes_the_name_only() {
    let mut m = manifest();
    let a = add(&mut m, new_asset("Crate")).expect("a");
    add(&mut m, new_asset("Barrel")).expect("b");
    add_version(&mut m, &a.id, "models/crate_v1.glb".into(), None).expect("v1");
    let renamed = rename(&mut m, &a.id, "  Old crate ").expect("rename");
    assert_eq!(renamed.name, "Old crate");
    assert_eq!(renamed.id, "crate");
    assert_eq!(renamed.versions[0].file, "models/crate_v1.glb");
    // The new name finds the asset, and so does the first one.
    assert_eq!(
        find_by_name(&m, "old CRATE").map(|a| a.id.as_str()),
        Some("crate")
    );
    assert_eq!(
        find_by_name(&m, "Crate").map(|a| a.id.as_str()),
        Some("crate")
    );
    // The same name in another case is fine for the same asset.
    assert!(rename(&mut m, &a.id, "OLD crate").is_ok());
    assert!(rename(&mut m, &a.id, "barrel").is_err());
    assert!(rename(&mut m, &a.id, "  ").is_err());
    assert!(rename(&mut m, &a.id, &"x".repeat(81)).is_err());
    assert_eq!(
        rename(&mut m, "nope", "Thing").expect_err("missing").code(),
        "not-found"
    );
}

#[test]
fn thumbnails_follow_the_latest_version_only() {
    let mut m = manifest();
    let a = add(&mut m, new_asset("Crate")).expect("add");
    add_version(&mut m, &a.id, "models/crate_v1.glb".into(), None).expect("v1");
    add_version(&mut m, &a.id, "models/crate_v2.glb".into(), None).expect("v2");
    assert!(!set_thumbnail(&mut m, &a.id, 1, "thumbs/crate_v1.png".into()).expect("old"));
    assert_eq!(find(&m, &a.id).expect("find").thumbnail, None);
    assert!(set_thumbnail(&mut m, &a.id, 2, "thumbs/crate_v2.png".into()).expect("new"));
    assert!(!set_thumbnail(&mut m, &a.id, 2, "thumbs/crate_v2.png".into()).expect("same"));
    assert_eq!(
        find(&m, &a.id).expect("find").thumbnail.as_deref(),
        Some("thumbs/crate_v2.png")
    );
}

#[test]
fn json_shape() {
    assert_eq!(
        serde_json::to_string(&Stage::Blender).expect("json"),
        "\"blender\""
    );
    assert_eq!(
        serde_json::to_string(&AssetKind::Character).expect("json"),
        "\"character\""
    );
    assert!(Stage::Concept < Stage::Ship);
}
