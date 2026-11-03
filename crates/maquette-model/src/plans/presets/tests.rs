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
    let changed =
        create_seeded(root, "Act 1", PlanPreset::Story, ProjectTarget::Game, None).expect("create");
    let made = changed.doc;
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
    let entry_id = changed.entry.expect("line").id;
    assert_eq!(lines[0].id, entry_id, "the answer names the line to undo");
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
    assert!(
        blank.entry.is_none(),
        "a blank canvas answers with no change id: its line undoes nothing"
    );
    let blank = blank.doc;
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
    .expect("mechanics")
    .doc;
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

#[test]
fn a_seed_whose_line_cannot_be_logged_leaves_no_canvas() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    super::super::create::FAIL_LOG.with(|f| f.set(true));
    let err = create_seeded(root, "Act 1", PlanPreset::Story, ProjectTarget::Game, None)
        .expect_err("refused");
    assert!(err.to_string().contains("log"), "{err}");
    assert!(!root.join("plans/act-1.plan.json").exists());
    assert!(!root.join("plans/act-1.log.jsonl").exists());
    let again = create_seeded(root, "Act 1", PlanPreset::Story, ProjectTarget::Game, None)
        .expect("made once the log writes");
    assert_eq!(again.doc.id, "act-1", "the id is free again");
}

#[test]
fn a_torn_first_line_is_cleaned_up_so_the_slug_stays_free() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    super::super::create::TEAR_LOG.with(|f| f.set(true));
    create_seeded(root, "Act 1", PlanPreset::Story, ProjectTarget::Game, None)
        .expect_err("refused");
    assert!(!root.join("plans/act-1.plan.json").exists());
    assert!(
        !root.join("plans/act-1.log.jsonl").exists(),
        "the half line this call wrote goes"
    );
    let again =
        create_seeded(root, "Act 1", PlanPreset::Story, ProjectTarget::Game, None).expect("made");
    assert_eq!(again.doc.id, "act-1", "not act-1-2: the slug was not spent");
}

#[test]
fn a_seeded_canvas_that_cannot_be_taken_back_logs_its_line_to_stay_undoable() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    // The line fails, then the canvas is held open (Windows) and will not go.
    super::super::create::FAIL_LOG.with(|f| f.set(true));
    super::super::create::FAIL_REMOVE.with(|f| f.set(true));
    let changed = create_seeded(root, "Act 1", PlanPreset::Story, ProjectTarget::Game, None)
        .expect("the canvas stays with its line");
    let made = changed.doc;
    assert_eq!(made.plan.nodes.len(), 8);
    let lines = log::lines(root, &made.id, 5).expect("log");
    assert_eq!(lines.len(), 1);
    let entry_id = changed.entry.expect("line").id;
    assert_eq!(lines[0].id, entry_id);
    assert!(lines[0].undoable, "the starters can be undone");
    let undone = undo(root, &made.id, &[entry_id], None, None).expect("undo");
    assert!(undone.doc.plan.nodes.is_empty() && undone.doc.plan.edges.is_empty());
}

/// Every seed as plain data, nodes in card order and links by card index:
/// what `seeds.json` holds and the mock's test compares against.
fn seeds_json() -> serde_json::Value {
    let mut all = serde_json::Map::new();
    for preset in PRESETS {
        for target in TARGETS {
            let (nodes, edges) = seed(preset, target, None);
            let at = |id: &str| nodes.iter().position(|n| n.id == id).expect("node");
            let key = format!(
                "{}/{}",
                serde_json::to_value(preset)
                    .expect("preset")
                    .as_str()
                    .expect("name"),
                serde_json::to_value(target)
                    .expect("target")
                    .as_str()
                    .expect("name")
            );
            let cards: Vec<_> = nodes
                .iter()
                .map(
                    |n| serde_json::json!({ "title": n.title, "text": n.text, "x": n.x, "y": n.y }),
                )
                .collect();
            let links: Vec<_> = edges
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "from": at(&e.from), "to": at(&e.to), "kind": e.kind,
                        "fromSide": e.from_side, "toSide": e.to_side,
                    })
                })
                .collect();
            all.insert(key, serde_json::json!({ "nodes": cards, "links": links }));
        }
    }
    serde_json::Value::Object(all)
}

/// `seeds.json` is the seeds as data for the mock backend's test, so both
/// sides check the same cards, order, places and links. Set
/// `MAQUETTE_BLESS_SEEDS=1` to write it again after changing a seed.
#[test]
fn seeds_json_matches_the_seeds() {
    let file =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/plans/presets/seeds.json");
    let now = seeds_json();
    if std::env::var_os("MAQUETTE_BLESS_SEEDS").is_some() {
        let text = serde_json::to_string_pretty(&now).expect("json") + "\n";
        std::fs::write(&file, text).expect("write");
    }
    let kept: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).expect("seeds.json")).expect("parse");
    assert_eq!(
        kept, now,
        "run with MAQUETTE_BLESS_SEEDS=1 to write seeds.json again"
    );
}
