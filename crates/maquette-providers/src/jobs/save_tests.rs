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
use super::{JOBS_FILE, Options, ProviderJobs, update};
use crate::error::CoreResult;
use crate::secrets::MemoryStore;
use maquette_base::ports::TextureBake;
use maquette_base::tools::Toolbox;
use maquette_model::assets::AssetKind;
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

#[test]
fn closing_while_a_job_saves_keeps_its_saved_send() {
    let root = tempfile::tempdir().expect("dir");
    let (jobs, _, path) = one_job(root.path());
    assert_eq!(saved_ids(&path), ["lantern-1"]);

    // The save stops right after its change and waits for close to finish,
    // or gives up waiting when close cannot finish until the save does.
    let (changed_tx, changed) = mpsc::channel();
    let (closed, closed_rx) = mpsc::channel::<()>();
    *jobs.shared.before_save.lock().expect("hook") = Some(Box::new(move || {
        changed_tx.send(()).expect("changed");
        let _ = closed_rx.recv_timeout(Duration::from_millis(300));
    }));
    let saving = {
        let shared = Arc::clone(&jobs.shared);
        std::thread::spawn(move || {
            update(&shared, "lantern-1", true, |s| {
                s.chain.credits.insert("m-model".to_string(), 20.0);
                Ok(())
            })
        })
    };
    changed.recv().expect("the save changed the job");
    jobs.close();
    let _ = closed.send(());
    saving.join().expect("save thread").expect("saved");

    assert_eq!(
        saved_ids(&path),
        ["lantern-1"],
        "close keeps the saved state"
    );
    let file: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    assert_eq!(file["jobs"][0]["chain"]["credits"]["m-model"], json!(20.0));

    // A job thread that saves after close finds nothing and writes nothing.
    assert!(update(&jobs.shared, "lantern-1", true, |_| Ok(())).is_err());
    assert_eq!(saved_ids(&path), ["lantern-1"]);
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
    let stale = Run::current(&jobs.shared, "lantern-1");

    // A copy of the workspace, so its job has the same id.
    let second = tempfile::tempdir().expect("dir");
    let (config, path) = workspace_with_job(second.path());
    jobs.open_workspace(second.path(), &config);
    stale.save(|s| s.job.message = Some("from the old workspace".to_string()));

    let job = jobs.get("lantern-1").expect("the new workspace's job");
    assert_ne!(job.message.as_deref(), Some("from the old workspace"));
    let saved = fs::read_to_string(&path).expect("read");
    assert!(!saved.contains("from the old workspace"), "{saved}");
}
