use std::fs;

use super::*;
use crate::ledger::{self, LedgerLine, LineEvent, SpendOutcome};

const NOW: f64 = 1_800_000_000_000.0;

fn sent(job: &str, at_ms: f64, task: &str) -> LedgerLine {
    let mut line = LedgerLine::new(LineEvent::Sent, at_ms, SpendOutcome::Pending);
    line.project = "moon-island".into();
    line.job = job.into();
    line.asset = "lantern".into();
    line.asset_name = "Stone lantern".into();
    line.task = Some(task.into());
    line.summary = "Tripo H3.1, 1 image".into();
    line.estimate = Some(30.0);
    line.via_agent = true;
    line
}

fn approved(job: &str, at_ms: f64, source: DecisionSource) -> DecisionRecord {
    let by = DecidedBy {
        input: Some(DecisionInput::Pointer),
        trusted: Some(true),
        window_focused: Some(false),
        window_visible: Some(true),
        ..DecidedBy::from(source)
    };
    let mut record = DecisionRecord::new("approval-3", "send-to-3d", Decision::Approved, by, at_ms);
    record.project = "moon-island".into();
    record.jobs = vec![job.into()];
    record.provider = Some("Tripo".into());
    record.asset = Some("Stone lantern".into());
    record.estimate = Some(30.0);
    record.requested_at_ms = at_ms - 22_000.0;
    record
}

#[test]
fn decisions_append_and_read_back_skipping_what_cannot_be_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(read(dir.path()).expect("none yet").is_empty());
    let first = approved("lantern-1", NOW, DecisionSource::FloatingCard);
    append(dir.path(), &first).expect("append");
    let mut file = fs::read_to_string(path(dir.path())).expect("file");
    file.push_str("{ not json\n");
    file.push_str(
        &serde_json::to_string(&DecisionRecord {
            v: 99,
            ..first.clone()
        })
        .expect("newer"),
    );
    file.push('\n');
    fs::write(path(dir.path()), file).expect("write");
    assert_eq!(read(dir.path()).expect("read"), vec![first]);
}

#[test]
fn a_line_keeps_where_how_and_the_window_of_the_answer() {
    let line = serde_json::to_value(approved("lantern-1", NOW, DecisionSource::AgentsPanel))
        .expect("json");
    assert_eq!(line["decision"], "approved");
    assert_eq!(line["by"]["source"], "agents-panel");
    assert_eq!(line["by"]["input"], "pointer");
    assert_eq!(line["by"]["trusted"], true);
    assert_eq!(line["by"]["windowFocused"], false);
    assert_eq!(line["jobs"][0], "lantern-1");
    assert_eq!(line["requestedAtMs"], NOW - 22_000.0);
}

#[test]
fn an_old_ledger_without_decisions_reads_as_before() {
    let dir = tempfile::tempdir().expect("tempdir");
    // A line as v0.5.0 wrote it, before sources and decisions were kept.
    fs::write(
        ledger::path(dir.path()),
        "{\"v\":1,\"atMs\":1800000000000.0,\"event\":\"sent\",\"project\":\"moon-island\",\
         \"job\":\"lantern-1\",\"asset\":\"lantern\",\"assetName\":\"Stone lantern\",\
         \"service\":\"tripo\",\"task\":\"t-1\",\"summary\":\"Tripo H3.1, 1 image\",\
         \"estimate\":30.0,\"credits\":null,\"outcome\":\"pending\",\"redoOf\":null,\
         \"viaAgent\":true}\n",
    )
    .expect("old ledger");
    let read = ledger::read(dir.path()).expect("read");
    assert_eq!(read.skipped, 0);
    assert_eq!(read.spends.len(), 1);
    assert_eq!(read.spends[0].decision, None);
    assert_eq!(read.spends[0].credits, 30.0);
}

#[test]
fn each_paid_send_carries_the_decision_that_let_it_go() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    // lantern-1 was sent twice: the job id came back after a clear.
    append(
        root,
        &approved("lantern-1", NOW - 100_000.0, DecisionSource::AgentsPanel),
    )
    .expect("first yes");
    ledger::append(root, &sent("lantern-1", NOW - 90_000.0, "t-1")).expect("first send");
    append(
        root,
        &approved("lantern-1", NOW - 10_000.0, DecisionSource::FloatingCard),
    )
    .expect("second yes");
    ledger::append(root, &sent("lantern-1", NOW, "t-2")).expect("second send");
    // A send made in the app has no decision.
    let mut own = sent("cup-1", NOW - 50_000.0, "t-3");
    own.via_agent = false;
    ledger::append(root, &own).expect("own send");

    let spends = ledger::read(root).expect("read").spends;
    let by_task = |task: &str| {
        spends
            .iter()
            .find(|s| s.task.as_deref() == Some(task))
            .expect("spend")
    };
    let first = by_task("t-1").decision.as_ref().expect("first decision");
    assert_eq!(first.by.source, DecisionSource::AgentsPanel);
    let second = by_task("t-2").decision.as_ref().expect("second decision");
    assert_eq!(second.by.source, DecisionSource::FloatingCard);
    assert_eq!(second.at_ms, NOW - 10_000.0);
    assert_eq!(by_task("t-3").decision, None);
}

#[test]
fn a_batch_decision_covers_each_of_its_jobs() {
    let mut batch = approved("lantern-1", NOW, DecisionSource::FloatingCard);
    batch.jobs = vec!["lantern-1".into(), "cup-1".into()];
    let all = vec![batch];
    assert!(for_job(&all, "moon-island", "cup-1", NOW + 1.0).is_some());
    assert!(for_job(&all, "other", "cup-1", NOW + 1.0).is_none());
    assert!(
        for_job(&all, "moon-island", "cup-1", NOW - 1.0).is_none(),
        "made later"
    );
}

#[test]
fn only_the_app_cards_count_as_card_sources() {
    assert!(DecisionSource::FloatingCard.is_card());
    assert!(DecisionSource::AgentsPanel.is_card());
    assert!(DecisionSource::ShipView.is_card());
    for other in [
        DecisionSource::Avatar,
        DecisionSource::RigDoctor,
        DecisionSource::JobsCancelled,
        DecisionSource::RunCancelled,
        DecisionSource::QuestionChanged,
        DecisionSource::Quit,
        DecisionSource::Unknown,
    ] {
        assert!(!other.is_card(), "{other:?}");
    }
}
