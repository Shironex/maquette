use std::path::Path;

use super::*;
use crate::animations::recipes::BaseStatus;
use crate::animations::recipes::library::{self, UseRecipe, use_recipe};
use crate::animations::store;
use crate::animations::tests::library_clip;
use crate::recipes::{self, LayerKind};
use crate::workspace::WorkspaceConfig;

fn project(root: &Path, config: &WorkspaceConfig) -> String {
    crate::project::create(
        root,
        &config.layout,
        crate::project::NewProject {
            name: "Frostveil".into(),
            template: crate::project::ProjectTemplate::FloatingDiorama,
            target: crate::game::ProjectTarget::Website,
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project")
    .id
}

fn using(project: &str, entry: &str) -> UseRecipe {
    UseRecipe {
        project: project.into(),
        entry: entry.into(),
        revision: None,
        name: None,
    }
}

#[test]
fn every_starter_is_a_valid_v1_recipe_naming_its_clip_and_never_its_keys() {
    let kinds: Vec<LayerKind> = SPECS
        .iter()
        .flat_map(|s| (s.layers)())
        .map(|l| l.kind)
        .collect();
    for kind in [
        LayerKind::Reach,
        LayerKind::Nudge,
        LayerKind::LookAt,
        LayerKind::Wave,
    ] {
        assert!(kinds.contains(&kind), "no starter shows {kind:?}");
    }
    for spec in &SPECS {
        let recipe = recipe_of(spec, &[]);
        let written = recipes::normalized(&recipe);
        recipes::validate(&written).unwrap_or_else(|e| panic!("{}: {e}", spec.name));
        assert_eq!(written.version, recipes::RECIPE_VERSION_1);
        assert!(written.base.hash.is_empty(), "{} carries a hash", spec.name);
        assert!(!written.base.mixamo);
        assert!(written.from.is_none());
        if spec.clips.is_empty() {
            assert!(written.base.rest, "{}", spec.name);
        } else {
            assert!(written.base.name_only(), "{}", spec.name);
            assert_eq!(written.base.name, spec.clips[0]);
        }
        let text = serde_json::to_string(&written).expect("json");
        assert!(!text.contains("keys") && !text.contains("glb"), "{text}");
    }
    let wave = recipe_of(&SPECS[0], &[]);
    assert_eq!(wave.base.name, recipes::REST_POSE_NAME);
}

#[test]
fn listing_starters_writes_nothing_and_their_ids_are_never_library_ids() {
    let tmp = tempfile::tempdir().expect("temp");
    let root = tmp.path();
    let listed = list(root).expect("list");
    assert_eq!(listed.len(), SPECS.len());
    assert!(!library::dir(root).exists(), "the library stays empty");
    assert!(library::list(root).expect("library").is_empty());
    for starter in &listed {
        assert!(starter.id.starts_with(STARTER_PREFIX));
        assert!(!recipes::is_recipe_id(&starter.id), "{}", starter.id);
    }
    let wave = &listed[0];
    assert!(wave.rest);
    assert_eq!(wave.base.status, BaseStatus::Rest);
    let idle = listed
        .iter()
        .find(|s| s.id == "starter:lean-in")
        .expect("lean in");
    assert_eq!(idle.base.status, BaseStatus::Missing);
    assert_eq!(idle.base_name, "Idle");
    assert!(get(root, "starter:nope").is_err());
    assert!(get(root, "lean-in").is_err(), "a starter id has its prefix");
}

#[test]
fn a_starter_takes_the_first_clip_of_its_names_whatever_its_frames() {
    let tmp = tempfile::tempdir().expect("temp");
    let root = tmp.path();
    let mut clip = library_clip("breathing", "breathing idle", "Singles", "h7");
    clip.frames = 412;
    store(root, &clip).expect("clip");
    let starter = get(root, "starter:lean-in").expect("starter");
    assert_eq!(starter.base.status, BaseStatus::Found);
    assert_eq!(starter.base.clip.as_deref(), Some("breathing"));
    assert_eq!(starter.base_name, "breathing idle");
    assert!(starter.mixamo, "the fixture clip came from a Mixamo rig");
    let file = recipe(root, "starter:lean-in").expect("recipe");
    assert_eq!(
        (file.base.frames, file.base.hash.as_str()),
        (412, "h7"),
        "used, it keeps the library clip's base"
    );
}

#[test]
fn using_a_starter_makes_a_project_recipe_with_no_library_link() {
    let tmp = tempfile::tempdir().expect("temp");
    let root = tmp.path();
    let config = WorkspaceConfig::default();
    let frost = project(root, &config);
    store(root, &library_clip("idle", "Idle", "Singles", "h1")).expect("clip");

    let lean = use_recipe(root, &config.layout, &using(&frost, "starter:lean-in")).expect("use");
    assert_eq!(lean.doc.id, "lean-in");
    assert_eq!(lean.doc.base.status, BaseStatus::Found);
    assert!(lean.doc.recipe.from.is_none());
    assert_eq!(lean.doc.recipe.layers.len(), 2);
    assert!(lean.notes.is_empty(), "{:?}", lean.notes);

    let walk = use_recipe(
        root,
        &config.layout,
        &using(&frost, "starter:point-while-walking"),
    )
    .expect("use");
    assert_eq!(walk.doc.base.status, BaseStatus::Missing);
    assert!(walk.doc.recipe.base.name_only());
    assert!(
        walk.notes.iter().any(|n| n.contains("Walking")),
        "{:?}",
        walk.notes
    );

    let wave = use_recipe(root, &config.layout, &using(&frost, "starter:wave-hello")).expect("use");
    assert!(wave.doc.recipe.base.rest);
    assert_eq!(wave.doc.base.status, BaseStatus::Rest);
    assert!(!wave.doc.mixamo);

    let reach = use_recipe(
        root,
        &config.layout,
        &using(&frost, "starter:reach-for-prop"),
    )
    .expect("use");
    assert!(
        reach.doc.recipe.blanks.iter().any(|b| !b.filled()),
        "the prop blank is open"
    );

    let mut named = using(&frost, "starter:lean-in");
    named.name = Some("Lean in close".into());
    let second = use_recipe(root, &config.layout, &named).expect("named");
    assert_eq!(second.doc.recipe.name, "Lean in close");

    let mut revised = using(&frost, "starter:lean-in");
    revised.revision = Some(1);
    assert!(use_recipe(root, &config.layout, &revised).is_err());
    assert!(
        !library::dir(root).exists(),
        "using a starter never fills the library"
    );
}

#[test]
fn a_starter_copied_to_the_library_is_an_ordinary_recipe_at_r1() {
    let tmp = tempfile::tempdir().expect("temp");
    let root = tmp.path();
    let copied = copy_to_library(root, "starter:wave-hello").expect("copy");
    assert_eq!(copied.entry, "wave-hello");
    let doc = library::get(root, &copied.entry, None).expect("entry");
    assert_eq!(doc.revision, 1);
    assert!(doc.recipe.base.rest);
    assert_eq!(doc.base.status, BaseStatus::Rest);
    assert_eq!(
        doc.entry.revisions[0].imported.as_deref(),
        Some("Starter: Wave hello")
    );
    let again = copy_to_library(root, "starter:wave-hello").expect("again");
    assert_eq!(
        again.entry, "wave-hello-2",
        "a second copy is another recipe"
    );
    assert!(copy_to_library(root, "starter:nope").is_err());
}

#[test]
fn the_rest_pose_starter_runs_as_long_as_a_new_rest_pose_recipe() {
    let dir = tempfile::tempdir().expect("temp");
    let wave = recipe(dir.path(), "starter:wave-hello").expect("wave");
    assert!(wave.base.rest);
    assert_eq!(wave.base.frames, recipes::REST_FRAMES);
}
