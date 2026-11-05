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
    let doc = create(&root, &project, "idle", None).expect("create");
    assert_eq!(doc.id, "standing-idle");
    assert_eq!(doc.file, "recipes/standing-idle.recipe.json");
    assert_eq!(doc.recipe.base.name, "Standing Idle");
    assert_eq!(
        (doc.recipe.base.frames, doc.recipe.base.hash.as_str()),
        (38, "h1")
    );
    assert!(doc.recipe.base.mixamo && doc.mixamo);
    assert_eq!(doc.base.status, BaseStatus::Found);
    let named = create(&root, &project, "idle", Some("Wave hello")).expect("named");
    assert_eq!(named.recipe.name, "Wave hello");
    assert!(create(&root, &project, "nope", None).is_err());
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
    };
    let doc = doc_of("idle", read, std::slice::from_ref(&clip));
    assert!(doc.mixamo, "the library clip came from a Mixamo rig");
    assert!(!doc.recipe.base.mixamo);
}
