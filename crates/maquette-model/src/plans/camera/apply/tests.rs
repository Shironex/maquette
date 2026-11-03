use std::fs;

use super::super::tests::{main_ids, set_stops, site, stop, tour};
use super::super::{CAMERA_PLAN_ID, ensure, stop_node};
use super::*;
use crate::plans::{EdgeKind, save, undo};

/// The camera plan with its main line reordered to `ids` (new ids become
/// new stops), saved as the person would.
fn plan_line(root: &Path, ids: &[&str]) -> PlanFile {
    let mut plan = ensure(root, None).expect("camera plan").plan;
    let base = plan.camera.clone().expect("base").stops;
    plan.nodes
        .retain(|n| n.kind != crate::plans::NodeKind::Stop);
    plan.edges.retain(|e| e.kind != EdgeKind::LeadsTo);
    let stops: Vec<LayoutStop> = ids
        .iter()
        .map(|id| {
            base.iter()
                .find(|s| s.id == *id)
                .cloned()
                .unwrap_or_else(|| stop(id, 7.0))
        })
        .collect();
    let (nodes, edges) = super::super::line_of(&stops, 0, 0, None);
    plan.nodes = nodes;
    plan.edges = edges;
    save(root, CAMERA_PLAN_ID, &plan).expect("save").plan
}

#[test]
fn apply_writes_the_main_line_logs_it_and_keeps_the_old_layout() {
    let s = site(&["porch", "schedule", "browser", "moon", "pullback"]);
    plan_line(
        &s.root,
        &[
            "porch", "moon", "lantern", "schedule", "browser", "pullback",
        ],
    );
    let shown = review(&s.root, CAMERA_PLAN_ID).expect("review");
    assert!(shown.problems.is_empty(), "{:?}", shown.problems);
    let moon = shown
        .diff
        .stops
        .iter()
        .find(|s| s.id == "moon")
        .expect("moon");
    assert!(moon.moved, "the moon moving shows: {moon:?}");
    assert_eq!((shown.diff.added, shown.diff.moved), (1, 1));

    let (doc, entry) =
        apply(&s.root, CAMERA_PLAN_ID, &shown.layout_key, &shown.plan_key).expect("apply");
    let written = layout::load(&s.root).expect("layout");
    let ids: Vec<&str> = written.stops.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "porch", "moon", "lantern", "schedule", "browser", "pullback"
        ]
    );
    // The person's change, with the old layout kept beside the plans.
    assert_eq!(entry.by, None);
    assert_eq!(entry.mark, ChangeMark::Apply);
    let applied = entry.applied.clone().expect("applied");
    let kept: SceneLayout =
        serde_json::from_str(&fs::read_to_string(s.root.join(&applied.history)).expect("history"))
            .expect("json");
    assert_eq!(
        kept.stops,
        tour(&["porch", "schedule", "browser", "moon", "pullback"])
    );
    assert_eq!(applied.after, layout_key(&written));
    // The plan is now based on what it wrote, so nothing is left to apply.
    assert_eq!(doc.plan.camera.expect("base").stops, written.stops);
    let again = review(&s.root, CAMERA_PLAN_ID).expect("review");
    assert!(
        again
            .problems
            .iter()
            .any(|p| p.contains("Nothing to apply")),
        "{:?}",
        again.problems
    );
    let lines = log::lines(&s.root, CAMERA_PLAN_ID, 10).expect("lines");
    assert_eq!(lines[0].mark, ChangeMark::Apply);
    assert!(
        lines[0].summary.contains("1 stop added · 1 moved"),
        "{}",
        lines[0].summary
    );
}

#[test]
fn apply_refuses_when_layout_json_or_the_plan_changed_since_the_review() {
    let s = site(&["porch", "schedule", "moon"]);
    plan_line(&s.root, &["porch", "moon", "schedule"]);
    let shown = review(&s.root, CAMERA_PLAN_ID).expect("review");
    // Someone moves a model in the Inspector: the layout differs from the review.
    layout::update(&s.root, |l| {
        let mut next = l.clone();
        next.desk = Some(layout::Desk { size: [1.0, 1.0] });
        Ok(next)
    })
    .expect("outside change");
    let refused =
        apply(&s.root, CAMERA_PLAN_ID, &shown.layout_key, &shown.plan_key).expect_err("stale");
    assert!(refused.to_string().contains("Review again"), "{refused}");
    let ids: Vec<String> = layout::load(&s.root)
        .expect("layout")
        .stops
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, ["porch", "schedule", "moon"], "nothing applied");
    assert!(
        !s.root.join("plans/history").exists(),
        "no history for a refused Apply"
    );

    // A fresh review goes; the plan changing after it refuses again.
    let shown = review(&s.root, CAMERA_PLAN_ID).expect("review");
    let mut plan = ensure(&s.root, None).expect("plan").plan;
    plan.nodes[0].stop.as_mut().expect("stop").fov = 50.0;
    save(&s.root, CAMERA_PLAN_ID, &plan).expect("edit");
    let refused = apply(&s.root, CAMERA_PLAN_ID, &shown.layout_key, &shown.plan_key)
        .expect_err("plan changed");
    assert!(refused.to_string().contains("plan changed"), "{refused}");
}

#[test]
fn a_plan_behind_layout_json_can_not_be_applied_until_it_is_rebased() {
    let s = site(&["porch", "moon"]);
    plan_line(&s.root, &["moon", "porch"]);
    set_stops(&s.root, tour(&["porch", "moon", "dock"]));
    let shown = review(&s.root, CAMERA_PLAN_ID).expect("review");
    assert!(shown.stale);
    assert!(
        shown.problems.iter().any(|p| p.contains("rebase")),
        "{:?}",
        shown.problems
    );
    assert!(apply(&s.root, CAMERA_PLAN_ID, &shown.layout_key, &shown.plan_key).is_err());
}

#[test]
fn undo_puts_the_old_layout_back_only_while_it_is_unchanged() {
    let s = site(&["porch", "schedule", "moon"]);
    plan_line(&s.root, &["porch", "moon", "schedule"]);
    let shown = review(&s.root, CAMERA_PLAN_ID).expect("review");
    let (_, entry) =
        apply(&s.root, CAMERA_PLAN_ID, &shown.layout_key, &shown.plan_key).expect("apply");

    // The generic undo (and so an agent's undo_run) refuses an Apply.
    let refused = undo(
        &s.root,
        CAMERA_PLAN_ID,
        std::slice::from_ref(&entry.id),
        Some("Camera agent"),
        Some("Camera agent"),
    )
    .expect_err("agent");
    assert!(
        refused
            .to_string()
            .contains("Only the person undoes an Apply"),
        "{refused}"
    );

    let (doc, line) = undo_apply(&s.root, CAMERA_PLAN_ID, &entry.id).expect("undo");
    assert_eq!(
        layout::load(&s.root).expect("layout").stops,
        tour(&["porch", "schedule", "moon"])
    );
    assert_eq!(line.undoes, vec![entry.id.clone()]);
    assert_eq!(
        doc.plan.camera.as_ref().expect("base").stops,
        tour(&["porch", "schedule", "moon"])
    );
    // The plan keeps its line, so it can be applied again.
    assert_eq!(main_ids(&doc.plan), ["porch", "moon", "schedule"]);
    assert!(
        undo_apply(&s.root, CAMERA_PLAN_ID, &entry.id).is_err(),
        "undone already"
    );

    // Applied again, then layout.json changes: the undo is refused and nothing moves.
    let shown = review(&s.root, CAMERA_PLAN_ID).expect("review");
    let (_, entry) =
        apply(&s.root, CAMERA_PLAN_ID, &shown.layout_key, &shown.plan_key).expect("apply again");
    let mut changed = layout::load(&s.root).expect("layout").stops;
    changed[0].fov = 60.0;
    set_stops(&s.root, changed.clone());
    let refused = undo_apply(&s.root, CAMERA_PLAN_ID, &entry.id).expect_err("changed since");
    assert!(
        refused.to_string().contains("changed since this Apply"),
        "{refused}"
    );
    assert_eq!(layout::load(&s.root).expect("layout").stops, changed);
}

#[test]
fn an_agent_proposal_is_reviewed_but_never_the_line_apply_writes() {
    let s = site(&["porch", "schedule", "moon"]);
    let mut plan = ensure(&s.root, None).expect("plan").plan;
    let branch = stop_node(&stop("lantern", 2.0), 0, 400, Some("Camera agent"));
    let branch_id = branch.id.clone();
    plan.nodes.push(branch);
    let plan = save(&s.root, CAMERA_PLAN_ID, &plan).expect("save").plan;
    let layout = layout::load(&s.root).expect("layout");
    let proposal = review_of(&plan, &layout, Some(&branch_id));
    assert_eq!((proposal.diff.added, proposal.diff.removed), (1, 3));
    let main = review_of(&plan, &layout, None);
    assert!(
        main.problems.iter().any(|p| p.contains("Nothing to apply")),
        "{:?}",
        main.problems
    );
}

#[test]
fn an_apply_whose_log_can_not_be_written_leaves_layout_json_as_it_was() {
    let s = site(&["porch", "schedule", "moon"]);
    plan_line(&s.root, &["porch", "moon", "schedule"]);
    let before = fs::read_to_string(s.root.join(layout::LAYOUT_FILE)).expect("layout");
    let plan_before = fs::read_to_string(s.root.join("plans/camera-plan.plan.json")).expect("plan");
    let shown = review(&s.root, CAMERA_PLAN_ID).expect("review");
    // The log is a folder now: appending its line fails after layout.json was written.
    let log_file = s.root.join("plans/camera-plan.log.jsonl");
    let _ = fs::remove_file(&log_file);
    fs::create_dir_all(&log_file).expect("log in the way");

    let refused =
        apply(&s.root, CAMERA_PLAN_ID, &shown.layout_key, &shown.plan_key).expect_err("no log");
    assert!(refused.to_string().contains("put back"), "{refused}");
    let after = fs::read_to_string(s.root.join(layout::LAYOUT_FILE)).expect("layout");
    assert_eq!(
        layout_key(&serde_json::from_str::<SceneLayout>(&after).expect("json")),
        layout_key(&serde_json::from_str::<SceneLayout>(&before).expect("json")),
        "layout.json is as it was"
    );
    let plan_after = fs::read_to_string(s.root.join("plans/camera-plan.plan.json")).expect("plan");
    assert_eq!(
        serde_json::from_str::<PlanFile>(&plan_after)
            .expect("plan")
            .camera,
        serde_json::from_str::<PlanFile>(&plan_before)
            .expect("plan")
            .camera,
        "the plan's base did not move"
    );
    let kept = fs::read_dir(s.root.join("plans/history")).map_or(0, Iterator::count);
    assert_eq!(kept, 0, "no history copy stays behind");
}

#[test]
fn an_apply_line_naming_a_file_outside_plans_history_is_refused() {
    let s = site(&["porch"]);
    for bad in [
        "../layout.json",
        "/etc/passwd",
        "plans/other.json",
        "plans/history/../../x",
    ] {
        assert!(history_file(&s.root, bad).is_err(), "{bad}");
    }
    assert!(history_file(&s.root, "plans/history/c-1.layout.json").is_ok());
}

#[test]
fn a_person_change_whose_undo_line_would_be_cut_is_refused_with_nothing_written() {
    let s = site(&["porch", "moon"]);
    let plan = ensure(&s.root, None).expect("plan").plan;
    let file = s.root.join("plans/camera-plan.plan.json");
    let before = fs::read_to_string(&file).expect("plan");
    let mut next = plan.clone();
    next.nodes[0].stop.as_mut().expect("stop").fov = 50.0;
    let line = person_line("x".repeat(log::LINE_MAX_BYTES), ChangeMark::Rebase, None);
    let refused = write_person(&s.root, CAMERA_PLAN_ID, &plan, &next, line).expect_err("too long");
    assert!(
        refused.to_string().contains("too big to keep for undo"),
        "{refused}"
    );
    assert_eq!(fs::read_to_string(&file).expect("plan"), before);
}

#[test]
fn a_failed_undo_never_rolls_back_over_a_newer_layout() {
    let s = site(&["porch", "schedule", "moon"]);
    plan_line(&s.root, &["porch", "moon", "schedule"]);
    let shown = review(&s.root, CAMERA_PLAN_ID).expect("review");
    let (_, entry) =
        apply(&s.root, CAMERA_PLAN_ID, &shown.layout_key, &shown.plan_key).expect("apply");
    let applied = entry.applied.expect("applied");
    let file = history_file(&s.root, &applied.history).expect("kept");
    // The undo puts the old layout back...
    let (replaced, written) = put_back(&s.root, &file, &applied.after, "changed").expect("back");
    assert_eq!(layout_key(&replaced), applied.after);
    // ...another writer lands before its log line fails...
    layout::update(&s.root, |l| {
        let mut next = l.clone();
        next.desk = Some(layout::Desk { size: [2.0, 2.0] });
        Ok(next)
    })
    .expect("other writer");
    // ...and the rollback leaves their layout alone.
    assert!(roll_back(&s.root, &written, &replaced).is_err());
    let now = layout::load(&s.root).expect("layout");
    assert_eq!(now.desk.map(|d| d.size), Some([2.0, 2.0]));

    // With no other writer, the rollback puts the Apply's layout back.
    let other = layout::load(&s.root).expect("layout");
    assert!(roll_back(&s.root, &other, &replaced).is_ok());
    assert_eq!(
        layout_key(&layout::load(&s.root).expect("layout")),
        applied.after
    );
}

#[test]
fn an_apply_over_a_hand_edited_layout_can_still_be_undone() {
    let s = site(&["porch", "schedule", "moon"]);
    // By hand: an older version and a clip choice that only says ship, which a write tidies away.
    let file = s.root.join("layout.json");
    let mut raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&file).expect("layout")).expect("json");
    raw["version"] = serde_json::json!(0);
    raw["clips"] = serde_json::json!({ "lantern": { "idle": { "ship": true } } });
    fs::write(&file, serde_json::to_string_pretty(&raw).expect("text")).expect("hand edit");
    plan_line(&s.root, &["porch", "moon", "schedule"]);
    let shown = review(&s.root, CAMERA_PLAN_ID).expect("review");
    let (_, entry) =
        apply(&s.root, CAMERA_PLAN_ID, &shown.layout_key, &shown.plan_key).expect("apply");
    // The after key is the file as stored, not the layout handed to the write.
    let stored = layout::load(&s.root).expect("layout");
    assert!(stored.clips.is_empty(), "the write tidied the clip away");
    assert_eq!(entry.applied.expect("applied").after, layout_key(&stored));
    undo_apply(&s.root, CAMERA_PLAN_ID, &entry.id).expect("undo works");
    let ids: Vec<String> = layout::load(&s.root)
        .expect("layout")
        .stops
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, ["porch", "schedule", "moon"]);
}
