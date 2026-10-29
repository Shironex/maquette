//! The watcher against a real workspace in a temp folder, with a temp
//! folder standing in for Downloads. Nothing here reads a real Downloads
//! folder or talks to a provider.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::json;

use super::*;
use crate::inbox::fixture;
use crate::providers::arrivals::pending_handoffs;
use crate::providers::fake::{FakeServer, Reply};
use crate::providers::ledger::{SpendOutcome, SpendSource};
use crate::providers::presets::PresetVariant;
use crate::providers::tests::{World, add_objects};

mod render;

pub(super) struct Rig {
    pub(super) world: World,
    _server: FakeServer,
    pub(super) jobs: ProviderJobs,
    pub(super) arrivals: Arrivals,
    pub(super) seen: Arc<Mutex<Vec<Arrival>>>,
    downloads: PathBuf,
}

pub(super) fn rig() -> Rig {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let seen: Arc<Mutex<Vec<Arrival>>> = Arc::default();
    let sink = Arc::clone(&seen);
    let timing = WatchTiming {
        tick: Duration::from_millis(50),
        quiet: Duration::from_millis(250),
        give_up: Duration::from_secs(30),
    };
    let seen_dir = world.root.parent().expect("parent").join("app-data");
    let arrivals = Arrivals::new(jobs.clone(), timing, seen_dir, move |a| {
        sink.lock().expect("lock").push(a.clone());
    });
    let downloads = world.root.parent().expect("parent").join("Downloads");
    fs::create_dir_all(&downloads).expect("downloads");
    Rig {
        world,
        _server: server,
        jobs,
        arrivals,
        seen,
        downloads,
    }
}

impl Rig {
    pub(super) fn by_hand(&self, asset: &str) -> ProviderJob {
        let mut request = self
            .world
            .request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
        request.asset = asset.to_string();
        self.jobs.send(&request, false, false).expect("send")
    }

    pub(super) fn download(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.downloads.join(name);
        fs::write(&path, bytes).expect("download");
        path
    }
}

#[test]
fn a_named_export_is_copied_into_the_inbox_once_and_settles_the_ledger() {
    let rig = rig();
    let job = rig.by_hand(&rig.world.asset.clone());
    let card = job.card.clone().expect("card");
    assert_eq!(card.export_name, "maq_stone-lantern_v1_tripo");
    let file = rig.download(
        "maq_stone-lantern_v1_tripo (1).glb",
        &fixture::character(false),
    );

    let arrival = rig.arrivals.take(&file).expect("taken");
    assert_eq!(arrival.state, ArrivalState::Imported, "{arrival:?}");
    assert_eq!(arrival.job.as_deref(), Some(job.id.as_str()));
    assert_eq!(arrival.reason, "Named for Stone lantern");
    let inbox_file = arrival.inbox_file.clone().expect("inbox file");
    assert!(rig.world.inbox().join(&inbox_file).is_file());
    assert!(file.is_file(), "the download stays where it was");
    assert_eq!(rig.jobs.get(&job.id).expect("job").state, JobState::Inbox);
    assert_eq!(rig.seen.lock().expect("lock").len(), 1, "one quiet notice");

    // The same bytes again, under any name, are left alone.
    assert!(rig.arrivals.take(&file).is_none());
    let copy = rig.download("model.glb", &fixture::character(false));
    assert!(rig.arrivals.take(&copy).is_none());

    let ledger = rig.jobs.ledger().expect("ledger");
    let spend = ledger
        .spends
        .iter()
        .find(|s| s.job == job.id)
        .expect("subscription spend");
    assert_eq!(spend.source, SpendSource::Subscription);
    assert_eq!(spend.outcome, SpendOutcome::Succeeded);
    assert_eq!((spend.credits, spend.estimated), (30.0, true));
    assert!(spend.settled_at_ms.is_some());
}

#[test]
fn an_unplaced_download_waits_for_which_asset_is_this() {
    let rig = rig();
    let ids = add_objects(&rig.world, &["Desk lamp"]);
    let lantern = rig.by_hand(&rig.world.asset.clone());
    let lamp = rig.by_hand(&ids[0]);
    let file = rig.download("model (3).glb", &fixture::character(false));

    let arrival = rig.arrivals.take(&file).expect("seen");
    assert_eq!(arrival.state, ArrivalState::Unmatched);
    assert_eq!(arrival.reason, "2 sends are waiting");
    assert!(rig.arrivals.take(&file).is_none(), "asked once");
    let status = rig.arrivals.status();
    assert_eq!(status.pending.len(), 2);
    assert_eq!(status.arrivals.len(), 1);

    // A file you dismiss is never asked about again.
    let other = rig.download("scan.zip", b"PK\x03\x04 not really a zip");
    let asked = rig.arrivals.take(&other).expect("asked");
    assert_eq!(asked.state, ArrivalState::Unmatched);
    let gone = rig.arrivals.dismiss(&asked.id).expect("dismiss");
    assert_eq!(gone.state, ArrivalState::Dismissed);
    assert!(rig.arrivals.take(&other).is_none());

    // Attached by asset: into the inbox for its waiting send, as a named export.
    assert!(
        rig.arrivals
            .attach(&arrival.id, &rig.world.project, "nope")
            .is_err()
    );
    let done = rig
        .arrivals
        .attach(&arrival.id, &rig.world.project, &ids[0])
        .expect("attach");
    assert_eq!(done.state, ArrivalState::Attached);
    assert_eq!(done.asset_name.as_deref(), Some("Desk lamp"));
    assert_eq!(done.job.as_deref(), Some(lamp.id.as_str()));
    let inbox_file = done.inbox_file.clone().expect("inbox file");
    assert!(rig.world.inbox().join(&inbox_file).is_file());
    assert_eq!(rig.jobs.get(&lamp.id).expect("lamp").state, JobState::Inbox);
    assert_eq!(
        rig.jobs.get(&lantern.id).expect("lantern").state,
        JobState::Waiting
    );
    assert!(file.is_file(), "the download stays where it was");
    assert!(
        rig.arrivals
            .attach(&arrival.id, &rig.world.project, &rig.world.asset)
            .is_err(),
        "taken"
    );

    // With one send left, an unnamed new file is only suggested for it.
    let last = rig.download("Untitled.glb", &fixture::character(true));
    let taken = rig.arrivals.take(&last).expect("taken");
    assert_eq!(taken.state, ArrivalState::Review);
    assert_eq!(taken.job.as_deref(), Some(lantern.id.as_str()));
    assert_eq!(taken.reason, "The only send waiting, for Stone lantern");
}

/// Accepts an inbox file as the Inbox's Accept does, with no asset picked,
/// so it goes where the inbox says. Returns the asset it went to.
fn accept(rig: &Rig, item: &str) -> String {
    let result = crate::inbox::accept(
        &rig.world.root,
        &rig.world.config,
        &crate::inbox::AcceptRequest {
            item: item.to_string(),
            project: rig.world.project.clone(),
            asset: rig
                .jobs
                .list(None)
                .iter()
                .find(|j| j.inbox_file.as_deref() == Some(item))
                .map(|j| j.asset.clone()),
            name: None,
            prep: None,
        },
    )
    .expect("accept");
    rig.jobs.accepted(
        item,
        &rig.world.project,
        &result.asset.id,
        result.version.version,
    );
    result.asset.id
}

/// The project's asset `id` as it is on disk.
fn asset_on_disk(rig: &Rig, id: &str) -> crate::assets::Asset {
    let manifest = crate::project::load(
        &rig.world.root,
        &rig.world.config.layout,
        &rig.world.project,
    )
    .expect("project");
    crate::assets::find(&manifest, id).expect("asset").clone()
}

#[test]
fn a_close_name_waits_for_a_look_and_lands_only_when_attached() {
    let rig = rig();
    let ids = add_objects(&rig.world, &["Stone ruin", "Wooden log"]);
    let ruin = rig.by_hand(&ids[0]);
    let log = rig.by_hand(&ids[1]);
    // The site's batch names: close to one send each, never taken alone.
    let file = rig.download("stone ruins 3d model.glb", &fixture::character(false));
    let arrival = rig.arrivals.take(&file).expect("seen");
    assert_eq!(arrival.state, ArrivalState::Review, "{arrival:?}");
    assert_eq!(arrival.reason, "Name close to Stone ruin");
    assert_eq!(arrival.job.as_deref(), Some(ruin.id.as_str()));
    assert_eq!(arrival.asset.as_deref(), Some(ids[0].as_str()));
    assert_eq!(
        rig.jobs.get(&ruin.id).expect("ruin").state,
        JobState::Waiting
    );
    assert!(asset_on_disk(&rig, &ids[0]).versions.is_empty());
    assert!(rig.arrivals.take(&file).is_none(), "looked at once");

    // Its thumbnail renders from a GLB copy in the app data, not the download.
    let preview = rig.arrivals.preview_file(&arrival.id).expect("preview");
    assert!(preview.is_file());
    assert!(preview.starts_with(rig.world.root.parent().expect("parent").join("app-data")));

    // Pick another asset: into the inbox for the log's send; the ruin waits on.
    let done = rig
        .arrivals
        .attach(&arrival.id, &rig.world.project, &ids[1])
        .expect("attach");
    assert_eq!(done.state, ArrivalState::Attached);
    assert_eq!(done.reason, "You attached it to Wooden log");
    assert_eq!(rig.jobs.get(&log.id).expect("log").state, JobState::Inbox);
    assert_eq!(
        rig.jobs.get(&ruin.id).expect("ruin").state,
        JobState::Waiting
    );
    assert!(
        asset_on_disk(&rig, &ids[1]).versions.is_empty(),
        "only accepting in the Inbox makes a version"
    );
    assert!(
        !preview.exists(),
        "the preview copy goes once it is attached"
    );
    assert!(rig.arrivals.preview_file(&arrival.id).is_err());
}

#[test]
fn attaching_closes_every_waiting_send_of_the_asset_or_none() {
    let rig = rig();
    let ids = add_objects(&rig.world, &["Lantern post"]);
    let first = rig.by_hand(&ids[0]);
    let again = rig.by_hand(&ids[0]);
    let file = rig.download("model.glb", &fixture::character(false));
    let arrival = rig.arrivals.take(&file).expect("seen");
    assert_eq!(arrival.state, ArrivalState::Unmatched);

    let done = rig
        .arrivals
        .attach(&arrival.id, &rig.world.project, &ids[0])
        .expect("attach");
    // The newest send took it into the inbox; the older one waits until it is accepted.
    assert_eq!(done.job.as_deref(), Some(again.id.as_str()));
    assert_eq!(
        rig.jobs.get(&again.id).expect("again").state,
        JobState::Inbox
    );
    assert_eq!(
        rig.jobs.get(&first.id).expect("first").state,
        JobState::Waiting
    );
    let item = done.inbox_file.expect("inbox file");
    assert_eq!(accept(&rig, &item), ids[0]);
    assert_eq!(
        rig.jobs.get(&again.id).expect("again").state,
        JobState::Accepted
    );
    let older = rig.jobs.get(&first.id).expect("first");
    assert_eq!(older.state, JobState::Accepted);
    assert_eq!(
        older.message.as_deref(),
        Some("Model added to the asset, no longer waiting")
    );
    assert!(pending_handoffs(&rig.jobs).is_empty());

    // An asset with no send waiting: the file waits in the inbox aimed at it.
    let send = rig.by_hand(&rig.world.asset.clone());
    let other = rig.download("stray.glb", &fixture::character(true));
    let stray = rig.arrivals.take(&other).expect("seen");
    rig.jobs.cancel(&send.id).expect("stop waiting");
    let done = rig
        .arrivals
        .attach(&stray.id, &rig.world.project, &ids[0])
        .expect("attach");
    assert_eq!(done.job, None);
    let item = done.inbox_file.expect("inbox file");
    let listed = crate::inbox::item(&rig.world.root, &rig.world.config, &item).expect("item");
    let aim = listed.aim.expect("aimed");
    assert_eq!(
        (aim.asset.as_str(), aim.asset_name.as_str()),
        (ids[0].as_str(), "Lantern post")
    );
    assert_eq!(asset_on_disk(&rig, &ids[0]).versions.len(), 1);
    // Accept with no asset picked goes to the aimed asset, and forgets the aim.
    assert_eq!(accept(&rig, &item), ids[0]);
    assert_eq!(asset_on_disk(&rig, &ids[0]).versions.len(), 2);
    let aims = crate::inbox::list(&rig.world.root, &rig.world.config).expect("list");
    assert!(aims.iter().all(|i| i.aim.is_none()));
}

#[test]
fn not_this_one_is_remembered_after_a_restart() {
    let rig = rig();
    let ids = add_objects(&rig.world, &["Stone ruin"]);
    let ruin = rig.by_hand(&ids[0]);
    rig.by_hand(&rig.world.asset.clone());
    let file = rig.download("stone ruins 3d model.glb", &fixture::character(false));
    let arrival = rig.arrivals.take(&file).expect("seen");
    assert_eq!(arrival.state, ArrivalState::Review);
    let gone = rig.arrivals.dismiss(&arrival.id).expect("dismiss");
    assert_eq!(gone.state, ArrivalState::Dismissed);
    assert_eq!(gone.reason, "You said it is not Stone ruin");
    assert_eq!(
        rig.jobs.get(&ruin.id).expect("ruin").state,
        JobState::Waiting
    );
    assert!(
        rig.arrivals
            .attach(&arrival.id, &rig.world.project, &ids[0])
            .is_err()
    );

    // A new watcher (the app opened again) reads the same list.
    let seen_dir = rig.world.root.parent().expect("parent").join("app-data");
    let again = Arrivals::new(rig.jobs.clone(), WatchTiming::APP, seen_dir, |_| {});
    assert!(again.take(&file).is_none(), "never asked about again");
    assert!(again.status().arrivals.is_empty());
}

#[test]
fn fbx_exports_are_matched_but_left_for_a_glb() {
    let rig = rig();
    let job = rig.by_hand(&rig.world.asset.clone());
    let file = rig.download("maq_stone-lantern_v1_tripo.fbx", b"Kaydara FBX Binary");
    let arrival = rig.arrivals.take(&file).expect("seen");
    assert_eq!(arrival.state, ArrivalState::NeedsGlb);
    assert_eq!(arrival.job.as_deref(), Some(job.id.as_str()));
    assert_eq!(rig.jobs.get(&job.id).expect("job").state, JobState::Waiting);
    let models = fs::read_dir(rig.world.inbox())
        .map(|d| {
            d.flatten()
                .filter(|e| crate::sends::is_model(&e.path()))
                .count()
        })
        .unwrap_or(0);
    assert_eq!(models, 0);
}

#[test]
fn nothing_is_read_while_no_send_waits() {
    let rig = rig();
    let file = rig.download("maq_stone-lantern_v1_tripo.glb", &fixture::character(false));
    assert!(rig.arrivals.take(&file).is_none());
    assert!(rig.arrivals.status().arrivals.is_empty());
}

#[test]
fn the_watch_follows_its_switch_and_picks_up_a_new_export() {
    let rig = rig();
    let job = rig.by_hand(&rig.world.asset.clone());
    rig.arrivals.configure(false, Some(rig.downloads.clone()));
    let off = rig.arrivals.status();
    assert!(!off.enabled && !off.watching);

    rig.arrivals
        .configure(true, Some(rig.downloads.join("missing")));
    let broken = rig.arrivals.status();
    assert!(!broken.watching && broken.error.is_some(), "{broken:?}");

    rig.arrivals.configure(true, Some(rig.downloads.clone()));
    assert!(rig.arrivals.status().watching);
    std::thread::sleep(Duration::from_millis(300));
    rig.download("maq_stone-lantern_v1_tripo.glb", &fixture::character(false));
    let start = Instant::now();
    while rig.seen.lock().expect("lock").is_empty() && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(50));
    }
    let seen = rig.seen.lock().expect("lock").clone();
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert_eq!(seen[0].state, ArrivalState::Imported);
    assert_eq!(rig.jobs.get(&job.id).expect("job").state, JobState::Inbox);
    rig.arrivals.stop();
    assert!(!rig.arrivals.status().watching);
}

#[test]
fn a_send_you_stop_waiting_for_settles_at_nothing() {
    let rig = rig();
    let job = rig.by_hand(&rig.world.asset.clone());
    rig.jobs.cancel(&job.id).expect("stop waiting");
    let ledger = rig.jobs.ledger().expect("ledger");
    let spend = ledger
        .spends
        .iter()
        .find(|s| s.job == job.id)
        .expect("spend");
    assert_eq!(spend.source, SpendSource::Subscription);
    assert_eq!(spend.outcome, SpendOutcome::Cancelled);
    assert_eq!((spend.credits, spend.estimated), (0.0, false));
    let file = rig.download("maq_stone-lantern_v1_tripo.glb", &fixture::character(false));
    assert!(rig.arrivals.take(&file).is_none(), "nothing waits any more");
}
