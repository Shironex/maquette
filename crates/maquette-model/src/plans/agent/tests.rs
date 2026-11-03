//! An agent's tools on a canvas: placed nodes, links by `$n`, edits, the
//! `by` rule for removing, and tidying only the agent's own nodes.

use std::fs;

use super::super::*;
use super::{Agent, NewLink, NewNode, NodeEdit, add, layout, remove, update};

const RESEARCH: Agent<'static> = Agent {
    name: "Research agent",
    run: None,
};
const CAMERA: Agent<'static> = Agent {
    name: "Camera agent",
    run: None,
};

fn setup() -> (tempfile::TempDir, std::path::PathBuf, String) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("moon-island");
    fs::create_dir_all(root.join("refs")).expect("refs");
    fs::write(root.join("refs/porch.png"), b"png").expect("picture");
    let made = create(&root, "Hero ideas", PlanPreset::Brainstorm).expect("create");
    let mut plan = made.plan.clone();
    plan.nodes = vec![PlanNode {
        id: "n-porch".into(),
        kind: NodeKind::Note,
        x: 0,
        y: 0,
        width: None,
        title: "Porch at dusk".into(),
        text: String::new(),
        image: None,
        url: None,
        asset: None,
        direction: None,
        by: None,
        person_wrote: false,
        stop: None,
    }];
    save(&root, &made.id, &plan).expect("save");
    (dir, root, made.id)
}

fn note(text: &str) -> NewNode {
    NewNode {
        text: text.into(),
        ..NewNode::default()
    }
}

fn link(from: &str, to: &str) -> NewLink {
    NewLink {
        from: from.into(),
        to: to.into(),
        kind: EdgeKind::Relates,
        label: None,
    }
}

fn node<'a>(plan: &'a PlanFile, id: &str) -> &'a PlanNode {
    plan.nodes.iter().find(|n| n.id == id).expect("node")
}

#[test]
fn added_nodes_land_beside_near_clear_of_each_other() {
    let (_dir, root, id) = setup();
    let picture = NewNode {
        image: Some("refs/porch.png".into()),
        title: "Porch ref".into(),
        ..NewNode::default()
    };
    let done = add(
        &root,
        &id,
        &RESEARCH,
        Some("n-porch"),
        &[note("Dusk light"), picture, note("Lantern")],
        &[link("n-porch", "$0"), link("$0", "$1"), link("$1", "$0")],
    )
    .expect("add");
    assert_eq!(done.nodes.len(), 3);
    assert_eq!(done.placed, 3);
    assert_eq!(done.edges.len(), 2, "the repeated pair is left out");
    assert_eq!(done.notes.len(), 1);
    let plan = &done.changed.doc.plan;
    let first = node(plan, &done.nodes[0]);
    assert!(first.x > 220, "right of the porch note: {}", first.x);
    assert_eq!(first.by.as_deref(), Some("Research agent"));
    assert_eq!(node(plan, &done.nodes[1]).kind, NodeKind::Image);
    let ys: Vec<i32> = done.nodes.iter().map(|n| node(plan, n).y).collect();
    assert!(
        ys[0] < ys[1] && ys[1] < ys[2],
        "stacked down a column: {ys:?}"
    );
    let edge = plan
        .edges
        .iter()
        .find(|e| e.from == "n-porch")
        .expect("edge");
    assert_eq!(edge.from_side, Some(Side::Right));
    assert_eq!(edge.by.as_deref(), Some("Research agent"));
    // Bad input is refused whole.
    let missing = NewNode {
        image: Some("refs/none.png".into()),
        ..NewNode::default()
    };
    assert!(add(&root, &id, &RESEARCH, None, &[missing], &[]).is_err());
    assert!(add(&root, &id, &RESEARCH, Some("n-gone"), &[note("x")], &[]).is_err());
    assert!(add(&root, &id, &RESEARCH, None, &[], &[link("$4", "n-porch")]).is_err());
    let asset_with_text = NewNode {
        asset: Some("lantern".into()),
        text: "a lantern".into(),
        ..NewNode::default()
    };
    assert!(add(&root, &id, &RESEARCH, None, &[asset_with_text], &[]).is_err());
    assert_eq!(get(&root, &id).expect("get").plan.nodes.len(), 4);
}

#[test]
fn an_empty_canvas_starts_at_the_origin() {
    let (_dir, root, _) = setup();
    let empty = create(&root, "Empty", PlanPreset::Story).expect("create");
    let done = add(&root, &empty.id, &RESEARCH, None, &[note("first")], &[]).expect("add");
    let first = node(&done.changed.doc.plan, &done.nodes[0]);
    assert_eq!((first.x, first.y), (0, 0));
}

#[test]
fn agents_edit_any_node_but_remove_only_their_own() {
    let (_dir, root, id) = setup();
    let done = add(&root, &id, &RESEARCH, Some("n-porch"), &[note("Mine")], &[]).expect("add");
    let mine = done.nodes[0].clone();
    let edited = update(
        &root,
        &id,
        &RESEARCH,
        &[NodeEdit {
            id: "n-porch".into(),
            text: Some("Keep it under the lantern line.".into()),
            ..NodeEdit::default()
        }],
    )
    .expect("edit a person's note");
    assert_eq!(
        node(&edited.changed.doc.plan, "n-porch").text,
        "Keep it under the lantern line."
    );
    assert!(
        update(
            &root,
            &id,
            &RESEARCH,
            &[NodeEdit {
                id: "n-nope".into(),
                ..NodeEdit::default()
            }]
        )
        .is_err()
    );

    let refused = remove(&root, &id, &RESEARCH, &["n-porch".into()], &[]).expect_err("person");
    assert!(refused.to_string().contains("ask the person"), "{refused}");
    assert!(
        remove(&root, &id, &CAMERA, std::slice::from_ref(&mine), &[]).is_err(),
        "another agent's"
    );

    // A person links to the agent's note: now the note takes a person's link with it.
    let mut plan = get(&root, &id).expect("get").plan;
    plan.edges.push(PlanEdge {
        id: "e-person".into(),
        from: "n-porch".into(),
        to: mine.clone(),
        from_side: None,
        to_side: None,
        kind: EdgeKind::Relates,
        label: None,
        by: None,
    });
    save(&root, &id, &plan).expect("save");
    assert!(remove(&root, &id, &RESEARCH, std::slice::from_ref(&mine), &[]).is_err());
    assert!(remove(&root, &id, &RESEARCH, &[], &["e-person".into()]).is_err());

    plan.edges.clear();
    save(&root, &id, &plan).expect("save");
    let gone = remove(
        &root,
        &id,
        &RESEARCH,
        std::slice::from_ref(&mine),
        &["e-none".into()],
    )
    .expect("remove own");
    assert_eq!(gone.changed.doc.plan.nodes.len(), 1);
    assert_eq!(gone.notes.len(), 1, "a missing link is noted, not refused");
}

#[test]
fn layout_moves_only_the_agents_overlapping_nodes() {
    let (_dir, root, id) = setup();
    // Two of the agent's notes dropped on the person's note.
    let at = |text: &str| NewNode {
        at: Some((10, 10)),
        ..note(text)
    };
    let done = add(
        &root,
        &id,
        &RESEARCH,
        None,
        &[at("A"), at("B")],
        &[link("n-porch", "$0")],
    )
    .expect("add");
    assert_eq!(done.placed, 0);
    // Auto layout after agents is on by default and tidied them at once, as
    // a line of its own; undo just that to tidy them by hand.
    let tidied = done
        .tidied
        .clone()
        .and_then(|t| t.entry)
        .expect("auto layout line");
    assert_eq!(tidied.summary, "Moved 2 nodes with auto layout");
    undo(
        &root,
        &id,
        &[tidied.id],
        Some("Research agent"),
        Some("Research agent"),
    )
    .expect("undo the auto layout");
    assert!(
        layout(&root, &id, &RESEARCH, &["n-porch".into()]).is_err(),
        "not the agent's"
    );
    let tidy = layout(&root, &id, &RESEARCH, &[]).expect("layout");
    assert_eq!(tidy.placed, 2);
    let plan = &tidy.changed.doc.plan;
    assert_eq!((node(plan, "n-porch").x, node(plan, "n-porch").y), (0, 0));
    let a = node(plan, &done.nodes[0]);
    let b = node(plan, &done.nodes[1]);
    assert!(
        a.x >= 220 && b.x >= 220,
        "beside the porch: {} {}",
        a.x,
        b.x
    );
    assert_ne!((a.x, a.y), (b.x, b.y));
    let again = layout(&root, &id, &RESEARCH, &[]).expect("layout again");
    assert_eq!(again.placed, 0, "clear nodes stay");
    assert!(again.changed.entry.is_none());
}
