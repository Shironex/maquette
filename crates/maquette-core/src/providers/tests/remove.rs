//! Removing an asset with real provider jobs: a send waiting for a model
//! is stopped the way the 3D queue stops it, and a paid send being made
//! refuses the remove until it is over.

use super::*;
use crate::assets::remove::{Busy, RemoveState, remove_assets};

fn remove(world: &World, jobs: &ProviderJobs) -> crate::assets::remove::AssetsRemoved {
    remove_assets(
        &world.root,
        &world.config.layout,
        &world.project,
        std::slice::from_ref(&world.asset),
        &Busy::default(),
        jobs,
    )
    .expect("remove")
}

#[test]
fn removing_an_asset_stops_its_waiting_by_hand_send() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let mut request = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    request.service = ProviderId::Meshy;
    let waiting = jobs.send(&request, true, false).expect("send");
    assert_eq!(waiting.state, JobState::Waiting);
    let handoff = world.project_dir().join("handoff").join(&waiting.id);
    assert!(handoff.is_dir());

    let done = remove(&world, &jobs);
    let out = &done.outcomes[0];
    assert_eq!(out.state, RemoveState::Removed, "{out:?}");
    assert_eq!(out.stopped, std::slice::from_ref(&waiting.id));
    let stopped = jobs.get(&waiting.id).expect("job");
    assert_eq!(stopped.state, JobState::Cancelled);
    assert_eq!(
        stopped.message.as_deref(),
        Some("Stopped waiting for this model.")
    );
    assert!(!handoff.exists());
    let trash = world
        .root
        .join("trash")
        .join(&world.project)
        .join(out.trash.as_deref().expect("trash"))
        .join("files");
    assert!(
        trash
            .join("handoff")
            .join(&waiting.id)
            .join("front.png")
            .is_file()
    );
    assert!(
        trash.join("refs/front.png").is_file(),
        "its sent ref goes too"
    );
    assert!(world.project_dir().join("refs/left.png").is_file());
    assert!(done.project.assets.is_empty());
    assert_eq!(server.count("POST", "/task"), 0, "nothing is sent anywhere");
}

#[test]
fn a_paid_send_being_made_refuses_the_remove() {
    let world = World::new();
    let (server, _fake) = meshy_server(true);
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let end = std::time::Instant::now() + WAIT;
    while jobs
        .get(&job.id)
        .is_none_or(|j| j.state != JobState::Running)
    {
        assert!(std::time::Instant::now() < end, "never running");
        std::thread::sleep(Duration::from_millis(5));
    }
    let done = remove(&world, &jobs);
    let out = &done.outcomes[0];
    assert_eq!(out.state, RemoveState::Refused);
    assert!(
        out.reason
            .as_deref()
            .is_some_and(|r| r.contains("Meshy is making the model of Stone lantern now")),
        "{out:?}"
    );
    assert_eq!(jobs.get(&job.id).expect("job").state, JobState::Running);
    assert_eq!(done.project.assets.len(), 1);
    assert!(world.project_dir().join("refs/front.png").is_file());
}
