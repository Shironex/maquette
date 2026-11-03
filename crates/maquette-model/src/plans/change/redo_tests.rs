//! An undo can not be undone, Keep names the earlier changes it sits on,
//! an agent can not undo a kept change,
//! a kind change round-trips and the index counts a line as a read does.

use std::io::Write as _;
use std::path::Path;

use super::super::log::{self, PlanLogEntry};
use super::super::ops::{self, Merge, PlanOp};
use super::super::summary::ChangeMark;
use super::super::{NodeKind, get};
use super::keep_tests::{RESEARCH, canvas, rewrite, text_of};
use super::{keep, keep_needs, undo};

fn held(root: &Path, id: &str) -> u64 {
    log::keeper_bytes(&log::entries(root, id).expect("entries"))
}

fn agent_edit(root: &Path, id: &str, node: &str, text: &str) -> String {
    super::super::agent::update(root, id, &RESEARCH, &[rewrite(node, text.into())])
        .expect("agent edit")
        .changed
        .entry
        .expect("logged")
        .id
}

fn agent_undo(root: &Path, id: &str, change: &str) -> Result<String, String> {
    undo(
        root,
        id,
        &[change.to_string()],
        Some(RESEARCH.name),
        Some(RESEARCH.name),
    )
    .map(|done| done.entry.expect("line").id)
    .map_err(|e| e.to_string())
}

fn undoable(root: &Path, id: &str, change: &str) -> bool {
    log::lines(root, id, 50)
        .expect("lines")
        .into_iter()
        .find(|l| l.id == change)
        .expect("line")
        .undoable
}

#[test]
fn undoing_an_undo_is_refused_so_the_words_stay_held() {
    let (_dir, root, id) = canvas(&["orig".to_string()]);
    let first = agent_edit(&root, &id, "n-0", "a");
    let back = agent_undo(&root, &id, &first).expect("undo");
    assert_eq!(text_of(&root, &id, "n-0"), "orig");
    assert!(
        !undoable(&root, &id, &back),
        "the panel offers no Undo on it"
    );
    // The agent's redo of its own undo, and the person's.
    let by_agent = agent_undo(&root, &id, &back).expect_err("refused");
    assert!(by_agent.contains("an undo can not be undone"), "{by_agent}");
    let by_person = undo(&root, &id, std::slice::from_ref(&back), None, None)
        .expect_err("refused")
        .to_string();
    assert!(
        by_person.contains("an undo can not be undone"),
        "{by_person}"
    );
    assert_eq!(text_of(&root, &id, "n-0"), "orig");
}

#[test]
fn a_redo_can_not_open_the_way_to_release_an_older_change() {
    let (_dir, root, id) = canvas(&["orig".to_string()]);
    let first = agent_edit(&root, &id, "n-0", "a");
    let second = agent_edit(&root, &id, "n-0", "b");
    let back = agent_undo(&root, &id, &second).expect("undo the newer");
    let before = held(&root, &id);
    let redo = agent_undo(&root, &id, &back).expect_err("no redo");
    assert!(redo.contains("can not be undone"), "{redo}");
    assert_eq!(held(&root, &id), before, "nothing released");
    assert_eq!(text_of(&root, &id, "n-0"), "a");
    // With the newer one undone, the older one brings the words back.
    agent_undo(&root, &id, &first).expect("undo the older");
    assert_eq!(text_of(&root, &id, "n-0"), "orig");
    assert_eq!(held(&root, &id), 0);
}

#[test]
fn a_keep_names_the_older_changes_it_sits_on_or_is_refused() {
    let (_dir, root, id) = canvas(&["orig".to_string(), "other".to_string()]);
    let first = agent_edit(&root, &id, "n-0", "a");
    let second = agent_edit(&root, &id, "n-0", "b");
    let elsewhere = agent_edit(&root, &id, "n-1", "c");
    let needs = keep_needs(&root, &id, std::slice::from_ref(&second)).expect("needs");
    assert_eq!(needs.len(), 1);
    assert_eq!(needs[0].id, first);
    assert_eq!(needs[0].parts, vec!["text".to_string()]);
    // A short list keeps nothing.
    let before = held(&root, &id);
    let short = keep(&root, &id, std::slice::from_ref(&second)).expect_err("short");
    assert!(short.to_string().contains(&first), "{short}");
    assert_eq!(held(&root, &id), before);
    let kept = keep(&root, &id, &[second.clone(), first.clone()]).expect("keep both");
    assert_eq!(kept.keeps, vec![second, first.clone()]);
    // Only the other node's change still holds room.
    let lines = log::lines(&root, &id, 50).expect("lines");
    let keepable: Vec<&str> = lines
        .iter()
        .filter(|l| l.keepable)
        .map(|l| l.id.as_str())
        .collect();
    assert_eq!(keepable, vec![elsewhere.as_str()]);
    assert!(
        keep_needs(&root, &id, std::slice::from_ref(&elsewhere))
            .expect("needs")
            .is_empty()
    );
}

#[test]
fn the_chain_a_keep_needs_crosses_nodes_through_a_change_to_both() {
    let (_dir, root, id) = canvas(&["origM".to_string(), "origN".to_string()]);
    let a0 = agent_edit(&root, &id, "n-0", "m0");
    let a1 = super::super::agent::update(
        &root,
        &id,
        &RESEARCH,
        &[rewrite("n-1", "n1".into()), rewrite("n-0", "m1".into())],
    )
    .expect("both")
    .changed
    .entry
    .expect("logged")
    .id;
    let a2 = agent_edit(&root, &id, "n-1", "n2");
    let needs = keep_needs(&root, &id, std::slice::from_ref(&a2)).expect("needs");
    let ids: Vec<&str> = needs.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, vec![a1.as_str(), a0.as_str()]);
    let refused = keep(&root, &id, &[a2.clone(), a1.clone()]).expect_err("A0 not named");
    assert!(refused.to_string().contains(&a0), "{refused}");
}

#[test]
fn an_agent_can_not_undo_a_change_the_person_kept() {
    let (_dir, root, id) = canvas(&["orig".to_string()]);
    let first = agent_edit(&root, &id, "n-0", "a");
    keep(&root, &id, std::slice::from_ref(&first)).expect("keep");
    let refused = agent_undo(&root, &id, &first).expect_err("kept");
    assert!(refused.contains("The person kept the change"), "{refused}");
    assert_eq!(text_of(&root, &id, "n-0"), "a");
    undo(&root, &id, std::slice::from_ref(&first), None, None).expect("the person can");
    assert_eq!(text_of(&root, &id, "n-0"), "orig");
}

#[test]
fn a_kind_change_round_trips_through_undo() {
    let (_dir, root, id) = canvas(&["orig".to_string()]);
    let plan = get(&root, &id).expect("get").plan;
    let before = plan.nodes[0].clone();
    let mut after = before.clone();
    after.kind = NodeKind::Link;
    after.url = Some("https://example.com".into());
    let set = [PlanOp::SetNode {
        before: before.clone(),
        after: after.clone(),
    }];
    let changed = ops::apply(&plan, &set, Merge::Mine).plan;
    assert_eq!(changed.nodes[0].kind, NodeKind::Link);
    let back = ops::apply(&changed, &ops::invert(&set), Merge::Careful);
    assert_eq!(back.skipped, 0);
    assert_eq!(back.plan.nodes[0].kind, NodeKind::Note);
}

#[test]
fn a_line_after_a_torn_one_counts_as_a_read_of_the_log_counts_it() {
    let (_dir, root, id) = canvas(&["orig".to_string()]);
    agent_edit(&root, &id, "n-0", "a");
    let file = log::path(&root, &id).expect("path");
    let mut handle = std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .expect("open");
    handle.write_all(b"{\"torn").expect("torn");
    drop(handle);
    let probe = PlanLogEntry {
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
        applied: None,
        proposes: None,
        resolves: Vec::new(),
    };
    // The index reads the torn file, then takes in a line that closes it.
    log::held_after(&root, &id, &probe, 0).expect("index");
    agent_edit(&root, &id, "n-0", "b");
    assert_eq!(
        log::held_after(&root, &id, &probe, 0).expect("held"),
        held(&root, &id)
    );
}

#[test]
fn a_confirmed_keep_takes_exactly_the_list_the_person_saw() {
    let (_dir, root, id) = canvas(&["orig".to_string(), "other".to_string()]);
    let first = agent_edit(&root, &id, "n-0", "a");
    let second = agent_edit(&root, &id, "n-0", "b");
    let elsewhere = agent_edit(&root, &id, "n-1", "c");
    let before = held(&root, &id);
    // An extra id the list never named is refused, and so is a short list.
    let extra = super::keep_confirmed(
        &root,
        &id,
        std::slice::from_ref(&second),
        &[first.clone(), elsewhere.clone()],
    )
    .expect_err("extra");
    assert!(extra.to_string().contains("Review again"), "{extra}");
    let short =
        super::keep_confirmed(&root, &id, std::slice::from_ref(&second), &[]).expect_err("short");
    assert!(short.to_string().contains("The list changed"), "{short}");
    assert_eq!(held(&root, &id), before);
    // The confirmed need released meanwhile (kept in another window): the list
    // changed, which says to review again rather than "nothing to keep".
    keep(&root, &id, std::slice::from_ref(&first)).expect("kept elsewhere");
    let stale = super::keep_confirmed(&root, &id, std::slice::from_ref(&second), &[first])
        .expect_err("stale list");
    assert!(stale.to_string().contains("The list changed"), "{stale}");
    let kept =
        super::keep_confirmed(&root, &id, std::slice::from_ref(&second), &[]).expect("as listed");
    assert_eq!(kept.keeps, vec![second]);
}
