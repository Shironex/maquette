//! A By hand send closes once its asset has a model that came in another
//! way, and a Tripo or Meshy send of the same asset runs on.

use super::*;
use crate::providers::arrivals::pending_handoffs;

const ANOTHER_WAY: &str = "Model added to the asset, no longer waiting";

fn send_by_hand(world: &World, jobs: &ProviderJobs) -> ProviderJob {
    let request = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    jobs.send(&request, false, false).expect("send")
}

fn give_model(world: &World) {
    project::modify(&world.root, &world.config.layout, &world.project, |m| {
        assets::add_version(m, &world.asset, "models/lantern_v1.glb".into(), None).map(|_| ())
    })
    .expect("version");
}

/// Drops a model into the inbox under a name no send matches, and accepts
/// it for the world's asset, as accept_inbox_model with an `asset` does.
fn accept_for_asset(world: &World, jobs: &ProviderJobs) -> usize {
    fs::create_dir_all(world.inbox()).expect("inbox");
    fs::write(
        world.inbox().join("lantern_final.glb"),
        fixture::clean_prop(),
    )
    .expect("model");
    let result = inbox::accept(
        &world.root,
        &world.config,
        &inbox::AcceptRequest {
            item: "lantern_final.glb".into(),
            project: world.project.clone(),
            asset: Some(world.asset.clone()),
            name: None,
            prep: None,
        },
    )
    .expect("accept");
    jobs.accepted(
        "lantern_final.glb",
        &world.project,
        &result.asset.id,
        result.version.version,
    )
}

/// The send's lines in the credits ledger file, in order.
fn ledger_lines(world: &World) -> Vec<String> {
    fs::read_to_string(world.root.join(ledger::LEDGER_FILE))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// The By hand send's spend in the ledger.
fn hand_spend(jobs: &ProviderJobs, id: &str) -> SpendOutcome {
    jobs.ledger()
        .expect("ledger")
        .spends
        .iter()
        .find(|s| s.job == id)
        .expect("spend")
        .outcome
}

fn server() -> FakeServer {
    FakeServer::start(|_, _| Reply::json(404, &json!({})))
}

fn assert_closed(job: &ProviderJob) {
    assert_eq!(job.state, JobState::Accepted, "{job:?}");
    assert_eq!(job.message.as_deref(), Some(ANOTHER_WAY));
    assert_eq!(job.version, None, "the model was not this send's");
    assert!(job.finished_at_ms.is_some());
}

#[test]
fn accepting_a_model_for_the_asset_closes_its_waiting_by_hand_send() {
    let world = World::new();
    let jobs = world.jobs(&server());
    let job = send_by_hand(&world, &jobs);
    assert_eq!(world.asset_status().1, AssetStatus::Queued);

    assert_eq!(accept_for_asset(&world, &jobs), 1);
    assert_closed(&jobs.get(&job.id).expect("job"));
    assert_eq!(world.asset_status(), (Stage::Blender, AssetStatus::Review));
    assert!(pending_handoffs(&jobs).is_empty(), "list_handoffs is empty");
    // Most likely made on the site under another name: it counts as spent.
    assert_eq!(hand_spend(&jobs, &job.id), SpendOutcome::Succeeded);
    assert_eq!(ledger_lines(&world).len(), 2, "sent, then settled");

    // Saved that way, so it stays closed after a restart.
    jobs.close();
    let again = world.jobs(&server());
    assert_closed(&again.get(&job.id).expect("job"));
    assert_eq!(world.asset_status().1, AssetStatus::Review);
}

#[test]
fn opening_the_workspace_closes_sends_whose_asset_got_a_model_after_them() {
    let world = World::new();
    let first = world.jobs(&server());
    let job = send_by_hand(&world, &first);
    first.close();
    // A model landed while the app was not watching the send.
    std::thread::sleep(Duration::from_millis(5));
    give_model(&world);
    assert_eq!(world.asset_status().1, AssetStatus::Queued);

    let again = world.jobs(&server());
    assert_closed(&again.get(&job.id).expect("job"));
    assert_eq!(world.asset_status().1, AssetStatus::Review);
    assert!(pending_handoffs(&again).is_empty());
    assert_eq!(hand_spend(&again, &job.id), SpendOutcome::Succeeded);
}

#[test]
fn a_library_model_closes_the_send_and_leaves_the_ledger_alone() {
    let world = World::new();
    let jobs = world.jobs(&server());
    let job = send_by_hand(&world, &jobs);
    let before = ledger_lines(&world);
    assert_eq!(before.len(), 1, "the By hand sent line");
    std::thread::sleep(Duration::from_millis(5));
    // What a library update or use leaves behind: a newer version, no send.
    give_model(&world);

    assert_eq!(jobs.settle_handed(&world.project), 1);
    assert_closed(&jobs.get(&job.id).expect("job"));
    assert_eq!(world.asset_status().1, AssetStatus::Review);
    assert!(pending_handoffs(&jobs).is_empty());
    assert_eq!(ledger_lines(&world), before, "nothing written");
    assert_eq!(hand_spend(&jobs, &job.id), SpendOutcome::Pending);
    assert_eq!(jobs.settle_handed(&world.project), 0, "closed once");

    // Nor does the next open count it.
    jobs.close();
    let _again = world.jobs(&server());
    assert_eq!(ledger_lines(&world), before);
}

#[test]
fn a_model_older_than_the_send_leaves_it_waiting() {
    let world = World::new();
    give_model(&world);
    std::thread::sleep(Duration::from_millis(5));
    let first = world.jobs(&server());
    let job = send_by_hand(&world, &first);
    first.close();

    let again = world.jobs(&server());
    assert_eq!(again.get(&job.id).expect("job").state, JobState::Waiting);
    assert_eq!(world.asset_status().1, AssetStatus::Queued);
    assert_eq!(again.settle_handed(&world.project), 0);
}

#[test]
fn a_running_api_send_of_the_asset_is_left_alone() {
    let world = World::new();
    let fake = TripoFake::new();
    fake.hold.store(true, Ordering::SeqCst);
    let (server, _) = tripo_server(fake);
    let jobs = world.jobs(&server);
    let api = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let end = std::time::Instant::now() + WAIT;
    while jobs.get(&api.id).and_then(|j| j.remote_id).is_none() {
        assert!(std::time::Instant::now() < end, "never sent");
        std::thread::sleep(Duration::from_millis(5));
    }
    let hand = send_by_hand(&world, &jobs);

    assert_eq!(accept_for_asset(&world, &jobs), 1, "only the By hand wait");
    assert_closed(&jobs.get(&hand.id).expect("job"));
    let running = jobs.get(&api.id).expect("job");
    assert_eq!(running.state, JobState::Running, "{running:?}");
    // The card follows the send still running.
    assert_eq!(world.asset_status().1, AssetStatus::Running);
    jobs.close();
}

#[test]
fn discarding_the_model_of_a_send_closes_it_and_leaves_the_ledger_alone() {
    let world = World::new();
    let jobs = world.jobs(&server());
    let job = send_by_hand(&world, &jobs);
    let file = world.root.join("made_on_the_site.glb");
    fs::write(&file, fixture::clean_prop()).expect("model");
    let back = jobs.hand_back(&job.id, &file).expect("hand back");
    assert_eq!(back.state, JobState::Inbox);
    let name = back.inbox_file.clone().expect("inbox file");
    let lines = ledger_lines(&world);
    assert_eq!(lines.len(), 2, "sent, then settled");

    inbox::discard(&world.root, &world.config, &name).expect("discard");
    assert_eq!(jobs.discarded(&name).as_deref(), Some(job.id.as_str()));
    let closed = jobs.get(&job.id).expect("job");
    assert_eq!(closed.state, JobState::Cancelled, "{closed:?}");
    assert_eq!(closed.inbox_file, None);
    assert_eq!(closed.message.as_deref(), Some("Discarded from the Inbox"));
    assert_eq!(ledger_lines(&world), lines, "nothing written");
    assert_eq!(hand_spend(&jobs, &job.id), SpendOutcome::Succeeded);
    assert!(pending_handoffs(&jobs).is_empty());
    assert_eq!(jobs.discarded(&name), None, "closed once");

    // A model nobody sent: nothing to close.
    assert_eq!(jobs.discarded("lantern_final.glb"), None);
}
