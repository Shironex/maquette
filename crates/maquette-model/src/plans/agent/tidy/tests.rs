//! "Auto layout after agents": on by default, it tidies an agent's own
//! overlapping nodes after its writes as a line of its own; off, it leaves
//! the canvas to the person and layout_canvas is refused. A person's nodes
//! never move either way.

use std::fs;

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

#[test]
fn on_by_default_it_tidies_only_the_agents_nodes_as_its_own_line() {
    let (_dir, root, id) = setup();
    assert!(get(&root, &id).expect("get").plan.auto_layout);
    let done = add(&root, &id, &RESEARCH, None, &[dropped("Lantern", 5)], &[]).expect("add");
    let line = done
        .tidied
        .as_ref()
        .and_then(|t| t.entry.clone())
        .expect("tidy line");
    assert_eq!(line.summary, "Moved 1 node with auto layout");
    assert_eq!(line.by.as_deref(), Some("Research agent"));
    assert_eq!(line.run.as_deref(), Some("r-1"));
    let plan = get(&root, &id).expect("get").plan;
    assert_eq!(spot(&plan, "n-a"), (0, 0), "a person's node never moves");
    assert_eq!(spot(&plan, "n-b"), (20, 0), "even when it overlaps another");
    assert_ne!(spot(&plan, &done.nodes[0]), (5, 10));

    // An agent's move of its own node onto another is tidied too; an edit of
    // a person's node is not, and a write already clear writes no tidy line.
    let mine = done.nodes[0].clone();
    let moved = NodeEdit {
        id: mine.clone(),
        x: Some(0),
        y: Some(0),
        ..NodeEdit::default()
    };
    let again = update(&root, &id, &RESEARCH, &[moved]).expect("move");
    assert!(again.tidied.is_some());
    let person = NodeEdit {
        id: "n-a".into(),
        x: Some(20),
        ..NodeEdit::default()
    };
    let edited = update(&root, &id, &RESEARCH, &[person]).expect("edit");
    assert!(edited.tidied.is_none());
    assert_eq!(spot(&edited.changed.doc.plan, "n-a"), (20, 0));
    let clear = add(&root, &id, &RESEARCH, None, &[dropped("Moon", 4000)], &[]).expect("add");
    assert!(clear.tidied.is_none());
}

#[test]
fn off_it_leaves_the_layout_to_the_person() {
    let (_dir, root, id) = setup();
    set_auto_layout(&root, &id, false).expect("off");
    let done = add(&root, &id, &RESEARCH, None, &[dropped("Lantern", 5)], &[]).expect("add");
    assert!(done.tidied.is_none());
    assert_eq!(spot(&done.changed.doc.plan, &done.nodes[0]), (5, 10));
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

    set_auto_layout(&root, &id, true).expect("on");
    let tidy = layout(&root, &id, &RESEARCH, &[]).expect("layout");
    assert_eq!(tidy.placed, 1);
}
