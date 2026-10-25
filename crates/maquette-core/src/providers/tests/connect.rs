//! A create that cannot connect: tried again while nothing was sent, and
//! failed plainly (with the real cause) when the network stays away.

use std::net::TcpListener;
use std::time::Duration;

use super::*;

/// A local address nothing listens on (yet).
fn closed_address() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    drop(listener);
    addr
}

fn saved_chain(world: &World, id: &str) -> Value {
    let path = world.project_dir().join(jobs::JOBS_FILE);
    let file: Value = serde_json::from_str(&fs::read_to_string(path).expect("read")).expect("json");
    file["jobs"]
        .as_array()
        .and_then(|jobs| jobs.iter().find(|s| s["job"]["id"] == id))
        .map(|s| s["chain"].clone())
        .expect("saved job")
}

#[test]
fn a_create_that_cannot_connect_fails_saying_why_and_that_nothing_was_sent() {
    let world = World::new();
    let url = format!("http://{}", closed_address());
    let jobs = world.jobs_at(&url, Timing::instant());
    let job = jobs
        .send(
            &world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let failed = finished(&jobs, &job.id);
    assert_eq!(failed.state, JobState::Failed);
    let message = failed.message.unwrap_or_default();
    assert!(message.contains("tcp connect error"), "{message}");
    assert!(message.contains("nothing was sent"), "{message}");
    assert!(!message.contains("127.0.0.1"), "no address in {message}");
    assert_eq!(
        saved_chain(&world, &job.id)["sending"],
        false,
        "a create that never left is not in flight"
    );
    assert!(
        settled(&jobs, 0).spends.is_empty(),
        "nothing reached the ledger"
    );
}

#[test]
fn a_create_goes_out_once_the_connection_comes_back() {
    let world = World::new();
    let addr = closed_address();
    let timing = Timing {
        connect_retries: 6,
        connect_backoff: Duration::from_millis(20),
        ..Timing::instant()
    };
    let jobs = world.jobs_at(&format!("http://{addr}"), timing);
    let job = jobs
        .send(
            &world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    // The network comes back after the first failed connects.
    std::thread::sleep(Duration::from_millis(60));
    let fake = Arc::new(MeshyFake {
        seen: Mutex::default(),
        hold_pending: AtomicBool::new(false),
    });
    let handler = Arc::clone(&fake);
    let server = FakeServer::start_at(&addr, move |base, req| handler.handle(base, req));
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    assert_eq!(server.count("POST", "/image-to-3d"), 1, "created once");
}

#[test]
fn an_answer_that_never_came_keeps_the_send_in_flight_and_says_to_check() {
    let world = World::new();
    // Takes the create, then answers with something that is not JSON: the
    // task may exist, so the job must not look like it never left.
    let server = FakeServer::start(|_, _| Reply::bytes(200, b"<html>".to_vec()));
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let failed = finished(&jobs, &job.id);
    assert_eq!(failed.state, JobState::Failed);
    let message = failed.message.unwrap_or_default();
    assert!(message.contains("do not pay twice"), "{message}");
    assert_eq!(saved_chain(&world, &job.id)["sending"], true);
    assert_eq!(server.count("POST", "/image-to-3d"), 1);
}
