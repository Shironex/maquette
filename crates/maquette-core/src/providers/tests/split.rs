//! Splits into parts against fake Tripo and Meshy servers: the request
//! bodies, by task or by upload, Meshy's model rules, the approval, the
//! ledger, a refund, a resume, and the bake back once the split lands.

use super::*;
use crate::pipeline::tools::Toolbox;
use crate::providers::split::{
    CompletionMode, Granularity, MeshySplit, MeshySplitMode, SegmentModel, TripoSplit,
};

/// Both providers in one double. Generations return the textured crate;
/// splits return two untextured crates, the way a split loses the texture.
#[derive(Default)]
struct SplitFake {
    hold: AtomicBool,
    fail: AtomicBool,
}

impl SplitFake {
    fn handle(&self, base: &str, req: &Request) -> Reply {
        let path = req.path.as_str();
        match (req.method.as_str(), path) {
            ("POST", "/files") => tripo_data(json!({ "file_token": "file_model" })),
            ("POST", "/generation/image-to-model") => {
                tripo_data(json!({ "task_id": "task_model" }))
            }
            ("POST", "/mesh/segment") => tripo_data(json!({ "task_id": "task_seg" })),
            ("POST", "/mesh/complete") => tripo_data(json!({ "task_id": "task_done" })),
            ("GET", p) if p.starts_with("/tasks/") => {
                let id = &p["/tasks/".len()..];
                if self.hold.load(Ordering::SeqCst) && id != "task_model" {
                    return tripo_data(
                        json!({ "task_id": id, "status": "running", "progress": 10 }),
                    );
                }
                let credits = match id {
                    "task_model" => 30.0,
                    "task_seg" => 40.0,
                    _ => 100.0,
                };
                tripo_data(json!({
                    "task_id": id, "status": "success", "progress": 100,
                    "output": { "model_url": format!("{base}/dl/{id}.glb") },
                    "credits_consumed": credits,
                }))
            }
            ("POST", "/image-to-3d") => Reply::ok(&json!({ "result": "m-model" })),
            ("POST", "/print/split") => Reply::ok(&json!({ "result": "m-split" })),
            ("DELETE", "/print/split/m-split") => Reply::ok(&json!({})),
            ("GET", "/image-to-3d/m-model") => Reply::ok(&json!({
                "id": "m-model", "status": "SUCCEEDED", "progress": 100, "consumed_credits": 20,
                "model_urls": { "glb": format!("{base}/dl/m-model.glb") },
            })),
            ("GET", "/print/split/m-split") => {
                let (status, credits) = if self.hold.load(Ordering::SeqCst) {
                    ("PENDING", 10)
                } else if self.fail.load(Ordering::SeqCst) {
                    ("FAILED", 0)
                } else {
                    ("SUCCEEDED", 10)
                };
                Reply::ok(&json!({
                    "id": "m-split", "type": "print-split", "status": status, "progress": 100,
                    "part_count": 2, "consumed_credits": credits,
                    "task_error": { "message": if status == "FAILED" { "The model could not be cut." } else { "" } },
                    "model_urls": { "glb": format!("{base}/dl/m-split.glb") },
                }))
                .with_header("Retry-After", "0.01")
            }
            ("GET", "/dl/task_model.glb" | "/dl/m-model.glb") => Reply::bytes(200, glb()),
            ("GET", p) if p.starts_with("/dl/") => Reply::bytes(200, fixture::merged_islands()),
            _ => Reply::json(404, &json!({ "message": "no route" })),
        }
    }
}

fn split_server() -> (FakeServer, Arc<SplitFake>) {
    let fake = Arc::new(SplitFake::default());
    let handler = Arc::clone(&fake);
    (
        FakeServer::start(move |base, req| handler.handle(base, req)),
        fake,
    )
}

/// Accepts an inbox model as the next version of the world's asset.
fn accept(world: &World, jobs: &ProviderJobs, item: &str) -> u32 {
    let request = inbox::AcceptRequest {
        item: item.to_string(),
        project: world.project.clone(),
        asset: Some(world.asset.clone()),
        name: None,
        prep: None,
    };
    let result = inbox::accept(&world.root, &world.config, &request).expect("accept");
    jobs.accepted(item, &world.project, &world.asset, result.version.version);
    result.version.version
}

/// Makes the asset's first version with `provider`, as a send would.
fn made_by(world: &World, jobs: &ProviderJobs, provider: Provider) -> u32 {
    let job = jobs
        .send(
            &world.request(provider, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let done = finished(jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    accept(world, jobs, done.inbox_file.as_deref().expect("file"))
}

fn split_request(world: &World, provider: Provider) -> SplitRequest {
    SplitRequest {
        project: world.project.clone(),
        asset: world.asset.clone(),
        version: None,
        item: None,
        provider,
        tripo: None,
        meshy: None,
    }
}

#[test]
fn a_tripo_split_goes_by_the_task_that_made_the_model_then_completes_the_parts() {
    let world = World::new();
    let (server, _) = split_server();
    let jobs = world.jobs(&server);
    assert_eq!(made_by(&world, &jobs, Provider::Tripo), 1);
    let source = jobs
        .split_source(&world.project, &world.asset, None, None)
        .expect("source");
    assert_eq!(source.tripo_task.as_deref(), Some("task_model"));
    assert_eq!(source.made_by, Some(Provider::Tripo));
    assert!(
        source
            .meshy_problem
            .is_some_and(|p| p.contains("did not come from Meshy"))
    );
    assert_eq!(source.textures, 3);

    let mut request = split_request(&world, Provider::Tripo);
    request.tripo = Some(TripoSplit {
        model: SegmentModel::Semantic,
        granularity: Granularity::Detailed,
        split_by_connectivity: true,
        complete: Some(CompletionMode::AiCompletion),
    });
    let job = jobs.send_split(&request, false, false).expect("split");
    assert_eq!(job.estimate, Some(140.0));
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    assert_eq!(done.credits, Some(140.0));
    let segment = server
        .find("POST", "/mesh/segment")
        .expect("segment")
        .json();
    assert_eq!(
        segment,
        json!({
            "input": "task_model", "model": "v2.0-20260430",
            "segmentation_granularity": "detailed", "split_by_connectivity": true,
        })
    );
    let complete = server
        .find("POST", "/mesh/complete")
        .expect("complete")
        .json();
    assert_eq!(
        complete,
        json!({ "input": "task_seg", "completion_mode": "ai_completion" })
    );
    assert_eq!(
        server.count("POST", "/files"),
        1,
        "only the generation uploaded"
    );
    let split = done.split.clone().expect("split");
    assert_eq!(split.texture_kept, Some(false));
    assert!(
        done.message
            .is_some_and(|m| m.contains("Tripo did not keep the texture"))
    );

    // It lands as v2 through the Inbox; Tripo's lost texture is offered back.
    let item = done.inbox_file.expect("item");
    assert_eq!(accept(&world, &jobs, &item), 2);
    let bake = jobs.get(&job.id).and_then(|j| j.split?.bake).expect("bake");
    assert_eq!((bake.state, bake.onto), (BakeState::Offered, 2));
    assert!(jobs.scheduled_bakes().is_empty(), "Tripo only offers it");
    let spend = settled(&jobs, 2)
        .spends
        .into_iter()
        .find(|s| s.job == job.id)
        .expect("spend");
    assert_eq!(
        (spend.outcome, spend.credits),
        (SpendOutcome::Succeeded, 140.0)
    );
    assert!(
        spend
            .summary
            .starts_with("Tripo segment, semantic, detailed"),
        "{}",
        spend.summary
    );
}

#[test]
fn a_tripo_split_uploads_a_model_tripo_did_not_make() {
    let world = World::new();
    let (server, _) = split_server();
    let jobs = world.jobs(&server);
    fs::write(world.inbox().join("crate.glb"), glb()).expect("drop");
    assert_eq!(accept(&world, &jobs, "crate.glb"), 1);
    let source = jobs
        .split_source(&world.project, &world.asset, Some(1), None)
        .expect("source");
    assert_eq!((source.made_by, source.tripo_task), (None, None));
    let job = jobs
        .send_split(&split_request(&world, Provider::Tripo), false, false)
        .expect("split");
    assert_eq!(finished(&jobs, &job.id).state, JobState::Inbox);
    assert_eq!(server.count("POST", "/files"), 1);
    let segment = server
        .find("POST", "/mesh/segment")
        .expect("segment")
        .json();
    assert_eq!(
        segment,
        json!({ "input": "file_model", "model": "v1.0-20250506" })
    );
    assert!(server.find("POST", "/mesh/complete").is_none());
    // Meshy refuses a model it did not make, before anything is sent.
    let err = jobs
        .send_split(&split_request(&world, Provider::Meshy), false, false)
        .expect_err("not Meshy's");
    assert!(err.to_string().contains("did not come from Meshy"), "{err}");
}

#[test]
fn meshy_refuses_smart_topology_and_an_empty_part_list() {
    let world = World::new();
    let (server, _) = split_server();
    let jobs = world.jobs(&server);
    let mut send = world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"]);
    let mut preset = world.config.providers.meshy(PresetVariant::Prop).clone();
    preset.ai_model = "meshy-t2.5".into();
    send.meshy = Some(preset);
    let job = jobs.send(&send, false, false).expect("send");
    let item = finished(&jobs, &job.id).inbox_file.expect("item");
    // From the Inbox, before it is a version.
    let source = jobs
        .split_source(&world.project, &world.asset, None, Some(&item))
        .expect("source");
    assert!(
        source
            .meshy_problem
            .is_some_and(|p| p.contains("Smart Topology"))
    );
    let mut request = split_request(&world, Provider::Meshy);
    request.item = Some(item);
    request.meshy = Some(MeshySplit {
        mode: MeshySplitMode::ByParts,
        parts: Vec::new(),
    });
    let err = jobs
        .send_split(&request, false, false)
        .expect_err("no names");
    assert!(err.to_string().contains("Name at least one part"), "{err}");
    assert_eq!(server.count("POST", "/print/split"), 0);
}

#[test]
fn a_meshy_split_waits_for_approval_lands_as_a_version_and_bakes_the_texture_back() {
    let world = World::new();
    let (server, _) = split_server();
    let jobs = world.jobs(&server);
    assert_eq!(made_by(&world, &jobs, Provider::Meshy), 1);
    let mut request = split_request(&world, Provider::Meshy);
    request.meshy = Some(MeshySplit {
        mode: MeshySplitMode::ByParts,
        parts: vec!["lid".into(), "cage".into()],
    });
    let job = jobs.send_split(&request, true, true).expect("split");
    assert_eq!(job.state, JobState::Approval);
    assert!(
        job.summary()
            .starts_with("Meshy Auto Split, by parts, 2 names of v1")
    );
    assert_eq!(
        server.count("POST", "/print/split"),
        0,
        "nothing before approval"
    );
    jobs.approve(&job.id).expect("approve");
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    assert_eq!(done.credits, Some(10.0));
    let body = server.find("POST", "/print/split").expect("split").json();
    assert_eq!(
        body,
        json!({
            "input_task_id": "m-model", "mode": "by_parts", "prompt": "lid, cage",
            "target_formats": ["glb"], "layout": "assembled",
        })
    );
    assert!(
        done.message
            .is_some_and(|m| m.contains("Meshy drops the texture"))
    );

    assert_eq!(accept(&world, &jobs, &done.inbox_file.expect("item")), 2);
    assert_eq!(jobs.scheduled_bakes(), std::slice::from_ref(&job.id));

    // Without Blender the bake fails, says why and can be scheduled again.
    let failed = jobs.bake_back(&job.id, &Toolbox::default()).expect("ran");
    let bake = failed.split.and_then(|s| s.bake).expect("bake");
    assert_eq!(bake.state, BakeState::Failed);
    assert!(bake.message.is_some_and(|m| m.contains("Blender")));
    jobs.schedule_bake(&job.id).expect("again");
    assert_eq!(jobs.scheduled_bakes().len(), 1);

    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let baked = jobs.run_scheduled_bakes(&crate::test_blender::toolbox(&blender));
    let bake = baked[0].split.clone().and_then(|s| s.bake).expect("bake");
    assert_eq!(
        (bake.state, bake.version),
        (BakeState::Done, Some(3)),
        "{bake:?}"
    );
    let manifest = project::load(&world.root, &world.config.layout, &world.project).expect("load");
    let asset = assets::find(&manifest, &world.asset).expect("asset");
    assert_eq!(asset.versions.len(), 3, "the untextured v2 stays");
    let baked_file = world.project_dir().join(&asset.versions[2].file);
    let report = inbox::analyse::analyse(&baked_file).expect("analyse");
    let parts = crate::parts::read(&baked_file).expect("parts");
    assert_eq!(
        report.textures.len(),
        parts.len(),
        "one baked image per part"
    );
    assert!(!parts.is_empty());
    assert!(
        jobs.bake_back(&job.id, &Toolbox::default()).is_err(),
        "done once"
    );
}

#[test]
fn a_failed_meshy_split_is_refunded_and_a_pending_one_cancels_for_free() {
    let world = World::new();
    let (server, fake) = split_server();
    let jobs = world.jobs(&server);
    made_by(&world, &jobs, Provider::Meshy);
    fake.fail.store(true, Ordering::SeqCst);
    let job = jobs
        .send_split(&split_request(&world, Provider::Meshy), false, false)
        .expect("split");
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Failed);
    assert_eq!(done.message.as_deref(), Some("The model could not be cut."));
    let spend = settled(&jobs, 2)
        .spends
        .into_iter()
        .find(|s| s.job == job.id)
        .expect("spend");
    assert_eq!((spend.outcome, spend.credits), (SpendOutcome::Failed, 0.0));

    fake.fail.store(false, Ordering::SeqCst);
    fake.hold.store(true, Ordering::SeqCst);
    let job = jobs
        .send_split(&split_request(&world, Provider::Meshy), false, false)
        .expect("split");
    let end = std::time::Instant::now() + WAIT;
    while jobs.get(&job.id).and_then(|j| j.remote_id).is_none() {
        assert!(std::time::Instant::now() < end, "never sent");
        std::thread::sleep(Duration::from_millis(5));
    }
    let cancelled = jobs.cancel(&job.id).expect("cancel");
    assert_eq!(cancelled.state, JobState::Cancelled);
    assert_eq!(server.count("DELETE", "/print/split/m-split"), 1);
    let spend = settled(&jobs, 3)
        .spends
        .into_iter()
        .find(|s| s.job == job.id)
        .expect("spend");
    assert_eq!(spend.outcome, SpendOutcome::Refunded);
}

#[test]
fn a_split_resumes_after_a_restart_without_paying_again() {
    let world = World::new();
    let (server, fake) = split_server();
    let first = world.jobs(&server);
    made_by(&world, &first, Provider::Tripo);
    fake.hold.store(true, Ordering::SeqCst);
    let job = first
        .send_split(&split_request(&world, Provider::Tripo), false, false)
        .expect("split");
    let end = std::time::Instant::now() + WAIT;
    while first.get(&job.id).and_then(|j| j.remote_id).is_none() {
        assert!(std::time::Instant::now() < end, "never sent");
        std::thread::sleep(Duration::from_millis(5));
    }
    first.close();
    fake.hold.store(false, Ordering::SeqCst);
    let second = world.jobs(&server);
    let done = finished(&second, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    assert_eq!(server.count("POST", "/mesh/segment"), 1);
    assert!(done.split.is_some_and(|s| s.from_version == Some(1)));
}
