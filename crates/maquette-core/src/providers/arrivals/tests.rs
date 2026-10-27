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
use crate::providers::fake::{FakeServer, Reply};
use crate::providers::ledger::{SpendOutcome, SpendSource};
use crate::providers::presets::PresetVariant;
use crate::providers::tests::{World, add_objects};

struct Rig {
    world: World,
    _server: FakeServer,
    jobs: ProviderJobs,
    arrivals: Arrivals,
    seen: Arc<Mutex<Vec<Arrival>>>,
    downloads: PathBuf,
}

fn rig() -> Rig {
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
    fn by_hand(&self, asset: &str) -> ProviderJob {
        let mut request = self
            .world
            .request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
        request.asset = asset.to_string();
        self.jobs.send(&request, false, false).expect("send")
    }

    fn download(&self, name: &str, bytes: &[u8]) -> PathBuf {
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

    assert!(rig.arrivals.assign(&arrival.id, "nope-1").is_err());
    let done = rig.arrivals.assign(&arrival.id, &lamp.id).expect("assign");
    assert_eq!(done.state, ArrivalState::Imported);
    assert_eq!(done.asset_name.as_deref(), Some("Desk lamp"));
    assert_eq!(rig.jobs.get(&lamp.id).expect("lamp").state, JobState::Inbox);
    assert_eq!(
        rig.jobs.get(&lantern.id).expect("lantern").state,
        JobState::Waiting
    );
    assert!(
        rig.arrivals.assign(&arrival.id, &lantern.id).is_err(),
        "taken"
    );

    // With one send left, an unnamed new file goes to it.
    let last = rig.download("Untitled.glb", &fixture::character(true));
    let taken = rig.arrivals.take(&last).expect("taken");
    assert_eq!(taken.job.as_deref(), Some(lantern.id.as_str()));
    assert_eq!(taken.reason, "The only send waiting, for Stone lantern");
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
                .filter(|e| crate::inbox::watch::is_model(&e.path()))
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
