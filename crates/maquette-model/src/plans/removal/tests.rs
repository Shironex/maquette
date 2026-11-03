//! Removal proposals: an agent asks, only the person answers, and a
//! proposal for a node that changed or went is never carried out.

use std::fs;

use super::super::agent::{self, Agent, NewNode};
use super::super::ops::PlanOp;
use super::super::*;
use super::{AskedState, MAX_OPEN, MAX_OPEN_PER_AGENT, accept, dismiss, open, propose};

const RESEARCH: Agent<'static> = Agent {
    name: "Research agent",
    run: Some("r-1"),
};

fn person_note(id: &str, title: &str, x: i32) -> PlanNode {
    PlanNode {
        id: id.into(),
        kind: NodeKind::Note,
        x,
        y: 0,
        width: None,
        title: title.into(),
        text: String::new(),
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

/// A canvas with two of the person's notes joined by their link.
fn setup() -> (tempfile::TempDir, std::path::PathBuf, String) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("moon-island");
    fs::create_dir_all(&root).expect("project");
    let made = create(&root, "Hero ideas", PlanPreset::Brainstorm).expect("create");
    let mut plan = made.plan.clone();
    plan.nodes = vec![
        person_note("n-island", "Old hero idea: the whole island", 0),
        person_note("n-porch", "Open on the porch", 400),
    ];
    plan.edges = vec![PlanEdge {
        id: "e-1".into(),
        from: "n-porch".into(),
        to: "n-island".into(),
        from_side: None,
        to_side: None,
        kind: EdgeKind::Relates,
        label: None,
        by: None,
    }];
    save(&root, &made.id, &plan).expect("save");
    (dir, root, made.id)
}

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn an_agent_asks_and_the_person_removes_as_their_own_undoable_change() {
    let (_dir, root, id) = setup();
    let before = get(&root, &id).expect("get").plan;
    let asked = propose(
        &root,
        &id,
        &RESEARCH,
        &ids(&["n-island"]),
        "The porch note replaces it.",
    )
    .expect("propose");
    assert_eq!(
        asked.summary,
        "Asked to remove: Old hero idea: the whole island"
    );
    assert_eq!(
        get(&root, &id).expect("get").plan,
        before,
        "asking changes nothing"
    );
    // Asking never lets the agent remove it itself.
    let refused = agent::remove(&root, &id, &RESEARCH, &ids(&["n-island"]), &[]);
    assert!(refused.is_err());

    let waiting = open(&root, &id).expect("open");
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].by, "Research agent");
    assert_eq!(waiting[0].run.as_deref(), Some("r-1"));
    assert_eq!(waiting[0].reason, "The porch note replaces it.");
    assert!(!waiting[0].stale);
    assert_eq!(waiting[0].links, 1);

    // The links Remove takes are read again: a count the person did not see is refused.
    let changed = accept(&root, &id, &asked.id, None, Some(0)).expect_err("links changed");
    assert_eq!(changed.code(), "invalid");
    assert!(
        changed
            .to_string()
            .contains("take 1 link with the nodes, not the 0 shown"),
        "{changed}"
    );
    assert_eq!(open(&root, &id).expect("open").len(), 1, "still waiting");

    let done = accept(&root, &id, &asked.id, None, Some(waiting[0].links)).expect("accept");
    let line = done.entry.expect("line");
    assert!(line.by.is_none(), "the person's change");
    assert_eq!(line.resolves, vec![asked.id.clone()]);
    assert_eq!(
        line.summary,
        "Removed Old hero idea: the whole island, as Research agent asked"
    );
    assert!(done.doc.plan.nodes.iter().all(|n| n.id != "n-island"));
    assert!(done.doc.plan.edges.is_empty(), "its link goes with it");
    assert!(open(&root, &id).expect("open").is_empty());
    assert!(
        accept(&root, &id, &asked.id, None, None).is_err(),
        "answered already"
    );

    // The person's undo brings the node and its link back; the ask stays answered.
    let undone = undo(&root, &id, &[line.id], None, None).expect("undo");
    assert_eq!(undone.doc.plan.nodes.len(), 2);
    assert_eq!(undone.doc.plan.edges.len(), 1);
    assert!(open(&root, &id).expect("open").is_empty());
}

#[test]
fn keep_it_answers_without_changing_the_canvas() {
    let (_dir, root, id) = setup();
    let asked = propose(&root, &id, &RESEARCH, &ids(&["n-island", "n-porch"]), "").expect("ask");
    assert_eq!(asked.summary, "Asked to remove 2 nodes");
    let before = get(&root, &id).expect("get").plan;
    let kept = dismiss(&root, &id, &asked.id, None).expect("keep");
    assert_eq!(kept.summary, "Kept 2 nodes Research agent asked to remove");
    assert_eq!(get(&root, &id).expect("get").plan, before);
    assert!(open(&root, &id).expect("open").is_empty());
    assert!(
        dismiss(&root, &id, &asked.id, None).is_err(),
        "answered already"
    );
    assert!(dismiss(&root, &id, "c-nothing", None).is_err());
    // No agent undoes the ask or the answer.
    let refused = undo(
        &root,
        &id,
        std::slice::from_ref(&asked.id),
        Some("Research agent"),
        Some("Research agent"),
    );
    assert!(refused.is_err());
    let refused = undo(
        &root,
        &id,
        &[kept.id],
        Some("Research agent"),
        Some("Research agent"),
    );
    assert!(refused.is_err());
}

#[test]
fn a_node_that_changed_or_went_makes_the_proposal_stale() {
    let (_dir, root, id) = setup();
    let asked = propose(&root, &id, &RESEARCH, &ids(&["n-island"]), "").expect("ask");
    let created = get(&root, &id).expect("get").plan.created;
    // Moving it is no change to what was asked.
    let moved = PlanNode {
        x: 900,
        ..person_note("n-island", "Old hero idea: the whole island", 0)
    };
    let before = person_note("n-island", "Old hero idea: the whole island", 0);
    apply_person(
        &root,
        &id,
        &created,
        vec![PlanOp::SetNode {
            before: before.clone(),
            after: moved.clone(),
        }],
    )
    .expect("move");
    assert!(!open(&root, &id).expect("open")[0].stale);

    let edited = PlanNode {
        title: "The whole island, at the end".into(),
        ..moved.clone()
    };
    apply_person(
        &root,
        &id,
        &created,
        vec![PlanOp::SetNode {
            before: moved,
            after: edited.clone(),
        }],
    )
    .expect("edit");
    let waiting = open(&root, &id).expect("open");
    assert!(waiting[0].stale);
    assert_eq!(waiting[0].nodes[0].state, AskedState::Changed);
    assert_eq!(
        waiting[0].nodes[0].name, "Old hero idea: the whole island",
        "the name as asked"
    );
    let refused = accept(&root, &id, &asked.id, None, None).expect_err("stale");
    assert!(refused.to_string().contains("changed or went"), "{refused}");
    assert!(
        get(&root, &id)
            .expect("get")
            .plan
            .nodes
            .iter()
            .any(|n| n.id == "n-island")
    );

    apply_person(
        &root,
        &id,
        &created,
        vec![PlanOp::RemoveNode { node: edited }],
    )
    .expect("remove");
    assert_eq!(
        open(&root, &id).expect("open")[0].nodes[0].state,
        AskedState::Gone
    );
    dismiss(&root, &id, &asked.id, None).expect("keep it clears a stale proposal");
}

#[test]
fn asks_are_checked_and_capped() {
    let (_dir, root, id) = setup();
    assert!(propose(&root, &id, &RESEARCH, &[], "").is_err());
    assert!(propose(&root, &id, &RESEARCH, &ids(&["n-ghost"]), "").is_err());
    let many: Vec<String> = (0..=agent::MAX_PER_CALL)
        .map(|n| format!("n-{n}"))
        .collect();
    assert!(propose(&root, &id, &RESEARCH, &many, "").is_err());
    assert!(
        propose(
            &root,
            &id,
            &RESEARCH,
            &ids(&["n-porch"]),
            &"why ".repeat(100)
        )
        .is_err()
    );
    for _ in 0..MAX_OPEN_PER_AGENT {
        propose(&root, &id, &RESEARCH, &ids(&["n-porch"]), "").expect("ask");
    }
    let refused = propose(&root, &id, &RESEARCH, &ids(&["n-porch"]), "").expect_err("its share");
    assert!(
        refused.to_string().contains("the most one agent may"),
        "{refused}"
    );
    // Other agents still ask, up to the canvas's own cap.
    let names = ["Writer agent", "Story agent", "Level agent"];
    for n in 0..MAX_OPEN - MAX_OPEN_PER_AGENT {
        let other = Agent {
            name: names[n / MAX_OPEN_PER_AGENT],
            run: None,
        };
        propose(&root, &id, &other, &ids(&["n-porch"]), "").expect("ask");
    }
    let last = Agent {
        name: "Late agent",
        run: None,
    };
    let refused = propose(&root, &id, &last, &ids(&["n-porch"]), "").expect_err("full");
    assert!(
        refused.to_string().contains("waiting for the person"),
        "{refused}"
    );
    assert!(
        log::lines(&root, &id, 500)
            .expect("log")
            .iter()
            .all(|l| !l.undoable || l.by.is_none())
    );
}

#[test]
fn an_open_proposal_outlives_log_rotation() {
    let (_dir, root, id) = setup();
    let asked = propose(&root, &id, &RESEARCH, &ids(&["n-porch"]), "").expect("ask");
    // An agent's own notes, big enough to rotate the log several times over.
    let words = "dusk ".repeat(3_000);
    for _ in 0..200 {
        let node = NewNode {
            text: words.clone(),
            ..NewNode::default()
        };
        agent::add(&root, &id, &RESEARCH, None, &[node], &[]).expect("add");
    }
    let file = log::path(&root, &id).expect("log path");
    assert!(fs::metadata(&file).expect("log").len() <= log::LOG_MAX_BYTES + 32 * 1024);
    let waiting = open(&root, &id).expect("open");
    assert_eq!(waiting.len(), 1, "the ask stays until the person answers");
    dismiss(&root, &id, &asked.id, None).expect("keep");
}

#[test]
fn remove_shows_and_takes_the_links_drawn_since_the_ask() {
    let (_dir, root, id) = setup();
    let asked = propose(&root, &id, &RESEARCH, &ids(&["n-island"]), "").expect("ask");
    assert_eq!(open(&root, &id).expect("open")[0].links, 1);
    // The person draws another link to it after the agent asked.
    let plan = get(&root, &id).expect("get").plan;
    let cave = person_note("n-cave", "Sea cave", 800);
    let link = PlanEdge {
        id: "e-2".into(),
        from: "n-cave".into(),
        to: "n-island".into(),
        from_side: None,
        to_side: None,
        kind: EdgeKind::Relates,
        label: None,
        by: None,
    };
    apply_person(
        &root,
        &id,
        &plan.created,
        vec![
            PlanOp::AddNode { node: cave },
            PlanOp::AddEdge { edge: link },
        ],
    )
    .expect("draw");
    let waiting = open(&root, &id).expect("open");
    assert_eq!(waiting[0].links, 2, "the card shows both before Remove");
    assert!(!waiting[0].stale, "a new link is no change to the node");
    let done = accept(&root, &id, &asked.id, Some(&plan.created), None).expect("remove");
    assert!(done.doc.plan.edges.is_empty());
    assert_eq!(done.doc.plan.nodes.len(), 2);
}

#[test]
fn answers_land_only_on_the_canvas_the_person_opened() {
    let (_dir, root, id) = setup();
    let asked = propose(&root, &id, &RESEARCH, &ids(&["n-island"]), "").expect("ask");
    let other = Some("1999-01-01T00:00:00Z");
    for refused in [
        accept(&root, &id, &asked.id, other, None).map(|_| ()),
        dismiss(&root, &id, &asked.id, other).map(|_| ()),
        set_auto_layout(&root, &id, false, other).map(|_| ()),
    ] {
        let err = refused.expect_err("another canvas");
        assert!(err.to_string().contains("replaced on disk"), "{err}");
    }
    assert_eq!(open(&root, &id).expect("open").len(), 1, "nothing answered");
    assert!(get(&root, &id).expect("get").plan.auto_layout);
    let created = get(&root, &id).expect("get").plan.created;
    dismiss(&root, &id, &asked.id, Some(&created)).expect("the opened one");
}
