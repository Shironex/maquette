use std::path::Path;

use super::*;
use crate::assets::{self, AssetKind, NewAsset, Stage};
use crate::project::ProjectManifest;

fn base() -> RecipeBase {
    RecipeBase {
        name: "Standing Idle".into(),
        frames: 120,
        length: 4.0,
        hash: "abc123".into(),
        mixamo: true,
    }
}

fn lantern() -> RecipeBlank {
    RecipeBlank {
        id: "lantern".into(),
        label: "The prop the hand reaches".into(),
        kind: BlankKind::Prop,
        prop: None,
        point: None,
    }
}

fn reach(target: &str) -> RecipeLayer {
    RecipeLayer {
        id: String::new(),
        kind: LayerKind::Reach,
        bone: "rightHand".into(),
        settings: LayerSettings {
            start: 0.2,
            end: 0.6,
            target: Some(target.into()),
            ..LayerSettings::default()
        },
    }
}

fn manifest_with(asset: &str) -> ProjectManifest {
    let mut manifest: ProjectManifest = serde_json::from_value(serde_json::json!({
        "version": 1, "id": "frostveil", "name": "Frostveil", "template": "floating-diorama",
        "createdAt": "2026-11-05T10:00:00.000Z", "assets": []
    }))
    .expect("manifest");
    assets::add(
        &mut manifest,
        NewAsset {
            name: asset.into(),
            kind: AssetKind::Prop,
            stage: Stage::Refs,
        },
    )
    .expect("asset");
    manifest
}

fn made(dir: &Path) -> String {
    create(dir, "Reach for the lantern", base())
        .expect("create")
        .0
}

#[test]
fn a_new_recipe_writes_one_file_with_its_base_and_nothing_baked() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, recipe) = create(dir.path(), "  Reach for the lantern ", base()).expect("create");
    assert_eq!(id, "reach-for-the-lantern");
    assert_eq!(recipe.name, "Reach for the lantern");
    assert_eq!(recipe.revision, 1);
    assert!(recipe.base.mixamo);
    let text =
        std::fs::read_to_string(dir.path().join("recipes/reach-for-the-lantern.recipe.json"))
            .expect("file");
    for word in ["keys", "mesh", "skeleton", "fbx"] {
        assert!(!text.to_lowercase().contains(word), "{word} in {text}");
    }
    assert_eq!(get(dir.path(), &id).expect("get"), recipe);
    let again = create(dir.path(), "Reach for the lantern", base()).expect("again");
    assert_eq!(again.0, "reach-for-the-lantern-2");
}

#[test]
fn rename_keeps_the_file_and_id() {
    let dir = tempfile::tempdir().expect("temp");
    let id = made(dir.path());
    let renamed = rename(dir.path(), &id, "Grab the lantern").expect("rename");
    assert_eq!(renamed.id, id);
    assert_eq!(renamed.name, "Grab the lantern");
    assert!(rename(dir.path(), &id, "  ").is_err());
    assert_eq!(list(dir.path()).expect("list").len(), 1);
}

#[test]
fn remove_moves_the_file_to_the_trash() {
    let dir = tempfile::tempdir().expect("temp");
    let id = made(dir.path());
    let trash = dir.path().join("trash");
    let to = remove(dir.path(), &trash, &id).expect("remove");
    assert!(Path::new(&to).is_file());
    assert!(to.contains("recipes"));
    assert!(list(dir.path()).expect("list").is_empty());
    assert!(matches!(
        remove(dir.path(), &trash, &id),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn update_checks_the_whole_recipe_and_gives_changes_free_ids() {
    let dir = tempfile::tempdir().expect("temp");
    let id = made(dir.path());
    let manifest = manifest_with("Lantern");
    let refused = update(
        dir.path(),
        &manifest,
        &id,
        RecipeChange {
            layers: Some(vec![reach("lantern")]),
            blanks: None,
        },
    );
    assert!(refused.is_err(), "a reach at a blank the recipe lacks");
    let mut filled = lantern();
    filled.prop = Some("lantern".into());
    let done = update(
        dir.path(),
        &manifest,
        &id,
        RecipeChange {
            layers: Some(vec![reach("lantern"), reach("lantern")]),
            blanks: Some(vec![filled]),
        },
    )
    .expect("update");
    let ids: Vec<&str> = done.layers.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["l-1", "l-2"]);
    let listed = &list(dir.path()).expect("list")[0];
    assert_eq!((listed.layers, listed.blanks, listed.open), (2, 1, 0));
    assert_eq!(done.revision, 1, "an edit never moves the revision");

    let mut stranger = lantern();
    stranger.prop = Some("not-here".into());
    let wrong = update(
        dir.path(),
        &manifest,
        &id,
        RecipeChange {
            layers: None,
            blanks: Some(vec![stranger]),
        },
    );
    assert!(wrong.is_err(), "a prop that is no asset of the project");
}

#[test]
fn each_kind_keeps_to_its_bones_and_settings() {
    let mut recipe = RecipeFile::new("Wave hello", base());
    recipe.blanks.push(lantern());
    let mut layer = reach("lantern");
    layer.id = "a".into();
    layer.bone = "head".into();
    recipe.layers = vec![layer.clone()];
    assert!(validate(&recipe).is_err(), "a reach moves hands and feet");

    layer.kind = LayerKind::Nudge;
    layer.bone = "spine".into();
    recipe.layers = vec![layer.clone()];
    assert!(
        validate(&normalized(&recipe)).is_err(),
        "a nudge needs a turn or a move"
    );
    layer.settings.rotation = Some([0.0, 15.0, 0.0]);
    recipe.layers = vec![layer.clone()];
    let nudged = normalized(&recipe);
    validate(&nudged).expect("nudge");
    assert_eq!(
        nudged.layers[0].settings.target, None,
        "a nudge has no target"
    );

    layer.bone = "tail".into();
    recipe.layers = vec![layer.clone()];
    assert!(validate(&recipe).is_err(), "not a humanoid bone");

    layer.kind = LayerKind::Wave;
    layer.bone = "rightHand".into();
    layer.settings.cycles = Some(3.0);
    layer.settings.amount = Some(120.0);
    recipe.layers = vec![layer.clone()];
    assert!(
        validate(&recipe).is_err(),
        "a wave swings at most 90 degrees"
    );
    layer.settings.amount = Some(30.0);
    layer.settings.start = 0.7;
    layer.settings.end = 0.3;
    recipe.layers = vec![layer];
    assert!(validate(&recipe).is_err(), "it starts before it ends");
}

#[test]
fn a_broken_or_newer_file_lists_with_its_problem() {
    let dir = tempfile::tempdir().expect("temp");
    let folder = dir.path().join("recipes");
    std::fs::create_dir_all(&folder).expect("dir");
    std::fs::write(folder.join("odd.recipe.json"), "{ not json").expect("odd");
    let mut newer = serde_json::to_value(RecipeFile::new("Later", base())).expect("value");
    newer["version"] = serde_json::json!(RECIPE_VERSION + 1);
    std::fs::write(folder.join("later.recipe.json"), newer.to_string()).expect("later");
    std::fs::write(folder.join("notes.txt"), "not a recipe").expect("notes");
    let found = list(dir.path()).expect("list");
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|r| r.problem.is_some()), "{found:?}");
    assert!(matches!(
        get(dir.path(), "later"),
        Err(CoreError::UnsupportedVersion { .. })
    ));
    assert!(matches!(
        get(dir.path(), "../x"),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn a_file_over_the_cap_is_refused_unread() {
    let dir = tempfile::tempdir().expect("temp");
    let folder = dir.path().join("recipes");
    std::fs::create_dir_all(&folder).expect("dir");
    let big = " ".repeat(usize::try_from(MAX_FILE_BYTES).expect("cap") + 1);
    std::fs::write(folder.join("big.recipe.json"), big).expect("big");
    let err = get(dir.path(), "big").expect_err("too big");
    assert!(err.to_string().contains("too big"), "{err}");
}

#[test]
fn reading_normalises_what_a_kind_does_not_use() {
    let dir = tempfile::tempdir().expect("temp");
    let id = made(dir.path());
    let file = path(dir.path(), &id).expect("path");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).expect("read")).expect("json");
    value["name"] = serde_json::json!("  Padded  ");
    value["blanks"] = serde_json::json!([{
        "id": "spot", "label": "Where to look", "kind": "point",
        "prop": "lantern", "point": [0.0, 1.5, 2.0]
    }]);
    std::fs::write(&file, value.to_string()).expect("write");
    let recipe = get(dir.path(), &id).expect("get");
    assert_eq!(recipe.name, "Padded");
    assert_eq!(recipe.blanks[0].prop, None, "a point blank holds no prop");
    assert!(recipe.blanks[0].filled());
}
