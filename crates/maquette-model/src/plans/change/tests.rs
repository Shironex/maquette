//! Operations, the change log, undo, and a person's and an agent's writes
//! meeting on one canvas file.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use super::super::agent::{self, Agent, NewNode};
use super::super::log::{self, LINE_MAX_BYTES, LOG_KEEP_BYTES, LOG_MAX_BYTES, PlanLogEntry};
use super::super::ops::{Merge, PlanOp, apply, diff, invert};
use super::super::summary::{self, ChangeMark};
use super::super::*;
use super::{Writer, change, undo};

fn project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("moon-island");
    fs::create_dir_all(&root).expect("project");
    (dir, root)
}

fn note(id: &str, x: i32, text: &str) -> PlanNode {
    PlanNode {
        id: id.into(),
        kind: NodeKind::Note,
        x,
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
        stop: None,
    }
}

fn edge(id: &str, from: &str, to: &str) -> PlanEdge {
    PlanEdge {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        from_side: None,
        to_side: None,
        kind: EdgeKind::Relates,
        label: None,
        by: None,
    }
}

const RESEARCH: Agent<'static> = Agent {
    name: "Research agent",
    run: Some("r-1"),
};

/// A canvas with two of a person's notes joined by a link.
fn canvas(root: &std::path::Path) -> PlanDoc {
    let made = create(root, "Hero ideas", PlanPreset::Brainstorm).expect("create");
    let mut plan = made.plan.clone();
    plan.nodes = vec![note("n-1", 0, "Porch first"), note("n-2", 400, "Moon last")];
    plan.edges = vec![edge("e-1", "n-1", "n-2")];
    save(root, &made.id, &plan).expect("save")
}

fn agent_note(text: &str) -> NewNode {
    NewNode {
        text: text.into(),
        ..NewNode::default()
    }
}

#[test]
fn a_diff_applies_and_turns_around() {
    let mut a = PlanFile::new("A", PlanPreset::Brainstorm);
    a.nodes = vec![note("n-1", 0, "one"), note("n-2", 10, "two")];
    a.edges = vec![edge("e-1", "n-1", "n-2")];
    let mut b = a.clone();
    b.nodes.remove(1);
    b.edges.clear();
    b.nodes[0].x = 50;
    b.nodes.push(note("n-3", 90, "three"));
    let ops = diff(&a, &b);
    assert_eq!(apply(&a, &ops, Merge::Mine).plan.nodes.len(), 2);
    assert_eq!(
        normalized(&apply(&a, &ops, Merge::Mine).plan),
        normalized(&b)
    );
    let back = apply(&b, &invert(&ops), Merge::Careful);
    assert_eq!(back.skipped, 0);
    assert_eq!(normalized(&back.plan), normalized(&a));
}

#[test]
fn edits_to_different_fields_of_one_node_both_stay() {
    let mut base = PlanFile::new("A", PlanPreset::Brainstorm);
    base.nodes = vec![note("n-1", 0, "one")];
    // The agent moved it; the person changed its text from the same base.
    let mut theirs = base.clone();
    theirs.nodes[0].x = 300;
    let mut mine = base.clone();
    mine.nodes[0].text = "one, edited".into();
    let merged = apply(&theirs, &diff(&base, &mine), Merge::Mine).plan;
    assert_eq!(merged.nodes[0].x, 300);
    assert_eq!(merged.nodes[0].text, "one, edited");
    // A careful undo of the move leaves a field someone changed since.
    let mut later = merged.clone();
    later.nodes[0].x = 999;
    let undone = apply(&later, &invert(&diff(&base, &theirs)), Merge::Careful);
    assert_eq!(undone.skipped, 1);
    assert_eq!(undone.plan.nodes[0].x, 999);
}

#[test]
fn summaries_name_one_node_and_count_many() {
    let one = [PlanOp::AddNode {
        node: note("n-1", 0, "Dusk light"),
    }];
    assert_eq!(summary::summary(&one), "Added note: Dusk light");
    assert_eq!(summary::mark(&one), ChangeMark::Add);
    let mut moved = note("n-2", 50, "b");
    let many = [
        PlanOp::AddNode {
            node: note("n-1", 0, "a"),
        },
        PlanOp::AddNode {
            node: note("n-3", 0, "c"),
        },
        PlanOp::SetNode {
            before: note("n-2", 0, "b"),
            after: moved.clone(),
        },
    ];
    assert_eq!(summary::summary(&many), "Added 2 notes, moved 1 note");
    assert_eq!(summary::mark(&many), ChangeMark::Mixed);
    moved.text = "changed".into();
    let edit = [PlanOp::SetNode {
        before: note("n-2", 50, "b"),
        after: moved,
    }];
    assert_eq!(summary::summary(&edit), "Edited note: changed");
}

#[test]
fn every_write_lands_in_the_log_in_order() {
    let (_dir, root) = project();
    let doc = canvas(&root);
    let made = agent::add(
        &root,
        &doc.id,
        &RESEARCH,
        Some("n-1"),
        &[agent_note("Dusk")],
        &[],
    )
    .expect("agent add");
    assert!(made.changed.entry.is_some());
    rename(&root, &doc.id, "Hero section ideas").expect("rename");
    let lines = log::lines(&root, &doc.id, 10).expect("lines");
    let marks: Vec<_> = lines.iter().map(|l| l.mark).collect();
    assert_eq!(
        marks,
        [
            ChangeMark::Rename,
            ChangeMark::Add,
            ChangeMark::Add,
            ChangeMark::Create
        ]
    );
    assert_eq!(lines[1].by.as_deref(), Some("Research agent"));
    assert_eq!(lines[1].run.as_deref(), Some("r-1"));
    assert!(lines[1].undoable);
    assert!(!lines[0].undoable, "a rename changed no node");
    // A save that changes nothing writes no line.
    save(&root, &doc.id, &get(&root, &doc.id).expect("get").plan).expect("same");
    assert_eq!(log::lines(&root, &doc.id, 10).expect("lines").len(), 4);
    // A line cut short by a sync is skipped, the rest still read.
    let file = log::path(&root, &doc.id).expect("path");
    let mut text = fs::read_to_string(&file).expect("read");
    text.push_str("{\"id\":\"c-broken\n");
    fs::write(&file, text).expect("write");
    assert_eq!(log::lines(&root, &doc.id, 10).expect("lines").len(), 4);
}

#[test]
fn the_log_keeps_its_newest_lines_under_the_cap() {
    let (_dir, root) = project();
    let doc = canvas(&root);
    let big = "x".repeat(20_000);
    for n in 0..80 {
        let entry = PlanLogEntry {
            id: format!("c-{n:03}"),
            at: String::new(),
            by: None,
            run: None,
            summary: big.clone(),
            mark: ChangeMark::Edit,
            ops: Vec::new(),
            undoes: Vec::new(),
            trimmed: false,
            applied: None,
            proposes: None,
            resolves: Vec::new(),
            keeps: Vec::new(),
        };
        log::append(&root, &doc.id, &entry).expect("append");
    }
    let size = fs::metadata(log::path(&root, &doc.id).expect("path"))
        .expect("meta")
        .len();
    assert!(size <= LOG_MAX_BYTES, "{size}");
    let entries = log::entries(&root, &doc.id).expect("entries");
    assert_eq!(entries.last().map(|e| e.id.as_str()), Some("c-079"));
    assert!(entries.len() * 20_000 <= usize::try_from(LOG_MAX_BYTES).expect("fits"));
    assert!(entries.len() * 20_000 >= usize::try_from(LOG_KEEP_BYTES / 2).expect("fits"));
    // A line too long to keep keeps its summary, not its operations.
    let huge = PlanLogEntry {
        id: "c-huge".into(),
        at: String::new(),
        by: None,
        run: None,
        summary: "Pasted a lot".into(),
        mark: ChangeMark::Add,
        ops: vec![PlanOp::AddNode {
            node: note("n-9", 0, &"y".repeat(LINE_MAX_BYTES)),
        }],
        undoes: Vec::new(),
        trimmed: false,
        applied: None,
        proposes: None,
        resolves: Vec::new(),
        keeps: Vec::new(),
    };
    log::append(&root, &doc.id, &huge).expect("append");
    let last = log::entries(&root, &doc.id)
        .expect("entries")
        .pop()
        .expect("last");
    assert!(last.trimmed && last.ops.is_empty());
}

#[test]
fn a_persons_ops_keep_an_agents_write_made_meanwhile() {
    let (_dir, root) = project();
    let doc = canvas(&root);
    let base = doc.plan.clone();
    // The person, on what they opened: a new note, a moved note, an edit.
    let mut local = base.clone();
    local.nodes.push(note("n-3", 800, "Lantern line"));
    local.nodes[0].x = 40;
    local.nodes[1].text = "Moon at the last stop".into();
    // Meanwhile an agent adds a note and edits the person's first note.
    agent::add(
        &root,
        &doc.id,
        &RESEARCH,
        Some("n-1"),
        &[agent_note("Dusk light")],
        &[],
    )
    .expect("agent");
    agent::update(
        &root,
        &doc.id,
        &RESEARCH,
        &[agent::NodeEdit {
            id: "n-1".into(),
            text: Some("Porch first, at dusk".into()),
            ..agent::NodeEdit::default()
        }],
    )
    .expect("agent edit");
    let ops = diff(&base, &local);
    let saved = change(&root, &doc.id, &Writer::PERSON, |_| Ok(ops)).expect("person");
    let plan = &saved.doc.plan;
    assert_eq!(plan.nodes.len(), 4, "the agent's note stays");
    let first = plan.nodes.iter().find(|n| n.id == "n-1").expect("n-1");
    assert_eq!((first.x, first.text.as_str()), (40, "Porch first, at dusk"));
    assert!(plan.nodes.iter().any(|n| n.text == "Moon at the last stop"));
    assert!(
        plan.nodes
            .iter()
            .any(|n| n.by.as_deref() == Some("Research agent"))
    );
}

#[test]
fn persons_and_agents_writing_at_once_lose_nothing() {
    let (_dir, root) = project();
    let doc = canvas(&root);
    let root = Arc::new(root);
    let id = Arc::new(doc.id.clone());
    let person = {
        let (root, id) = (Arc::clone(&root), Arc::clone(&id));
        std::thread::spawn(move || {
            for n in 0..25 {
                let node = note(&format!("n-p{n:02}"), n * 10, "mine");
                change(&root, &id, &Writer::PERSON, |_| {
                    Ok(vec![PlanOp::AddNode { node }])
                })
                .expect("person");
            }
        })
    };
    let agent = {
        let (root, id) = (Arc::clone(&root), Arc::clone(&id));
        std::thread::spawn(move || {
            for n in 0..25 {
                agent::add(
                    &root,
                    &id,
                    &RESEARCH,
                    None,
                    &[agent_note(&format!("a{n}"))],
                    &[],
                )
                .expect("agent");
            }
        })
    };
    person.join().expect("person thread");
    agent.join().expect("agent thread");
    let plan = get(&root, &id).expect("get").plan;
    assert_eq!(plan.nodes.len(), 52);
    assert_eq!(log::entries(&root, &id).expect("log").len(), 1 + 1 + 50);
}

#[test]
fn an_undo_takes_back_an_agents_run_but_keeps_later_edits() {
    let (_dir, root) = project();
    let doc = canvas(&root);
    let added = agent::add(
        &root,
        &doc.id,
        &RESEARCH,
        Some("n-2"),
        &[agent_note("One"), agent_note("Two")],
        &[agent::NewLink {
            from: "n-2".into(),
            to: "$0".into(),
            kind: EdgeKind::Source,
            label: None,
        }],
    )
    .expect("add");
    let entry = added.changed.entry.expect("entry").id;
    // A person edits one of the agent's notes before the undo.
    let edited = added.nodes[1].clone();
    change(&root, &doc.id, &Writer::PERSON, |plan| {
        let before = plan
            .nodes
            .iter()
            .find(|n| n.id == edited)
            .expect("two")
            .clone();
        let mut after = before.clone();
        after.text = "Two, kept".into();
        Ok(vec![PlanOp::SetNode { before, after }])
    })
    .expect("edit");
    let other = Agent {
        name: "Camera agent",
        run: None,
    };
    let refused = undo(
        &root,
        &doc.id,
        std::slice::from_ref(&entry),
        Some(other.name),
        Some(other.name),
    );
    assert!(refused.is_err(), "agents undo only their own changes");
    let done = undo(&root, &doc.id, std::slice::from_ref(&entry), None, None).expect("undo");
    assert_eq!(done.skipped, 1, "the edited note stays");
    let plan = &done.doc.plan;
    assert_eq!(plan.nodes.len(), 3);
    assert!(plan.nodes.iter().any(|n| n.text == "Two, kept"));
    assert_eq!(plan.edges.len(), 1, "the agent's link went");
    assert!(
        undo(&root, &doc.id, std::slice::from_ref(&entry), None, None).is_err(),
        "undone already"
    );
    let lines = log::lines(&root, &doc.id, 10).expect("lines");
    assert_eq!(lines[0].mark, ChangeMark::Undo);
    assert!(
        lines
            .iter()
            .any(|l| l.id == entry && l.undone && !l.undoable)
    );
}

#[test]
fn removing_a_canvas_takes_its_log_to_the_trash() {
    let (dir, root) = project();
    let doc = canvas(&root);
    let trash = dir.path().join("trash");
    let moved = remove(&root, &trash, &doc.id).expect("remove");
    assert!(!log::path(&root, &doc.id).expect("path").exists());
    let log_in_trash = PathBuf::from(moved.replace(".plan.json", ".log.jsonl"));
    assert!(log_in_trash.is_file(), "{}", log_in_trash.display());
}
