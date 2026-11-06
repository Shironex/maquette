//! A recipe on the rest pose: no base clip is needed, the job asks the
//! bake script to hold the rest pose, and the result names no base clip,
//! so a clip library change never makes it stale and it is never Mixamo's.

use std::fs;

use super::job::{self, JOB_FILE, REPORT_FILE};
use super::result::{read_run, result_of};
use super::tests::{Project, REPORT, humanoid, point, project_on, reach};
use super::*;
use crate::animations::recipes::BaseStatus;
use crate::recipes::{REST_FRAMES, REST_POSE_NAME};

/// Frostveil with Elsheria and a recipe on the rest pose reaching its right
/// hand to a point.
fn resting() -> (tempfile::TempDir, Project) {
    let tmp = tempfile::tempdir().expect("temp");
    let mut p = project_on(tmp.path(), None, humanoid());
    p.change(
        vec![reach("l-1", "rightHand", "b-1")],
        vec![point("b-1", Some([-0.3, 1.2, 0.35]))],
    );
    (tmp, p)
}

#[test]
fn a_rest_pose_recipe_needs_no_base_clip_and_is_never_mixamo() {
    let (_tmp, p) = resting();
    let base = &p.recipe.recipe.base;
    assert!(base.rest);
    assert_eq!(
        (base.name.as_str(), base.frames, base.hash.as_str()),
        (REST_POSE_NAME, REST_FRAMES, "")
    );
    assert_eq!(p.recipe.base.status, BaseStatus::Rest);
    assert!(p.recipe.base.warning.is_none());
    assert!(!p.recipe.mixamo);

    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    assert!(prepared.clip.is_none());
    assert_eq!(prepared.setting.name, "reach_out");
    assert_eq!(prepared.setting.end, REST_FRAMES);
    assert!(prepared.bones.contains_key("rightUpperArm"));
}

#[test]
fn staging_a_rest_pose_bake_copies_no_clip_and_asks_for_the_rest_pose() {
    let (_tmp, p) = resting();
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let work = p.root().join("work");
    job::stage(p.root(), &prepared, &work).expect("stage");
    assert!(work.join("in/model.glb").is_file());
    assert!(!work.join("in/clips").exists(), "no clip goes in");
    let job: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join(JOB_FILE)).expect("job")).expect("json");
    assert_eq!(job["retarget"]["clips"], serde_json::json!([]));
    assert!(job["recipe"]["clip"].is_null());
    assert_eq!(job["recipe"]["rest"]["name"], "reach_out");
    assert_eq!(job["recipe"]["rest"]["frames"], REST_FRAMES);
    assert_eq!(job["recipe"]["rest"]["fps"], 30.0);
}

#[test]
fn a_rest_pose_result_names_no_base_clip_and_never_goes_stale_on_one() {
    let (_tmp, p) = resting();
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let work = p.root().join("work");
    fs::create_dir_all(&work).expect("work");
    fs::write(work.join(REPORT_FILE), REPORT).expect("report");
    let result = result_of(&read_run(&work).expect("run"), &prepared, &work, 1.0);
    assert_eq!(
        (result.base_clip.as_str(), result.base_hash.as_str()),
        ("", "")
    );
    assert!(result.recipe_base_hash.is_none());
    assert!(!result.mixamo);
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
}
