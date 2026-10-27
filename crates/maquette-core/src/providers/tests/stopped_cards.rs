//! The status a card shows after a send of it is stopped, and the heal
//! for cards an older stop left on Queued.

use super::*;

fn send_by_hand(world: &World, jobs: &ProviderJobs) -> ProviderJob {
    let request = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    jobs.send(&request, false, false).expect("send")
}

fn force_status(world: &World, status: AssetStatus) {
    project::modify(&world.root, &world.config.layout, &world.project, |m| {
        assets::set_status(m, &world.asset, status)
    })
    .expect("status");
}

fn give_model(world: &World) {
    project::modify(&world.root, &world.config.layout, &world.project, |m| {
        assets::add_version(m, &world.asset, "models/lantern_v1.glb".into(), None).map(|_| ())
    })
    .expect("version");
}

fn server() -> FakeServer {
    FakeServer::start(|_, _| Reply::json(404, &json!({})))
}

#[test]
fn stopping_a_send_of_an_asset_without_a_model_leaves_it_queued() {
    let world = World::new();
    let jobs = world.jobs(&server());
    let job = send_by_hand(&world, &jobs);
    jobs.cancel(&job.id).expect("cancel");
    assert_eq!(world.asset_status().1, AssetStatus::Queued);
}

#[test]
fn stopping_a_send_of_an_asset_with_a_model_puts_it_back_in_review() {
    let world = World::new();
    give_model(&world);
    let jobs = world.jobs(&server());
    let job = send_by_hand(&world, &jobs);
    assert_eq!(world.asset_status().1, AssetStatus::Queued);
    let stopped = jobs.cancel(&job.id).expect("cancel");
    assert_eq!(stopped.state, JobState::Cancelled);
    assert_eq!(world.asset_status().1, AssetStatus::Review);
}

#[test]
fn stopping_one_send_keeps_the_status_of_another_live_one() {
    let world = World::new();
    give_model(&world);
    let jobs = world.jobs(&server());
    let first = send_by_hand(&world, &jobs);
    let second = send_by_hand(&world, &jobs);
    jobs.cancel(&first.id).expect("cancel");
    assert_eq!(
        world.asset_status().1,
        AssetStatus::Queued,
        "the second send still waits"
    );
    jobs.cancel(&second.id).expect("cancel");
    assert_eq!(world.asset_status().1, AssetStatus::Review);
}

#[test]
fn opening_the_workspace_fixes_cards_a_stop_left_on_queued() {
    let world = World::new();
    give_model(&world);
    force_status(&world, AssetStatus::Queued);
    let jobs = world.jobs(&server());
    assert_eq!(world.asset_status().1, AssetStatus::Review);
    jobs.close();
    // A stuck Running card without a model goes back to Queued.
    let bare = World::new();
    force_status(&bare, AssetStatus::Running);
    let _jobs = bare.jobs(&server());
    assert_eq!(bare.asset_status().1, AssetStatus::Queued);
}

#[test]
fn the_heal_never_touches_a_card_with_a_live_job() {
    let world = World::new();
    give_model(&world);
    let first = world.jobs(&server());
    send_by_hand(&world, &first);
    first.close();
    force_status(&world, AssetStatus::Queued);
    let again = world.jobs(&server());
    assert_eq!(again.list(None).len(), 1);
    assert_eq!(world.asset_status().1, AssetStatus::Queued);
}
