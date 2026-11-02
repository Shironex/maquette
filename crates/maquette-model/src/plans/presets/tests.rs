use std::collections::BTreeSet;

use super::super::place::rect_of;
use super::super::summary::ChangeMark;
use super::super::{
    EdgeKind, NodeKind, PLAN_VERSION_PLAIN, PlanFile, PlanPreset, create_seeded, get, log,
    normalized, undo, validate,
};
use super::*;

const PRESETS: [PlanPreset; 3] = [
    PlanPreset::Brainstorm,
    PlanPreset::Story,
    PlanPreset::Mechanics,
];
const TARGETS: [ProjectTarget; 2] = [ProjectTarget::Game, ProjectTarget::Website];

/// Two boxes with at least `gap` pixels between them on one axis.
fn apart(a: &PlanNode, b: &PlanNode, gap: i32) -> bool {
    let (a, b) = (rect_of(a), rect_of(b));
    a.x + a.w + gap <= b.x
        || b.x + b.w + gap <= a.x
        || a.y + a.h + gap <= b.y
        || b.y + b.h + gap <= a.y
}

#[test]
fn every_seed_is_a_valid_plain_canvas_of_notes_for_both_targets() {
    for preset in PRESETS {
        for target in TARGETS {
            let mut plan = PlanFile::new("Seed", preset);
            (plan.nodes, plan.edges) = seed(preset, target, None);
            let plan = normalized(&plan);
            validate(&plan).unwrap_or_else(|e| panic!("{preset:?} {target:?}: {e}"));
            assert_eq!(
                plan.version, PLAN_VERSION_PLAIN,
                "{preset:?} stays version 2"
            );
            let ids: BTreeSet<&str> = plan.nodes.iter().map(|n| n.id.as_str()).collect();
            assert_eq!(ids.len(), plan.nodes.len(), "{preset:?} ids are unique");
            for node in &plan.nodes {
                assert_eq!(node.kind, NodeKind::Note, "presets add no node kind");
                assert!(node.by.is_none(), "a person's seed is theirs");
                for words in [&node.title, &node.text] {
                    assert!(
                        !words.contains('\u{2014}') && !words.contains('\u{2013}'),
                        "no long dashes in {words}"
                    );
                    assert!(!words.trim().is_empty(), "{preset:?} has no empty card");
                }
            }
            for edge in &plan.edges {
                assert!(ids.contains(edge.from.as_str()) && ids.contains(edge.to.as_str()));
                assert_ne!(edge.from, edge.to);
            }
            for (i, a) in plan.nodes.iter().enumerate() {
                for b in &plan.nodes[i + 1..] {
                    assert!(
                        apart(a, b, 32),
                        "{preset:?} {target:?}: {} on {}",
                        a.title,
                        b.title
                    );
                }
            }
        }
    }
}

#[test]
fn brainstorm_is_blank_and_story_and_mechanics_have_their_shape() {
    for target in TARGETS {
        assert_eq!(seed(PlanPreset::Brainstorm, target, None), (vec![], vec![]));
        let (nodes, edges) = seed(PlanPreset::Story, target, None);
        let beats: Vec<&PlanNode> = nodes
            .iter()
            .filter(|n| n.title.starts_with("Beat"))
            .collect();
        assert_eq!(beats.len(), 4);
        assert!(
            beats
                .windows(2)
                .all(|w| w[0].x < w[1].x && w[0].y == w[1].y),
            "left to right"
        );
        let row = beats[0].y;
        assert!(
            nodes
                .iter()
                .filter(|n| n.title.starts_with("Character"))
                .all(|n| n.y < row)
        );
        assert!(
            nodes
                .iter()
                .filter(|n| n.title.starts_with("Place"))
                .all(|n| n.y > row)
        );
        assert_eq!(
            edges.iter().filter(|e| e.kind == EdgeKind::LeadsTo).count(),
            3
        );
        let (systems, _) = seed(PlanPreset::Mechanics, target, None);
        let systems = systems.iter().filter(|n| n.title.starts_with("System"));
        assert!(systems.clone().count() >= 3);
        assert!(systems.into_iter().all(|n| {
            n.text.contains("Verb:") && n.text.contains("Look:") && n.text.contains("Input:")
        }));
    }
    let game = seed(PlanPreset::Story, ProjectTarget::Game, None).0;
    let site = seed(PlanPreset::Story, ProjectTarget::Website, None).0;
    assert!(game.iter().any(|n| n.text.contains("player")));
    assert!(site.iter().any(|n| n.text.contains("visitor")));
}

#[test]
fn an_agents_seed_carries_its_name() {
    let (nodes, edges) = seed(PlanPreset::Story, ProjectTarget::Game, Some("Story agent"));
    assert!(nodes.iter().all(|n| n.by.as_deref() == Some("Story agent")));
    assert!(edges.iter().all(|e| e.by.as_deref() == Some("Story agent")));
}

#[test]
fn a_seeded_canvas_is_one_undoable_first_line() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    let made =
        create_seeded(root, "Act 1", PlanPreset::Story, ProjectTarget::Game, None).expect("create");
    assert_eq!(made.plan.preset, PlanPreset::Story);
    assert_eq!(made.plan.nodes.len(), 8);
    assert_eq!(made.plan.edges.len(), 7);
    let text = std::fs::read_to_string(root.join("plans/act-1.plan.json")).expect("file");
    assert!(
        text.contains("\"version\": 2"),
        "a seeded canvas stays version 2"
    );
    assert_eq!(get(root, &made.id).expect("get").plan, made.plan);

    let lines = log::lines(root, &made.id, 10).expect("log");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].mark, ChangeMark::Create);
    assert_eq!(
        lines[0].summary,
        "Made the canvas Act 1 from the Story preset"
    );
    assert!(lines[0].by.is_none() && lines[0].undoable);
    let entry = &log::entries(root, &made.id).expect("entries")[0];
    assert_eq!(entry.ops.len(), 15, "an add per node and link");

    let undone = undo(root, &made.id, &[lines[0].id.clone()], None, None).expect("undo");
    assert!(undone.doc.plan.nodes.is_empty() && undone.doc.plan.edges.is_empty());
}

#[test]
fn a_blank_canvas_logs_as_before_and_an_agent_names_itself() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    let blank = create_seeded(
        root,
        "Ideas",
        PlanPreset::Brainstorm,
        ProjectTarget::Website,
        None,
    )
    .expect("blank");
    let line = &log::lines(root, &blank.id, 1).expect("log")[0];
    assert_eq!(line.summary, "Made the canvas Ideas");
    assert!(!line.undoable, "nothing to undo on a blank canvas");

    let made = create_seeded(
        root,
        "Combat",
        PlanPreset::Mechanics,
        ProjectTarget::Game,
        Some("Design agent"),
    )
    .expect("mechanics");
    let line = &log::lines(root, &made.id, 1).expect("log")[0];
    assert_eq!(line.by.as_deref(), Some("Design agent"));
    assert_eq!(
        line.summary,
        "Made the canvas Combat from the Game mechanics preset"
    );
    assert!(
        made.plan
            .nodes
            .iter()
            .all(|n| n.by.as_deref() == Some("Design agent"))
    );
}
