use std::fs;

use super::*;

const NOW: f64 = 1_800_000_000_000.0;

fn line(job: &str, event: LineEvent, at_ms: f64, outcome: SpendOutcome) -> LedgerLine {
    let mut line = LedgerLine::new(event, at_ms, outcome);
    line.project = "moon-island".into();
    line.job = job.into();
    line.asset = job.rsplit_once('-').map_or(job, |(a, _)| a).into();
    line.asset_name = "Stone lantern".into();
    line.task = Some(format!("task-{job}"));
    line.summary = "Tripo H3.1, 1 image".into();
    line.estimate = Some(30.0);
    line
}

#[test]
fn appended_lines_read_back_as_one_spend_per_send() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(read(dir.path()).expect("empty"), Ledger::default());
    append(
        dir.path(),
        &line(
            "lantern-1",
            LineEvent::Sent,
            NOW - 60_000.0,
            SpendOutcome::Pending,
        ),
    )
    .expect("sent");
    let mut settled = line(
        "lantern-1",
        LineEvent::Settled,
        NOW,
        SpendOutcome::Succeeded,
    );
    settled.credits = Some(40.0);
    append(dir.path(), &settled).expect("settled");
    let ledger = read(dir.path()).expect("read");
    assert_eq!(ledger.skipped, 0);
    assert_eq!(ledger.spends.len(), 1);
    let spend = &ledger.spends[0];
    assert_eq!(spend.outcome, SpendOutcome::Succeeded);
    assert_eq!(spend.credits, 40.0);
    assert!(!spend.estimated);
    assert_eq!(spend.sent_at_ms, NOW - 60_000.0);
    assert_eq!(spend.settled_at_ms, Some(NOW));
    assert_eq!(spend.task.as_deref(), Some("task-lantern-1"));
    // The file only grew: one line per event.
    let text = fs::read_to_string(path(dir.path())).expect("file");
    assert_eq!(text.lines().count(), 2);
    assert!(text.lines().all(|l| l.contains("\"v\":1")));
}

#[test]
fn the_price_list_stands_in_until_the_provider_reports() {
    let pending = parse(
        &serde_json::to_string(&line(
            "lantern-1",
            LineEvent::Sent,
            NOW,
            SpendOutcome::Pending,
        ))
        .expect("json"),
    );
    assert_eq!(pending.spends[0].credits, 30.0);
    assert!(pending.spends[0].estimated);

    assert_eq!(
        charged(SpendOutcome::Succeeded, None, Some(30.0)),
        (30.0, true)
    );
    assert_eq!(
        charged(SpendOutcome::Succeeded, Some(25.0), Some(30.0)),
        (25.0, false)
    );
    // Neither service charges a failed task; a refund is free.
    assert_eq!(
        charged(SpendOutcome::Failed, None, Some(30.0)),
        (0.0, false)
    );
    assert_eq!(
        charged(SpendOutcome::Failed, Some(30.0), Some(75.0)),
        (30.0, false)
    );
    assert_eq!(
        charged(SpendOutcome::Refunded, Some(20.0), Some(30.0)),
        (0.0, false)
    );
    assert_eq!(charged(SpendOutcome::Pending, None, None), (0.0, false));
}

#[test]
fn broken_and_newer_lines_are_skipped_not_fatal() {
    let good = serde_json::to_string(&line(
        "lantern-1",
        LineEvent::Sent,
        NOW,
        SpendOutcome::Pending,
    ))
    .expect("json");
    let mut newer = line("lantern-2", LineEvent::Sent, NOW, SpendOutcome::Pending);
    newer.v = 99;
    let newer = serde_json::to_string(&newer).expect("json");
    let text = format!("{good}\n{{\"v\":1,\"atMs\":\n\nnot json at all\n{newer}\n{good}");
    let ledger = parse(&text);
    assert_eq!(ledger.skipped, 3);
    assert_eq!(ledger.spends.len(), 1);
    assert_eq!(ledger.spends[0].job, "lantern-1");
}

#[test]
fn spends_list_newest_first_and_keep_projects_apart() {
    let mut other = line("lantern-1", LineEvent::Sent, NOW, SpendOutcome::Pending);
    other.project = "rooftop".into();
    let lines = vec![
        line(
            "lantern-1",
            LineEvent::Sent,
            NOW - 5.0,
            SpendOutcome::Pending,
        ),
        other,
    ];
    let spends = fold(&lines);
    assert_eq!(spends.len(), 2);
    assert_eq!(spends[0].project, "rooftop");
}

#[test]
fn summaries_count_the_week_the_month_and_the_redo_share() {
    let day = DAY_MS;
    let mut spends = Vec::new();
    let mut add = |job: &str, service, ago: f64, credits: f64, redo: bool| {
        let mut l = line(job, LineEvent::Settled, NOW - ago, SpendOutcome::Succeeded);
        l.service = service;
        l.credits = Some(credits);
        l.redo_of = redo.then(|| "lantern-1".to_string());
        spends.push(l);
    };
    add("lantern-1", ProviderId::Tripo, 2.0 * day, 30.0, false);
    add("lantern-2", ProviderId::Tripo, 1.0 * day, 30.0, true);
    add("spire-1", ProviderId::Meshy, 20.0 * day, 20.0, false);
    add("spire-2", ProviderId::Meshy, 90.0 * day, 35.0, false);
    let totals = summarise(&fold(&spends), NOW);
    assert_eq!(
        totals.len(),
        6,
        "three API rows, then three subscription rows"
    );
    let tripo = &totals[0];
    assert_eq!(tripo.service, Some(ProviderId::Tripo));
    assert_eq!((tripo.week, tripo.month, tripo.total), (60.0, 60.0, 60.0));
    assert_eq!(tripo.redos, 1);
    assert_eq!(tripo.redo_share, 0.5);
    let meshy = &totals[1];
    assert_eq!((meshy.week, meshy.month, meshy.total), (0.0, 20.0, 55.0));
    let all = &totals[2];
    assert_eq!(all.service, None);
    assert_eq!(all.sends, 4);
    assert_eq!(all.total, 115.0);
    assert!((all.redo_share - 30.0 / 115.0).abs() < 1e-9);
    assert_eq!(summarise(&[], NOW)[2].redo_share, 0.0);
    assert!(
        totals[3..]
            .iter()
            .all(|t| t.source == SpendSource::Subscription && t.sends == 0)
    );
}

fn subscription(job: &str, event: LineEvent, at_ms: f64, outcome: SpendOutcome) -> LedgerLine {
    let base = line(job, event, at_ms, outcome);
    LedgerLine {
        task: None,
        summary: "By hand on the Tripo site, textured image to 3D".into(),
        ..LedgerLine::subscription(event, at_ms, outcome)
    }
    .with_send(&base)
}

impl LedgerLine {
    fn with_send(mut self, from: &LedgerLine) -> Self {
        self.project.clone_from(&from.project);
        self.job.clone_from(&from.job);
        self.asset.clone_from(&from.asset);
        self.asset_name.clone_from(&from.asset_name);
        self.estimate = from.estimate;
        self
    }
}

#[test]
fn subscription_sends_settle_on_import_and_never_mix_with_api_totals() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sent = subscription(
        "lantern-1",
        LineEvent::Sent,
        NOW - 60_000.0,
        SpendOutcome::Pending,
    );
    append(dir.path(), &sent).expect("sent");
    let ledger = read(dir.path()).expect("read");
    let spend = &ledger.spends[0];
    assert_eq!(spend.source, SpendSource::Subscription);
    assert_eq!(spend.outcome, SpendOutcome::Pending);
    assert_eq!((spend.credits, spend.estimated), (30.0, true));

    append(
        dir.path(),
        &subscription(
            "lantern-1",
            LineEvent::Settled,
            NOW,
            SpendOutcome::Succeeded,
        ),
    )
    .expect("settled");
    let mut paid = line(
        "lantern-1",
        LineEvent::Settled,
        NOW,
        SpendOutcome::Succeeded,
    );
    paid.credits = Some(40.0);
    append(dir.path(), &paid).expect("api");
    let ledger = read(dir.path()).expect("read");
    assert_eq!(
        ledger.spends.len(),
        2,
        "the same job id, two sources, two sends"
    );
    let by_hand = ledger
        .spends
        .iter()
        .find(|s| s.source == SpendSource::Subscription)
        .expect("subscription spend");
    assert_eq!(by_hand.outcome, SpendOutcome::Succeeded);
    assert_eq!(by_hand.settled_at_ms, Some(NOW));
    assert_eq!(by_hand.credits, 30.0);

    let totals = summarise(&ledger.spends, NOW);
    let api_all = totals
        .iter()
        .find(|t| t.source == SpendSource::Api && t.service.is_none())
        .expect("api sum");
    let web_all = totals
        .iter()
        .find(|t| t.source == SpendSource::Subscription && t.service.is_none())
        .expect("subscription sum");
    assert_eq!((api_all.total, api_all.sends), (40.0, 1));
    assert_eq!((web_all.total, web_all.sends), (30.0, 1));

    // Version 2 lines: an older reader (version 1 only) skips them.
    let text = fs::read_to_string(path(dir.path())).expect("file");
    let versions: Vec<bool> = text.lines().map(|l| l.contains("\"v\":2")).collect();
    assert_eq!(versions, vec![true, true, false]);
    assert!(
        text.lines()
            .next()
            .is_some_and(|l| l.contains("\"source\":\"subscription\""))
    );
}

#[test]
fn lines_without_a_source_read_as_api() {
    let old = r#"{"v":1,"atMs":5,"event":"sent","project":"p","job":"a-1","asset":"a","assetName":"A","service":"tripo","task":"t","summary":"","estimate":30,"credits":null,"outcome":"pending","redoOf":null,"viaAgent":false}"#;
    let ledger = parse(old);
    assert_eq!(ledger.skipped, 0);
    assert_eq!(ledger.spends[0].source, SpendSource::Api);
    assert_eq!(parse(&old.replace("\"v\":1", "\"v\":3")).skipped, 1);
}

#[test]
fn a_job_id_used_again_is_a_new_send() {
    // Clear finished frees `lantern-1`; the next send gets it again with a
    // new provider task.
    let first_sent = line(
        "lantern-1",
        LineEvent::Sent,
        NOW - 90_000.0,
        SpendOutcome::Pending,
    );
    let mut first_done = line(
        "lantern-1",
        LineEvent::Settled,
        NOW - 80_000.0,
        SpendOutcome::Succeeded,
    );
    first_done.credits = Some(40.0);
    let mut again = line("lantern-1", LineEvent::Sent, NOW, SpendOutcome::Pending);
    again.task = Some("task-second".into());
    let ledger = fold(&[first_sent, first_done, again]);
    assert_eq!(ledger.len(), 2);
    assert_eq!(ledger[0].task.as_deref(), Some("task-second"));
    assert_eq!(ledger[0].outcome, SpendOutcome::Pending);
    assert_eq!(ledger[0].credits, 30.0);
    assert_eq!(ledger[1].outcome, SpendOutcome::Succeeded);
    assert_eq!(ledger[1].credits, 40.0);

    // Lines without a task still fold by project and job.
    let mut sent = line("lantern-2", LineEvent::Sent, NOW, SpendOutcome::Pending);
    sent.task = None;
    let mut settled = line("lantern-2", LineEvent::Settled, NOW, SpendOutcome::Failed);
    settled.task = None;
    assert_eq!(fold(&[sent, settled]).len(), 1);
}
