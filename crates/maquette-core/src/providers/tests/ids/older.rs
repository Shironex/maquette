//! A workspace as a Maquette from before per project numbering left it,
//! opened by this one: every saved job keeps its id and the name it was
//! handed out with, a file saved under that older name still finds its
//! waiting send, the models waiting in the inbox keep their links, and only
//! sends made from now on are numbered in their project.
//!
//! The text files under `older_workspace/` were written by Maquette 0.7.1
//! itself, the last version before this numbering, and are kept as it wrote
//! them: the projects `moon-island` and `lantern-bay`, each with the asset
//! `stone-lantern`, whose five sends were numbered across the workspace.
//!
//! | Job | Project | What | Where it stands |
//! | --- | --- | --- | --- |
//! | `stone-lantern-1` | moon-island | Tripo | accepted as v1 |
//! | `stone-lantern-2` | lantern-bay | By hand on Tripo | waiting, `maq_stone-lantern_v2_tripo` |
//! | `stone-lantern-3` | moon-island | Tripo split of v1 | accepted as v2, bake back offered |
//! | `stone-lantern-4` | lantern-bay | Meshy | in the inbox as `stone-lantern-4.glb` |
//! | `stone-lantern-5` | moon-island | By hand on Meshy | waiting, `maq_stone-lantern_v5_meshy` |
//!
//! The models and images it wrote beside them are made here again, from the
//! same fixtures it got them from.

use std::path::Path;

use super::*;
use crate::base::ports::TextureBake;
use crate::error::{CoreError, CoreResult};
use crate::pipeline::tools::Toolbox;
use crate::providers::arrivals::pending_handoffs;
use crate::sends::{InboxPort, SendPorts};

const MOON: &str = "moon-island";
const BAY: &str = "lantern-bay";
const LANTERN: &str = "stone-lantern";

macro_rules! saved {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("older_workspace/", $path)))),*]
    };
}

/// Every text file of the older workspace, by its path in it.
const FILES: &[(&str, &str)] = saved![
    "maquette.json",
    "credits.jsonl",
    "cache/inbox/round.json",
    "projects/lantern-bay/project.json",
    "projects/lantern-bay/providers.json",
    "projects/lantern-bay/handoff/stone-lantern-2/send.json",
    "projects/lantern-bay/handoff/stone-lantern-2/settings.json",
    "projects/lantern-bay/handoff/stone-lantern-2/settings.md",
    "projects/moon-island/project.json",
    "projects/moon-island/providers.json",
    "projects/moon-island/prepare.json",
    "projects/moon-island/handoff/stone-lantern-5/send.json",
    "projects/moon-island/handoff/stone-lantern-5/settings.json",
    "projects/moon-island/handoff/stone-lantern-5/settings.md",
];

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

fn write(root: &Path, path: &str, bytes: &[u8]) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().expect("parent")).expect("folder");
    fs::write(file, bytes).expect("write");
}

/// The older workspace in a temp folder, as it was when its Maquette closed.
fn older_world() -> World {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("ws");
    for (path, text) in FILES {
        write(&root, path, text.as_bytes());
    }
    let (model, parts) = (fixture::clean_prop(), fixture::merged_islands());
    for (path, bytes) in [
        ("inbox/stone-lantern-4.glb", model.as_slice()),
        ("cache/splits/stone-lantern-3/source.glb", model.as_slice()),
        (
            "projects/moon-island/models/stone_lantern_v1.glb",
            model.as_slice(),
        ),
        (
            "projects/moon-island/models/stone_lantern_v2.glb",
            parts.as_slice(),
        ),
        ("projects/moon-island/refs/front.png", PNG),
        ("projects/moon-island/refs/left.png", PNG),
        ("projects/moon-island/refs/back.png", PNG),
        ("projects/lantern-bay/refs/front.png", PNG),
        (
            "projects/moon-island/handoff/stone-lantern-5/front.png",
            PNG,
        ),
        (
            "projects/lantern-bay/handoff/stone-lantern-2/front.png",
            PNG,
        ),
    ] {
        write(&root, path, bytes);
    }
    let config = workspace::open(&root).expect("the older workspace opens");
    let secrets = Arc::new(MemoryStore::default());
    secrets
        .set(ProviderId::Tripo, "tsk_test_key_0001")
        .expect("key");
    secrets
        .set(ProviderId::Meshy, "msy_test_key_0002")
        .expect("key");
    World {
        _dir: dir,
        root,
        config,
        project: MOON.to_string(),
        asset: LANTERN.to_string(),
        secrets,
        events: Arc::default(),
    }
}

fn saved_file(path: &str) -> &'static str {
    FILES
        .iter()
        .find(|(p, _)| *p == path)
        .map(|(_, text)| *text)
        .expect("a file of the older workspace")
}

fn no_server() -> FakeServer {
    FakeServer::start(|_, _| Reply::json(404, &json!({})))
}

fn state(jobs: &ProviderJobs, project: &str, id: &str) -> JobState {
    jobs.get(project, id)
        .unwrap_or_else(|| panic!("{id} of {project}"))
        .state
}

#[test]
fn an_older_workspace_opens_with_every_id_and_name_as_it_was() {
    let world = older_world();
    let server = no_server();
    let jobs = world.jobs(&server);

    let ids = |project: &str| -> Vec<String> {
        jobs.list(Some(project)).into_iter().map(|j| j.id).collect()
    };
    assert_eq!(
        ids(MOON),
        ["stone-lantern-1", "stone-lantern-3", "stone-lantern-5"]
    );
    assert_eq!(ids(BAY), ["stone-lantern-2", "stone-lantern-4"]);
    assert_eq!(state(&jobs, MOON, "stone-lantern-1"), JobState::Accepted);
    assert_eq!(state(&jobs, MOON, "stone-lantern-3"), JobState::Accepted);
    assert_eq!(state(&jobs, MOON, "stone-lantern-5"), JobState::Waiting);
    assert_eq!(state(&jobs, BAY, "stone-lantern-2"), JobState::Waiting);
    assert_eq!(state(&jobs, BAY, "stone-lantern-4"), JobState::Inbox);
    // An id is a job only with its project.
    assert!(jobs.get(BAY, "stone-lantern-5").is_none());
    assert!(jobs.get(MOON, "stone-lantern-2").is_none());

    // The waiting sends keep the names they were handed out with, and their folders.
    let pending = pending_handoffs(&jobs);
    let mut waits: Vec<(&str, &str, &str)> = pending
        .iter()
        .map(|p| (p.project.as_str(), p.job.as_str(), p.expected_name.as_str()))
        .collect();
    waits.sort_unstable();
    assert_eq!(
        waits,
        [
            (BAY, "stone-lantern-2", "maq_stone-lantern_v2_tripo"),
            (MOON, "stone-lantern-5", "maq_stone-lantern_v5_meshy"),
        ]
    );
    for wait in &pending {
        let folder = PathBuf::from(wait.handoff.clone().expect("folder"));
        assert_eq!(
            folder,
            world
                .root
                .join("projects")
                .join(&wait.project)
                .join("handoff")
                .join(&wait.job)
        );
        assert!(folder.join("send.json").is_file(), "{}", folder.display());
    }
    let bake = jobs
        .get(MOON, "stone-lantern-3")
        .and_then(|j| j.split?.bake)
        .expect("the offered bake back");
    assert_eq!((bake.state, bake.onto), (BakeState::Offered, 2));

    // Opening rewrote nothing: the saved jobs and the ledger are byte for
    // byte what the older Maquette left.
    jobs.close();
    for path in [
        "projects/moon-island/providers.json",
        "projects/lantern-bay/providers.json",
        "credits.jsonl",
    ] {
        assert_eq!(
            fs::read_to_string(world.root.join(path)).expect("read"),
            saved_file(path),
            "{path} was rewritten"
        );
    }
}

#[test]
fn the_ledger_of_an_older_workspace_reads_as_it_did() {
    let world = older_world();
    let server = no_server();
    let jobs = world.jobs(&server);
    let ledger = jobs.ledger().expect("ledger");
    assert_eq!(ledger.skipped, 0);
    let mut spends: Vec<(&str, &str, SpendOutcome, f64)> = ledger
        .spends
        .iter()
        .map(|s| (s.project.as_str(), s.job.as_str(), s.outcome, s.credits))
        .collect();
    spends.sort_by(|a, b| a.1.cmp(b.1));
    assert_eq!(
        spends,
        [
            (MOON, "stone-lantern-1", SpendOutcome::Succeeded, 30.0),
            (BAY, "stone-lantern-2", SpendOutcome::Pending, 30.0),
            (MOON, "stone-lantern-3", SpendOutcome::Succeeded, 40.0),
            (BAY, "stone-lantern-4", SpendOutcome::Succeeded, 20.0),
            (MOON, "stone-lantern-5", SpendOutcome::Pending, 30.0),
        ]
    );
}

/// A watcher over `world`, with a folder standing in for Downloads.
fn watcher(world: &World, jobs: &ProviderJobs) -> (Arrivals, PathBuf) {
    let parent = world.root.parent().expect("parent");
    let arrivals = Arrivals::new(
        jobs.clone(),
        WatchTiming::APP,
        parent.join("app-data"),
        |_| {},
    );
    let downloads = parent.join("Downloads");
    fs::create_dir_all(&downloads).expect("downloads");
    (arrivals, downloads)
}

fn download(downloads: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let file = downloads.join(name);
    fs::write(&file, bytes).expect("download");
    file
}

#[test]
fn a_file_saved_under_an_older_name_still_matches_its_waiting_send() {
    let world = older_world();
    let server = no_server();
    let jobs = world.jobs(&server);
    let (arrivals, downloads) = watcher(&world, &jobs);

    // The name lantern-bay's send was handed out with, before projects
    // were part of it, with the copy number a browser adds.
    let file = download(
        &downloads,
        "maq_stone-lantern_v2_tripo (1).glb",
        &fixture::character(false),
    );
    let arrival = arrivals.take(&file).expect("taken");
    assert_eq!(arrival.state, ArrivalState::Imported, "{arrival:?}");
    assert_eq!(arrival.project.as_deref(), Some(BAY));
    assert_eq!(arrival.job.as_deref(), Some("stone-lantern-2"));
    assert_eq!(arrival.inbox_file.as_deref(), Some("stone-lantern-2.glb"));
    assert_eq!(state(&jobs, BAY, "stone-lantern-2"), JobState::Inbox);
    assert_eq!(
        state(&jobs, MOON, "stone-lantern-5"),
        JobState::Waiting,
        "the other project's send still waits"
    );

    // And moon-island's, on Meshy.
    let file = download(&downloads, "MAQ_Stone-Lantern_V5_Meshy.glb", &glb());
    let arrival = arrivals.take(&file).expect("taken");
    assert_eq!(arrival.state, ArrivalState::Imported, "{arrival:?}");
    assert_eq!(arrival.project.as_deref(), Some(MOON));
    assert_eq!(state(&jobs, MOON, "stone-lantern-5"), JobState::Inbox);
}

#[test]
fn the_models_waiting_in_the_inbox_keep_their_links() {
    let world = older_world();
    let server = no_server();
    let jobs = world.jobs(&server);

    // The model Meshy made for lantern-bay, as the older Maquette saved it.
    let mut items = inbox::list(&world.root, &world.config).expect("inbox");
    jobs.link_inbox(&mut items);
    assert_eq!(items.len(), 1);
    let link = items[0].link.clone().expect("linked");
    assert_eq!(items[0].id, "stone-lantern-4.glb");
    assert_eq!(
        (link.project.as_str(), link.job.as_str()),
        (BAY, "stone-lantern-4")
    );
    assert_eq!(link.provider, Provider::Meshy);

    // A model saved into the inbox by hand under the older export name
    // arrives for the send that waits for that name.
    fs::write(
        world.inbox().join("maq_stone-lantern_v5_meshy.glb"),
        fixture::character(false),
    )
    .expect("saved by hand");
    let mut items = inbox::list(&world.root, &world.config).expect("inbox");
    jobs.link_inbox(&mut items);
    let by_hand = items
        .iter()
        .find(|i| i.id == "maq_stone-lantern_v5_meshy.glb")
        .and_then(|i| i.link.clone())
        .expect("linked by its name");
    assert_eq!(
        (by_hand.project.as_str(), by_hand.job.as_str()),
        (MOON, "stone-lantern-5")
    );
    assert_eq!(state(&jobs, MOON, "stone-lantern-5"), JobState::Inbox);
    assert_eq!(state(&jobs, BAY, "stone-lantern-2"), JobState::Waiting);

    // Accepting lantern-bay's model closes its own send and no other.
    let accepted = inbox::accept(
        &world.root,
        &world.config,
        &inbox::AcceptRequest {
            item: "stone-lantern-4.glb".into(),
            project: BAY.into(),
            asset: Some(LANTERN.into()),
            name: None,
            prep: None,
        },
    )
    .expect("accept");
    jobs.accepted(
        "stone-lantern-4.glb",
        BAY,
        LANTERN,
        accepted.version.version,
    );
    let done = jobs.get(BAY, "stone-lantern-4").expect("job");
    assert_eq!((done.state, done.version), (JobState::Accepted, Some(1)));
    assert_eq!(state(&jobs, MOON, "stone-lantern-1"), JobState::Accepted);
    assert_eq!(
        jobs.get(MOON, "stone-lantern-1").and_then(|j| j.version),
        Some(1),
        "the other project's first version is its own"
    );
}

#[test]
fn only_sends_made_from_now_on_are_numbered_in_their_project() {
    let world = older_world();
    let server = no_server();
    let jobs = world.jobs(&server);
    let (arrivals, downloads) = watcher(&world, &jobs);

    // lantern-bay holds 2 and 4 from the older numbering: its next send is
    // its 5, never one of those again and never a number that reads older.
    let sent = jobs
        .send(&request_in(&world, BAY, Provider::ByHand), false, false)
        .expect("send");
    assert_eq!(sent.id, "stone-lantern-5");
    let name = sent.export_name().expect("name");
    assert_eq!(name, "maq_lantern-bay_stone-lantern_v5_tripo");
    // moon-island holds a 5 of its own from before: two jobs, one id.
    assert_eq!(jobs.with_id("stone-lantern-5").len(), 2);
    assert_eq!(
        jobs.get(MOON, "stone-lantern-5")
            .and_then(|j| j.export_name()),
        Some("maq_stone-lantern_v5_meshy".to_string()),
        "the older send keeps its name"
    );

    // The older records are saved again as they were; only the new one
    // says it is numbered in its project.
    let read = |text: &str| -> Value { serde_json::from_str(text).expect("json") };
    let before = read(saved_file("projects/lantern-bay/providers.json"));
    let after =
        read(&fs::read_to_string(project_dir(&world, BAY).join(jobs::JOBS_FILE)).expect("saved"));
    assert_eq!(
        after["version"],
        json!(1),
        "an older Maquette still reads the file"
    );
    let (before, after) = (
        before["jobs"].as_array().expect("jobs"),
        after["jobs"].as_array().expect("jobs"),
    );
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(&after[..before.len()], before.as_slice());
    assert!(before.iter().all(|s| s.get("scope").is_none()));
    assert_eq!(after[before.len()]["scope"], json!("project"));
    assert_eq!(after[before.len()]["job"]["id"], json!("stone-lantern-5"));

    // Each of the two 5s gets the file saved under its own name.
    let bare = download(
        &downloads,
        "stone-lantern-5.glb",
        &fixture::merged_islands(),
    );
    let asked = arrivals.take(&bare).expect("seen");
    assert_eq!(asked.state, ArrivalState::Unmatched, "{asked:?}");
    assert_eq!(
        asked.reason,
        "2 sends named stone-lantern-5 are waiting, in lantern-bay and moon-island"
    );
    let older = download(&downloads, "maq_stone-lantern_v5_meshy.glb", &glb());
    let arrival = arrivals.take(&older).expect("taken");
    assert_eq!(arrival.state, ArrivalState::Imported, "{arrival:?}");
    assert_eq!(arrival.project.as_deref(), Some(MOON));
    assert_eq!(state(&jobs, BAY, "stone-lantern-5"), JobState::Waiting);
    let newer = download(
        &downloads,
        &format!("{name}.glb"),
        &fixture::character(false),
    );
    let arrival = arrivals.take(&newer).expect("taken");
    assert_eq!(arrival.state, ArrivalState::Imported, "{arrival:?}");
    assert_eq!(arrival.project.as_deref(), Some(BAY));
    assert_eq!(state(&jobs, BAY, "stone-lantern-5"), JobState::Inbox);

    // moon-island goes on from its own highest.
    let next = jobs
        .send(&request_in(&world, MOON, Provider::ByHand), false, false)
        .expect("send");
    assert_eq!(next.id, "stone-lantern-6");

    // The ledger tells the two 5s apart by their project.
    let ledger = jobs.ledger().expect("ledger");
    let fives: Vec<&str> = ledger
        .spends
        .iter()
        .filter(|s| s.job == "stone-lantern-5")
        .map(|s| s.project.as_str())
        .collect();
    assert_eq!(fives.len(), 2, "{ledger:?}");
    assert!(fives.contains(&MOON) && fives.contains(&BAY));
}

/// A bake that only notes where it was asked to read from and work in.
#[derive(Default)]
struct NotedBake {
    asked: Mutex<Vec<(PathBuf, PathBuf)>>,
}

impl TextureBake for NotedBake {
    fn bake(&self, dir: &Path, source: &Path, _: &Path, _: &Toolbox) -> CoreResult<PathBuf> {
        self.asked
            .lock()
            .expect("lock")
            .push((dir.to_path_buf(), source.to_path_buf()));
        Err(CoreError::Invalid("No Blender in this test.".to_string()))
    }
}

#[test]
fn an_older_split_bakes_back_from_the_copy_it_kept() {
    let world = older_world();
    let bake = Arc::new(NotedBake::default());
    let jobs = ProviderJobs::new(
        Arc::clone(&world.secrets) as Arc<dyn SecretStore>,
        Options::default(),
        SendPorts {
            inbox: Arc::new(inbox::Inbox) as Arc<dyn InboxPort>,
            bake: Arc::clone(&bake) as Arc<dyn TextureBake>,
        },
        |_| {},
    );
    jobs.open_workspace(&world.root, &world.config);

    let done = jobs
        .bake_back(MOON, "stone-lantern-3", &Toolbox::default())
        .expect("the bake ran");
    let state = done.split.and_then(|s| s.bake).map(|b| b.state);
    assert_eq!(state, Some(BakeState::Failed), "the stand in bakes nothing");
    // It read the model the older Maquette copied when the split went out,
    // where that one put it, and nothing was moved.
    let kept = world
        .root
        .join("cache")
        .join("splits")
        .join("stone-lantern-3");
    assert_eq!(
        *bake.asked.lock().expect("lock"),
        [(kept.join("bake"), kept.join("source.glb"))]
    );
    assert!(kept.join("source.glb").is_file());
    assert!(!world.root.join("cache").join("sends").exists());
    // The same id in the other project is no job, so nothing bakes there.
    assert!(
        jobs.bake_back(BAY, "stone-lantern-3", &Toolbox::default())
            .is_err()
    );
}
