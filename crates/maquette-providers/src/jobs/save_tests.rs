//! Tests for saving `providers.json` while the workspace closes: a change
//! and its write land together, before or after close, never around it.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

use super::tests::meshy_job;
use super::worker::Run;
use super::{JOBS_FILE, JobKey, Options, ProviderJobs, update};
use super::{Provider, SendRequest};
use crate::error::CoreResult;
use crate::presets::{PresetVariant, ProviderId};
use crate::secrets::MemoryStore;
use maquette_base::ports::TextureBake;
use maquette_base::tools::Toolbox;
use maquette_model::assets::{self, AssetKind, NewAsset, Stage};
use maquette_model::project::{self, NewProject, ProjectTemplate};
use maquette_model::sends::{InboxAim, InboxPort, SendPorts};
use maquette_model::workspace::{self, WorkspaceConfig};

/// Ports no test here reaches.
struct Unused;

impl InboxPort for Unused {
    fn set_kind(&self, _: &Path, _: &WorkspaceConfig, _: &str, _: AssetKind) -> CoreResult<()> {
        Ok(())
    }

    fn set_aim(
        &self,
        _: &Path,
        _: &WorkspaceConfig,
        _: &str,
        _: Option<InboxAim>,
    ) -> CoreResult<()> {
        Ok(())
    }

    fn path_of(&self, _: &Path, _: &WorkspaceConfig, file: &str) -> CoreResult<String> {
        Ok(file.to_string())
    }
}

impl TextureBake for Unused {
    fn bake(&self, _: &Path, _: &Path, _: &Path, _: &Toolbox) -> CoreResult<PathBuf> {
        panic!("nothing here bakes");
    }
}

/// Jobs on a fresh workspace whose project `moon-island` has one saved
/// finished Meshy job, `lantern-1`. Returns the jobs, the workspace config
/// and the project's jobs file.
fn one_job(root: &Path) -> (ProviderJobs, WorkspaceConfig, PathBuf) {
    let (config, path) = workspace_with_job(root);
    let unused = Arc::new(Unused);
    let jobs = ProviderJobs::new(
        Arc::new(MemoryStore::default()),
        Options::default(),
        SendPorts {
            inbox: Arc::clone(&unused) as Arc<dyn InboxPort>,
            bake: unused,
        },
        |_| {},
    );
    jobs.open_workspace(root, &config);
    assert_eq!(jobs.list(None).len(), 1, "the saved job loaded");
    (jobs, config, path)
}

/// Makes a workspace at `root` with the saved job of [`one_job`].
fn workspace_with_job(root: &Path) -> (WorkspaceConfig, PathBuf) {
    let config = workspace::create(root, &WorkspaceConfig::default()).expect("workspace");
    let manifest = project::create(
        root,
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
    let mut stored = meshy_job("failed", json!({}));
    stored.job.project.clone_from(&manifest.id);
    let path = project::dir(root, &config.layout, &manifest.id)
        .expect("project dir")
        .join(JOBS_FILE);
    let file = json!({ "version": 1, "jobs": [stored] });
    fs::write(&path, file.to_string()).expect("jobs file");
    (config, path)
}

fn saved_ids(path: &Path) -> Vec<String> {
    let file: Value = serde_json::from_str(&fs::read_to_string(path).expect("read")).expect("json");
    file["jobs"]
        .as_array()
        .expect("jobs")
        .iter()
        .map(|s| s["job"]["id"].as_str().expect("id").to_string())
        .collect()
}

/// The saved job of [`one_job`]: `lantern-1` of `moon-island`.
fn lantern() -> JobKey {
    JobKey::new("moon-island", "lantern-1")
}

/// How long a test waits on a thread before it fails instead of hanging.
const WAIT: Duration = Duration::from_secs(5);

/// Waits for one message, panicking with what never came after `WAIT`.
fn wait<T>(rx: &mpsc::Receiver<T>, what: &str) -> T {
    rx.recv_timeout(WAIT)
        .unwrap_or_else(|err| panic!("timed out after {WAIT:?} waiting for {what}: {err}"))
}

#[test]
fn closing_while_a_job_saves_keeps_its_saved_send() {
    let root = tempfile::tempdir().expect("dir");
    let (jobs, _, path) = one_job(root.path());
    assert_eq!(saved_ids(&path), ["lantern-1"]);

    // The save parks right after its change until the test lets it go.
    let (changed_tx, changed) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    *jobs.shared.before_save.lock().expect("hook") = Some(Box::new(move || {
        changed_tx.send(()).expect("changed");
        wait(&release_rx, "the test to release the parked save");
    }));
    let saving = {
        let shared = Arc::clone(&jobs.shared);
        std::thread::spawn(move || {
            update(&shared, &lantern(), true, |s| {
                s.chain.credits.insert("m-model".to_string(), 20.0);
                Ok(())
            })
        })
    };
    wait(&changed, "the save to change the job");

    // close() runs on another thread while the save is parked.
    let (started_tx, started) = mpsc::channel();
    let (done_tx, done) = mpsc::channel();
    let closing = {
        let jobs = jobs.clone();
        std::thread::spawn(move || {
            started_tx.send(()).expect("started");
            jobs.close();
            done_tx.send(()).expect("done");
        })
    };
    wait(&started, "close() to start");
    let finished_early = done.recv_timeout(Duration::from_millis(200)).is_ok();
    release.send(()).expect("release");
    saving.join().expect("save thread").expect("saved");
    closing.join().expect("close thread");
    assert!(
        !finished_early,
        "close() finished while a save was between its change and its write"
    );

    assert_eq!(
        saved_ids(&path),
        ["lantern-1"],
        "close keeps the saved state"
    );
    let file: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    assert_eq!(file["jobs"][0]["chain"]["credits"]["m-model"], json!(20.0));

    // A job thread that saves after close finds nothing and writes nothing.
    assert!(update(&jobs.shared, &lantern(), true, |_| Ok(())).is_err());
    assert_eq!(saved_ids(&path), ["lantern-1"]);
}

#[test]
fn a_saved_change_is_on_disk_before_anyone_reads_it() {
    let root = tempfile::tempdir().expect("dir");
    let (jobs, _, path) = one_job(root.path());

    // The save parks right after its change until the test lets it go.
    let (changed_tx, changed) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    *jobs.shared.before_save.lock().expect("hook") = Some(Box::new(move || {
        changed_tx.send(()).expect("changed");
        wait(&release_rx, "the test to release the parked save");
    }));
    let saving = {
        let shared = Arc::clone(&jobs.shared);
        std::thread::spawn(move || {
            update(&shared, &lantern(), true, |s| {
                s.job.message = Some("on disk first".to_string());
                Ok(())
            })
        })
    };
    wait(&changed, "the save to change the job");

    // A reader while the save is parked, as a finished job is read and
    // another open of the workspace then loads the file.
    let (read_tx, read) = mpsc::channel();
    let reader = {
        let jobs = jobs.clone();
        let path = path.clone();
        std::thread::spawn(move || {
            let message = jobs.get("moon-island", "lantern-1").expect("job").message;
            let saved = fs::read_to_string(&path).expect("read");
            read_tx.send((message, saved)).expect("read");
        })
    };
    let early = read.recv_timeout(Duration::from_millis(200));
    release.send(()).expect("release");
    saving.join().expect("save thread").expect("saved");
    reader.join().expect("reader");
    let (message, saved) = early.unwrap_or_else(|_| wait(&read, "the reader's result"));

    assert_eq!(message.as_deref(), Some("on disk first"));
    assert!(
        saved.contains("on disk first"),
        "the job read as changed while its file still had the old state: {saved}"
    );
}

#[test]
fn clearing_finished_jobs_after_close_writes_nothing() {
    let root = tempfile::tempdir().expect("dir");
    let (jobs, _, path) = one_job(root.path());
    jobs.close();
    let cleared = jobs.clear_finished("moon-island");
    assert_eq!(saved_ids(&path), ["lantern-1"], "the saved job is kept");
    assert!(
        format!("{cleared:?}").contains("Open a workspace"),
        "{cleared:?}"
    );
}

#[test]
fn a_job_thread_of_a_closed_workspace_cannot_save_onto_the_next_one() {
    let first = tempfile::tempdir().expect("dir");
    let (jobs, _, _) = one_job(first.path());
    // A job thread of the first workspace, still going after the switch.
    let key = lantern();
    let stale = Run::current(&jobs.shared, &key);

    // A copy of the workspace, so its job has the same id.
    let second = tempfile::tempdir().expect("dir");
    let (config, path) = workspace_with_job(second.path());
    jobs.open_workspace(second.path(), &config);
    stale.save(|s| s.job.message = Some("from the old workspace".to_string()));

    let job = jobs
        .get("moon-island", "lantern-1")
        .expect("the new workspace's job");
    assert_ne!(job.message.as_deref(), Some("from the old workspace"));
    let saved = fs::read_to_string(&path).expect("read");
    assert!(!saved.contains("from the old workspace"), "{saved}");
}

/// Adds the asset `stone-lantern` with a front ref to `moon-island`.
fn add_asset(root: &Path, config: &WorkspaceConfig) -> SendRequest {
    let mut asset = String::new();
    project::modify(root, &config.layout, "moon-island", |m| {
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
    let refs = project::dir(root, &config.layout, "moon-island")
        .expect("dir")
        .join("refs");
    fs::create_dir_all(&refs).expect("refs");
    fs::write(refs.join("front.png"), b"\x89PNG\r\n\x1a\nfake").expect("image");
    SendRequest {
        project: "moon-island".to_string(),
        asset,
        provider: Provider::ByHand,
        service: ProviderId::Tripo,
        variant: PresetVariant::Prop,
        images: vec!["refs/front.png".to_string()],
        tripo: None,
        meshy: None,
        prompt: None,
        redo_of: None,
        studio: None,
    }
}

#[test]
fn a_send_read_before_a_switch_does_not_queue_in_the_next_workspace() {
    let first = tempfile::tempdir().expect("dir");
    let (jobs, config, _) = one_job(first.path());
    let request = add_asset(first.path(), &config);
    let second = tempfile::tempdir().expect("dir");
    let (next, path) = workspace_with_job(second.path());
    add_asset(second.path(), &next);

    // The workspace switches after the send read the first one.
    let switch = jobs.clone();
    let root = second.path().to_path_buf();
    *jobs.shared.before_queue.lock().expect("hook") = Some(Box::new(move || {
        switch.open_workspace(&root, &next);
    }));
    let sent = jobs.send(&request, false, false);

    assert!(sent.is_err(), "{sent:?}");
    assert_eq!(saved_ids(&path), ["lantern-1"], "nothing new saved there");
    assert_eq!(jobs.list(None).len(), 1, "nothing new queued there");
}
