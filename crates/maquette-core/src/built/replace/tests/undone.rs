//! An undo after the built model shipped puts the old model back at the
//! next Ship, so that Ship's approval names it like a replace. And an undo
//! that would take off a version no build made says so first.

use super::ship::shipped_after;
use super::*;
use crate::pipeline::cache::project_cache_dir;
use crate::pipeline::history::{self, StepState};

/// Records `run` in the project's run history, as the runner does.
fn record(w: &World, run: &history::RunRecord) {
    let cache = project_cache_dir(&w.root.join(&w.config.layout.cache), &w.project);
    history::record_run(&cache, run).expect("run history");
}

#[test]
fn an_undo_after_a_ship_is_named_until_the_next_ship() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.replace(&planner.id, &desk.id);
    // A run built the replaced asset and shipped it into the project's repo.
    let mut shipped = shipped_after(1, StepState::Done);
    shipped.finished_at_ms = Some(history::now_ms());
    record(&w, &shipped);
    let runs = [shipped];
    assert!(swaps(&w.dir, &w.manifest(), &runs).is_empty(), "shipped");

    let plan = undo_plan(&w.env(), &planner.id).expect("plan");
    assert!(
        plan.warnings.iter().any(|w| w.contains("has shipped")),
        "{:?}",
        plan.warnings
    );
    undo(&w.env(), &planner.id).expect("undo");

    let listed = swaps(&w.dir, &w.manifest(), &runs);
    assert_eq!(listed.len(), 1, "the undo is a swap back");
    let back = &listed[0];
    assert!(back.undone);
    assert_eq!(back.asset, "spiral-planner");
    assert_eq!(back.script, "built/planner.py");
    assert_eq!(back.old_file.as_deref(), Some("spiral_planner_v2.glb"));
    assert_eq!(back.new_file.as_deref(), Some("spiral_planner_v1.glb"));
    assert_eq!((back.old_version, back.new_version), (2, 1));
    assert_eq!(
        (back.old_triangles, back.new_triangles),
        (Some(36), Some(12))
    );
    // The next run that ships the project writes it.
    let mut next = shipped_after(2, StepState::Done);
    next.finished_at_ms = Some(history::now_ms() + 60_000.0);
    assert!(swaps(&w.dir, &w.manifest(), &[runs[0].clone(), next]).is_empty());
}

#[test]
fn an_undo_before_any_ship_names_nothing() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.replace(&planner.id, &desk.id);

    undo(&w.env(), &planner.id).expect("undo");

    assert!(swaps(&w.dir, &w.manifest(), &[]).is_empty());
    assert!(load(&w.dir).expect("built").undone.is_empty());
}

#[test]
fn an_undo_names_a_version_no_build_made() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.replace(&planner.id, &desk.id);
    // A model accepted from the Inbox onto the replaced asset.
    w.add_model(&planner.id, &scan_model());

    let plan = undo_plan(&w.env(), &planner.id).expect("plan");

    let off: Vec<u32> = plan.taken_off.iter().map(|v| v.version).collect();
    assert_eq!(off, [2, 3]);
    assert_eq!(plan.not_built, [3]);
    assert!(
        plan.warnings.iter().any(|w| w.contains("v3")),
        "{:?}",
        plan.warnings
    );
}

#[test]
fn a_second_replace_and_undo_keep_the_swap_back_of_the_first() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.replace(&planner.id, &desk.id);
    let mut shipped = shipped_after(1, StepState::Done);
    shipped.finished_at_ms = Some(history::now_ms());
    record(&w, &shipped);
    let runs = [shipped];
    undo(&w.env(), &planner.id).expect("undo");

    // Replaced again, then undone again before any Ship: the site still has
    // the built model from the first replace.
    w.replace(&planner.id, &desk.id);
    undo(&w.env(), &planner.id).expect("undo again");

    let listed = swaps(&w.dir, &w.manifest(), &runs);
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert!(listed[0].undone);
    assert_eq!(listed[0].old_file.as_deref(), Some("spiral_planner_v2.glb"));
    assert_eq!(listed[0].new_file.as_deref(), Some("spiral_planner_v1.glb"));
}
