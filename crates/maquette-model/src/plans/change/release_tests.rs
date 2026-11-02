//! How the room kept for undo is freed again: Keep, an undo that finds
//! nothing left to undo, an agent's undo near the limit, and protection
//! that stays once a person wrote in a node.

use std::path::Path;

use super::super::agent::{self, NewNode};
use super::super::log::{self, KEEP_MAX_BYTES, PlanLogEntry};
use super::super::ops::diff;
use super::super::summary::ChangeMark;
use super::keep_tests::{RESEARCH, canvas, rewrite, text_of, widest};
use super::{Writer, change, keep, undo};

/// The keepers' bytes, read whole from the log.
fn held(root: &Path, id: &str) -> u64 {
    log::keeper_bytes(&log::entries(root, id).expect("entries"))
}

/// The keepers' bytes as the in-memory index has them: what a line that
/// keeps nothing would leave held.
fn indexed(root: &Path, id: &str) -> u64 {
    let plain = PlanLogEntry {
        id: "c-probe".into(),
        at: String::new(),
        by: None,
        run: None,
        summary: String::new(),
        mark: ChangeMark::Edit,
        ops: Vec::new(),
        undoes: Vec::new(),
        keeps: Vec::new(),
        trimmed: false,
    };
    log::held_after(root, id, &plain, 0).expect("held")
}

fn agent_edit(root: &Path, id: &str, node: &str, text: String) -> String {
    agent::update(root, id, &RESEARCH, &[rewrite(node, text)])
        .expect("agent edit")
        .changed
        .entry
        .expect("logged")
        .id
}

fn person_text(root: &Path, id: &str, node: &str, text: &str) {
    change(root, id, &Writer::PERSON, |plan| {
        let mut next = plan.clone();
        if let Some(n) = next.nodes.iter_mut().find(|n| n.id == node) {
            n.text = text.into();
        }
        Ok(diff(plan, &next))
    })
    .expect("person edit");
}

#[test]
fn keep_releases_an_agents_change_and_the_budget_returns_to_zero() {
    let (_dir, root, id) = canvas(&["Porch first".to_string(), "Moon".to_string()]);
    let one = agent_edit(&root, &id, "n-0", "Agent porch".into());
    let two = agent_edit(&root, &id, "n-1", "Agent moon".into());
    assert!(held(&root, &id) > 0);
    assert_eq!(indexed(&root, &id), held(&root, &id));
    let line = keep(&root, &id, &[one.clone(), two.clone()]).expect("keep");
    assert_eq!(line.mark, ChangeMark::Keep);
    assert_eq!(line.summary, "Kept 2 agent changes");
    assert_eq!(held(&root, &id), 0);
    assert_eq!(indexed(&root, &id), 0);
    assert_eq!(
        text_of(&root, &id, "n-0"),
        "Agent porch",
        "keeping changes nothing"
    );
    let lines = log::lines(&root, &id, 10).expect("lines");
    let kept = lines.iter().find(|l| l.id == one).expect("line");
    assert!(kept.kept && !kept.keepable && kept.undoable);
    let again = keep(&root, &id, &[one]).expect_err("kept already");
    assert!(again.to_string().contains("nothing to keep"), "{again}");
}

#[test]
fn an_undo_with_nothing_left_to_undo_releases_and_says_so() {
    let (_dir, root, id) = canvas(&["Porch first".to_string()]);
    let edit = agent_edit(&root, &id, "n-0", "Agent porch".into());
    person_text(&root, &id, "n-0", "My own words");
    let done = undo(&root, &id, std::slice::from_ref(&edit), None, None).expect("undo");
    let line = done.entry.expect("an undo always writes its line");
    assert!(line.ops.is_empty());
    assert_eq!(line.undoes, vec![edit.clone()]);
    assert!(
        line.summary.starts_with("Nothing to undo, already changed"),
        "{}",
        line.summary
    );
    assert_eq!(text_of(&root, &id, "n-0"), "My own words");
    assert_eq!(held(&root, &id), 0);
    assert_eq!(indexed(&root, &id), 0);
    let lines = log::lines(&root, &id, 10).expect("lines");
    assert!(lines.iter().find(|l| l.id == edit).expect("line").undone);
}

#[test]
fn an_agent_undoes_its_own_change_near_the_limit_and_frees_the_room() {
    let notes = usize::try_from(KEEP_MAX_BYTES / 80_000).expect("count") + 2;
    let texts: Vec<String> = (0..notes).map(|_| "Mine".to_string()).collect();
    let (_dir, root, id) = canvas(&texts);
    let mut made = Vec::new();
    let mut refused_at = None;
    for i in 0..notes {
        match agent::update(
            &root,
            &id,
            &RESEARCH,
            &[rewrite(&format!("n-{i}"), widest('A'))],
        ) {
            Ok(done) => made.push(done.changed.entry.expect("entry").id),
            Err(_) => {
                refused_at = Some(i);
                break;
            }
        }
    }
    let at = refused_at.expect("the limit refuses");
    // The agent undoes its own changes, one at a time, all within the limit.
    for change_id in &made {
        undo(
            &root,
            &id,
            std::slice::from_ref(change_id),
            Some(RESEARCH.name),
            Some(RESEARCH.name),
        )
        .expect("an agent undo is never refused");
    }
    assert_eq!(held(&root, &id), 0, "undo lines are never keepers");
    assert_eq!(indexed(&root, &id), 0);
    assert_eq!(text_of(&root, &id, "n-0"), "Mine");
    agent::update(
        &root,
        &id,
        &RESEARCH,
        &[rewrite(&format!("n-{at}"), widest('B'))],
    )
    .expect("room again");
}

#[test]
fn a_person_writing_in_an_agents_node_makes_it_theirs() {
    let (_dir, root, id) = canvas(&["Porch first".to_string()]);
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
    // Moving it is not writing in it.
    change(&root, &id, &Writer::PERSON, |plan| {
        let mut next = plan.clone();
        next.nodes
            .iter_mut()
            .filter(|n| n.id == node)
            .for_each(|n| n.x += 40);
        Ok(diff(plan, &next))
    })
    .expect("move");
    let file = |root: &Path| super::super::get(root, &id).expect("get").plan;
    assert!(!file(&root).nodes.iter().any(|n| n.person_wrote));
    person_text(&root, &id, &node, "My words in it");
    assert!(
        file(&root)
            .nodes
            .iter()
            .find(|n| n.id == node)
            .expect("node")
            .person_wrote
    );
    let refused = agent::remove(&root, &id, &RESEARCH, std::slice::from_ref(&node), &[])
        .expect_err("theirs now");
    assert!(refused.to_string().contains("ask the person"), "{refused}");
    // The agent's rewrite of it keeps the person's words for undo, and the
    // node stays theirs.
    let rewrite_id = agent_edit(&root, &id, &node, "Agent again".into());
    assert!(held(&root, &id) > 0, "a keeper");
    assert!(
        file(&root)
            .nodes
            .iter()
            .find(|n| n.id == node)
            .expect("node")
            .person_wrote
    );
    undo(&root, &id, &[rewrite_id], None, None).expect("undo");
    assert_eq!(text_of(&root, &id, &node), "My words in it");
    assert!(
        file(&root)
            .nodes
            .iter()
            .find(|n| n.id == node)
            .expect("node")
            .person_wrote
    );
}
