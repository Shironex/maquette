//! Starting a canvas from a node: the node and its linked nodes, one undo,
//! the source untouched, and no camera stops in a plain canvas.

use std::fs;

use super::super::agent::{self, Agent};
use super::super::*;
use super::start_from;
use crate::layout::{self, LayoutStop};

const RESEARCH: Agent<'static> = Agent {
    name: "Research agent",
    run: None,
};

fn node(id: &str, kind: NodeKind, x: i32, y: i32, by: Option<&str>) -> PlanNode {
    PlanNode {
        id: id.into(),
        kind,
        x,
        y,
        width: None,
        title: if kind == NodeKind::Asset {
            String::new()
        } else {
            format!("Title {id}")
        },
        text: String::new(),
        image: None,
        url: None,
        asset: (kind == NodeKind::Asset).then(|| "frost-warden".to_string()),
        direction: None,
        by: by.map(str::to_string),
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
        kind: EdgeKind::LeadsTo,
        label: Some("then".into()),
        by: None,
    }
}

/// A mechanics canvas: a warden asset linked to a person's note and an
/// agent's note, and a note on its own.
fn setup() -> (tempfile::TempDir, std::path::PathBuf, PlanDoc) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("frostveil");
    fs::create_dir_all(&root).expect("project");
    let made = create(&root, "Combat systems", PlanPreset::Mechanics).expect("create");
    let mut plan = made.plan.clone();
    plan.nodes = vec![
        node("n-warden", NodeKind::Asset, 500, 300, None),
        node("n-veil", NodeKind::Note, 800, 300, None),
        node("n-agent", NodeKind::Note, 500, 600, Some("Research agent")),
        node("n-alone", NodeKind::Note, 0, 0, None),
    ];
    plan.edges = vec![
        edge("e-1", "n-warden", "n-veil"),
        edge("e-2", "n-agent", "n-warden"),
        edge("e-3", "n-veil", "n-agent"),
    ];
    let saved = save(&root, &made.id, &plan).expect("save");
    (dir, root, saved)
}

#[test]
fn a_canvas_starts_from_a_node_and_its_links_as_one_undo() {
    let (_dir, root, source) = setup();
    let started = start_from(&root, &source.id, "n-warden", None, None).expect("start");
    assert!(started.left_out.is_empty());
    let doc = &started.changed.doc;
    assert_eq!(
        doc.plan.name, "frost-warden",
        "an asset node's name is its asset id"
    );
    assert_eq!(doc.plan.preset, PlanPreset::Mechanics);
    assert_eq!(doc.plan.nodes.len(), 3, "the node on its own stays behind");
    assert_eq!(doc.plan.edges.len(), 3, "the links among them come along");
    let seed = doc
        .plan
        .nodes
        .iter()
        .find(|n| n.kind == NodeKind::Asset)
        .expect("seed");
    assert_eq!((seed.x, seed.y), (0, 0));
    assert_eq!(
        seed.asset.as_deref(),
        Some("frost-warden"),
        "still a link by id"
    );
    assert!(
        doc.plan.nodes.iter().all(|n| !n.id.starts_with("n-warden")),
        "fresh ids"
    );
    let veil = doc
        .plan
        .nodes
        .iter()
        .find(|n| n.title == "Title n-veil")
        .expect("veil");
    assert_eq!((veil.x, veil.y), (300, 0));
    assert!(
        doc.plan
            .edges
            .iter()
            .all(|e| e.label.as_deref() == Some("then"))
    );

    let line = started.changed.entry.clone().expect("first line");
    assert!(line.by.is_none());
    assert_eq!(line.summary, "Started from frost-warden on Combat systems");
    assert_eq!(line.ops.len(), 6);
    assert_eq!(
        get(&root, &source.id).expect("source").plan,
        source.plan,
        "source untouched"
    );

    let undone = undo(&root, &doc.id, &[line.id], None, None).expect("undo");
    assert!(undone.doc.plan.nodes.is_empty());
}

#[test]
fn an_agent_starts_one_and_copies_keep_who_made_them() {
    let (_dir, root, source) = setup();
    let started = start_from(
        &root,
        &source.id,
        "n-veil",
        Some("  Veil ideas  "),
        Some("Research agent"),
    )
    .expect("start");
    let doc = &started.changed.doc;
    assert_eq!(doc.plan.name, "Veil ideas");
    assert_eq!(doc.id, "veil-ideas");
    let line = started.changed.entry.clone().expect("line");
    assert_eq!(line.by.as_deref(), Some("Research agent"));
    let veil = doc
        .plan
        .nodes
        .iter()
        .find(|n| n.title == "Title n-veil")
        .expect("veil");
    assert!(veil.by.is_none(), "the person's note stays theirs");
    let refused = agent::remove(
        &root,
        &doc.id,
        &RESEARCH,
        std::slice::from_ref(&veil.id),
        &[],
    );
    assert!(
        refused.is_err(),
        "an agent still never removes a person's note"
    );
    // The agent can undo its start as one change.
    undo(
        &root,
        &doc.id,
        &[line.id],
        Some("Research agent"),
        Some("Research agent"),
    )
    .expect("undo");
    assert!(start_from(&root, &source.id, "n-ghost", None, None).is_err());
}

#[test]
fn camera_stops_never_start_or_join_a_plain_canvas() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("moon-island");
    fs::create_dir_all(&root).expect("project");
    let stop = |id: &str, x: f32| LayoutStop {
        id: id.into(),
        name: id.into(),
        position: [x, 1.0, 3.0],
        target: [0.0, 0.5, 0.0],
        fov: 35.0,
        models: None,
    };
    layout::update(&root, |l| {
        let mut next = l.clone();
        next.stops = vec![stop("porch", 0.0), stop("moon", 1.0)];
        Ok(next)
    })
    .expect("layout");
    let camera = camera::ensure(&root, None).expect("camera plan");
    let first = camera.plan.nodes[0].id.clone();
    let refused = start_from(&root, camera::CAMERA_PLAN_ID, &first, None, None).expect_err("stop");
    assert!(refused.to_string().contains("camera stop"), "{refused}");

    let mut plan = camera.plan.clone();
    let mut note = node("n-note", NodeKind::Note, 0, 400, None);
    note.title = "Lantern shot".into();
    plan.nodes.push(note);
    plan.edges.push(edge("e-note", "n-note", &first));
    save(&root, camera::CAMERA_PLAN_ID, &plan).expect("save");
    let started = start_from(&root, camera::CAMERA_PLAN_ID, "n-note", None, None).expect("start");
    assert_eq!(started.left_out, vec![first]);
    let doc = &started.changed.doc;
    assert_eq!(doc.plan.nodes.len(), 1);
    assert!(doc.plan.edges.is_empty());
    assert_eq!(doc.plan.preset, PlanPreset::Brainstorm);
    assert_eq!(doc.plan.version, PLAN_VERSION_PLAIN);
    assert_ne!(doc.id, camera::CAMERA_PLAN_ID);
    // A canvas named like the camera plan never takes its id.
    let named = start_from(
        &root,
        camera::CAMERA_PLAN_ID,
        "n-note",
        Some("Camera plan"),
        None,
    )
    .expect("start");
    assert_eq!(named.changed.doc.id, "camera-plan-2");
}
