//! The inbox is one folder for the whole workspace, and two projects may
//! each have a job `<asset>-1`: a model there links to the job of its own
//! project, through accept and discard, also when the inbox gives a name
//! out a second time, and a model saved by hand links only when one send
//! can be meant.

use super::*;

/// Accepts the inbox model `item` as the next version of the world's asset
/// in `project`, as the app does.
fn accept_in(world: &World, jobs: &ProviderJobs, project: &str, item: &str) -> u32 {
    let accepted = inbox::accept(
        &world.root,
        &world.config,
        &inbox::AcceptRequest {
            item: item.to_string(),
            project: project.to_string(),
            asset: Some(world.asset.clone()),
            name: None,
            prep: None,
        },
    )
    .expect("accept");
    jobs.accepted(item, project, &world.asset, accepted.version.version);
    accepted.version.version
}

/// The project each inbox model is linked to, by its file name.
fn links(world: &World, jobs: &ProviderJobs) -> Vec<(String, Option<String>)> {
    let mut items = inbox::list(&world.root, &world.config).expect("inbox");
    jobs.link_inbox(&mut items);
    let mut found: Vec<(String, Option<String>)> = items
        .into_iter()
        .map(|i| (i.id, i.link.map(|l| l.project)))
        .collect();
    found.sort();
    found
}

fn job(jobs: &ProviderJobs, sent: &ProviderJob) -> ProviderJob {
    jobs.get(&sent.project, &sent.id).expect("job")
}

#[test]
fn an_inbox_model_links_to_the_job_of_its_own_project_through_accept_and_discard() {
    let world = World::new();
    let other = second_project(&world);
    let (server, _) = meshy_server(false);
    let jobs = world.jobs(&server);
    let send = |project: &str| {
        let sent = jobs
            .send(&request_in(&world, project, Provider::Meshy), false, false)
            .expect("send");
        finished(&jobs, &sent)
    };
    let (mine, theirs) = (send(&world.project), send(&other));
    assert_eq!(mine.id, theirs.id);
    let (my_file, their_file) = (
        mine.inbox_file.clone().expect("file"),
        theirs.inbox_file.clone().expect("file"),
    );

    let mut want = vec![
        (my_file.clone(), Some(world.project.clone())),
        (their_file.clone(), Some(other.clone())),
    ];
    want.sort();
    assert_eq!(links(&world, &jobs), want);

    // Accepting the first project's model closes its job, not the other's.
    assert_eq!(accept_in(&world, &jobs, &world.project, &my_file), 1);
    let (done, waiting) = (job(&jobs, &mine), job(&jobs, &theirs));
    assert_eq!((done.state, done.version), (JobState::Accepted, Some(1)));
    assert_eq!((waiting.state, waiting.version), (JobState::Inbox, None));

    // Discarding the second project's model closes that one alone.
    inbox::discard(&world.root, &world.config, &their_file).expect("discard");
    assert_eq!(jobs.discarded(&their_file), Some(theirs.id.clone()));
    let (done, gone) = (job(&jobs, &mine), job(&jobs, &theirs));
    assert_eq!((done.state, done.version), (JobState::Accepted, Some(1)));
    assert_eq!(gone.state, JobState::Cancelled);
    assert_eq!(gone.message.as_deref(), Some("Discarded from the Inbox"));
    assert_eq!(jobs.discarded(&their_file), None, "closed once");
}

#[test]
fn a_name_the_inbox_gives_out_again_belongs_to_the_job_whose_model_it_is() {
    let world = World::new();
    let other = second_project(&world);
    let (server, _) = meshy_server(false);
    let jobs = world.jobs(&server);
    let send = |project: &str| {
        let sent = jobs
            .send(&request_in(&world, project, Provider::Meshy), false, false)
            .expect("send");
        finished(&jobs, &sent)
    };

    // The first project's model is accepted, which takes it out of the inbox.
    let mine = send(&world.project);
    let name = mine.inbox_file.clone().expect("file");
    assert_eq!(name, "stone-lantern-1.glb");
    accept_in(&world, &jobs, &world.project, &name);
    assert!(!world.inbox().join(&name).exists());

    // The second project's first send lands under the same, free name,
    // which the first project's accepted job still remembers as its own.
    let theirs = send(&other);
    assert_eq!(theirs.inbox_file.as_deref(), Some(name.as_str()));
    assert_eq!(job(&jobs, &mine).inbox_file.as_deref(), Some(name.as_str()));
    assert_eq!(links(&world, &jobs), [(name.clone(), Some(other.clone()))]);

    // A split of that inbox model is the second project's to ask for.
    let source = jobs
        .split_source(&other, &world.asset, None, Some(&name))
        .expect("the model its own send made");
    assert_eq!(source.made_by, Some(Provider::Meshy));
    let wrong = jobs.split_source(&world.project, &world.asset, None, Some(&name));
    assert!(
        wrong.is_err_and(|e| e.to_string().contains("sent for another asset")),
        "not the first project's model"
    );

    // Accepting it records the version on the second project's job only.
    let before = job(&jobs, &mine);
    assert_eq!(accept_in(&world, &jobs, &other, &name), 1);
    let done = job(&jobs, &theirs);
    assert_eq!((done.state, done.version), (JobState::Accepted, Some(1)));
    assert_eq!(
        job(&jobs, &mine),
        before,
        "the first project's job is untouched"
    );

    // With both accepted, the bare name is no one job's: nothing is guessed.
    assert_eq!(jobs.discarded(&name), None);
    assert_eq!(job(&jobs, &mine).state, JobState::Accepted);
    assert_eq!(job(&jobs, &theirs).state, JobState::Accepted);
}

#[test]
fn a_model_saved_into_the_inbox_by_hand_links_only_when_one_send_can_be_meant() {
    let world = World::new();
    let other = second_project(&world);
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let wait = |project: &str| {
        jobs.send(&request_in(&world, project, Provider::ByHand), false, false)
            .expect("by hand")
    };
    let (mine, theirs) = (wait(&world.project), wait(&other));
    assert_eq!(mine.id, theirs.id);

    // Saved under the job id, which both projects' sends have: no link, and
    // neither send is taken as arrived.
    let bare = "stone-lantern-1.glb".to_string();
    fs::write(world.inbox().join(&bare), glb()).expect("saved by hand");
    assert_eq!(links(&world, &jobs), [(bare.clone(), None)]);
    assert_eq!(job(&jobs, &mine).state, JobState::Waiting);
    assert_eq!(job(&jobs, &theirs).state, JobState::Waiting);

    // Saved under the export name, which carries the project: its send.
    let named = "maq_lantern-bay_stone-lantern_v1_tripo.glb".to_string();
    fs::write(world.inbox().join(&named), fixture::character(false)).expect("saved by hand");
    let found = links(&world, &jobs);
    assert!(
        found.contains(&(named.clone(), Some(other.clone()))),
        "{found:?}"
    );
    let arrived = job(&jobs, &theirs);
    assert_eq!(arrived.state, JobState::Inbox);
    assert_eq!(arrived.inbox_file.as_deref(), Some(named.as_str()));

    // Now only one send still waits under that id, so the bare name is its.
    let found = links(&world, &jobs);
    assert!(
        found.contains(&(bare.clone(), Some(world.project.clone()))),
        "{found:?}"
    );
    let arrived = job(&jobs, &mine);
    assert_eq!(arrived.state, JobState::Inbox);
    assert_eq!(arrived.inbox_file.as_deref(), Some(bare.as_str()));
    assert_eq!(
        job(&jobs, &theirs).inbox_file.as_deref(),
        Some(named.as_str()),
        "the other project's model stays its own"
    );
}
