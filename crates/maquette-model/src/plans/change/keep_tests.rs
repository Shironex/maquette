//! What keeps a person's earlier content undoable: agent writes the log
//! could not keep are refused, rotation keeps agents' changes to a
//! person's nodes, an undo never drops a person's link, torn lines, and a
//! person saving from a stale base while an agent writes.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::super::agent::{self, Agent, NewLink, NewNode, NodeEdit};
use super::super::log::{self, KEEP_MAX_BYTES, LOG_MAX_BYTES, PlanLogEntry};
use super::super::ops::{PlanOp, diff};
use super::super::*;
use super::{Writer, change, undo};

pub(super) const RESEARCH: Agent<'static> = Agent {
    name: "Research agent",
    run: Some("r-1"),
};

/// 20 000 characters of four byte text: the longest a note holds, at its widest.
pub(super) fn widest(seed: char) -> String {
    std::iter::once(seed)
        .chain(std::iter::repeat_n('𝄞', 19_999))
        .collect()
}

pub(super) fn note(id: &str, text: &str) -> PlanNode {
    PlanNode {
        id: id.into(),
        kind: NodeKind::Note,
        x: 0,
        y: 0,
        width: None,
        title: String::new(),
        text: text.into(),
        image: None,
        url: None,
        asset: None,
        direction: None,
        by: None,
        person_wrote: false,
        person_placed: false,
        stop: None,
    }
}

/// A canvas holding the person's notes `texts`, as n-0, n-1 and so on.
pub(super) fn canvas(texts: &[String]) -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("moon-island");
    fs::create_dir_all(&root).expect("project");
    let made = create(&root, "Notes", PlanPreset::Story).expect("create");
    let mut plan = made.plan.clone();
    plan.nodes = texts
        .iter()
        .enumerate()
        .map(|(i, t)| note(&format!("n-{i}"), t))
        .collect();
    save(&root, &made.id, &plan).expect("save");
    (dir, root, made.id)
}

pub(super) fn rewrite(id: &str, text: String) -> NodeEdit {
    NodeEdit {
        id: id.into(),
        title: None,
        text: Some(text),
        url: None,
        x: None,
        y: None,
        width: None,
        stop: None,
    }
}

pub(super) fn text_of(root: &Path, canvas: &str, node: &str) -> String {
    let plan = get(root, canvas).expect("get").plan;
    plan.nodes
        .iter()
        .find(|n| n.id == node)
        .expect("node")
        .text
        .clone()
}

#[test]
fn an_agent_rewrite_of_the_longest_multibyte_note_stays_undoable() {
    let original = widest('O');
    let (_dir, root, id) = canvas(std::slice::from_ref(&original));
    let done = agent::update(&root, &id, &RESEARCH, &[rewrite("n-0", widest('A'))])
        .expect("one edit fits");
    let entry = done.changed.entry.expect("logged");
    assert!(!entry.trimmed && !entry.ops.is_empty());
    undo(&root, &id, &[entry.id], None, None).expect("undo");
    assert_eq!(text_of(&root, &id, "n-0"), original);
}

#[test]
fn an_agent_write_too_big_to_undo_is_refused_and_changes_nothing() {
    let texts: Vec<String> = (0..3).map(|i| widest(char::from(b'a' + i))).collect();
    let (_dir, root, id) = canvas(&texts);
    let edits: Vec<NodeEdit> = (0..3)
        .map(|i| rewrite(&format!("n-{i}"), widest('Z')))
        .collect();
    let err = agent::update(&root, &id, &RESEARCH, &edits).expect_err("too big for one line");
    assert!(err.to_string().contains("smaller calls"), "{err}");
    assert_eq!(text_of(&root, &id, "n-1"), texts[1]);
    assert_eq!(
        log::entries(&root, &id).expect("log").len(),
        2,
        "made and saved only"
    );
}

#[test]
fn rotation_keeps_an_agents_change_to_a_persons_note() {
    let (_dir, root, id) = canvas(&["Porch first".to_string()]);
    let edit = agent::update(&root, &id, &RESEARCH, &[rewrite("n-0", "Rewritten".into())])
        .expect("edit")
        .changed
        .entry
        .expect("logged")
        .id;
    // The person writes well past the log's size: their own lines rotate out.
    let bulk = "x".repeat(19_000);
    for n in 0..80 {
        change(&root, &id, &Writer::PERSON, |plan| {
            let mut next = plan.clone();
            next.nodes.retain(|node| node.id != "n-bulk");
            next.nodes.push(note("n-bulk", &format!("{n}{bulk}")));
            Ok(diff(plan, &next))
        })
        .expect("person");
    }
    let path = log::path(&root, &id).expect("path");
    assert!(fs::metadata(&path).expect("log").len() < LOG_MAX_BYTES + 200_000);
    let entries = log::entries(&root, &id).expect("entries");
    assert!(entries.iter().any(|e| e.id == edit), "the keeper stays");
    undo(&root, &id, &[edit], None, None).expect("still undoable");
    assert_eq!(text_of(&root, &id, "n-0"), "Porch first");
}

#[test]
fn agents_cannot_change_a_persons_notes_past_the_keep_limit() {
    let notes = usize::try_from(KEEP_MAX_BYTES / 80_000).expect("count") + 2;
    let texts: Vec<String> = (0..notes).map(|_| "Mine".to_string()).collect();
    let (_dir, root, id) = canvas(&texts);
    let mut refused = None;
    for i in 0..notes {
        if let Err(err) = agent::update(
            &root,
            &id,
            &RESEARCH,
            &[rewrite(&format!("n-{i}"), widest('A'))],
        ) {
            refused = Some((i, err));
            break;
        }
    }
    let (at, err) = refused.expect("the keep limit refuses");
    assert!(err.to_string().contains("press Keep"), "{err}");
    assert_eq!(text_of(&root, &id, &format!("n-{at}")), "Mine");
    // Adding the agent's own nodes still works.
    agent::add(
        &root,
        &id,
        &RESEARCH,
        None,
        &[NewNode {
            text: "new".into(),
            ..NewNode::default()
        }],
        &[],
    )
    .expect("adding still works");
}

#[test]
fn undoing_an_agents_node_keeps_a_link_a_person_drew_to_it() {
    let (_dir, root, id) = canvas(&["Porch first".to_string()]);
    let added = agent::add(
        &root,
        &id,
        &RESEARCH,
        Some("n-0"),
        &[NewNode {
            text: "Dusk light".into(),
            ..NewNode::default()
        }],
        &[NewLink {
            from: "$0".into(),
            to: "n-0".into(),
            kind: EdgeKind::Relates,
            label: None,
        }],
    )
    .expect("add");
    let agent_node = added.nodes[0].clone();
    // The person links their own note to the agent's.
    change(&root, &id, &Writer::PERSON, |plan| {
        let mut next = plan.clone();
        let mut edge = next.edges[0].clone();
        edge.id = "e-person".into();
        edge.by = None;
        edge.from = "n-0".into();
        edge.to = agent_node.clone();
        next.edges.push(edge);
        Ok(diff(plan, &next))
    })
    .expect("link");
    let done = undo(
        &root,
        &id,
        &[added.changed.entry.expect("entry").id],
        None,
        None,
    )
    .expect("undo");
    let plan = &done.doc.plan;
    assert_eq!(done.skipped, 1, "the node is kept");
    assert!(plan.nodes.iter().any(|n| n.id == agent_node));
    assert!(
        plan.edges.iter().any(|e| e.id == "e-person"),
        "the person's link stays"
    );
    assert_eq!(plan.edges.len(), 1, "the agent's own link went");
}

#[test]
fn a_line_after_a_torn_one_starts_on_its_own() {
    let (_dir, root, id) = canvas(&["Porch first".to_string()]);
    let path = log::path(&root, &id).expect("path");
    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open")
        .write_all(b"{\"id\":\"c-torn\",\"at\":")
        .expect("torn write");
    agent::update(
        &root,
        &id,
        &RESEARCH,
        &[rewrite("n-0", "After the crash".into())],
    )
    .expect("edit");
    let entries: Vec<PlanLogEntry> = log::entries(&root, &id).expect("entries");
    assert_eq!(
        entries.last().expect("last").by.as_deref(),
        Some("Research agent")
    );
    assert_eq!(entries.len(), 3);
}

#[test]
fn a_person_saving_from_a_stale_base_keeps_the_agents_writes() {
    let texts: Vec<String> = (0..20).map(|i| format!("Note {i}")).collect();
    let (_dir, root, id) = canvas(&texts);
    let base = get(&root, &id).expect("get").plan;
    let root = Arc::new(root);
    let id = Arc::new(id);
    let agent = {
        let (root, id) = (Arc::clone(&root), Arc::clone(&id));
        std::thread::spawn(move || {
            for i in 0..10 {
                agent::update(
                    &root,
                    &id,
                    &RESEARCH,
                    &[rewrite(&format!("n-{i}"), format!("Agent {i}"))],
                )
                .expect("agent edit");
            }
        })
    };
    let person = {
        let (root, id, base) = (Arc::clone(&root), Arc::clone(&id), base.clone());
        std::thread::spawn(move || {
            // Every save diffs against the base the view opened with: never against the agent's file.
            for i in 10..20 {
                let mut next = base.clone();
                let at = next
                    .nodes
                    .iter()
                    .position(|n| n.id == format!("n-{i}"))
                    .expect("node");
                if i % 2 == 0 {
                    next.nodes[at].title = format!("Mine {i}");
                } else {
                    next.nodes.remove(at);
                }
                let ops: Vec<PlanOp> = diff(&base, &next);
                change(&root, &id, &Writer::PERSON, |_| Ok(ops)).expect("person save");
            }
        })
    };
    agent.join().expect("agent thread");
    person.join().expect("person thread");
    let plan = get(&root, &id).expect("get").plan;
    for i in 0..10 {
        assert_eq!(text_of(&root, &id, &format!("n-{i}")), format!("Agent {i}"));
    }
    for i in (10..20).step_by(2) {
        let node = plan
            .nodes
            .iter()
            .find(|n| n.id == format!("n-{i}"))
            .expect("kept");
        assert_eq!(node.title, format!("Mine {i}"));
    }
    assert_eq!(plan.nodes.len(), 15, "the person's five removals happened");
}
