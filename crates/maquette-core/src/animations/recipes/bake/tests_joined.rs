//! Baking a recipe that joins clips and edits its base: what the bake
//! takes, what it stages for the script, and how a joined clip that moved
//! in the library makes the bake out of date.

use std::fs;

use super::job::{self, JOB_FILE};
use super::tests::{Project, reaching};
use super::*;
use crate::animations::recipes::{self as library_recipes, ClipPick, clips_of, kept_of};
use crate::animations::tests::library_clip;
use crate::animations::{CLIP_GLB, clip_dir, store};
use crate::recipes::{BlendWindow, RecipeChange, TimeEdit, TimeKind};

/// The reaching recipe with a second library clip joined after its trimmed
/// base, a freeze on it and mirror on.
fn joined() -> Project {
    let mut p = reaching();
    store(
        p.root(),
        &library_clip("turn", "Turn Around", "Singles", "h2"),
    )
    .expect("store");
    fs::write(
        clip_dir(p.root(), "turn").expect("dir").join(CLIP_GLB),
        b"glb",
    )
    .expect("clip");
    let picks = [
        ClipPick {
            keep: Some(0),
            trim: Some([2, 30]),
            ..ClipPick::default()
        },
        ClipPick {
            clip: Some("turn".into()),
            blend: Some(BlendWindow {
                frames: 6,
                ..BlendWindow::default()
            }),
            ..ClipPick::default()
        },
    ];
    let clips = clips_of(p.root(), Some(&p.recipe.recipe), &picks).expect("clips");
    let manifest = crate::project::load(p.root(), &p.config.layout, &p.id).expect("manifest");
    p.recipe = library_recipes::update(
        p.root(),
        &p.dir,
        &manifest,
        &p.recipe.id,
        RecipeChange {
            clips: Some(clips),
            kept: Some(kept_of(&picks)),
            time: Some(vec![TimeEdit::hold(TimeKind::Freeze, 1, 10, 5)]),
            mirror: Some(true),
            expected: Some(p.recipe.version.clone()),
            ..RecipeChange::default()
        },
    )
    .expect("joined");
    p
}

#[test]
fn joined_clips_trims_freezes_and_mirror_bake_now() {
    let p = joined();
    assert_eq!(not_bakeable(&p.recipe), None);
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    assert_eq!(prepared.joined.len(), 1);
    let work = p.root().join("work");
    job::stage(p.root(), &prepared, &work).expect("stage");
    assert!(work.join("recipe_motion.py").is_file());
    assert!(work.join("in/clips/turn.glb").is_file());
    let job: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join(JOB_FILE)).expect("job")).expect("json");
    let clips = job["retarget"]["clips"].as_array().expect("clips");
    assert_eq!(clips.len(), 2, "each library clip goes on once");
    assert!(clips.iter().all(|c| {
        c["name"]
            .as_str()
            .is_some_and(|n| n.starts_with("maquette_recipe_part"))
    }));
    let recipe = &job["recipe"];
    assert_eq!(recipe["name"], "reach_out");
    // 28 kept + 38 joined less a 6 frame blend, plus 5 frozen.
    assert_eq!(recipe["timeline"]["frames"], 28 + 38 - 6 + 5);
    assert_eq!(recipe["timeline"]["mirror"], true);
    assert_eq!(recipe["parts"][1]["clip"], "turn");
    assert!(
        !recipe["mirrorPairs"].as_array().expect("pairs").is_empty(),
        "left and right pairs"
    );
}

#[test]
fn a_plain_recipe_stages_no_timeline() {
    let p = reaching();
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let work = p.root().join("work");
    job::stage(p.root(), &prepared, &work).expect("stage");
    let job: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join(JOB_FILE)).expect("job")).expect("json");
    assert!(job["recipe"]["timeline"].is_null());
    assert_eq!(job["retarget"]["clips"][0]["name"], "reach_out");
}

#[test]
fn a_joined_clip_missing_from_the_library_is_refused_naming_it() {
    let p = joined();
    fs::remove_dir_all(clip_dir(p.root(), "turn").expect("dir")).expect("remove");
    let doc = library_recipes::get(
        p.root(),
        &p.dir,
        &crate::project::load(p.root(), &p.config.layout, &p.id).expect("m"),
        &p.recipe.id,
    )
    .expect("doc");
    assert_eq!(
        doc.joined[0].status,
        crate::animations::recipes::BaseStatus::Missing
    );
    let err = prepare(p.root(), &p.config, &p.request())
        .err()
        .expect("refused");
    assert!(err.to_string().contains("Clip 2 (Turn Around)"), "{err}");
}

#[test]
fn a_joined_clip_imported_again_makes_the_bake_out_of_date_naming_it() {
    let p = joined();
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let work = p.root().join("work");
    fs::create_dir_all(&work).expect("work");
    fs::write(
        work.join(super::job::REPORT_FILE),
        r#"{"clip": "reach_out", "frames": 65}"#,
    )
    .expect("report");
    let run = super::result::read_run(&work).expect("run");
    let result = super::result::result_of(&run, &prepared, &work, 1.0);
    assert_eq!(result.joined_clips.len(), 1);
    assert_eq!(result.joined_clips[0].clip, "turn");
    assert!(result.mixamo, "any Mixamo clip marks the bake");
    let record = RecipeBakeRecord {
        project: p.id.clone(),
        recipe: p.recipe.id.clone(),
        recipe_name: p.recipe.recipe.name.clone(),
        asset: "elsheria".into(),
        state: RecipeBakeState::Done,
        started_by: RecipeBakeStarter::Person,
        progress: None,
        error: None,
        started_ms: 0.0,
        updated_ms: 0.0,
        finished_ms: Some(1.0),
        result: Some(result),
        stale: None,
        review: None,
        added_version: None,
    };
    assert_eq!(stale_of(p.root(), &p.config, &record), None);
    store(
        p.root(),
        &library_clip("turn", "Turn Around", "Singles", "h3"),
    )
    .expect("again");
    let stale = stale_of(p.root(), &p.config, &record).expect("stale");
    assert!(stale.starts_with("Turn Around (clip 2)"), "{stale}");
}
