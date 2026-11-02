//! After the jobs close (app exit, workspace switch), a send that still
//! arrives writes nothing over the saved jobs.

use super::*;

fn saved_ids(world: &World) -> Vec<String> {
    let path = project::dir(&world.root, &world.config.layout, &world.project)
        .expect("dir")
        .join(jobs::JOBS_FILE);
    let file: Value = serde_json::from_str(&fs::read_to_string(path).expect("read")).expect("json");
    file["jobs"]
        .as_array()
        .expect("jobs")
        .iter()
        .map(|s| s["job"]["id"].as_str().expect("id").to_string())
        .collect()
}

#[test]
fn a_send_after_close_keeps_the_saved_jobs() {
    let world = World::new();
    let (server, _) = meshy_server(false);
    let jobs = world.jobs(&server);
    let first = jobs
        .send(
            &world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("first send");
    assert_eq!(saved_ids(&world), std::slice::from_ref(&first.id));

    jobs.close();
    let late = jobs.send(
        &world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]),
        false,
        false,
    );

    assert_eq!(saved_ids(&world), [first.id], "the saved send is kept");
    let err = late.expect_err("a send after close is refused");
    assert!(err.to_string().contains("Open a workspace"), "{err}");
}
