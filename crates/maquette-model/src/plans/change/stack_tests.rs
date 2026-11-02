//! An undo never releases a person's words that only it still holds: a
//! later agent change on the same field makes it wait, and a chain undone
//! newest first, or all at once, brings the words back.

use std::path::Path;

use super::super::agent::{self, NewNode, NodeEdit};
use super::super::log;
use super::super::ops::diff;
use super::keep_tests::{RESEARCH, canvas, rewrite, text_of};
use super::{Writer, change, undo};

fn held(root: &Path, id: &str) -> u64 {
    log::keeper_bytes(&log::entries(root, id).expect("entries"))
}

fn agent_edit(root: &Path, id: &str, node: &str, text: &str) -> String {
    agent::update(root, id, &RESEARCH, &[rewrite(node, text.into())])
        .expect("agent edit")
        .changed
        .entry
        .expect("logged")
        .id
}

fn person_edit(root: &Path, id: &str, node: &str, edit: impl Fn(&mut super::PlanNode)) {
    change(root, id, &Writer::PERSON, |plan| {
        let mut next = plan.clone();
        next.nodes
            .iter_mut()
            .filter(|n| n.id == node)
            .for_each(&edit);
        Ok(diff(plan, &next))
    })
    .expect("person edit");
}

/// An agent's node the person wrote "orig" in, so its protection rests on
/// who wrote it, not who made it.
fn agent_node_with_person_words() -> (tempfile::TempDir, std::path::PathBuf, String, String) {
    let (dir, root, id) = canvas(&["Porch first".to_string()]);
    let added = agent::add(
        &root,
        &id,
        &RESEARCH,
        Some("n-0"),
        &[NewNode {
            text: "Agent idea".into(),
            ..NewNode::default()
        }],
        &[],
    )
    .expect("add");
    let node = added.nodes[0].clone();
    person_edit(&root, &id, &node, |n| n.text = "orig".into());
    (dir, root, id, node)
}

fn wrote(root: &Path, id: &str, node: &str) -> bool {
    super::super::get(root, id)
        .expect("get")
        .plan
        .nodes
        .iter()
        .find(|n| n.id == node)
        .expect("node")
        .person_wrote
}

#[test]
fn an_undo_under_a_later_agent_change_waits_and_keeps_the_hold() {
    let (_dir, root, id, node) = agent_node_with_person_words();
    let first = agent_edit(&root, &id, &node, "a");
    let after_first = held(&root, &id);
    let second = agent_edit(&root, &id, &node, "b");
    assert!(wrote(&root, &id, &node), "an agent's write never clears it");
    assert!(
        held(&root, &id) > after_first,
        "the second change is held too"
    );
    let before = held(&root, &id);
    // The person's Undo of the first change alone.
    let waits = undo(&root, &id, std::slice::from_ref(&first), None, None).expect_err("waits");
    assert!(
        waits.to_string().contains("Undo the later change first"),
        "{waits}"
    );
    assert!(waits.to_string().contains(&second), "{waits}");
    // The agent trying to release the person's words by undoing its own change.
    let laundered = undo(
        &root,
        &id,
        std::slice::from_ref(&first),
        Some(RESEARCH.name),
        Some(RESEARCH.name),
    )
    .expect_err("waits for the agent too");
    assert!(
        laundered.to_string().contains("can not be undone yet"),
        "{laundered}"
    );
    assert_eq!(held(&root, &id), before, "nothing released");
    assert_eq!(text_of(&root, &id, &node), "b");
    let lines = log::lines(&root, &id, 10).expect("lines");
    assert!(!lines.iter().find(|l| l.id == first).expect("line").undone);
}

#[test]
fn a_chain_undone_newest_first_brings_the_words_back_and_frees_the_room() {
    let (_dir, root, id, node) = agent_node_with_person_words();
    let first = agent_edit(&root, &id, &node, "a");
    let second = agent_edit(&root, &id, &node, "b");
    undo(&root, &id, std::slice::from_ref(&second), None, None).expect("second");
    assert_eq!(text_of(&root, &id, &node), "a");
    undo(&root, &id, std::slice::from_ref(&first), None, None).expect("first");
    assert_eq!(text_of(&root, &id, &node), "orig");
    assert!(wrote(&root, &id, &node));
    assert_eq!(held(&root, &id), 0);
}

#[test]
fn a_chain_undone_together_brings_the_words_back() {
    let (_dir, root, id) = canvas(&["orig".to_string()]);
    let first = agent_edit(&root, &id, "n-0", "a");
    let second = agent_edit(&root, &id, "n-0", "b");
    // Named oldest first, as a run's lines are: undone newest first.
    let done = undo(
        &root,
        &id,
        &[first, second],
        Some(RESEARCH.name),
        Some(RESEARCH.name),
    )
    .expect("both");
    assert!(!done.entry.expect("line").ops.is_empty());
    assert_eq!(text_of(&root, &id, "n-0"), "orig");
    assert_eq!(held(&root, &id), 0);
}

#[test]
fn a_later_change_to_another_field_does_not_block() {
    let (_dir, root, id) = canvas(&["orig".to_string()]);
    let first = agent_edit(&root, &id, "n-0", "a");
    agent::update(
        &root,
        &id,
        &RESEARCH,
        &[NodeEdit {
            id: "n-0".into(),
            x: Some(300),
            ..NodeEdit::default()
        }],
    )
    .expect("move");
    let before = held(&root, &id);
    undo(&root, &id, std::slice::from_ref(&first), None, None).expect("not on top");
    assert_eq!(text_of(&root, &id, "n-0"), "orig");
    let after = held(&root, &id);
    assert!(after > 0 && after < before, "only the move is held now");
}

#[test]
fn a_partial_undo_restores_what_it_can_and_releases_only_what_the_person_replaced() {
    let (_dir, root, id) = canvas(&["orig".to_string()]);
    let first = agent::update(
        &root,
        &id,
        &RESEARCH,
        &[NodeEdit {
            title: Some("Agent title".into()),
            ..rewrite("n-0", "a".into())
        }],
    )
    .expect("edit")
    .changed
    .entry
    .expect("logged")
    .id;
    // The person writes their own text; the agent's title still holds the old one.
    person_edit(&root, &id, "n-0", |n| n.text = "mine".into());
    let done = undo(&root, &id, std::slice::from_ref(&first), None, None).expect("undo");
    assert!(!done.entry.expect("line").ops.is_empty());
    assert_eq!(
        text_of(&root, &id, "n-0"),
        "mine",
        "the person's text stays"
    );
    let title = super::super::get(&root, &id).expect("get").plan.nodes[0]
        .title
        .clone();
    assert_eq!(
        title, "",
        "the title held only by the undone change is back"
    );
    assert_eq!(held(&root, &id), 0);
}

#[test]
fn whether_a_person_wrote_comes_back_only_with_the_content() {
    use super::super::ops::{self, Merge, PlanOp};
    let (_dir, root, id, node) = agent_node_with_person_words();
    let plan = super::super::get(&root, &id).expect("get").plan;
    let now = plan
        .nodes
        .iter()
        .find(|n| n.id == node)
        .expect("node")
        .clone();
    let mut before = now.clone();
    before.text = "Agent idea".into();
    before.person_wrote = false;
    // An undo of the person's write, with the text since changed again.
    let mut current = plan.clone();
    if let Some(n) = current.nodes.iter_mut().find(|n| n.id == node) {
        n.text = "newer".into();
    }
    let undo = [PlanOp::SetNode {
        before: now.clone(),
        after: before.clone(),
    }];
    let held = ops::apply(&current, &undo, Merge::Careful);
    let kept = held.plan.nodes.iter().find(|n| n.id == node).expect("node");
    assert_eq!(kept.text, "newer");
    assert!(kept.person_wrote, "held content keeps the mark");
    // With the text still as the person left it, both come back.
    let done = ops::apply(&plan, &undo, Merge::Careful);
    let back = done.plan.nodes.iter().find(|n| n.id == node).expect("node");
    assert_eq!(back.text, "Agent idea");
    assert!(!back.person_wrote);
}
