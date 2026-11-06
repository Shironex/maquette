use super::super::tests::library_clip;
use super::super::{SourceRig, store};
use super::*;

#[test]
fn a_recipe_on_a_library_clip_keeps_its_name_frames_hash_and_mixamo_mark() {
    let dir = tempfile::tempdir().expect("temp");
    let (root, project) = (dir.path().join("ws"), dir.path().join("project"));
    store(
        &root,
        &library_clip("idle", "Standing Idle", "Singles", "h1"),
    )
    .expect("store");
    let doc = create(&root, &project, Some("idle"), None).expect("create");
    assert_eq!(doc.id, "standing-idle");
    assert_eq!(doc.file, "recipes/standing-idle.recipe.json");
    assert_eq!(doc.recipe.base.name, "Standing Idle");
    assert_eq!(
        (doc.recipe.base.frames, doc.recipe.base.hash.as_str()),
        (38, "h1")
    );
    assert!(doc.recipe.base.mixamo && doc.mixamo);
    assert_eq!(doc.base.status, BaseStatus::Found);
    let named = create(&root, &project, Some("idle"), Some("Wave hello")).expect("named");
    assert_eq!(named.recipe.name, "Wave hello");
    assert!(create(&root, &project, Some("nope"), None).is_err());
}

#[test]
fn the_base_matches_by_name_and_frames_and_a_new_hash_only_warns() {
    let mut clip = library_clip("idle", "Standing Idle", "Singles", "h1");
    clip.rig.kind = SourceRig::Custom;
    let mut base = base_of(&clip);
    assert!(!base.mixamo);
    base.name = "standing idle".into();
    let (found, _) = match_base(std::slice::from_ref(&clip), &base);
    assert_eq!(found.status, BaseStatus::Found);

    base.hash = "h2".into();
    let (differs, matched) = match_base(std::slice::from_ref(&clip), &base);
    assert_eq!(differs.status, BaseStatus::HashDiffers);
    assert_eq!(differs.clip.as_deref(), Some("idle"));
    assert!(differs.warning.is_some() && matched.is_some());

    base.frames = 40;
    let (missing, none) = match_base(std::slice::from_ref(&clip), &base);
    assert_eq!(missing.status, BaseStatus::Missing);
    assert!(none.is_none());
    assert!(missing.warning.expect("warning").contains("40 frames"));
}

#[test]
fn a_mixamo_clip_in_the_library_marks_the_recipe_as_mixamo() {
    let clip = library_clip("idle", "Standing Idle", "Singles", "h1");
    let mut base = base_of(&clip);
    base.mixamo = false;
    let recipe = crate::recipes::RecipeFile::new("Idle", base);
    let read = crate::recipes::RecipeRead {
        recipe,
        warnings: Vec::new(),
        gone: Vec::new(),
        version: String::new(),
        motion_version: String::new(),
    };
    let doc = doc_of("idle", read, std::slice::from_ref(&clip));
    assert!(doc.mixamo, "the library clip came from a Mixamo rig");
    assert!(!doc.recipe.base.mixamo);
}

#[test]
fn lists_mark_mixamo_as_reading_one_recipe_does() {
    let dir = tempfile::tempdir().expect("temp");
    let (root, project) = (dir.path().join("ws"), dir.path().join("project"));
    store(
        &root,
        &library_clip("idle", "Standing Idle", "Singles", "h1"),
    )
    .expect("store");
    let doc = create(&root, &project, Some("idle"), None).expect("create");
    let mut unmarked = doc.recipe.clone();
    unmarked.base.mixamo = false;
    crate::recipes::create_whole(&project, &unmarked, Some("unmarked")).expect("write");
    let manifest: crate::project::ProjectManifest = serde_json::from_value(serde_json::json!({
        "version": 1, "id": "frostveil", "name": "Frostveil", "template": "floating-diorama",
        "createdAt": "2026-11-05T10:00:00.000Z", "assets": []
    }))
    .expect("manifest");
    let listed = summaries(&root, &project, &manifest).expect("list");
    let line = listed
        .iter()
        .find(|r| r.id == "unmarked")
        .expect("the unmarked copy");
    let read = get(&root, &project, &manifest, &line.id).expect("get");
    assert!(read.mixamo && line.mixamo, "list and get agree");
    let renamed = rename(&root, &project, &line.id, "Idle again").expect("rename");
    assert!(renamed.mixamo);

    let file = dir.path().join("unmarked.recipe.json");
    std::fs::write(&file, serde_json::to_string(&unmarked).expect("json")).expect("file");
    let added = library::import_to_library(&root, &file).expect("library");
    let entry = library::list(&root).expect("library list");
    let row = entry.iter().find(|e| e.id == added.entry).expect("row");
    let full = library::get(&root, &added.entry, None).expect("library get");
    assert!(full.mixamo && row.mixamo, "library list and get agree");
}

#[test]
fn an_empty_hash_matches_by_name_only_when_a_starter_marked_it() {
    let clip = library_clip("idle", "Idle", "Singles", "h1");
    let mut base = base_of(&clip);
    base.hash.clear();
    base.frames = 99;
    let (missing, _) = match_base(std::slice::from_ref(&clip), &base);
    assert_eq!(
        missing.status,
        BaseStatus::Missing,
        "an import keeps name and frames"
    );
    let named = RecipeBase::named("Idle");
    let (found, matched) = match_base(std::slice::from_ref(&clip), &named);
    assert_eq!(found.status, BaseStatus::Found);
    assert!(matched.is_some());
}

#[test]
fn a_joined_recipe_matches_each_clip_and_is_mixamo_when_any_clip_is() {
    let dir = tempfile::tempdir().expect("temp");
    let (root, project) = (dir.path().join("ws"), dir.path().join("project"));
    let mut own = library_clip("slash", "Own Slash", "Singles", "h1");
    own.rig.kind = SourceRig::Custom;
    store(&root, &own).expect("store");
    store(
        &root,
        &library_clip("idle", "Standing Idle", "Singles", "h2"),
    )
    .expect("store");
    let picks = [
        ClipPick {
            clip: Some("slash".into()),
            trim: Some([2, 30]),
            ..ClipPick::default()
        },
        ClipPick {
            clip: Some("idle".into()),
            ..ClipPick::default()
        },
    ];
    let doc = create_joined(&root, &project, &picks, false, None).expect("create");
    assert_eq!(doc.recipe.name, "Own Slash");
    assert_eq!(doc.recipe.version, crate::recipes::RECIPE_VERSION);
    assert_eq!(doc.recipe.base.trim, Some([2, 30]));
    assert_eq!(doc.recipe.base.blend, None);
    assert_eq!(
        doc.recipe.joined[0].blend,
        Some(crate::recipes::BlendWindow::default())
    );
    assert_eq!(doc.base.status, BaseStatus::Found);
    assert_eq!(doc.joined.len(), 1);
    assert_eq!(doc.joined[0].status, BaseStatus::Found);
    assert!(doc.mixamo, "the second clip is Mixamo's");

    let kept = clips_of(
        &root,
        Some(&doc.recipe),
        &[ClipPick {
            keep: Some(1),
            ..ClipPick::default()
        }],
    )
    .expect("keep");
    assert_eq!(kept[0].name, "Standing Idle");
    assert_eq!(kept[0].blend, None, "the first clip has no blend");
    for wrong in [
        ClipPick::default(),
        ClipPick {
            keep: Some(5),
            ..ClipPick::default()
        },
        ClipPick {
            clip: Some("idle".into()),
            keep: Some(0),
            ..ClipPick::default()
        },
    ] {
        assert!(clips_of(&root, Some(&doc.recipe), &[wrong]).is_err());
    }
    let one = create_joined(&root, &project, &picks[1..], false, Some("Idle")).expect("one");
    assert_eq!(one.recipe.version, crate::recipes::RECIPE_VERSION_1);
    assert!(one.joined.is_empty());
}
