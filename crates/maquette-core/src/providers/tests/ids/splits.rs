//! A split and a re-texture are sends too, numbered in their project: two
//! projects' jobs of one id keep the model they went out with apart, and a
//! split on its way in one project never holds up the other.

use super::*;

/// Adds `bytes` as the first version of the world's asset in `project`,
/// through the inbox, with no send behind it.
fn first_version(world: &World, project: &str, name: &str, bytes: &[u8]) {
    fs::write(world.inbox().join(name), bytes).expect("model");
    let accepted = inbox::accept(
        &world.root,
        &world.config,
        &inbox::AcceptRequest {
            item: name.to_string(),
            project: project.to_string(),
            asset: Some(world.asset.clone()),
            name: None,
            prep: None,
        },
    )
    .expect("accept");
    assert_eq!(accepted.version.version, 1);
}

fn split_of(world: &World, project: &str) -> SplitRequest {
    SplitRequest {
        project: project.to_string(),
        asset: world.asset.clone(),
        version: None,
        item: None,
        provider: Provider::Tripo,
        tripo: None,
        meshy: None,
        after: Default::default(),
        texture_prompt: None,
    }
}

/// The copy of the model the job `id` of `project` went out with.
fn source_copy(world: &World, project: &str, id: &str) -> PathBuf {
    world
        .root
        .join(&world.config.layout.cache)
        .join("sends")
        .join(project)
        .join(id)
        .join("source.glb")
}

#[test]
fn a_split_and_a_retexture_keep_their_model_and_their_turn_per_project() {
    let world = World::new();
    let other = second_project(&world);
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    // Two different models, so a copy says whose it is.
    let (mine, theirs) = (fixture::clean_prop(), fixture::merged_islands());
    assert_ne!(mine, theirs);
    first_version(&world, &world.project, "first.glb", &mine);
    first_version(&world, &other, "second.glb", &theirs);
    let state = |project: &str, id: &str| jobs.get(project, id).expect("job").state;

    // A split waiting in the first project is no split of the second.
    let split = jobs
        .send_split(&split_of(&world, &world.project), true, true)
        .expect("split");
    assert_eq!(split.id, "stone-lantern-1");
    let free = jobs
        .split_source(&other, &world.asset, None, None)
        .expect("source");
    assert_eq!(free.busy, None, "the other project's asset is not busy");
    let busy = jobs
        .split_source(&world.project, &world.asset, None, None)
        .expect("source");
    assert_eq!(busy.busy.as_deref(), Some("stone-lantern-1"));
    let their_split = jobs
        .send_split(&split_of(&world, &other), true, true)
        .expect("the other project splits too");
    assert_eq!(their_split.id, "stone-lantern-1", "numbered in its project");
    assert_eq!(their_split.project, other);

    // Each kept the model of its own project, under the one id.
    let read = |project: &str, id: &str| fs::read(source_copy(&world, project, id)).expect("copy");
    assert_eq!(read(&world.project, "stone-lantern-1"), mine);
    assert_eq!(read(&other, "stone-lantern-1"), theirs);
    assert!(
        !world
            .root
            .join(&world.config.layout.cache)
            .join("splits")
            .exists(),
        "nothing goes under the id alone any more"
    );

    // The first project is busy with its own split, whatever the id says.
    let again = jobs.send_split(&split_of(&world, &world.project), true, true);
    assert!(
        again.is_err_and(|e| e
            .to_string()
            .contains("already being split (stone-lantern-1)")),
        "one split of an asset at a time"
    );
    let paint = RetextureRequest {
        project: world.project.clone(),
        asset: world.asset.clone(),
        version: 1,
        prompt: "weathered granite".into(),
    };
    let held = jobs.send_retexture(&paint, true, true);
    assert!(
        held.is_err_and(|e| e.to_string().contains("on its way")),
        "its own split holds the re-texture"
    );

    // Stopping the first project's split leaves the second's waiting, and
    // the second's split does not hold up the first's re-texture.
    jobs.cancel(&world.project, &split.id).expect("cancel");
    assert_eq!(
        state(&world.project, "stone-lantern-1"),
        JobState::Cancelled
    );
    assert_eq!(state(&other, "stone-lantern-1"), JobState::Approval);
    let painted = jobs.send_retexture(&paint, true, true).expect("re-texture");
    assert_eq!(painted.id, "stone-lantern-2");
    assert_eq!(read(&world.project, "stone-lantern-2"), mine);
    assert_eq!(
        read(&other, "stone-lantern-1"),
        theirs,
        "still its own model"
    );

    jobs.cancel(&other, &their_split.id).expect("cancel");
    assert_eq!(state(&other, "stone-lantern-1"), JobState::Cancelled);
    assert_eq!(state(&world.project, "stone-lantern-2"), JobState::Approval);
    assert_eq!(server.requests().len(), 0, "nothing approved, nothing sent");
}
