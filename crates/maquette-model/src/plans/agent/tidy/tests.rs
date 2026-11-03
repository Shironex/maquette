//! "Auto layout after agents": on by default, it tidies the agent's edited
//! nodes the call gave no x or y inside the same change (one undo); a
//! position the agent gives is kept; off, it leaves the canvas to the
//! person and layout_canvas is refused. A person's nodes, and an agent's
//! node a person wrote in, never move either way.

use std::fs;

use super::super::super::ops::PlanOp;
use super::super::super::*;
use super::super::{Agent, NewNode, NodeEdit, add, layout, update};

const RESEARCH: Agent<'static> = Agent {
    name: "Research agent",
    run: Some("r-1"),
};

fn person_note(id: &str, x: i32) -> PlanNode {
    PlanNode {
        id: id.into(),
        kind: NodeKind::Note,
        x,
        y: 0,
        width: None,
        title: format!("Person {id}"),
        text: String::new(),
        image: None,
        url: None,
        asset: None,
        direction: None,
        by: None,
        person_wrote: false,
        stop: None,
    }
}

fn setup() -> (tempfile::TempDir, std::path::PathBuf, String) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("moon-island");
    fs::create_dir_all(&root).expect("project");
    let made = create(&root, "Hero ideas", PlanPreset::Brainstorm).expect("create");
    let mut plan = made.plan.clone();
    plan.nodes = vec![person_note("n-a", 0), person_note("n-b", 20)];
    save(&root, &made.id, &plan).expect("save");
    (dir, root, made.id)
}

fn dropped(text: &str, x: i32) -> NewNode {
    NewNode {
        text: text.into(),
        at: Some((x, 10)),
        ..NewNode::default()
    }
}

fn spot(plan: &PlanFile, id: &str) -> (i32, i32) {
    let n = plan.nodes.iter().find(|n| n.id == id).expect("node");
    (n.x, n.y)
}

fn text(id: &str, text: &str) -> NodeEdit {
    NodeEdit {
        id: id.into(),
        text: Some(text.into()),
        ..NodeEdit::default()
    }
}

#[test]
fn on_by_default_it_tidies_only_unplaced_edits_inside_the_same_line() {
    let (_dir, root, id) = setup();
    assert!(get(&root, &id).expect("get").plan.auto_layout);
    // A position the agent gives is kept, even on another node.
    let done = add(&root, &id, &RESEARCH, None, &[dropped("Lantern", 5)], &[]).expect("add");
    assert!(done.tidied.is_empty());
    let mine = done.nodes[0].clone();
    assert_eq!(spot(&done.changed.doc.plan, &mine), (5, 10));

    // An edit with no x or y moves it clear, in the same line.
    let lines = log::lines(&root, &id, 50).expect("log").len();
    let edited = update(&root, &id, &RESEARCH, &[text(&mine, "Lantern, lit")]).expect("edit");
    assert_eq!(edited.tidied, vec![mine.clone()]);
    assert_eq!(log::lines(&root, &id, 50).expect("log").len(), lines + 1);
    let plan = &edited.changed.doc.plan;
    assert_eq!(spot(plan, "n-a"), (0, 0), "a person's node never moves");
    assert_eq!(spot(plan, "n-b"), (20, 0), "even when it overlaps another");
    assert_ne!(spot(plan, &mine), (5, 10));

    // The person's one undo takes back the edit and the move together.
    let line = edited.changed.entry.expect("line").id;
    let undone = undo(&root, &id, &[line], None, None).expect("undo");
    let back = undone
        .doc
        .plan
        .nodes
        .iter()
        .find(|n| n.id == mine)
        .expect("node");
    assert_eq!(((back.x, back.y), back.text.as_str()), ((5, 10), "Lantern"));

    // An edit that gives x and y is kept; a person's node is never tidied.
    let moved = NodeEdit {
        x: Some(0),
        y: Some(0),
        ..text(&mine, "Lantern, here")
    };
    let kept = update(&root, &id, &RESEARCH, &[moved]).expect("move");
    assert!(kept.tidied.is_empty());
    assert_eq!(spot(&kept.changed.doc.plan, &mine), (0, 0));
    let person = update(&root, &id, &RESEARCH, &[text("n-a", "Mine")]).expect("edit");
    assert!(person.tidied.is_empty());
    assert_eq!(spot(&person.changed.doc.plan, "n-a"), (0, 0));
}

#[test]
fn a_node_a_person_wrote_in_never_moves() {
    let (_dir, root, id) = setup();
    let done = add(&root, &id, &RESEARCH, None, &[dropped("Lantern", 40)], &[]).expect("add");
    let mine = done.nodes[0].clone();
    let plan = get(&root, &id).expect("get").plan;
    let before = plan
        .nodes
        .iter()
        .find(|n| n.id == mine)
        .expect("node")
        .clone();
    let mut after = before.clone();
    after.text = "My lantern".into();
    (after.x, after.y) = (40, 0);
    apply_person(
        &root,
        &id,
        &plan.created,
        vec![PlanOp::SetNode { before, after }],
    )
    .expect("person writes in it");
    assert!(
        get(&root, &id)
            .expect("get")
            .plan
            .nodes
            .iter()
            .any(|n| n.id == mine && n.person_wrote)
    );

    // The agent edits only the title: it stays where the person left it.
    let title = NodeEdit {
        id: mine.clone(),
        title: Some("Lantern".into()),
        ..NodeEdit::default()
    };
    let edited = update(&root, &id, &RESEARCH, &[title]).expect("edit");
    assert!(edited.tidied.is_empty());
    assert_eq!(spot(&edited.changed.doc.plan, &mine), (40, 0));

    // layout_canvas leaves it too, and refuses to name it.
    let all = layout(&root, &id, &RESEARCH, &[]).expect("layout");
    assert_eq!(all.placed, 0);
    assert_eq!(spot(&all.changed.doc.plan, &mine), (40, 0));
    let refused = layout(&root, &id, &RESEARCH, std::slice::from_ref(&mine)).expect_err("theirs");
    assert!(refused.to_string().contains("person wrote in"), "{refused}");
}

#[test]
fn off_it_leaves_the_layout_to_the_person() {
    let (_dir, root, id) = setup();
    set_auto_layout(&root, &id, false, None).expect("off");
    let done = add(&root, &id, &RESEARCH, None, &[dropped("Lantern", 5)], &[]).expect("add");
    assert!(done.tidied.is_empty());
    assert_eq!(spot(&done.changed.doc.plan, &done.nodes[0]), (5, 10));
    let edited = update(&root, &id, &RESEARCH, &[text(&done.nodes[0], "Lit")]).expect("edit");
    assert!(edited.tidied.is_empty());
    assert_eq!(spot(&edited.changed.doc.plan, &done.nodes[0]), (5, 10));
    let refused = layout(&root, &id, &RESEARCH, &[]).expect_err("off");
    assert!(
        refused
            .to_string()
            .contains("turned Auto layout after agents off"),
        "{refused}"
    );

    // Placed nodes (no x and y) still land clear of the others.
    let placed = add(
        &root,
        &id,
        &RESEARCH,
        Some("n-a"),
        &[NewNode::default()],
        &[],
    )
    .expect("add");
    assert_eq!(placed.placed, 1);

    set_auto_layout(&root, &id, true, None).expect("on");
    let tidy = layout(&root, &id, &RESEARCH, &[]).expect("layout");
    assert_eq!(tidy.placed, 1);
}
