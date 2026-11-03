use serde_json::Value;

use super::*;
use crate::plans::{NodeKind, PlanFile, PlanNode, PlanPreset, normalized, validate};

const CASES: &str = include_str!("cases.json");

fn cases() -> Value {
    serde_json::from_str(CASES).expect("cases")
}

fn card(id: &str, title: &str, text: &str) -> PlanNode {
    PlanNode {
        id: id.into(),
        kind: NodeKind::Note,
        x: 0,
        y: 0,
        width: None,
        title: title.into(),
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

/// The systems of the design's combat canvas.
fn combat() -> PlanFile {
    let mut plan = PlanFile::new("Combat systems", PlanPreset::Mechanics);
    plan.nodes = vec![
        card(
            "n-1",
            "Sword chain",
            "Verb: strike\nLook: white arcs\nInput: tap left mouse",
        ),
        card(
            "n-2",
            "Rune combos",
            "Verb: chain\nLook: blue glyph rings\nInput: Q and E",
        ),
        card(
            "n-3",
            "Sigils",
            "Verb: draw to cast\nLook: blue glyph rings\nInput: hold right mouse",
        ),
        card(
            "n-4",
            "Dash",
            "Verb: evade\nLook: snow burst\nInput: tap Shift",
        ),
        card(
            "n-5",
            "Frost veil",
            "Verb: slow time around you\nLook: pale mist dome\nInput: hold Shift",
        ),
        card(
            "n-6",
            "Two overlaps",
            "Runes and sigils read as one system.",
        ),
    ];
    plan
}

#[test]
fn main_words_follow_the_shared_cases() {
    let all = cases();
    for case in all["words"].as_array().expect("words") {
        let part: OverlapPart = serde_json::from_value(case["part"].clone()).expect("part");
        let value = case["value"].as_str().expect("value");
        let want: Vec<String> = serde_json::from_value(case["words"].clone()).expect("words");
        assert_eq!(main_words(part, value), want, "{value}");
    }
    for pair in all["singular"].as_array().expect("singular") {
        let (word, want) = (
            pair[0].as_str().expect("word"),
            pair[1].as_str().expect("want"),
        );
        assert_eq!(singular(word), want, "{word}");
    }
    for case in all["keys"].as_array().expect("keys") {
        let node = |v: &Value| {
            card(
                v["id"].as_str().expect("id"),
                v["title"].as_str().expect("title"),
                v["text"].as_str().expect("text"),
            )
        };
        let (a, b) = (node(&case["a"]), node(&case["b"]));
        assert_eq!(pair_key(&a, &b), case["key"].as_str().expect("key"));
        assert_eq!(
            pair_key(&a, &b),
            pair_key(&b, &a),
            "the key follows id order"
        );
    }
}

#[test]
fn a_mechanics_canvas_flags_cards_that_share_a_look_or_a_key() {
    let found = overlaps(&combat());
    let pairs: Vec<(&str, &str, OverlapPart, &str)> = found
        .iter()
        .map(|o| (o.a.as_str(), o.b.as_str(), o.part, o.word.as_str()))
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("n-2", "n-3", OverlapPart::Look, "ring"),
            ("n-4", "n-5", OverlapPart::Input, "shift"),
        ],
        "left and right mouse stay apart, and a plain note is no card"
    );
    assert!(found.iter().all(|o| !o.fine));
    let mut brainstorm = combat();
    brainstorm.preset = PlanPreset::Brainstorm;
    assert!(
        overlaps(&brainstorm).is_empty(),
        "only Game mechanics canvases"
    );
}

#[test]
fn the_preset_starter_cards_flag_nothing_until_written() {
    let mut plan = PlanFile::new("Systems", PlanPreset::Mechanics);
    let starter =
        "Verb: how the player gets around\nLook: how it reads on screen\nInput: the key or button";
    plan.nodes = vec![
        card("n-1", "System: Movement", starter),
        card("n-2", "System: Defence", starter),
    ];
    assert!(overlaps(&plan).is_empty());
    plan.nodes[1].text =
        "Verb: block\nLook: how it reads on screen\nInput: the key or button".into();
    assert!(overlaps(&plan).is_empty());
    plan.nodes[0].text = "Verb: block\nLook: dust\nInput: Space".into();
    assert_eq!(overlaps(&plan).len(), 1);
}

#[test]
fn a_fine_mark_holds_until_either_cards_words_change() {
    let mut plan = combat();
    let (a, b) = (plan.nodes[3].clone(), plan.nodes[4].clone());
    plan.fine = vec![FineMark {
        a: a.id.clone(),
        b: b.id.clone(),
        key: pair_key(&a, &b),
    }];
    validate(&normalized(&plan)).expect("valid");
    let fine: Vec<bool> = overlaps(&plan).iter().map(|o| o.fine).collect();
    assert_eq!(fine, vec![false, true]);
    // Moving a card is no change to its words.
    plan.nodes[4].x = 900;
    assert!(overlaps(&plan)[1].fine);
    plan.nodes[4].text = "Verb: slow time\nLook: pale dome\nInput: hold Shift".into();
    assert!(!overlaps(&plan)[1].fine, "new words bring the flag back");
}

#[test]
fn marks_go_with_their_cards_and_bad_marks_are_refused() {
    let mut plan = combat();
    plan.fine = vec![FineMark {
        a: "n-4".into(),
        b: "n-5".into(),
        key: "0a1b2c3d".into(),
    }];
    assert_eq!(normalized(&plan).fine.len(), 1);
    assert_eq!(normalized(&plan).version, crate::plans::PLAN_VERSION);
    plan.nodes.retain(|n| n.id != "n-5");
    let pruned = normalized(&plan);
    assert!(pruned.fine.is_empty(), "a mark goes with its card");
    assert_eq!(pruned.version, crate::plans::PLAN_VERSION_PLAIN);
    for bad in [
        FineMark {
            a: "n-5".into(),
            b: "n-4".into(),
            key: "0a1b2c3d".into(),
        },
        FineMark {
            a: "n-1".into(),
            b: "n-2".into(),
            key: "not hex".into(),
        },
        FineMark {
            a: "n-1".into(),
            b: "n-2".into(),
            key: String::new(),
        },
    ] {
        let mut bad_plan = combat();
        bad_plan.fine = vec![bad];
        assert!(check_fine(&bad_plan).is_err());
    }
    let mut twice = combat();
    let mark = FineMark {
        a: "n-1".into(),
        b: "n-2".into(),
        key: "00".into(),
    };
    twice.fine = vec![mark.clone(), mark];
    assert!(check_fine(&twice).is_err(), "one mark per pair");
}

#[test]
fn settings_change_as_operations_that_turn_around() {
    use crate::plans::ops::{Merge, PlanOp, apply, diff, invert};
    let before = combat();
    let mut after = before.clone();
    after.auto_layout = false;
    after.fine = vec![FineMark {
        a: "n-4".into(),
        b: "n-5".into(),
        key: "0a".into(),
    }];
    let ops = diff(&before, &after);
    assert_eq!(ops.len(), 2);
    assert!(matches!(
        ops[0],
        PlanOp::SetAutoLayout {
            before: true,
            after: false
        }
    ));
    assert!(matches!(&ops[1], PlanOp::MarkFine { mark } if mark.a == "n-4"));
    let done = apply(&before, &ops, Merge::Mine);
    assert_eq!((done.plan.auto_layout, done.plan.fine.len()), (false, 1));
    let back = apply(&done.plan, &invert(&ops), Merge::Careful);
    assert_eq!(back.plan, before);
    assert_eq!(back.skipped, 0);
    // A careful undo leaves a mark the person made again for new words.
    let mut remarked = done.plan.clone();
    remarked.fine[0].key = "0b".into();
    let held = apply(&remarked, &invert(&ops), Merge::Careful);
    assert_eq!(held.plan.fine.len(), 1);
    assert_eq!(held.skipped, 1);
}

#[test]
fn the_person_marks_an_overlap_fine_and_can_undo_it() {
    use crate::plans::ops::PlanOp;
    use crate::plans::{apply_person, create, get, log, mark_fine, undo};
    let dir = tempfile::tempdir().expect("temp dir");
    let made = create(dir.path(), "Combat systems", PlanPreset::Mechanics).expect("create");
    let ops = combat()
        .nodes
        .into_iter()
        .map(|node| PlanOp::AddNode { node })
        .collect();
    apply_person(dir.path(), &made.id, &made.plan.created, ops).expect("cards");
    let doc = get(dir.path(), &made.id).expect("get");
    assert_eq!(
        doc.overlaps.len(),
        2,
        "the view reads the overlaps with the canvas"
    );

    let refused = mark_fine(dir.path(), &made.id, "n-1", "n-4", None).expect_err("no overlap");
    assert!(refused.to_string().contains("do not overlap"), "{refused}");
    let elsewhere = mark_fine(
        dir.path(),
        &made.id,
        "n-5",
        "n-4",
        Some("1999-01-01T00:00:00Z"),
    );
    assert!(elsewhere.is_err(), "only on the canvas the person opened");
    let created = get(dir.path(), &made.id).expect("get").plan.created;
    let marked = mark_fine(dir.path(), &made.id, "n-5", "n-4", Some(&created)).expect("mark");
    let line = marked.entry.clone().expect("line");
    assert_eq!(line.summary, "Marked as fine: Dash and Frost veil");
    assert!(line.by.is_none(), "the person's change");
    assert_eq!(marked.doc.plan.version, crate::plans::PLAN_VERSION);
    let fine: Vec<bool> = marked.doc.overlaps.iter().map(|o| o.fine).collect();
    assert_eq!(fine, vec![false, true]);
    assert!(
        mark_fine(dir.path(), &made.id, "n-4", "n-5", None).is_err(),
        "marked already"
    );

    let undone = undo(dir.path(), &made.id, &[line.id], None, None).expect("undo");
    assert!(undone.doc.overlaps.iter().all(|o| !o.fine));
    assert!(undone.doc.plan.fine.is_empty());
    let lines = log::lines(dir.path(), &made.id, 10).expect("log");
    assert!(lines[0].summary.starts_with("Undid: Marked as fine"));
}

#[test]
fn the_auto_layout_switch_is_a_logged_change() {
    use crate::plans::{create, get, log, set_auto_layout};
    let dir = tempfile::tempdir().expect("temp dir");
    let made = create(dir.path(), "Ideas", PlanPreset::Brainstorm).expect("create");
    let off = set_auto_layout(dir.path(), &made.id, false, None).expect("off");
    assert_eq!(
        off.entry.as_ref().map(|e| e.summary.as_str()),
        Some("Turned auto layout after agents off")
    );
    assert!(!get(dir.path(), &made.id).expect("get").plan.auto_layout);
    let again = set_auto_layout(dir.path(), &made.id, false, None).expect("again");
    assert!(again.entry.is_none(), "nothing to write");
    set_auto_layout(dir.path(), &made.id, true, None).expect("on");
    let lines = log::lines(dir.path(), &made.id, 10).expect("log");
    assert_eq!(lines[0].summary, "Turned auto layout after agents on");
    assert_eq!(get(dir.path(), &made.id).expect("get").plan.version, 2);
}

#[test]
fn a_full_canvas_of_look_alike_cards_stays_quick() {
    // Counted, not timed, so a slow or busy machine can not fail it. The
    // check before the caps paired every card with every other on all of
    // its words: here about 600 million looks (some 10 s and 450 MB).
    let keys: Vec<String> = (0..300).map(|k| format!("k{k}")).collect();
    let text = format!("Verb: dash\nLook: rings\nInput: {}", keys.join(", "));
    let nodes = crate::plans::MAX_NODES;
    let mut plan = PlanFile::new("Systems", PlanPreset::Mechanics);
    plan.nodes = (0..nodes)
        .map(|i| card(&format!("n-{i:04}"), "System", &text))
        .collect();
    let mut work = Work::default();
    let found = find_counted(&plan, &mut work);
    assert_eq!(found.len(), MAX_OVERLAPS);
    assert_eq!(
        (found[0].a.as_str(), found[0].b.as_str()),
        ("n-0000", "n-0001")
    );
    assert_eq!(found[0].word, "dash");
    assert_eq!(
        work,
        Work {
            // Each card has 302 words and keeps the first 32.
            most_words: MAX_CARD_WORDS,
            // The first card's pairs already reach the cap: it stops there.
            cards: 1,
            // Its 32 words, each shared with every later card.
            pairs: MAX_CARD_WORDS * (nodes - 1),
        }
    );

    // Only the last two cards share a word: every card is read once, and
    // only that pair is looked at.
    for (i, node) in plan.nodes.iter_mut().enumerate() {
        node.text = format!(
            "Verb: v{i}\nLook: l{i}\nInput: {}",
            keys.join(", ").replace('k', &format!("c{i}x"))
        );
    }
    let last = plan.nodes.len() - 1;
    plan.nodes[last].text = plan.nodes[last - 1].text.clone();
    let mut work = Work::default();
    let found = find_counted(&plan, &mut work);
    assert_eq!(found.len(), 3, "verb, look and input");
    assert_eq!(
        work,
        Work {
            most_words: MAX_CARD_WORDS,
            cards: nodes,
            pairs: MAX_CARD_WORDS,
        }
    );
}

#[test]
fn a_read_again_answers_from_the_last_check_until_the_words_change() {
    let mut plan = combat();
    // Its own cache: answers other tests leave in the shared one never count.
    let answers = cache::Cache::new();
    let first = answers.answer(&plan, find);
    assert_eq!(first, find(&plan));
    assert_eq!(answers.answer(&plan, |_| panic!("checked again")), first);
    // A move, a link or a rename of the canvas reads nothing the check reads.
    plan.nodes[0].x = 640;
    plan.name = "Combat, moved".into();
    assert_eq!(answers.answer(&plan, |_| panic!("checked again")), first);
    // New words, a node more or a fine mark is a new check.
    let key = cache::key_of(&plan);
    plan.nodes[1].title = "Rune chains".into();
    assert_ne!(cache::key_of(&plan), key);
    plan.nodes[1].title = "Rune combos".into();
    plan.nodes[2].text = "Verb: draw\nLook: red sparks\nInput: hold right mouse".into();
    assert_ne!(overlaps(&plan), first);
    let (a, b) = (plan.nodes[3].clone(), plan.nodes[4].clone());
    let before = cache::key_of(&plan);
    plan.fine = vec![FineMark {
        a: a.id.clone(),
        b: b.id.clone(),
        key: pair_key(&a, &b),
    }];
    assert_ne!(cache::key_of(&plan), before);
    assert_eq!(overlaps(&plan), find(&plan));
}
