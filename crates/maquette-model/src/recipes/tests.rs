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

fn lantern_project() -> ProjectManifest {
    manifest_with("Lantern")
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
    assert_eq!(
        get(dir.path(), &lantern_project(), &id)
            .expect("get")
            .recipe,
        recipe
    );
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
    assert_eq!(list(dir.path(), &lantern_project()).expect("list").len(), 1);
}

#[test]
fn remove_moves_the_file_to_the_trash() {
    let dir = tempfile::tempdir().expect("temp");
    let id = made(dir.path());
    let trash = dir.path().join("trash");
    let to = remove(dir.path(), &trash, &id).expect("remove");
    assert!(Path::new(&to).is_file());
    assert!(to.contains("recipes"));
    assert!(
        list(dir.path(), &lantern_project())
            .expect("list")
            .is_empty()
    );
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
            ..RecipeChange::default()
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
            name: None,
            ..RecipeChange::default()
        },
    )
    .expect("update")
    .recipe;
    let ids: Vec<&str> = done.layers.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["l-1", "l-2"]);
    let listed = &list(dir.path(), &lantern_project()).expect("list")[0];
    assert_eq!((listed.layers, listed.blanks, listed.open), (2, 1, 0));
    assert_eq!(done.revision, 1, "an edit never moves the revision");

    let mut stranger = lantern();
    stranger.prop = Some("not-here".into());
    let wrong = update(
        dir.path(),
        &manifest,
        &id,
        RecipeChange {
            blanks: Some(vec![stranger]),
            ..RecipeChange::default()
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
    let found = list(dir.path(), &lantern_project()).expect("list");
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|r| r.problem.is_some()), "{found:?}");
    assert!(matches!(
        get(dir.path(), &lantern_project(), "later"),
        Err(CoreError::UnsupportedVersion { .. })
    ));
    assert!(matches!(
        get(dir.path(), &lantern_project(), "../x"),
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
    let err = get(dir.path(), &lantern_project(), "big").expect_err("too big");
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
    let recipe = get(dir.path(), &lantern_project(), &id)
        .expect("get")
        .recipe;
    assert_eq!(recipe.name, "Padded");
    assert_eq!(recipe.blanks[0].prop, None, "a point blank holds no prop");
    assert!(recipe.blanks[0].filled());
}

#[test]
fn a_prop_removed_after_the_fill_reads_open_and_never_blocks_a_change() {
    let dir = tempfile::tempdir().expect("temp");
    let id = made(dir.path());
    let mut filled = lantern();
    filled.prop = Some("lantern".into());
    update(
        dir.path(),
        &lantern_project(),
        &id,
        RecipeChange {
            layers: Some(vec![reach("lantern")]),
            blanks: Some(vec![filled.clone()]),
            name: None,
            ..RecipeChange::default()
        },
    )
    .expect("fill");

    let gone = manifest_with("Barrel");
    let read = get(dir.path(), &gone, &id).expect("get");
    assert!(!read.recipe.blanks[0].filled(), "the lantern is gone");
    assert_eq!(read.warnings.len(), 1);
    assert!(read.warnings[0].contains("lantern"), "{:?}", read.warnings);
    let listed = &list(dir.path(), &gone).expect("list")[0];
    assert_eq!((listed.open, listed.warnings.len()), (1, 1));
    let on_disk = get(dir.path(), &lantern_project(), &id).expect("still");
    assert!(on_disk.recipe.blanks[0].filled(), "reading writes nothing");

    let mut nudge = reach("lantern");
    nudge.kind = LayerKind::Nudge;
    nudge.bone = "spine".into();
    nudge.settings.target = None;
    nudge.settings.rotation = Some([0.0, 10.0, 0.0]);
    let changed = update(
        dir.path(),
        &gone,
        &id,
        RecipeChange {
            layers: Some(vec![reach("lantern"), nudge]),
            ..RecipeChange::default()
        },
    )
    .expect("a changes only update goes through");
    assert_eq!(changed.warnings.len(), 1, "it says which blank reads open");
    assert!(!changed.recipe.blanks[0].filled());
    assert_eq!(changed.recipe.layers.len(), 2);

    let renamed = update(
        dir.path(),
        &gone,
        &id,
        RecipeChange {
            name: Some("Still the lantern".into()),
            ..RecipeChange::default()
        },
    )
    .expect("a name only update goes through");
    assert_eq!(renamed.warnings.len(), 1);
    let restored = get(dir.path(), &lantern_project(), &id).expect("restored");
    assert_eq!(
        restored.recipe.blanks[0].prop.as_deref(),
        Some("lantern"),
        "the file keeps the prop, so restoring the asset fills the blank again"
    );
    assert!(restored.warnings.is_empty());
    assert_eq!(restored.recipe.layers.len(), 2);

    let mut point = lantern();
    point.id = "spot".into();
    point.kind = BlankKind::Point;
    point.point = Some([0.0, 1.0, 0.5]);
    let mut open = lantern();
    open.prop = None;
    let resent = update(
        dir.path(),
        &gone,
        &id,
        RecipeChange {
            blanks: Some(vec![open, point]),
            ..RecipeChange::default()
        },
    )
    .expect("blanks sent with the gone prop's blank open go through");
    assert_eq!(resent.recipe.blanks.len(), 2);
    assert!(!resent.recipe.blanks[0].filled(), "it still reads open");
    let kept = get(dir.path(), &lantern_project(), &id).expect("kept");
    assert_eq!(
        kept.recipe.blanks[0].prop.as_deref(),
        Some("lantern"),
        "sending a blank that only read open keeps its prop"
    );

    let refused = update(
        dir.path(),
        &gone,
        &id,
        RecipeChange {
            blanks: Some(vec![filled]),
            ..RecipeChange::default()
        },
    );
    assert!(
        refused.is_err(),
        "sending the missing prop again is refused"
    );
}

/// A recipe whose lantern blank is filled, read where the lantern is gone.
fn with_gone_lantern(dir: &Path) -> (String, ProjectManifest) {
    let id = made(dir);
    let mut filled = lantern();
    filled.prop = Some("lantern".into());
    update(
        dir,
        &lantern_project(),
        &id,
        RecipeChange {
            layers: Some(vec![reach("lantern")]),
            blanks: Some(vec![filled]),
            ..RecipeChange::default()
        },
    )
    .expect("fill");
    (id, manifest_with("Barrel"))
}

#[test]
fn a_read_names_each_gone_prop_and_the_file_keeps_it() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, gone) = with_gone_lantern(dir.path());
    let read = get(dir.path(), &gone, &id).expect("get");
    assert_eq!(
        read.gone,
        [GoneProp {
            blank: "lantern".into(),
            prop: "lantern".into()
        }]
    );
    assert!(
        read.warnings[0].contains("reads as open") && read.warnings[0].contains("file keeps"),
        "{:?}",
        read.warnings
    );
    assert!(
        get(dir.path(), &lantern_project(), &id)
            .expect("here")
            .gone
            .is_empty()
    );
}

#[test]
fn clearing_a_gone_prop_empties_it_for_good() {
    let dir = tempfile::tempdir().expect("temp");
    let (id, gone) = with_gone_lantern(dir.path());
    let mut open = lantern();
    open.prop = None;
    let cleared = update(
        dir.path(),
        &gone,
        &id,
        RecipeChange {
            blanks: Some(vec![open]),
            clear: vec!["lantern".into()],
            ..RecipeChange::default()
        },
    )
    .expect("clear with the blanks sent");
    assert!(cleared.gone.is_empty() && cleared.warnings.is_empty());
    let back = get(dir.path(), &lantern_project(), &id).expect("back");
    assert_eq!(
        back.recipe.blanks[0].prop, None,
        "restoring the asset no longer fills it"
    );

    let (other, gone) = with_gone_lantern(&dir.path().join("other"));
    let cleared = update(
        &dir.path().join("other"),
        &gone,
        &other,
        RecipeChange {
            clear: vec!["lantern".into()],
            ..RecipeChange::default()
        },
    )
    .expect("clear with the blanks kept");
    assert_eq!(cleared.recipe.blanks[0].prop, None);
    let back = get(&dir.path().join("other"), &lantern_project(), &other).expect("back");
    assert_eq!(back.recipe.blanks[0].prop, None);

    let unknown = update(
        dir.path(),
        &gone,
        &id,
        RecipeChange {
            clear: vec!["nothing".into()],
            ..RecipeChange::default()
        },
    );
    assert!(unknown.is_err(), "a clear naming no prop blank");
}

#[test]
fn a_write_that_read_an_older_version_is_refused_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp");
    let id = made(dir.path());
    let read = get(dir.path(), &lantern_project(), &id).expect("read");
    let agent = update(
        dir.path(),
        &lantern_project(),
        &id,
        RecipeChange {
            blanks: Some(vec![lantern()]),
            ..RecipeChange::default()
        },
    )
    .expect("an agent writes meanwhile");
    let stale = update(
        dir.path(),
        &lantern_project(),
        &id,
        RecipeChange {
            name: Some("Mine".into()),
            blanks: Some(Vec::new()),
            expected: Some(read.recipe.edited.clone()),
            ..RecipeChange::default()
        },
    );
    let err = stale.expect_err("the file moved since it was read");
    assert_eq!(err.code(), "moved");
    let now = get(dir.path(), &lantern_project(), &id).expect("now");
    assert_eq!(now.recipe, agent.recipe, "nothing was written");

    let fresh = update(
        dir.path(),
        &lantern_project(),
        &id,
        RecipeChange {
            name: Some("Mine".into()),
            expected: Some(now.recipe.edited.clone()),
            ..RecipeChange::default()
        },
    )
    .expect("the version it read is the one on disk");
    assert_eq!(fresh.recipe.name, "Mine");
    assert_eq!(fresh.recipe.blanks.len(), 1);
}

#[test]
fn a_name_and_changes_land_in_one_write() {
    let dir = tempfile::tempdir().expect("temp");
    let id = made(dir.path());
    let done = update(
        dir.path(),
        &lantern_project(),
        &id,
        RecipeChange {
            name: Some("  Grab the lantern ".into()),
            blanks: Some(vec![lantern()]),
            layers: None,
            ..RecipeChange::default()
        },
    )
    .expect("update");
    assert_eq!(done.recipe.name, "Grab the lantern");
    let refused = update(
        dir.path(),
        &lantern_project(),
        &id,
        RecipeChange {
            name: Some(" ".into()),
            blanks: Some(Vec::new()),
            layers: None,
            ..RecipeChange::default()
        },
    );
    assert!(refused.is_err());
    let kept = get(dir.path(), &lantern_project(), &id)
        .expect("get")
        .recipe;
    assert_eq!(kept.blanks.len(), 1, "a refused name writes nothing");
}

#[test]
fn a_long_name_still_leaves_room_for_a_number() {
    let dir = tempfile::tempdir().expect("temp");
    let name = "a".repeat(100);
    let (first, _) = create(dir.path(), &name, base()).expect("first");
    assert!(first.len() + "-9999".len() <= ID_MAX_CHARS, "{first}");
    let (second, _) = create(dir.path(), &name, base()).expect("second");
    assert_eq!(second, format!("{first}-2"));
}
