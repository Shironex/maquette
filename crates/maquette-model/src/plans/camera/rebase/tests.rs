use super::super::tests::{main_ids, set_stops, site, stop, tour};
use super::super::{CAMERA_PLAN_ID, ensure, is_stale, line_of};
use super::*;
use crate::plans::{NodeKind, save};
use std::fs;

fn node_of<'a>(plan: &'a PlanFile, id: &str) -> &'a LayoutStop {
    plan.nodes
        .iter()
        .filter(|n| n.by.is_none())
        .find_map(|n| stop_of(n).filter(|s| s.id == id))
        .expect("stop node")
}

fn edit(root: &Path, id: &str, change: impl FnOnce(&mut LayoutStop)) {
    let mut plan = ensure(root, None).expect("plan").plan;
    let node = plan
        .nodes
        .iter_mut()
        .find(|n| n.stop.as_ref().is_some_and(|s| s.id == id))
        .expect("node");
    change(node.stop.as_mut().expect("stop"));
    save(root, CAMERA_PLAN_ID, &plan).expect("save");
}

#[test]
fn an_outside_change_shows_and_a_rebase_takes_it_keeping_the_plans_edits() {
    let s = site(&["porch", "schedule", "moon"]);
    edit(&s.root, "moon", |m| m.fov = 28.0);
    edit(&s.root, "porch", |p| p.name = "Front porch".into());
    let mut outside = tour(&["porch", "schedule", "moon"]);
    outside[2].fov = 40.0; // clashes with the plan's 28
    outside[1].target = [0.0, 2.0, 0.0]; // only the layout changed this
    set_stops(&s.root, outside.clone());
    let plan = ensure(&s.root, None).expect("plan").plan;
    assert!(is_stale(&plan, &layout::load(&s.root).expect("layout")));

    let (dry, preview) =
        rebase(&s.root, CAMERA_PLAN_ID, RebaseMode::Rebase, true, None).expect("dry");
    assert_eq!(dry.plan, plan, "a dry run writes nothing");
    assert_eq!(preview.taken, ["Schedule: target"]);
    assert_eq!(preview.clashes.len(), 1, "{preview:?}");

    let (doc, report) =
        rebase(&s.root, CAMERA_PLAN_ID, RebaseMode::Rebase, false, None).expect("rebase");
    assert!(report.change.is_some());
    assert_eq!(node_of(&doc.plan, "schedule").target, [0.0, 2.0, 0.0]);
    assert!(
        (node_of(&doc.plan, "moon").fov - 28.0).abs() < f32::EPSILON,
        "the plan's side stays"
    );
    assert_eq!(node_of(&doc.plan, "porch").name, "Front porch");
    assert_eq!(doc.plan.camera.expect("base").stops, outside);
    let lines = crate::plans::log::lines(&s.root, CAMERA_PLAN_ID, 5).expect("log");
    assert_eq!(lines[0].mark, ChangeMark::Rebase);
}

#[test]
fn stops_the_layout_added_join_and_ones_it_removed_go_unless_the_plan_changed_them() {
    let s = site(&["porch", "schedule", "browser", "moon"]);
    edit(&s.root, "browser", |b| b.fov = 50.0);
    set_stops(&s.root, tour(&["porch", "lantern", "moon"]));
    let (doc, report) =
        rebase(&s.root, CAMERA_PLAN_ID, RebaseMode::Rebase, false, None).expect("rebase");
    assert_eq!(report.added, ["Lantern"]);
    assert_eq!(report.removed, ["Schedule"]);
    assert!(
        report.clashes.iter().any(|c| c.starts_with("Browser")),
        "{report:?}"
    );
    assert_eq!(main_ids(&doc.plan), ["porch", "lantern", "browser", "moon"]);
}

#[test]
fn a_new_order_comes_in_only_when_the_plan_kept_the_old_one() {
    let s = site(&["porch", "schedule", "moon"]);
    ensure(&s.root, None).expect("plan");
    set_stops(&s.root, tour(&["porch", "moon", "schedule"]));
    let (doc, report) =
        rebase(&s.root, CAMERA_PLAN_ID, RebaseMode::Rebase, false, None).expect("rebase");
    assert!(
        report.taken.contains(&"the tour order".to_string()),
        "{report:?}"
    );
    assert_eq!(main_ids(&doc.plan), ["porch", "moon", "schedule"]);

    // Both reorder: the plan's order stays and the clash is listed.
    let mut plan = doc.plan;
    plan.nodes.retain(|n| n.kind != NodeKind::Stop);
    plan.edges.clear();
    let (nodes, edges) = line_of(&tour(&["schedule", "porch", "moon"]), 0, 0, None);
    plan.nodes.extend(nodes);
    plan.edges.extend(edges);
    save(&s.root, CAMERA_PLAN_ID, &plan).expect("save");
    set_stops(&s.root, tour(&["moon", "porch", "schedule"]));
    let (doc, report) =
        rebase(&s.root, CAMERA_PLAN_ID, RebaseMode::Rebase, false, None).expect("rebase");
    assert!(
        report.clashes.iter().any(|c| c.contains("order")),
        "{report:?}"
    );
    assert_eq!(main_ids(&doc.plan), ["schedule", "porch", "moon"]);
}

#[test]
fn start_over_builds_the_main_line_again_and_keeps_proposals() {
    let s = site(&["porch", "moon"]);
    let mut plan = ensure(&s.root, None).expect("plan").plan;
    let (branch, links) = line_of(&[stop("lantern", 3.0)], 0, 400, Some("Camera agent"));
    plan.nodes.extend(branch);
    plan.edges.extend(links);
    // A stop the person set aside, off the main line.
    let (aside, _) = line_of(&[stop("bell", 4.0)], 0, 800, None);
    let aside_id = aside[0].id.clone();
    plan.nodes.extend(aside);
    save(&s.root, CAMERA_PLAN_ID, &plan).expect("save");
    edit(&s.root, "moon", |m| m.fov = 20.0);
    set_stops(&s.root, tour(&["dock", "porch"]));
    let (doc, report) =
        rebase(&s.root, CAMERA_PLAN_ID, RebaseMode::StartOver, false, None).expect("over");
    assert_eq!(main_ids(&doc.plan), ["dock", "porch"]);
    assert!(
        doc.plan.nodes.iter().any(|n| n.id == aside_id),
        "a stop set aside stays"
    );
    // One person's change that undoes whole: the old main line comes back.
    let change = report.change.expect("change");
    let back = crate::plans::undo(&s.root, CAMERA_PLAN_ID, &[change], None, None).expect("undo");
    assert_eq!(main_ids(&back.doc.plan), ["porch", "moon"]);
    assert!(
        doc.plan
            .nodes
            .iter()
            .any(|n| n.by.as_deref() == Some("Camera agent"))
    );
    assert!(!is_stale(
        &doc.plan,
        &layout::load(&s.root).expect("layout")
    ));
}

#[test]
fn a_rebase_refuses_when_layout_json_changed_since_the_comparison_shown() {
    let s = site(&["porch", "schedule", "moon"]);
    ensure(&s.root, None).expect("plan");
    set_stops(&s.root, tour(&["porch", "moon"]));
    let (_, shown) =
        rebase(&s.root, CAMERA_PLAN_ID, RebaseMode::Rebase, true, None).expect("compare");
    assert_eq!(
        shown.layout_key,
        layout_key(&layout::load(&s.root).expect("layout"))
    );
    // layout.json moves again after the comparison was shown.
    set_stops(&s.root, tour(&["porch", "lantern", "moon"]));
    let before = fs::read_to_string(s.root.join("plans/camera-plan.plan.json")).expect("plan");
    for mode in [RebaseMode::Rebase, RebaseMode::StartOver] {
        let refused = rebase(
            &s.root,
            CAMERA_PLAN_ID,
            mode,
            false,
            Some(&shown.layout_key),
        )
        .expect_err("stale comparison");
        assert!(refused.to_string().contains("Review again"), "{refused}");
    }
    let after = fs::read_to_string(s.root.join("plans/camera-plan.plan.json")).expect("plan");
    assert_eq!(after, before, "nothing written");
    // A fresh comparison's key goes.
    let (_, fresh) =
        rebase(&s.root, CAMERA_PLAN_ID, RebaseMode::Rebase, true, None).expect("compare");
    let (_, done) = rebase(
        &s.root,
        CAMERA_PLAN_ID,
        RebaseMode::Rebase,
        false,
        Some(&fresh.layout_key),
    )
    .expect("rebase");
    assert!(done.change.is_some());
}
