//! Provider jobs against fake Tripo and Meshy servers. No test talks to
//! the real services, so none spends a credit.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use super::fake::{FakeServer, Reply, Request};
use super::http::Timing;
use super::*;
use crate::assets::{self, AssetKind, AssetStatus, NewAsset, Stage};
use crate::inbox;
use crate::inbox::fixture;
use crate::project::{self, NewProject, ProjectTemplate};
use crate::workspace::{self, WorkspaceConfig};

const WAIT: Duration = Duration::from_secs(20);

struct World {
    _dir: tempfile::TempDir,
    root: PathBuf,
    config: WorkspaceConfig,
    project: String,
    asset: String,
    secrets: Arc<MemoryStore>,
    events: Arc<Mutex<Vec<ProviderEvent>>>,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
        let manifest = project::create(
            &root,
            &config.layout,
            NewProject {
                name: "Moon Island".into(),
                template: ProjectTemplate::FloatingDiorama,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        let mut asset = String::new();
        project::modify(&root, &config.layout, &manifest.id, |m| {
            asset = assets::add(
                m,
                NewAsset {
                    name: "Stone lantern".into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Refs,
                },
            )?
            .id;
            Ok(())
        })
        .expect("asset");
        let refs = project::dir(&root, &config.layout, &manifest.id)
            .expect("dir")
            .join("refs");
        fs::create_dir_all(&refs).expect("refs");
        for name in ["front.png", "left.png", "back.png"] {
            fs::write(refs.join(name), b"\x89PNG\r\n\x1a\nfake").expect("image");
        }
        let secrets = Arc::new(MemoryStore::default());
        secrets
            .set(ProviderId::Tripo, "tsk_test_key_0001")
            .expect("key");
        secrets
            .set(ProviderId::Meshy, "msy_test_key_0002")
            .expect("key");
        Self {
            _dir: dir,
            root,
            config,
            project: manifest.id,
            asset,
            secrets,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn jobs(&self, server: &FakeServer) -> ProviderJobs {
        let options = Options {
            endpoints: Endpoints {
                tripo: server.url.clone(),
                meshy: server.url.clone(),
            },
            tripo: Timing::instant(),
            meshy: Timing::instant(),
            limit: 4,
        };
        let events = Arc::clone(&self.events);
        let jobs = ProviderJobs::new(
            Arc::clone(&self.secrets) as Arc<dyn SecretStore>,
            options,
            move |event| events.lock().expect("events").push(event),
        );
        jobs.open_workspace(&self.root, &self.config);
        jobs
    }

    fn request(&self, provider: Provider, variant: PresetVariant, images: &[&str]) -> SendRequest {
        SendRequest {
            project: self.project.clone(),
            asset: self.asset.clone(),
            provider,
            service: ProviderId::Tripo,
            variant,
            images: images.iter().map(|i| format!("refs/{i}")).collect(),
            tripo: None,
            meshy: None,
        }
    }

    fn project_dir(&self) -> PathBuf {
        project::dir(&self.root, &self.config.layout, &self.project).expect("dir")
    }

    fn inbox(&self) -> PathBuf {
        inbox::inbox_dir(&self.root, &self.config)
    }

    fn asset_status(&self) -> (Stage, AssetStatus) {
        let m = project::load(&self.root, &self.config.layout, &self.project).expect("load");
        let a = assets::find(&m, &self.asset).expect("asset");
        (a.stage, a.status)
    }
}

fn finished(jobs: &ProviderJobs, id: &str) -> ProviderJob {
    let end = std::time::Instant::now() + WAIT;
    loop {
        let job = jobs.get(id).expect("job");
        if !matches!(job.state, JobState::Queued | JobState::Running) {
            return job;
        }
        assert!(
            std::time::Instant::now() < end,
            "job {id} did not finish: {job:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn glb() -> Vec<u8> {
    fixture::clean_prop()
}

fn tripo_data(data: Value) -> Reply {
    Reply::ok(&json!({ "code": 0, "data": data }))
}

/// A Tripo double: uploads, tasks that run for `polls` checks, and
/// downloads under `/dl/`.
struct TripoFake {
    polls: u32,
    seen: Mutex<std::collections::HashMap<String, u32>>,
    fail: Option<&'static str>,
    hold: Arc<AtomicBool>,
    expire_first: bool,
    downloads: AtomicU32,
    riggable: bool,
}

impl TripoFake {
    fn new() -> Self {
        Self {
            polls: 2,
            seen: Mutex::default(),
            fail: None,
            hold: Arc::new(AtomicBool::new(false)),
            expire_first: false,
            downloads: AtomicU32::new(0),
            riggable: true,
        }
    }

    fn handle(&self, base: &str, req: &Request) -> Reply {
        let path = req.path.as_str();
        match (req.method.as_str(), path) {
            ("POST", "/files") => tripo_data(json!({ "file_token": "file_front" })),
            ("POST", "/generation/image-to-model") => {
                tripo_data(json!({ "task_id": "task_model" }))
            }
            ("POST", "/generation/multiview-to-model") => {
                tripo_data(json!({ "task_id": "task_model" }))
            }
            ("POST", "/animations/rig-check") => tripo_data(json!({ "task_id": "task_check" })),
            ("POST", "/animations/rig") => tripo_data(json!({ "task_id": "task_rig" })),
            ("POST", "/animations/retarget") => tripo_data(json!({ "task_id": "task_anim" })),
            ("GET", p) if p.starts_with("/tasks/") => self.task(base, &p["/tasks/".len()..]),
            ("GET", "/dl/old.glb") => Reply::json(403, &json!({ "message": "expired" })),
            ("GET", p) if p.starts_with("/dl/") => {
                self.downloads.fetch_add(1, Ordering::SeqCst);
                Reply::bytes(200, glb())
            }
            ("GET", "/account/balance") => tripo_data(json!({ "balance": 1234.5, "frozen": 20 })),
            _ => Reply::json(404, &json!({ "message": "no route" })),
        }
    }

    fn task(&self, base: &str, id: &str) -> Reply {
        let count = {
            let mut seen = self.seen.lock().expect("seen");
            let n = seen.entry(id.to_string()).or_default();
            *n += 1;
            *n
        };
        if self.hold.load(Ordering::SeqCst) || count <= self.polls {
            return tripo_data(json!({
                "task_id": id, "status": "running", "progress": count * 30,
            }));
        }
        if let Some(reason) = self.fail {
            return tripo_data(json!({
                "task_id": id, "status": "failed", "error_message": reason,
            }));
        }
        let output = match id {
            "task_check" => json!({ "riggable": self.riggable, "rig_type": "biped" }),
            "task_anim" => json!({ "model_urls": [format!("{base}/dl/animated.glb")] }),
            "task_rig" => json!({ "model_url": format!("{base}/dl/rigged.glb") }),
            _ => {
                let file = if self.expire_first && count == self.polls + 1 {
                    "old"
                } else {
                    "model"
                };
                json!({ "model_url": format!("{base}/dl/{file}.glb") })
            }
        };
        let credits = match id {
            "task_model" => 30.0,
            "task_rig" => 25.0,
            "task_anim" => 10.0,
            _ => 0.0,
        };
        tripo_data(json!({
            "task_id": id, "status": "success", "progress": 100,
            "output": output, "credits_consumed": credits,
        }))
    }
}

fn tripo_server(fake: TripoFake) -> (FakeServer, Arc<TripoFake>) {
    let fake = Arc::new(fake);
    let handler = Arc::clone(&fake);
    (
        FakeServer::start(move |base, req| handler.handle(base, req)),
        fake,
    )
}

#[test]
fn tripo_success_lands_in_the_inbox_linked_to_the_asset() {
    let world = World::new();
    let (server, _) = tripo_server(TripoFake::new());
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    assert_eq!(job.id, format!("{}-1", world.asset));
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    assert_eq!(done.credits, Some(30.0));
    assert_eq!(done.inbox_file.as_deref(), Some("stone-lantern-1.glb"));
    assert!(world.inbox().join("stone-lantern-1.glb").is_file());
    assert_eq!(world.asset_status(), (Stage::Tripo, AssetStatus::Review));

    // The request carried the key, the model and nothing a model refuses.
    let create = server
        .find("POST", "/generation/image-to-model")
        .expect("create");
    assert_eq!(
        create.header("authorization"),
        Some("Bearer tsk_test_key_0001")
    );
    let body = create.json();
    assert_eq!(body["model"], "v3.1-20260211");
    assert_eq!(body["input"], "file_front");
    assert_eq!(server.count("POST", "/files"), 1);

    // The inbox item links back and runs its checks.
    let mut items = inbox::list(&world.root, &world.config).expect("inbox");
    jobs.link_inbox(&mut items);
    let item = &items[0];
    let link = item.link.as_ref().expect("link");
    assert_eq!(link.asset, world.asset);
    assert_eq!(link.provider, Provider::Tripo);
    assert!(!item.checks.is_empty());

    // No event or saved file carries the key.
    let saved = fs::read_to_string(world.project_dir().join(jobs::JOBS_FILE)).expect("saved");
    assert!(!saved.contains("tsk_test_key"));
    let events = format!("{:?}", world.events.lock().expect("events"));
    assert!(!events.contains("tsk_test_key"));
}

#[test]
fn tripo_failure_fails_the_job_and_the_asset() {
    let world = World::new();
    let mut fake = TripoFake::new();
    fake.fail = Some("The image has no clear subject.");
    let (server, _) = tripo_server(fake);
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Failed);
    assert_eq!(
        done.message.as_deref(),
        Some("The image has no clear subject.")
    );
    assert_eq!(world.asset_status().1, AssetStatus::Failed);
    assert!(!world.inbox().join("stone-lantern-1.glb").exists());
}

#[test]
fn an_expired_download_link_is_fetched_again_from_the_task() {
    let world = World::new();
    let mut fake = TripoFake::new();
    fake.expire_first = true;
    let (server, fake) = tripo_server(fake);
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    assert_eq!(server.count("GET", "/dl/old.glb"), 1);
    assert_eq!(fake.downloads.load(Ordering::SeqCst), 1);
}

#[test]
fn out_of_credits_says_so() {
    let world = World::new();
    // Tripo answers 403 with code 2010; Meshy answers 402.
    let server = FakeServer::start(|_, req| match (req.method.as_str(), req.path.as_str()) {
        ("POST", "/files") => tripo_data(json!({ "file_token": "file_front" })),
        ("POST", "/generation/image-to-model") => Reply::json(
            403,
            &json!({ "code": 2010, "message": "You don't have enough credit to create this task" }),
        ),
        ("POST", "/image-to-3d") => Reply::json(402, &json!({ "message": "Insufficient credits" })),
        _ => Reply::json(404, &json!({})),
    });
    let jobs = world.jobs(&server);
    let tripo = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let done = finished(&jobs, &tripo.id);
    assert_eq!(done.state, JobState::Failed);
    assert!(
        done.message
            .as_deref()
            .is_some_and(|m| m.starts_with("Tripo says the account is out of credits")),
        "{done:?}"
    );
    let meshy = jobs
        .send(
            &world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let done = finished(&jobs, &meshy.id);
    assert!(
        done.message
            .as_deref()
            .is_some_and(|m| m.contains("Meshy says the account is out of credits")),
        "{done:?}"
    );
    assert_eq!(done.credits, None);
}

#[test]
fn rate_limits_back_off_with_retry_after_then_give_up() {
    let world = World::new();
    let tries = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&tries);
    let fake = Arc::new(TripoFake::new());
    let inner = Arc::clone(&fake);
    let server = FakeServer::start(move |base, req| {
        if req.path == "/generation/image-to-model" && counter.fetch_add(1, Ordering::SeqCst) < 2 {
            return Reply::json(429, &json!({ "code": 1007, "message": "slow down" }))
                .with_header("Retry-After", "0.02");
        }
        inner.handle(base, req)
    });
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    assert_eq!(server.count("POST", "/generation/image-to-model"), 3);

    // A server that never stops asking: the job gives up with a message.
    let server = FakeServer::start(|_, req| match req.path.as_str() {
        "/files" => tripo_data(json!({ "file_token": "file_front" })),
        _ => Reply::json(429, &json!({ "code": 2000, "message": "busy" })),
    });
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Failed);
    assert!(
        done.message
            .as_deref()
            .is_some_and(|m| m.contains("slow down"))
    );
    // 1 try plus Timing::instant().max_retries retries.
    assert_eq!(server.count("POST", "/generation/image-to-model"), 4);
}

#[test]
fn tripo_characters_check_rig_and_retarget() {
    let world = World::new();
    let (server, _) = tripo_server(TripoFake::new());
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(
                Provider::Tripo,
                PresetVariant::Character,
                &["front.png", "left.png"],
            ),
            false,
            false,
        )
        .expect("send");
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    assert_eq!(done.credits, Some(65.0));
    let create = server
        .find("POST", "/generation/multiview-to-model")
        .expect("multiview");
    assert_eq!(create.json()["inputs"][1]["left"], "file_front");
    assert_eq!(create.json()["face_limit"], 50_000);
    let check = server.find("POST", "/animations/rig-check").expect("check");
    assert_eq!(check.json()["input"], "task_model");
    let rig = server.find("POST", "/animations/rig").expect("rig").json();
    assert_eq!(rig["model"], "v3.0-20260909");
    assert_eq!(rig["spec"], "mixamo");
    assert_eq!(rig["rig_type"], "biped");
    let retarget = server
        .find("POST", "/animations/retarget")
        .expect("retarget")
        .json();
    assert_eq!(retarget["input"], "task_rig");
    assert_eq!(
        retarget["animations"],
        json!(["preset:idle", "preset:walk"])
    );
    assert_eq!(server.count("GET", "/dl/animated.glb"), 1);
    // The inbox already treats it as a character.
    let item = inbox::item(&world.root, &world.config, "stone-lantern-1.glb").expect("item");
    assert_eq!(item.kind, AssetKind::Character);
}

#[test]
fn an_unriggable_character_arrives_without_a_rig() {
    let world = World::new();
    let mut fake = TripoFake::new();
    fake.riggable = false;
    let (server, _) = tripo_server(fake);
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Character, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox);
    assert!(
        done.message
            .as_deref()
            .is_some_and(|m| m.contains("cannot be rigged"))
    );
    assert_eq!(
        server.count("POST", "/animations/rig"),
        1,
        "only the rig check"
    );
}

#[test]
fn a_restart_resumes_polling_without_sending_again() {
    let world = World::new();
    let fake = TripoFake::new();
    fake.hold.store(true, Ordering::SeqCst);
    let hold = Arc::clone(&fake.hold);
    let (server, _) = tripo_server(fake);
    let first = world.jobs(&server);
    let job = first
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let end = std::time::Instant::now() + WAIT;
    while first.get(&job.id).and_then(|j| j.remote_id).is_none() {
        assert!(std::time::Instant::now() < end, "never sent");
        std::thread::sleep(Duration::from_millis(5));
    }
    // The app closes mid poll.
    first.close();
    let saved = fs::read_to_string(world.project_dir().join(jobs::JOBS_FILE)).expect("saved");
    assert!(saved.contains("task_model"));
    std::thread::sleep(Duration::from_millis(50));

    hold.store(false, Ordering::SeqCst);
    let second = world.jobs(&server);
    let done = finished(&second, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    assert_eq!(server.count("POST", "/generation/image-to-model"), 1);
    assert_eq!(server.count("POST", "/files"), 1);
}

#[test]
fn a_job_cut_off_while_sending_fails_cautiously() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    // A saved job that set `sending` but never got a task id back.
    let job = jobs
        .send(
            &world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    jobs.close();
    let path = world.project_dir().join(jobs::JOBS_FILE);
    let mut file: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    let stored = &mut file["jobs"][0];
    stored["job"]["provider"] = json!("tripo");
    stored["job"]["state"] = json!("running");
    stored["chain"]["sending"] = json!(true);
    let approval = json!({ "job": { "state": "approval" }, "chain": {} });
    let mut second = stored.clone();
    second["job"]["id"] = json!("stone-lantern-9");
    second["job"]["state"] = approval["job"]["state"].clone();
    second["chain"] = json!({});
    file["jobs"].as_array_mut().expect("jobs").push(second);
    fs::write(&path, file.to_string()).expect("write");

    let jobs = world.jobs(&server);
    let cut = jobs.get(&job.id).expect("job");
    assert_eq!(cut.state, JobState::Failed);
    assert!(
        cut.message
            .as_deref()
            .is_some_and(|m| m.contains("do not pay twice"))
    );
    let waiting = jobs.get("stone-lantern-9").expect("job");
    assert_eq!(waiting.state, JobState::Failed);
    assert!(
        waiting
            .message
            .as_deref()
            .is_some_and(|m| m.contains("Nothing was spent"))
    );
    assert_eq!(server.requests().len(), 0, "nothing was sent on resume");
}

/// A Meshy double: tasks pending for one check, in progress for one, then
/// done. DELETE works while pending, 409 after.
struct MeshyFake {
    seen: Mutex<std::collections::HashMap<String, u32>>,
    hold_pending: AtomicBool,
}

impl MeshyFake {
    fn status(&self, id: &str) -> u32 {
        let mut seen = self.seen.lock().expect("seen");
        let n = seen.entry(id.to_string()).or_default();
        *n += 1;
        *n
    }

    fn handle(&self, base: &str, req: &Request) -> Reply {
        let path = req.path.as_str();
        match (req.method.as_str(), path) {
            ("POST", "/image-to-3d") => Reply::ok(&json!({ "result": "m-model" })),
            ("POST", "/multi-image-to-3d") => Reply::ok(&json!({ "result": "m-model" })),
            ("POST", "/rigging") => Reply::ok(&json!({ "result": "m-rig" })),
            ("DELETE", p) => {
                if self.hold_pending.load(Ordering::SeqCst) {
                    Reply::ok(&json!({}))
                } else {
                    let _ = p;
                    Reply::json(
                        409,
                        &json!({ "message": "Task is IN_PROGRESS and cannot be deleted." }),
                    )
                }
            }
            ("GET", "/balance") => Reply::ok(&json!({ "balance": 900 })),
            ("GET", p) if p.starts_with("/dl/") => Reply::bytes(200, glb()),
            ("GET", p) => {
                let id = p.rsplit('/').next().unwrap_or_default();
                let n = if self.hold_pending.load(Ordering::SeqCst) {
                    1
                } else {
                    self.status(id)
                };
                let status = match n {
                    1 => "PENDING",
                    2 => "IN_PROGRESS",
                    _ => "SUCCEEDED",
                };
                let mut body = json!({
                    "id": id, "status": status, "progress": if n >= 3 { 100 } else { 40 },
                    "task_error": { "message": "" },
                });
                if status == "SUCCEEDED" {
                    body["consumed_credits"] = json!(if id == "m-rig" { 5 } else { 20 });
                    body["model_urls"] = json!({ "glb": format!("{base}/dl/{id}.glb") });
                    body["result"] =
                        json!({ "rigged_character_glb_url": format!("{base}/dl/{id}-rigged.glb") });
                }
                Reply::ok(&body).with_header("Retry-After", "0.01")
            }
            _ => Reply::json(404, &json!({})),
        }
    }
}

fn meshy_server(hold: bool) -> (FakeServer, Arc<MeshyFake>) {
    let fake = Arc::new(MeshyFake {
        seen: Mutex::default(),
        hold_pending: AtomicBool::new(hold),
    });
    let handler = Arc::clone(&fake);
    (
        FakeServer::start(move |base, req| handler.handle(base, req)),
        fake,
    )
}

#[test]
fn meshy_success_and_the_rigging_chain() {
    let world = World::new();
    let (server, _) = meshy_server(false);
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(
                Provider::Meshy,
                PresetVariant::Character,
                &["front.png", "left.png"],
            ),
            false,
            false,
        )
        .expect("send");
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox, "{done:?}");
    assert_eq!(done.credits, Some(25.0));
    let create = server.find("POST", "/multi-image-to-3d").expect("create");
    let body = create.json();
    assert_eq!(
        create.header("authorization"),
        Some("Bearer msy_test_key_0002")
    );
    assert!(
        body["image_urls"][0]
            .as_str()
            .is_some_and(|u| u.starts_with("data:image/png;base64,"))
    );
    assert_eq!(body["target_formats"], json!(["glb"]));
    let rig = server.find("POST", "/rigging").expect("rig").json();
    assert_eq!(rig["input_task_id"], "m-model");
    assert_eq!(server.count("GET", "/dl/m-rig-rigged.glb"), 1);
}

#[test]
fn meshy_cancels_while_pending_and_says_plainly_when_it_cannot() {
    let world = World::new();
    let (server, fake) = meshy_server(true);
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    let end = std::time::Instant::now() + WAIT;
    while jobs.get(&job.id).and_then(|j| j.remote_id).is_none() {
        assert!(std::time::Instant::now() < end, "never sent");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(jobs.get(&job.id).expect("job").cancel, CancelMode::Provider);
    let cancelled = jobs.cancel(&job.id).expect("cancel");
    assert_eq!(cancelled.state, JobState::Cancelled);
    assert!(
        cancelled
            .message
            .as_deref()
            .is_some_and(|m| m.contains("refunds"))
    );
    assert_eq!(server.count("DELETE", "/image-to-3d/m-model"), 1);
    assert_eq!(world.asset_status().1, AssetStatus::Queued);

    // Once Meshy has started, DELETE answers 409 and the job carries on.
    drop(jobs);
    drop(fake);
    let world = World::new();
    let (server, _) = meshy_server(false);
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    while jobs.get(&job.id).and_then(|j| j.remote_id).is_none() {
        std::thread::sleep(Duration::from_millis(2));
    }
    match jobs.cancel(&job.id) {
        Err(err) => assert!(err.to_string().contains("cannot be cancelled"), "{err}"),
        // It may have finished first on a fast machine.
        Ok(job) => assert_eq!(job.state, JobState::Inbox),
    }
    let done = finished(&jobs, &job.id);
    assert_eq!(done.state, JobState::Inbox);

    // Tripo has no cancel at all once a task is sent.
    let fake = TripoFake::new();
    fake.hold.store(true, Ordering::SeqCst);
    let (server, _) = tripo_server(fake);
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    while jobs.get(&job.id).and_then(|j| j.remote_id).is_none() {
        std::thread::sleep(Duration::from_millis(2));
    }
    let err = jobs.cancel(&job.id).expect_err("no cancel");
    assert!(err.to_string().contains("Tripo cannot cancel"), "{err}");
    jobs.close();
}

#[test]
fn approval_holds_the_job_until_released_and_cancel_before_send_is_free() {
    let world = World::new();
    let (server, _) = tripo_server(TripoFake::new());
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            true,
            true,
        )
        .expect("send");
    assert_eq!(job.state, JobState::Approval);
    assert!(job.via_agent);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(server.requests().len(), 0, "nothing sent before approval");
    let cancelled = jobs.cancel(&job.id).expect("cancel");
    assert_eq!(
        cancelled.message.as_deref(),
        Some("Cancelled. Nothing was spent.")
    );
    let job = jobs
        .send(
            &world.request(
                Provider::Tripo,
                PresetVariant::Character,
                &["front.png", "left.png"],
            ),
            true,
            true,
        )
        .expect("send");
    assert_eq!(
        job.summary(),
        "Tripo H3.1, 2 images as multi-view, 50,000 faces, rig with Mixamo, 2 animations"
    );
    let denied = jobs.deny(&job.id).expect("deny");
    assert_eq!(denied.state, JobState::Cancelled);
    assert_eq!(
        denied.message.as_deref(),
        Some("Denied in Maquette. Nothing was spent.")
    );

    let job = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            true,
            true,
        )
        .expect("send");
    assert_eq!(job.id, format!("{}-3", world.asset));
    assert_eq!(job.summary(), "Tripo H3.1, 1 image");
    jobs.approve(&job.id).expect("approve");
    assert!(
        jobs.deny(&job.id).is_err(),
        "only a send waiting for approval"
    );
    assert_eq!(finished(&jobs, &job.id).state, JobState::Inbox);
}

#[test]
fn sends_are_checked_before_anything_starts() {
    let world = World::new();
    let (server, _) = tripo_server(TripoFake::new());
    let jobs = world.jobs(&server);
    let mut request = world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]);
    request.images = vec!["../../secrets.png".into()];
    assert!(jobs.send(&request, false, false).is_err());
    request.images = vec!["refs/missing.png".into()];
    assert!(jobs.send(&request, false, false).is_err());
    request.images = vec![];
    assert!(jobs.send(&request, false, false).is_err());
    let mut request = world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]);
    let mut preset = TripoPreset::prop();
    preset.model = "P1-20260311".into();
    preset.face_limit = 40_000;
    request.tripo = Some(preset);
    assert!(jobs.send(&request, false, false).is_err());
    world.secrets.remove(ProviderId::Tripo).expect("remove");
    let err = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect_err("no key");
    assert!(err.to_string().contains("Add a Tripo API key"));
    assert_eq!(server.requests().len(), 0);
}

#[test]
fn by_hand_writes_the_handoff_and_takes_the_model_back() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let mut request = world.request(
        Provider::ByHand,
        PresetVariant::Character,
        &["front.png", "left.png", "back.png"],
    );
    request.service = ProviderId::Meshy;
    // By hand needs no key.
    world.secrets.remove(ProviderId::Meshy).expect("remove");
    let job = jobs.send(&request, true, false).expect("send");
    assert_eq!(
        job.state,
        JobState::Waiting,
        "By hand never waits for approval"
    );
    assert_eq!(job.cancel, CancelMode::StopWaiting);
    let folder = PathBuf::from(job.handoff.clone().expect("folder"));
    assert_eq!(
        folder,
        world.project_dir().join("handoff").join("stone-lantern-1")
    );
    for name in [
        "front.png",
        "left.png",
        "back.png",
        "settings.md",
        "settings.json",
    ] {
        assert!(folder.join(name).is_file(), "{name}");
    }
    let md = fs::read_to_string(folder.join("settings.md")).expect("md");
    assert!(md.contains("https://www.meshy.ai/workspace"));
    assert!(md.contains("| AI model | Meshy 7.1 |"));
    assert!(md.contains("stone-lantern-1.glb"));
    let card = job.card.clone().expect("card");
    assert_eq!(card.mode, "Multi-image to 3D");
    assert_eq!(card.images, vec!["front.png", "left.png", "back.png"]);
    assert_eq!(world.asset_status(), (Stage::Tripo, AssetStatus::Queued));

    // Dropping something that is not a GLB is refused.
    let wrong = world.root.join("notes.txt");
    fs::write(&wrong, "hello").expect("write");
    assert!(jobs.hand_back(&job.id, &wrong).is_err());

    // A GLB dropped on the card goes to the inbox under the job's name.
    let made = world.root.join("Downloaded model.glb");
    fs::write(&made, fixture::character(false)).expect("write");
    let back = jobs.hand_back(&job.id, &made).expect("hand back");
    assert_eq!(back.state, JobState::Inbox);
    assert_eq!(back.inbox_file.as_deref(), Some("stone-lantern-1.glb"));
    assert_eq!(world.asset_status().1, AssetStatus::Review);

    // The Inbox sees it linked, as a character, with its checks run.
    let mut items = inbox::list(&world.root, &world.config).expect("inbox");
    jobs.link_inbox(&mut items);
    let item = items
        .iter()
        .find(|i| i.id == "stone-lantern-1.glb")
        .expect("item");
    assert_eq!(
        item.link.as_ref().map(|l| l.asset.as_str()),
        Some(world.asset.as_str())
    );
    assert_eq!(item.kind, AssetKind::Character);
    assert!(
        item.checks.iter().any(|c| c.id == "rig"),
        "{:?}",
        item.checks
    );

    // Accepting it records the version on the job.
    let result = inbox::accept(
        &world.root,
        &world.config,
        &inbox::AcceptRequest {
            item: item.id.clone(),
            project: world.project.clone(),
            asset: Some(world.asset.clone()),
            name: None,
        },
    )
    .expect("accept");
    jobs.accepted(&item.id, result.version.version);
    let accepted = jobs.get(&job.id).expect("job");
    assert_eq!(accepted.state, JobState::Accepted);
    assert_eq!(accepted.version, Some(1));
}

#[test]
fn a_model_saved_into_the_inbox_under_the_handoff_name_links_too() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let job = jobs
        .send(
            &world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    fs::write(world.inbox().join("Stone_Lantern_1_final.glb"), glb()).expect("write");
    let mut items = inbox::list(&world.root, &world.config).expect("inbox");
    jobs.link_inbox(&mut items);
    assert_eq!(
        items[0].link.as_ref().map(|l| l.job.as_str()),
        Some(job.id.as_str())
    );
    let job = jobs.get(&job.id).expect("job");
    assert_eq!(job.state, JobState::Inbox);
    assert_eq!(job.inbox_file.as_deref(), Some("Stone_Lantern_1_final.glb"));
    let stop = jobs.cancel(&job.id);
    assert!(stop.is_err(), "an arrived model has nothing to cancel");
}

#[test]
fn test_key_uses_the_free_balance_calls() {
    let (server, _) = tripo_server(TripoFake::new());
    let endpoints = Endpoints {
        tripo: server.url.clone(),
        meshy: server.url.clone(),
    };
    let tripo = test_key(&endpoints, ProviderId::Tripo, "tsk_x", Timing::instant());
    assert!(tripo.ok);
    assert_eq!(tripo.message, "The key works. 1,234.50 credits left.");
    let bad = FakeServer::start(|_, _| Reply::json(401, &json!({ "message": "bad key" })));
    let endpoints = Endpoints {
        tripo: bad.url.clone(),
        meshy: bad.url.clone(),
    };
    let meshy = test_key(&endpoints, ProviderId::Meshy, "msy_x", Timing::instant());
    assert!(!meshy.ok);
    assert!(meshy.message.contains("did not accept the API key"));
    assert_eq!(bad.count("GET", "/balance"), 1);
    assert_eq!(format_credits(900.0), "900");
}
