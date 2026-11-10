//! Job ids are numbered per project: two projects sending an asset of the
//! same id both get `<asset>-1`, with export names of their own, and no
//! lookup crosses from one project's job to the other's: approve, deny,
//! cancel, the queue, the saved files, the credits ledger and a redo.

use super::*;

mod inbox_links;
mod older;
mod splits;

/// A second project with an asset of the same id as the world's.
pub(super) fn second_project(world: &World) -> String {
    let manifest = project::create(
        &world.root,
        &world.config.layout,
        NewProject {
            name: "Lantern Bay".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    project::modify(&world.root, &world.config.layout, &manifest.id, |m| {
        let asset = assets::add(
            m,
            NewAsset {
                name: "Stone lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )?;
        assert_eq!(asset.id, world.asset, "the same asset id in both");
        Ok(())
    })
    .expect("asset");
    let refs = project::dir(&world.root, &world.config.layout, &manifest.id)
        .expect("dir")
        .join("refs");
    fs::create_dir_all(&refs).expect("refs");
    fs::write(refs.join("front.png"), b"\x89PNG\r\n\x1a\nfake").expect("image");
    manifest.id
}

/// The world's send, in `project`, to `provider`.
pub(super) fn request_in(world: &World, project: &str, provider: Provider) -> SendRequest {
    SendRequest {
        project: project.to_string(),
        ..world.request(provider, PresetVariant::Prop, &["front.png"])
    }
}

pub(super) fn project_dir(world: &World, project: &str) -> PathBuf {
    project::dir(&world.root, &world.config.layout, project).expect("dir")
}

/// The ids in a project's saved jobs file.
pub(super) fn saved_ids(world: &World, project: &str) -> Vec<String> {
    let text =
        fs::read_to_string(project_dir(world, project).join(jobs::JOBS_FILE)).expect("jobs file");
    let file: Value = serde_json::from_str(&text).expect("json");
    file["jobs"]
        .as_array()
        .expect("jobs")
        .iter()
        .map(|s| s["job"]["id"].as_str().expect("id").to_string())
        .collect()
}

#[test]
fn two_projects_sending_the_same_asset_both_get_the_first_number() {
    let world = World::new();
    let other = second_project(&world);
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);

    let mine = jobs
        .send(
            &request_in(&world, &world.project, Provider::ByHand),
            false,
            false,
        )
        .expect("first project");
    let theirs = jobs
        .send(&request_in(&world, &other, Provider::ByHand), false, false)
        .expect("second project");

    assert_eq!(mine.id, "stone-lantern-1");
    assert_eq!(theirs.id, "stone-lantern-1", "numbered in its own project");
    assert_eq!(mine.project, world.project);
    assert_eq!(theirs.project, other);

    // Each waits for a name of its own, with its project in it.
    let name = |job: &ProviderJob| job.card.as_ref().expect("card").export_name.clone();
    assert_eq!(name(&mine), "maq_moon-island_stone-lantern_v1_tripo");
    assert_eq!(name(&theirs), "maq_lantern-bay_stone-lantern_v1_tripo");
    assert_eq!(mine.export_name(), Some(name(&mine)));

    // Each has its own handoff folder, and the id finds it only with its project.
    for job in [&mine, &theirs] {
        let folder = PathBuf::from(job.handoff.clone().expect("folder"));
        assert_eq!(
            folder,
            project_dir(&world, &job.project)
                .join("handoff")
                .join("stone-lantern-1")
        );
        let sidecar: Value =
            serde_json::from_str(&fs::read_to_string(folder.join("send.json")).expect("sidecar"))
                .expect("json");
        assert_eq!(sidecar["project"], json!(job.project));
        assert_eq!(sidecar["version"], json!(1));
        assert_eq!(sidecar["expectedName"], json!(name(job)));
        let found = jobs.get(&job.project, &job.id).expect("by project and id");
        assert_eq!(found.handoff, job.handoff);
    }
    assert!(jobs.get("no-such-project", "stone-lantern-1").is_none());
    assert_eq!(jobs.with_id("stone-lantern-1").len(), 2);

    // The second send of each is its number two.
    let again = jobs
        .send(&request_in(&world, &other, Provider::ByHand), false, false)
        .expect("again");
    assert_eq!(again.id, "stone-lantern-2");
    assert_eq!(name(&again), "maq_lantern-bay_stone-lantern_v2_tripo");
    assert_eq!(saved_ids(&world, &world.project), ["stone-lantern-1"]);
    assert_eq!(
        saved_ids(&world, &other),
        ["stone-lantern-1", "stone-lantern-2"]
    );
}

#[test]
fn a_project_numbers_on_from_what_it_holds_whatever_other_projects_sent() {
    let world = World::new();
    let other = second_project(&world);
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let send = |project: &str| {
        jobs.send(&request_in(&world, project, Provider::ByHand), false, false)
            .expect("send")
    };
    let ids: Vec<String> = (0..3).map(|_| send(&world.project).id).collect();
    assert_eq!(
        ids,
        ["stone-lantern-1", "stone-lantern-2", "stone-lantern-3"]
    );
    assert_eq!(send(&other).id, "stone-lantern-1");

    // Stopped and cleared from the queue, their handoff folders still hold
    // the numbers: the next send is the fourth, not a second number one.
    for id in &ids {
        jobs.cancel(&world.project, id).expect("stop waiting");
    }
    jobs.clear_finished(&world.project).expect("clear");
    assert!(jobs.list(Some(&world.project)).is_empty());
    assert_eq!(send(&world.project).id, "stone-lantern-4");
    assert_eq!(
        send(&other).id,
        "stone-lantern-2",
        "the other project is its own count"
    );
}

#[test]
fn approve_deny_and_cancel_reach_only_the_job_of_their_project() {
    let world = World::new();
    let other = second_project(&world);
    let (server, _) = meshy_server(false);
    let jobs = world.jobs(&server);
    let ask = |project: &str| {
        jobs.send(&request_in(&world, project, Provider::Meshy), true, true)
            .expect("send")
    };
    let mine = ask(&world.project);
    let theirs = ask(&other);
    assert_eq!(mine.id, theirs.id, "the same id in both projects");
    assert_eq!(theirs.project, other, "the send answers with its own job");
    assert_eq!(
        (mine.state, theirs.state),
        (JobState::Approval, JobState::Approval)
    );
    let state = |job: &ProviderJob| jobs.get(&job.project, &job.id).expect("job").state;

    // Approving the second project's send releases that one alone.
    let approved = jobs.approve(&other, &theirs.id).expect("approve");
    assert_eq!(approved.project, other);
    assert_eq!(finished(&jobs, &theirs).state, JobState::Inbox);
    assert_eq!(
        state(&mine),
        JobState::Approval,
        "still waiting for its own yes"
    );
    assert_eq!(server.count("POST", "/image-to-3d"), 1, "one send went out");

    // Denying the first project's send leaves the second one's model alone.
    let denied = jobs.deny(&world.project, &mine.id).expect("deny");
    assert_eq!(denied.project, world.project);
    assert_eq!(denied.state, JobState::Cancelled);
    assert!(denied.denied);
    assert_eq!(state(&theirs), JobState::Inbox);
    assert!(
        jobs.deny(&other, &theirs.id).is_err(),
        "the other project's job is not waiting for approval"
    );
    assert_eq!(
        server.count("POST", "/image-to-3d"),
        1,
        "nothing else went out"
    );

    // Cancel: two sends waiting for a by hand model under one id.
    let wait = |project: &str| {
        jobs.send(&request_in(&world, project, Provider::ByHand), false, false)
            .expect("by hand")
    };
    let (mine, theirs) = (wait(&world.project), wait(&other));
    assert_eq!(mine.id, "stone-lantern-2");
    assert_eq!(theirs.id, "stone-lantern-2");
    let stopped = jobs.cancel(&world.project, &mine.id).expect("cancel");
    assert_eq!(stopped.project, world.project);
    assert_eq!(state(&mine), JobState::Cancelled);
    assert_eq!(
        state(&theirs),
        JobState::Waiting,
        "the other project still waits"
    );

    // A project that has no such job says so, and changes nothing.
    let err = jobs
        .cancel("no-such-project", &theirs.id)
        .expect_err("unknown");
    assert!(
        err.to_string()
            .contains("stone-lantern-2 in no-such-project"),
        "{err}"
    );
    assert_eq!(state(&theirs), JobState::Waiting);
}

#[test]
fn each_projects_job_runs_saves_and_spends_on_its_own() {
    let world = World::new();
    let other = second_project(&world);
    let (server, _) = meshy_server(false);
    let jobs = world.jobs(&server);
    // Both in the queue at once, under the same id.
    let send = |project: &str| {
        jobs.send(&request_in(&world, project, Provider::Meshy), false, false)
            .expect("send")
    };
    let (mine, theirs) = (send(&world.project), send(&other));
    assert_eq!(mine.id, theirs.id);
    let (mine, theirs) = (finished(&jobs, &mine), finished(&jobs, &theirs));
    for job in [&mine, &theirs] {
        assert_eq!(job.state, JobState::Inbox, "{job:?}");
        assert_eq!(job.credits, Some(20.0), "its own credits, counted once");
    }
    assert_eq!(server.count("POST", "/image-to-3d"), 2);

    // Two models in the one inbox, each saved by its own job.
    let files = [
        mine.inbox_file.clone().expect("file"),
        theirs.inbox_file.clone().expect("file"),
    ];
    assert_ne!(files[0], files[1], "one never overwrites the other");
    for file in &files {
        assert!(world.inbox().join(file).is_file(), "{file}");
    }

    // Each project's file holds its own job, with its own model.
    for job in [&mine, &theirs] {
        assert_eq!(saved_ids(&world, &job.project), ["stone-lantern-1"]);
        let saved = fs::read_to_string(project_dir(&world, &job.project).join(jobs::JOBS_FILE))
            .expect("saved");
        assert!(
            saved.contains(job.inbox_file.as_deref().expect("file")),
            "{saved}"
        );
        assert!(saved.contains("\"scope\": \"project\""), "{saved}");
    }

    // One ledger entry per send, told apart by the project.
    let ledger = settled(&jobs, 2);
    let mut keys: Vec<(String, String)> = ledger
        .spends
        .iter()
        .map(|s| (s.project.clone(), s.job.clone()))
        .collect();
    keys.sort();
    let mut want = vec![
        (world.project.clone(), mine.id.clone()),
        (other.clone(), theirs.id.clone()),
    ];
    want.sort();
    assert_eq!(keys, want, "{ledger:?}");
    assert!(
        ledger
            .spends
            .iter()
            .all(|s| s.outcome == SpendOutcome::Succeeded && s.credits == 20.0),
        "{ledger:?}"
    );
}

#[test]
fn a_cancelled_send_refunds_in_its_own_project_only() {
    let world = World::new();
    let other = second_project(&world);
    // Meshy holds every task pending, so both can still be called off.
    let (server, _) = meshy_server(true);
    let jobs = world.jobs(&server);
    let send = |project: &str| {
        let job = jobs
            .send(&request_in(&world, project, Provider::Meshy), false, false)
            .expect("send");
        let end = std::time::Instant::now() + WAIT;
        while jobs
            .get(project, &job.id)
            .and_then(|j| j.remote_id)
            .is_none()
        {
            assert!(std::time::Instant::now() < end, "never sent");
            std::thread::sleep(Duration::from_millis(5));
        }
        job
    };
    let (mine, theirs) = (send(&world.project), send(&other));
    assert_eq!(mine.id, theirs.id);

    jobs.cancel(&other, &theirs.id).expect("cancel");
    let ledger = settled(&jobs, 1);
    let outcome = |project: &str| {
        ledger
            .spends
            .iter()
            .find(|s| s.project == project)
            .unwrap_or_else(|| panic!("no spend of {project}: {ledger:?}"))
            .outcome
    };
    assert_eq!(outcome(&other), SpendOutcome::Refunded);
    assert_eq!(outcome(&world.project), SpendOutcome::Pending);
    assert_eq!(
        jobs.get(&world.project, &mine.id).expect("job").state,
        JobState::Running,
        "the first project's send goes on"
    );
    jobs.close();
}

#[test]
fn a_redo_names_the_send_of_its_own_project() {
    let world = World::new();
    let other = second_project(&world);
    let (server, _) = meshy_server(false);
    let jobs = world.jobs(&server);
    let first = jobs
        .send(
            &request_in(&world, &world.project, Provider::Meshy),
            false,
            false,
        )
        .expect("send");
    assert_eq!(finished(&jobs, &first).state, JobState::Inbox);

    // The other project has no send of that id to redo, whatever this one has.
    let mut redo = request_in(&world, &other, Provider::Meshy);
    redo.redo_of = Some(first.id.clone());
    let err = jobs.send(&redo, false, false).expect_err("not its send");
    assert!(err.to_string().contains("no send stone-lantern-1"), "{err}");

    // Once it has one, the redo is its own second send and points at it.
    redo.redo_of = None;
    let theirs = jobs.send(&redo, false, false).expect("their first");
    assert_eq!(finished(&jobs, &theirs).state, JobState::Inbox);
    redo.redo_of = Some(theirs.id.clone());
    let again = jobs.send(&redo, false, false).expect("redo");
    assert_eq!(again.project, other);
    assert_eq!(again.id, "stone-lantern-2");
    assert_eq!(again.redo_of.as_deref(), Some("stone-lantern-1"));
    assert_eq!(finished(&jobs, &again).state, JobState::Inbox);

    let ledger = settled(&jobs, 3);
    let redos: Vec<(&str, &str)> = ledger
        .spends
        .iter()
        .filter(|s| s.redo_of.is_some())
        .map(|s| (s.project.as_str(), s.job.as_str()))
        .collect();
    assert_eq!(redos, [(other.as_str(), "stone-lantern-2")], "{ledger:?}");
}

#[test]
fn the_lane_takes_two_projects_jobs_of_one_id_in_the_order_they_were_sent() {
    let world = World::new();
    let other = second_project(&world);
    let fake = TripoFake::new();
    fake.hold.store(true, Ordering::SeqCst);
    let hold = Arc::clone(&fake.hold);
    let (server, _) = tripo_server(fake);
    // One job at a time, so the second send has to wait its turn.
    let jobs = ProviderJobs::new(
        Arc::clone(&world.secrets) as Arc<dyn SecretStore>,
        Options {
            endpoints: Endpoints {
                tripo: server.url.clone(),
                meshy: server.url.clone(),
            },
            tripo: Timing::instant(),
            meshy: Timing::instant(),
            limit: 1,
        },
        inbox::send_ports(),
        |_| {},
    );
    jobs.open_workspace(&world.root, &world.config);
    let send = |project: &str| {
        jobs.send(&request_in(&world, project, Provider::Tripo), false, false)
            .expect("send")
    };
    let read = |job: &ProviderJob| jobs.get(&job.project, &job.id).expect("job");

    let first = send(&world.project);
    let second = send(&other);
    assert_eq!(first.id, second.id, "one id, two jobs in the queue");
    let end = std::time::Instant::now() + WAIT;
    while read(&first).remote_id.is_none() {
        assert!(std::time::Instant::now() < end, "the first never started");
        std::thread::sleep(Duration::from_millis(5));
    }
    // The first runs and holds the lane; the second is still queued, not
    // taken for the running one because it has the same id.
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(read(&first).state, JobState::Running);
    let waiting = read(&second);
    assert_eq!(waiting.state, JobState::Queued);
    assert_eq!((waiting.remote_id, waiting.started_at_ms), (None, None));
    assert_eq!(server.count("POST", "/generation/image-to-model"), 1);

    hold.store(false, Ordering::SeqCst);
    let (first, second) = (finished(&jobs, &first), finished(&jobs, &second));
    assert_eq!(first.state, JobState::Inbox, "{first:?}");
    assert_eq!(second.state, JobState::Inbox, "{second:?}");
    assert_eq!(
        server.count("POST", "/generation/image-to-model"),
        2,
        "each was sent once"
    );
    assert!(
        first.started_at_ms < second.started_at_ms,
        "in the order they were sent"
    );
    assert_ne!(first.inbox_file, second.inbox_file);
}
