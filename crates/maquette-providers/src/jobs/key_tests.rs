//! Tests for what follows from numbering ids per project: which job an
//! inbox file belongs to, where a job keeps its files, how a saved record
//! says which numbering it is from, and ids made unique inside one project.

use std::fs;

use serde_json::{Value, json};

use super::ids::{IdScope, make_unique};
use super::owner::{owner_of, saved_owner};
use super::records::write_inbox;
use super::split_files::{SOURCE_FILE, files_dir, source_of};
use super::tests::meshy_job;
use super::{JobState, Provider, ProviderJob, Stored, Workspace};
use maquette_model::workspace::WorkspaceConfig;

/// A job `id` of `project` in `state`, by hand on Tripo unless said.
fn job(project: &str, id: &str, state: JobState) -> ProviderJob {
    let mut stored = meshy_job("queued", json!({}));
    stored.job.project = project.into();
    stored.job.id = id.into();
    stored.job.asset = id.rsplit_once('-').map_or(id, |(asset, _)| asset).into();
    stored.job.provider = Provider::ByHand;
    stored.job.service = crate::presets::ProviderId::Tripo;
    stored.job.state = state;
    stored.job
}

/// `job`, with the model it saved in the inbox as `file` at `at_ms`.
fn saved(mut job: ProviderJob, file: &str, at_ms: f64) -> ProviderJob {
    job.inbox_file = Some(file.into());
    job.finished_at_ms = Some(at_ms);
    job
}

/// `job`, handed out with the export name `name`.
fn handed(mut job: ProviderJob, name: &str) -> ProviderJob {
    job.card = Some(crate::handoff::HandoffCard {
        service: job.service,
        site: String::new(),
        mode: "Image to 3D".into(),
        rows: Vec::new(),
        images: Vec::new(),
        return_name: format!("{name}.glb"),
        export_name: name.into(),
        credits: None,
        credits_note: None,
        prompt: None,
    });
    job
}

fn key(job: Option<&ProviderJob>) -> Option<(&str, &str)> {
    job.map(|j| (j.project.as_str(), j.id.as_str()))
}

#[test]
fn the_job_whose_model_is_in_the_inbox_owns_the_file_not_one_that_had_the_name() {
    // moon's model was accepted, so the name was free for bay's.
    let jobs = [
        saved(
            job("moon", "lamp-1", JobState::Accepted),
            "lamp-1.glb",
            10.0,
        ),
        saved(job("bay", "lamp-1", JobState::Inbox), "lamp-1.glb", 20.0),
        saved(
            job("cove", "lamp-1", JobState::Cancelled),
            "lamp-1.glb",
            30.0,
        ),
    ];
    assert_eq!(
        key(saved_owner("lamp-1.glb", jobs.iter())),
        Some(("bay", "lamp-1"))
    );
    assert_eq!(
        key(owner_of("lamp-1.glb", jobs.iter())),
        Some(("bay", "lamp-1"))
    );
    assert_eq!(key(saved_owner("lamp-2.glb", jobs.iter())), None);
    // Two that say the model is theirs (one file was deleted by hand): the
    // one that saved it last has it.
    let both = [
        saved(job("moon", "lamp-1", JobState::Inbox), "lamp-1.glb", 10.0),
        saved(job("bay", "lamp-1", JobState::Inbox), "lamp-1.glb", 20.0),
    ];
    assert_eq!(
        key(saved_owner("lamp-1.glb", both.iter())),
        Some(("bay", "lamp-1"))
    );
}

#[test]
fn a_file_saved_by_hand_is_one_sends_or_nobodys() {
    let waiting = |project: &str| job(project, "lamp-1", JobState::Waiting);
    // One send waits under the id: the file is its model.
    let one = [waiting("moon"), job("bay", "lamp-1", JobState::Accepted)];
    assert_eq!(
        key(owner_of("Lamp_1_final.glb", one.iter())),
        Some(("moon", "lamp-1"))
    );
    // Two wait under it, in two projects: no guess.
    let two = [waiting("moon"), waiting("bay")];
    assert_eq!(key(owner_of("lamp-1.glb", two.iter())), None);
    // None waits, one has its model in the inbox under another name.
    let landed = [
        job("moon", "lamp-1", JobState::Accepted),
        job("bay", "lamp-1", JobState::Inbox),
    ];
    assert_eq!(
        key(owner_of("lamp-1-b.glb", landed.iter())),
        Some(("bay", "lamp-1"))
    );
    // Only finished ones are left: nobody's, also when one job alone has
    // the id (another project's send of it may be gone with its project).
    let over = [
        job("moon", "lamp-1", JobState::Accepted),
        job("bay", "lamp-1", JobState::Cancelled),
    ];
    assert_eq!(key(owner_of("lamp-1.glb", over.iter())), None);
    assert_eq!(key(owner_of("lamp-1.glb", over[..1].iter())), None);
    assert_eq!(key(owner_of("lamp-1.glb", over[1..].iter())), None);
    assert_eq!(key(owner_of("crate-1.glb", two.iter())), None);
}

#[test]
fn an_export_name_says_which_projects_send_a_file_is_for() {
    let jobs = [
        handed(
            job("moon", "lamp-1", JobState::Waiting),
            "maq_moon_lamp_v1_tripo",
        ),
        handed(
            job("bay", "lamp-1", JobState::Waiting),
            "maq_bay_lamp_v1_tripo",
        ),
        // Handed out before the project was in the name.
        handed(
            job("cove", "lamp-7", JobState::Waiting),
            "maq_lamp_v7_tripo",
        ),
        // A card from before export names: the older form, from its id.
        job("cove", "crate-2", JobState::Waiting),
    ];
    let owner = |file: &str| key(owner_of(file, jobs.iter()));
    assert_eq!(owner("maq_bay_lamp_v1_tripo.glb"), Some(("bay", "lamp-1")));
    assert_eq!(
        owner("MAQ_Moon_Lamp_V1_Tripo (2).glb"),
        Some(("moon", "lamp-1"))
    );
    assert_eq!(owner("maq_lamp_v7_tripo.glb"), Some(("cove", "lamp-7")));
    assert_eq!(owner("maq_crate_v2_tripo.glb"), Some(("cove", "crate-2")));
    // Named for no send on record, or for the older form of a newer send.
    assert_eq!(owner("maq_reef_lamp_v1_tripo.glb"), None);
    assert_eq!(owner("maq_lamp_v1_tripo.glb"), None);
    assert_eq!(owner("maq_bay_lamp_v1_meshy.glb"), None);
}

#[test]
fn a_send_waits_for_the_name_on_its_card_and_an_api_send_for_none() {
    let by_hand = job("moon", "lamp-3", JobState::Waiting);
    assert_eq!(
        by_hand.export_name().as_deref(),
        Some("maq_lamp_v3_tripo"),
        "no card name: the older form"
    );
    let named = handed(by_hand, "maq_moon_lamp_v3_tripo");
    assert_eq!(
        named.export_name().as_deref(),
        Some("maq_moon_lamp_v3_tripo")
    );
    assert_eq!(meshy_job("queued", json!({})).job.export_name(), None);
}

fn workspace(root: &std::path::Path) -> Workspace {
    Workspace {
        root: root.to_path_buf(),
        config: WorkspaceConfig::default(),
        generation: 1,
    }
}

/// A saved split job `lamp-1` of `project`, numbered the older way or not.
fn split(project: &str, scope: IdScope) -> Stored {
    let mut stored = meshy_job("accepted", json!({}));
    stored.job.project = project.into();
    stored.job.id = "lamp-1".into();
    stored.job.split = Some(super::SplitJob::default());
    stored.scope = scope;
    stored
}

#[test]
fn a_job_keeps_its_files_under_its_project_and_an_older_one_where_it_put_them() {
    let dir = tempfile::tempdir().expect("dir");
    let ws = workspace(dir.path());
    let cache = dir.path().join(&ws.config.layout.cache);
    let older_dir = cache.join("splits").join("lamp-1");
    let newer = split("bay", IdScope::Project);
    let older = split("moon", IdScope::Workspace);
    assert_eq!(files_dir(&ws, &older), older_dir);
    assert_eq!(
        files_dir(&ws, &newer),
        cache.join("sends").join("bay").join("lamp-1")
    );

    // The older job's copy is there. A job numbered in its project never
    // reads it, also with its own copy gone: it is another project's model.
    fs::create_dir_all(&older_dir).expect("mkdir");
    fs::write(older_dir.join(SOURCE_FILE), b"glTF moon").expect("copy");
    assert_eq!(source_of(&ws, &older), Some(older_dir.join(SOURCE_FILE)));
    assert_eq!(source_of(&ws, &newer), None);
    let own = cache.join("sends").join("bay").join("lamp-1");
    fs::create_dir_all(&own).expect("mkdir");
    fs::write(own.join(SOURCE_FILE), b"glTF bay").expect("copy");
    assert_eq!(source_of(&ws, &newer), Some(own.join(SOURCE_FILE)));

    // A record that lost its scope (an older Maquette saved the file in
    // between) still finds the folder of its project and id first.
    let lost = split("bay", IdScope::Workspace);
    assert_eq!(files_dir(&ws, &lost), own);
    assert_eq!(source_of(&ws, &lost), Some(own.join(SOURCE_FILE)));
}

#[test]
fn a_saved_record_says_its_numbering_only_when_it_is_per_project() {
    // As an older Maquette wrote it: no scope, and none is added.
    let older = meshy_job("failed", json!({}));
    assert_eq!(older.scope, IdScope::Workspace);
    let text = serde_json::to_string(&older).expect("json");
    assert!(!text.contains("scope"), "{text}");
    // A job sent now says so, and reads back the same.
    let newer = Stored::new(older.job.clone());
    let value: Value = serde_json::to_value(&newer).expect("json");
    assert_eq!(value["scope"], json!("project"));
    let back: Stored = serde_json::from_value(value).expect("stored");
    assert_eq!(back.scope, IdScope::Project);
    assert_eq!(back, newer);
}

#[test]
fn two_records_of_one_project_sharing_an_id_get_ids_of_their_own() {
    let record = |state: &str, created: f64| {
        let mut stored = meshy_job(state, json!({}));
        stored.job.created_at_ms = created;
        stored
    };
    // The one waiting in the inbox keeps the id, though it is the newer.
    let mut jobs = vec![record("failed", 1.0), record("inbox", 2.0)];
    assert!(make_unique(&mut jobs, &[]));
    let ids: Vec<&str> = jobs.iter().map(|s| s.job.id.as_str()).collect();
    assert_eq!(ids, ["lantern-2", "lantern-1"]);
    assert!(!make_unique(&mut jobs, &[]), "nothing left to change");

    // With neither pinned the older keeps it, and a number a handoff
    // folder still holds is passed over.
    let mut jobs = vec![record("cancelled", 5.0), record("failed", 3.0)];
    assert!(make_unique(&mut jobs, &["lantern-2".to_string()]));
    let ids: Vec<&str> = jobs.iter().map(|s| s.job.id.as_str()).collect();
    assert_eq!(ids, ["lantern-3", "lantern-1"]);

    // Ids that differ are left as they are, whatever their numbers.
    let mut apart = vec![record("failed", 1.0), record("failed", 2.0)];
    apart[1].job.id = "lantern-9".into();
    assert!(!make_unique(&mut apart, &[]));
}

#[test]
fn models_landing_at_once_under_one_id_each_get_a_file_of_their_own() {
    use std::sync::{Arc, Barrier};

    // Two projects' jobs of one id want the same inbox name. Landing
    // together, each must still get its own file with its own model.
    const WRITERS: usize = 8;
    let dir = tempfile::tempdir().expect("dir");
    let ws = Arc::new(workspace(dir.path()));
    let inbox = maquette_model::sends::inbox_dir(&ws.root, &ws.config);
    for round in 0..40 {
        let id = format!("lamp-{round}");
        let start = Arc::new(Barrier::new(WRITERS));
        let writers: Vec<_> = (0..WRITERS)
            .map(|n| {
                let (ws, start, id) = (Arc::clone(&ws), Arc::clone(&start), id.clone());
                std::thread::spawn(move || {
                    let model = format!("glTF model {n}").into_bytes();
                    start.wait();
                    let name = write_inbox(&ws, &id, &model).expect("written");
                    (name, model)
                })
            })
            .collect();
        let mut names = Vec::new();
        for writer in writers {
            let (name, model) = writer.join().expect("writer");
            assert_eq!(
                fs::read(inbox.join(&name)).expect("file"),
                model,
                "{name} holds another job's model"
            );
            names.push(name);
        }
        names.sort();
        names.dedup();
        assert_eq!(names.len(), WRITERS, "round {round}: two jobs got one name");
    }
}
