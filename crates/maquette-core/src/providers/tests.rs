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

mod connect;
mod handoff_paths;
mod ids;
mod landed;
mod remove;
mod split;
mod stopped_cards;

const WAIT: Duration = Duration::from_secs(20);

pub(super) struct World {
    _dir: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) config: WorkspaceConfig,
    pub(super) project: String,
    pub(super) asset: String,
    secrets: Arc<MemoryStore>,
    events: Arc<Mutex<Vec<ProviderEvent>>>,
}

impl World {
    pub(super) fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
        let manifest = project::create(
            &root,
            &config.layout,
            NewProject {
                name: "Moon Island".into(),
                template: ProjectTemplate::FloatingDiorama,
                target: Default::default(),
                game: None,
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

    pub(super) fn jobs(&self, server: &FakeServer) -> ProviderJobs {
        self.jobs_at(&server.url, Timing::instant())
    }

    /// Jobs whose providers both live at `url`, on `timing`.
    pub(super) fn jobs_at(&self, url: &str, timing: Timing) -> ProviderJobs {
        let options = Options {
            endpoints: Endpoints {
                tripo: url.to_string(),
                meshy: url.to_string(),
            },
            tripo: timing,
            meshy: timing,
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

    pub(super) fn request(
        &self,
        provider: Provider,
        variant: PresetVariant,
        images: &[&str],
    ) -> SendRequest {
        SendRequest {
            project: self.project.clone(),
            asset: self.asset.clone(),
            provider,
            service: ProviderId::Tripo,
            variant,
            images: images.iter().map(|i| format!("refs/{i}")).collect(),
            tripo: None,
            meshy: None,
            prompt: None,
            redo_of: None,
            studio: None,
        }
    }

    pub(super) fn project_dir(&self) -> PathBuf {
        project::dir(&self.root, &self.config.layout, &self.project).expect("dir")
    }

    pub(super) fn inbox(&self) -> PathBuf {
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
    assert!(md.contains("maq_stone-lantern_v1_meshy"));
    assert!(folder.join("send.json").is_file());
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
            prep: None,
        },
    )
    .expect("accept");
    jobs.accepted(
        &item.id,
        &world.project,
        &world.asset,
        result.version.version,
    );
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
fn a_linked_model_takes_its_kind_and_map_choices_from_the_send() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let mut request = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    let mut preset = TripoPreset::prop();
    preset.texture = false;
    request.tripo = Some(preset);
    let job = jobs.send(&request, false, false).expect("send");
    // The file name alone reads as a scene.
    fs::write(
        world.inbox().join(format!("{}-island.glb", job.id)),
        fixture::merged_islands(),
    )
    .expect("write");
    let mut items = inbox::list(&world.root, &world.config).expect("inbox");
    assert_eq!(items[0].kind, AssetKind::Scene);
    jobs.link_inbox(&mut items);
    let item = &items[0];
    assert_eq!(item.kind, AssetKind::Prop);
    assert!(item.kind_from_send && !item.kind_guessed);
    let pbr = item.checks.iter().find(|c| c.id == "pbr").expect("pbr");
    assert_eq!(pbr.status, inbox::CheckStatus::Pass);
    assert_eq!(pbr.title, "Texture off, as sent");
    // The prop kind brings the merged mesh warning a scene would skip.
    assert!(
        item.checks
            .iter()
            .any(|c| c.id == "merged" && c.status == inbox::CheckStatus::Warning)
    );

    // A kind someone picked wins over the send.
    inbox::set_kind(&world.root, &world.config, &item.id, AssetKind::Scene).expect("kind");
    let mut items = inbox::list(&world.root, &world.config).expect("inbox");
    jobs.link_inbox(&mut items);
    assert_eq!(items[0].kind, AssetKind::Scene);
    assert!(!items[0].kind_from_send);
}

#[test]
fn a_game_projects_inbox_checks_use_the_folder_budget_of_the_asset() {
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
    fs::write(world.inbox().join(format!("{}.glb", job.id)), glb()).expect("write");
    let triangles_detail = || {
        let mut items = inbox::list(&world.root, &world.config).expect("inbox");
        jobs.link_inbox(&mut items);
        items[0]
            .checks
            .iter()
            .find(|c| c.id == "triangles")
            .and_then(|c| c.detail.clone())
            .expect("triangles row")
    };
    // A website keeps the flat Tris per model.
    assert!(
        triangles_detail().contains("60k per model budget"),
        "{}",
        triangles_detail()
    );
    project::modify(&world.root, &world.config.layout, &world.project, |m| {
        m.target = crate::settings::game::ProjectTarget::Game;
        let mut settings = crate::game::new_settings(
            &crate::settings::game::NewGame {
                format: crate::settings::game::GameFormat::Glb,
                models_dir: "assets/models".into(),
            },
            None,
        )?;
        settings.budgets = crate::game::default_budgets(&["props".into(), "characters".into()]);
        m.game = Some(Box::new(settings));
        Ok(())
    })
    .expect("game");
    // The prop maps to props/, so its budget is the props one.
    let detail = triangles_detail();
    assert_eq!(detail, "Within the props budget, 12k tris.");
}

#[test]
fn send_notes_read_the_pbr_and_texture_choices_of_either_service() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let mut request = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    let mut tripo = TripoPreset::prop();
    tripo.pbr = false;
    tripo.texture_quality = TripoTextureQuality::Detailed;
    request.tripo = Some(tripo);
    let job = jobs.send(&request, false, false).expect("send");
    let notes = super::jobs::send_notes(&job);
    assert_eq!(notes.maps, inbox::SentMaps::NoPbr);
    assert!(notes.large_textures);

    request.service = ProviderId::Meshy;
    let mut meshy = MeshyPreset::prop();
    meshy.enable_pbr = false;
    request.meshy = Some(meshy);
    let job = jobs.send(&request, false, false).expect("send");
    let notes = super::jobs::send_notes(&job);
    assert_eq!(notes.maps, inbox::SentMaps::NoPbr);
    assert!(!notes.large_textures);

    assert!(!notes.by_hand_tripo, "by hand on Meshy's site");

    let plain = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    let job = jobs.send(&plain, false, false).expect("send");
    let notes = super::jobs::send_notes(&job);
    assert_eq!(notes.maps, inbox::SentMaps::Any);
    assert!(notes.by_hand_tripo);
}

/// Adds props named `names` at Refs, each with its own ref image.
pub(super) fn add_objects(world: &World, names: &[&str]) -> Vec<String> {
    let refs = world.project_dir().join("refs");
    let mut ids = Vec::new();
    project::modify(&world.root, &world.config.layout, &world.project, |m| {
        for name in names {
            let asset = assets::add(
                m,
                NewAsset {
                    name: (*name).into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Refs,
                },
            )?;
            fs::write(
                refs.join(format!("{}.png", asset.id.replace('-', "_"))),
                b"\x89PNG\r\n\x1a\nfake",
            )
            .map_err(|e| crate::CoreError::io(&refs, e))?;
            ids.push(asset.id);
        }
        Ok(())
    })
    .expect("assets");
    ids
}

fn batch(world: &World, provider: Provider, ids: &[String]) -> BatchRequest {
    BatchRequest {
        project: world.project.clone(),
        provider,
        service: ProviderId::Tripo,
        items: ids
            .iter()
            .map(|id| BatchItem {
                asset: id.clone(),
                variant: PresetVariant::Prop,
                images: vec![format!("refs/{}.png", id.replace('-', "_"))],
                tripo: None,
                meshy: None,
                prompt: None,
                redo_of: None,
                studio: None,
            })
            .collect(),
        tripo_prop: None,
        tripo_character: None,
        meshy_prop: None,
        meshy_character: None,
        studio: None,
    }
}

#[test]
fn a_batch_item_can_carry_its_own_settings() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let ids = add_objects(&world, &["Desk lamp", "Plaster plinth", "Brush cup"]);
    let mut request = batch(&world, Provider::ByHand, &ids);
    let mut batch_wide = TripoPreset::prop();
    batch_wide.face_limit = 10_000;
    request.tripo_prop = Some(batch_wide.clone());
    let mut own = batch_wide.clone();
    own.face_limit = 2_000;
    own.pbr = false;
    request.items[1].tripo = Some(own);
    let sent = jobs.send_batch(&request, true, false).expect("batch");
    assert!(sent.failed.is_empty(), "{:?}", sent.failed);
    let tripo = |n: usize| sent.jobs[n].tripo.clone().expect("tripo settings");
    assert_eq!(tripo(0), batch_wide, "follows the batch settings");
    assert_eq!((tripo(1).face_limit, tripo(1).pbr), (2_000, false));
    assert_eq!(tripo(2), batch_wide);
}

#[test]
fn a_by_hand_batch_writes_one_handoff_per_asset() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let ids = add_objects(&world, &["Desk lamp", "Plaster plinth", "Brush cup"]);
    let mut request = batch(&world, Provider::ByHand, &ids);
    request.items[2].variant = PresetVariant::Character;
    let mut lowpoly = TripoPreset::prop();
    lowpoly.face_limit = 12_000;
    request.tripo_prop = Some(lowpoly);
    let sent = jobs.send_batch(&request, true, false).expect("batch");
    assert!(sent.failed.is_empty(), "{:?}", sent.failed);
    assert_eq!(sent.jobs.len(), 3);
    assert!(sent.batch.starts_with("batch-"));
    let names: Vec<&str> = sent.jobs.iter().map(|j| j.asset_name.as_str()).collect();
    assert_eq!(names, vec!["Desk lamp", "Plaster plinth", "Brush cup"]);
    for job in &sent.jobs {
        assert_eq!(job.state, JobState::Waiting, "By hand needs no approval");
        assert_eq!(job.batch.as_deref(), Some(sent.batch.as_str()));
        let folder = PathBuf::from(job.handoff.clone().expect("folder"));
        assert_eq!(folder, world.project_dir().join("handoff").join(&job.id));
        assert!(folder.join("front.png").is_file() && folder.join("settings.md").is_file());
    }
    let face = |job: &ProviderJob| job.tripo.as_ref().map(|t| t.face_limit);
    assert_eq!(face(&sent.jobs[0]), Some(12_000), "the batch settings");
    assert_eq!(sent.jobs[2].variant, PresetVariant::Character);
    assert_eq!(
        face(&sent.jobs[2]),
        Some(
            world
                .config
                .providers
                .tripo(PresetVariant::Character)
                .face_limit
        ),
        "a character starts from the character preset"
    );
    assert_eq!(jobs.batch(&sent.batch).len(), 3);
    let m = project::load(&world.root, &world.config.layout, &world.project).expect("load");
    for id in &ids {
        let a = assets::find(&m, id).expect("asset");
        assert_eq!((a.stage, a.status), (Stage::Tripo, AssetStatus::Queued));
    }

    // A zip from the web studio comes back for the second one.
    let zip = world.root.join("tripo_download.zip");
    {
        use std::io::Write as _;
        let mut writer = zip::ZipWriter::new(fs::File::create(&zip).expect("zip"));
        writer
            .start_file("model/plinth.glb", zip::write::SimpleFileOptions::default())
            .expect("entry");
        writer.write_all(&fixture::character(false)).expect("write");
        writer.finish().expect("finish");
    }
    let back = jobs.hand_back(&sent.jobs[1].id, &zip).expect("zip back");
    assert_eq!(back.state, JobState::Inbox);
    assert_eq!(back.inbox_file.as_deref(), Some("plaster-plinth-1.glb"));
    assert!(zip.is_file(), "the download stays where it was");
    assert_eq!(
        fs::read(world.inbox().join("plaster-plinth-1.glb")).expect("inbox"),
        fixture::character(false)
    );
}

#[test]
fn an_api_batch_is_checked_whole_and_waits_for_one_approval() {
    let world = World::new();
    let (server, _) = tripo_server(TripoFake::new());
    let jobs = world.jobs(&server);
    let ids = add_objects(&world, &["Desk lamp", "Laptop"]);
    let mut request = batch(&world, Provider::Tripo, &ids);
    request.items[1].images = vec!["refs/missing.png".into()];
    let err = jobs.send_batch(&request, true, true).expect_err("bad item");
    assert!(err.to_string().starts_with("Laptop: "), "{err}");
    assert!(jobs.list(None).is_empty(), "nothing went out");

    let mut twice = batch(&world, Provider::Tripo, &ids);
    twice.items.push(twice.items[0].clone());
    assert!(jobs.send_batch(&twice, true, true).is_err());
    assert!(
        jobs.send_batch(&batch(&world, Provider::Tripo, &[]), true, true)
            .is_err()
    );

    let sent = jobs
        .send_batch(&batch(&world, Provider::Tripo, &ids), true, true)
        .expect("batch");
    assert_eq!(sent.jobs.len(), 2);
    assert!(
        sent.jobs
            .iter()
            .all(|j| j.state == JobState::Approval && j.via_agent)
    );
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(server.requests().len(), 0, "nothing sent before approval");
    for job in &sent.jobs {
        jobs.approve(&job.id).expect("approve");
    }
    for job in &sent.jobs {
        assert_eq!(finished(&jobs, &job.id).state, JobState::Inbox);
    }
}

#[test]
fn check_downloads_offers_the_waiting_sends_of_the_project() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let ids = add_objects(&world, &["Desk lamp", "Brush cup"]);
    let sent = jobs
        .send_batch(&batch(&world, Provider::ByHand, &ids), false, false)
        .expect("batch");
    let downloads = world.root.join("fake-downloads");
    fs::create_dir_all(&downloads).expect("downloads");
    fs::write(downloads.join("desk_lamp_pbr.glb"), glb()).expect("glb");
    let scan = jobs.check_downloads(&world.project, &downloads);
    assert_eq!(scan.waiting.len(), 2);
    assert_eq!(scan.files.len(), 1);
    assert_eq!(scan.files[0].job.as_deref(), Some(sent.jobs[0].id.as_str()));
    let back = jobs
        .hand_back(&sent.jobs[0].id, std::path::Path::new(&scan.files[0].path))
        .expect("confirm");
    assert_eq!(back.state, JobState::Inbox);
    assert!(
        downloads.join("desk_lamp_pbr.glb").is_file(),
        "copied, not moved"
    );
    let again = jobs.check_downloads(&world.project, &downloads);
    assert_eq!(again.waiting.len(), 1);
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

#[test]
fn a_copied_asset_takes_its_finished_sends_along() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let jobs = world.jobs(&server);
    let mut request = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    request.service = ProviderId::Meshy;
    let stopped = jobs.send(&request, true, false).expect("send");
    jobs.cancel(&stopped.id).expect("cancel");
    let waiting = jobs.send(&request, true, false).expect("send again");
    assert_eq!(waiting.state, JobState::Waiting);

    let landing = project::create(
        &world.root,
        &world.config.layout,
        NewProject {
            name: "Maquette landing".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("landing");
    let copied = assets::copy::copy_to_project(
        &world.root,
        &world.config.layout,
        &world.project,
        &world.asset,
        &landing.id,
        &["refs/front.png".to_string()],
    )
    .expect("copy");
    let refs = vec![("refs/front.png".to_string(), "refs/front.png".to_string())];
    let count = jobs.copy_records(
        &world.project,
        &world.asset,
        &landing.id,
        &copied.asset,
        &refs,
    );
    assert_eq!(count, 1, "the waiting send stays where it is");

    let theirs = jobs.list(Some(&landing.id));
    assert_eq!(theirs.len(), 1);
    // Ids are unique across the workspace: the copy takes the next free one.
    assert_eq!(theirs[0].id, "stone-lantern-3");
    assert_eq!(theirs[0].asset, copied.asset.id);
    assert_eq!(theirs[0].state, JobState::Cancelled);
    assert_eq!(theirs[0].images, vec!["refs/front.png"]);
    assert_eq!(jobs.list(Some(&world.project)).len(), 2);
    let saved = fs::read_to_string(
        project::dir(&world.root, &world.config.layout, &landing.id)
            .expect("dir")
            .join("providers.json"),
    )
    .expect("saved");
    assert!(
        saved.contains("\"project\": \"maquette-landing\""),
        "{saved}"
    );
}

/// The ledger once `n` sends in it are settled; the line lands just after
/// the job's state does.
fn settled(jobs: &ProviderJobs, n: usize) -> Ledger {
    let end = std::time::Instant::now() + WAIT;
    loop {
        let ledger = jobs.ledger().expect("ledger");
        let done = ledger
            .spends
            .iter()
            .filter(|s| s.outcome != SpendOutcome::Pending)
            .count();
        if done >= n {
            return ledger;
        }
        assert!(
            std::time::Instant::now() < end,
            "ledger never settled: {ledger:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_paid_send_goes_into_the_credits_ledger_with_what_it_cost() {
    let world = World::new();
    let (server, _) = tripo_server(TripoFake::new());
    let jobs = world.jobs(&server);
    let request = world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]);
    assert_eq!(jobs.estimate(&request).expect("estimate"), Some(30.0));
    let job = jobs.send(&request, false, false).expect("send");
    assert_eq!(job.estimate, Some(30.0));
    assert_eq!(finished(&jobs, &job.id).state, JobState::Inbox);

    // By hand is written as subscription spend, apart from the API's.
    let by_hand = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    let hand = jobs.send(&by_hand, false, false).expect("by hand");

    let ledger = settled(&jobs, 1);
    assert_eq!(ledger.spends.len(), 2, "{ledger:?}");
    let web = ledger
        .spends
        .iter()
        .find(|s| s.source == SpendSource::Subscription)
        .expect("subscription spend");
    assert_eq!(web.job, hand.id);
    assert_eq!((web.outcome, web.credits), (SpendOutcome::Pending, 30.0));
    let spend = ledger
        .spends
        .iter()
        .find(|s| s.source == SpendSource::Api)
        .expect("api spend");
    assert_eq!(spend.job, job.id);
    assert_eq!(spend.service, ProviderId::Tripo);
    assert_eq!(spend.task.as_deref(), Some("task_model"));
    assert_eq!(spend.outcome, SpendOutcome::Succeeded);
    assert_eq!(spend.credits, 30.0);
    assert!(!spend.estimated);
    assert!(spend.summary.starts_with("Tripo H3.1"));
    let text = fs::read_to_string(world.root.join(ledger::LEDGER_FILE)).expect("file");
    assert_eq!(
        text.lines().count(),
        3,
        "one sent and one settled line, and the By hand sent line"
    );
    assert!(!text.contains("tsk_test_key"));
}

#[test]
fn a_failed_or_refunded_send_is_charged_nothing() {
    let world = World::new();
    let mut fake = TripoFake::new();
    fake.fail = Some("The image has no clear subject.");
    let (server, _) = tripo_server(fake);
    let jobs = world.jobs(&server);
    let request = world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]);
    let job = jobs.send(&request, false, false).expect("send");
    assert_eq!(finished(&jobs, &job.id).state, JobState::Failed);
    let spend = settled(&jobs, 1).spends[0].clone();
    assert_eq!(spend.outcome, SpendOutcome::Failed);
    assert_eq!(spend.credits, 0.0);
    jobs.close();

    let (server, _) = meshy_server(true);
    let jobs = world.jobs(&server);
    let request = world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"]);
    let job = jobs.send(&request, false, false).expect("send");
    let end = std::time::Instant::now() + WAIT;
    while jobs.get(&job.id).and_then(|j| j.remote_id).is_none() {
        assert!(std::time::Instant::now() < end, "never sent");
        std::thread::sleep(Duration::from_millis(5));
    }
    jobs.cancel(&job.id).expect("cancel");
    let ledger = settled(&jobs, 2);
    let spend = ledger
        .spends
        .iter()
        .find(|s| s.job == job.id)
        .expect("meshy spend");
    assert_eq!(spend.outcome, SpendOutcome::Refunded);
    assert_eq!(spend.credits, 0.0);
    jobs.close();
}

#[test]
fn an_approval_that_is_denied_never_reaches_the_ledger() {
    let world = World::new();
    let (server, _) = tripo_server(TripoFake::new());
    let jobs = world.jobs(&server);
    let request = world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]);
    let job = jobs.send(&request, true, true).expect("send");
    assert_eq!(job.state, JobState::Approval);
    jobs.deny(&job.id).expect("deny");
    assert!(jobs.ledger().expect("ledger").spends.is_empty());
    assert_eq!(server.requests().len(), 0);
}

#[test]
fn a_redo_sends_the_asset_again_as_a_new_version_and_says_so() {
    let world = World::new();
    let (server, _) = tripo_server(TripoFake::new());
    let jobs = world.jobs(&server);
    let first = jobs
        .send(
            &world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]),
            false,
            false,
        )
        .expect("send");
    assert_eq!(finished(&jobs, &first.id).state, JobState::Inbox);

    let mut redo = world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]);
    redo.redo_of = Some("stone-lantern-9".into());
    let err = jobs.send(&redo, false, false).expect_err("unknown send");
    assert!(err.to_string().contains("no send stone-lantern-9"), "{err}");

    redo.redo_of = Some(first.id.clone());
    let second = jobs.send(&redo, false, false).expect("redo");
    assert_eq!(second.id, format!("{}-2", world.asset));
    assert_eq!(second.redo_of.as_deref(), Some(first.id.as_str()));
    assert_eq!(finished(&jobs, &second.id).state, JobState::Inbox);

    // Both models wait side by side; the new one names the send it redid.
    let mut items = inbox::list(&world.root, &world.config).expect("inbox");
    jobs.link_inbox(&mut items);
    assert_eq!(items.len(), 2);
    let newer = items
        .iter()
        .find(|i| i.id == "stone-lantern-2.glb")
        .expect("redo model");
    assert_eq!(
        newer.link.as_ref().and_then(|l| l.redo_of.as_deref()),
        Some(first.id.as_str())
    );
    let ledger = settled(&jobs, 2);
    assert_eq!(ledger.spends.len(), 2);
    assert_eq!(ledger.spends[0].redo_of.as_deref(), Some(first.id.as_str()));
    assert_eq!(ledger.spends[1].redo_of, None);
}

#[test]
fn the_texture_prompt_goes_to_meshy_and_tripo_refuses_one() {
    let world = World::new();
    let (server, _) = meshy_server(false);
    let jobs = world.jobs(&server);
    let mut request = world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"]);
    request.prompt = Some("  weathered granite, moss in the cracks  ".into());
    let job = jobs.send(&request, false, false).expect("send");
    assert_eq!(
        job.prompt.as_deref(),
        Some("weathered granite, moss in the cracks")
    );
    assert_eq!(finished(&jobs, &job.id).state, JobState::Inbox);
    let body = server.find("POST", "/image-to-3d").expect("create").json();
    assert_eq!(
        body["texture_prompt"],
        "weathered granite, moss in the cracks"
    );

    request.prompt = Some("x".repeat(801));
    let err = jobs.send(&request, false, false).expect_err("too long");
    assert!(err.to_string().contains("800 characters"), "{err}");

    let mut tripo = world.request(Provider::Tripo, PresetVariant::Prop, &["front.png"]);
    tripo.prompt = Some("granite".into());
    let err = jobs.send(&tripo, false, false).expect_err("tripo");
    assert!(err.to_string().contains("Only Meshy"), "{err}");
    tripo.prompt = Some("   ".into());
    assert!(jobs.prepare(&tripo).is_ok(), "a blank prompt is no prompt");

    // By hand for Meshy puts it on the settings card.
    let mut by_hand = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    by_hand.service = ProviderId::Meshy;
    by_hand.prompt = Some("weathered granite".into());
    let card = jobs
        .send(&by_hand, false, false)
        .expect("by hand")
        .card
        .expect("card");
    assert!(
        card.rows
            .iter()
            .any(|r| r.label == "Texture prompt" && r.value == "weathered granite")
    );
}

#[test]
fn balances_ask_each_provider_that_has_a_key() {
    let server = FakeServer::start(|_, req| match req.path.as_str() {
        "/account/balance" => tripo_data(json!({ "balance": 1240.5, "frozen": 30 })),
        "/balance" => Reply::ok(&json!({ "balance": 380, "monthly_credits": 200 })),
        _ => Reply::json(404, &json!({})),
    });
    let world = World::new();
    let jobs = world.jobs(&server);
    let found = jobs.balances();
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].service, ProviderId::Tripo);
    assert_eq!(found[0].credits, Some(1240.5));
    assert_eq!(found[0].held, Some(30.0));
    assert_eq!(found[1].credits, Some(380.0));
    assert_eq!(found[1].held, None);
    assert_eq!(
        server
            .find("GET", "/balance")
            .and_then(|r| r.header("authorization").map(str::to_string)),
        Some("Bearer msy_test_key_0002".to_string())
    );

    // No key: not called. A refused key: says why.
    world.secrets.remove(ProviderId::Meshy).expect("remove");
    let bad = FakeServer::start(|_, _| Reply::json(401, &json!({ "message": "bad key" })));
    let jobs = world.jobs(&bad);
    let found = jobs.balances();
    assert!(!found[1].key);
    assert_eq!(found[1].credits, None);
    assert!(
        found[0]
            .error
            .as_deref()
            .is_some_and(|e| e.contains("did not accept the API key"))
    );
    assert_eq!(bad.requests().len(), 1);
}

#[test]
fn sends_at_once_never_share_an_id_or_both_pass_the_busy_check() {
    let world = World::new();
    let (server, _) = tripo_server(TripoFake::new());
    let jobs = world.jobs(&server);
    let send = |provider| {
        let request = world.request(provider, PresetVariant::Prop, &["front.png"]);
        std::thread::scope(|scope| {
            let runs: Vec<_> = (0..6)
                .map(|_| scope.spawn(|| jobs.send(&request, true, true)))
                .collect();
            runs.into_iter()
                .map(|run| run.join().expect("thread"))
                .collect::<Vec<_>>()
        })
    };

    // Six agents send the same asset to Tripo: one gets through, the rest
    // hear it is on its way.
    let tripo = send(Provider::Tripo);
    assert_eq!(tripo.iter().filter(|r| r.is_ok()).count(), 1, "{tripo:?}");
    assert!(
        tripo
            .iter()
            .filter_map(|r| r.as_ref().err())
            .all(|e| e.to_string().contains("already on its way"))
    );

    // By hand never counts as busy, so every send goes in, each with its own id.
    let by_hand = send(Provider::ByHand);
    let mut ids: Vec<String> = by_hand
        .into_iter()
        .map(|r| r.expect("by hand").id)
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 6, "{ids:?}");
    assert_eq!(server.requests().len(), 0, "nothing approved, nothing sent");
    jobs.close();
}
