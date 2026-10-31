//! The pack service without Blender: what it refuses, decisions on a pack
//! that waits, a pack the app closed on, and going on from a stop.

use std::path::Path;
use std::sync::{Arc, Mutex};

use super::record::{self, PackItemState, PackState};
use super::tests::done_record;
use super::*;
use crate::pipeline::CompressDefaults;
use crate::pipeline::runner::{NullSink, PipelineRunner};
use crate::pipeline::tools::Toolbox;
use crate::workspace::{MeshoptLevel, TextureCodec};

fn defaults() -> CompressDefaults {
    CompressDefaults {
        texture_size: 2048,
        codec: TextureCodec::Etc1s,
        meshopt: MeshoptLevel::Medium,
    }
}

fn service() -> (Packs, Arc<Mutex<Vec<PackState>>>) {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&heard);
    let packs = Packs::new(
        PipelineRunner::new(Arc::new(NullSink)),
        move |r: &PackRecord| {
            sink.lock().expect("heard").push(r.state);
        },
    );
    (packs, heard)
}

#[test]
fn a_pack_needs_blender() {
    let dir = tempfile::tempdir().expect("dir");
    let (packs, _) = service();
    let request = PackRequest {
        source: PackSource::Entries {
            entries: vec!["m".into()],
        },
        settings: PackSettings::default(),
        out: None,
        preset: None,
    };
    let plan = PackPlan {
        label: "1 model from the library".into(),
        models: Vec::new(),
        kinds: Vec::new(),
        estimate_bytes: 0.0,
        mixamo: 0,
        skipped: Vec::new(),
    };
    let err = packs
        .start(
            dir.path(),
            Toolbox::default(),
            defaults(),
            &request,
            plan,
            true,
        )
        .expect_err("no blender");
    assert!(err.to_string().contains("Blender is not installed"));
}

#[test]
fn keeping_the_last_warning_finishes_a_waiting_pack() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path().join("ws");
    let mut waiting = done_record(dir.path());
    waiting.items.remove(1);
    record::save(&root, &waiting).expect("save");
    let (packs, heard) = service();
    let tools = Toolbox::default();
    let low = PackDecision::Raise {
        key: "rake".into(),
        triangles: 2_000,
    };
    assert!(
        packs
            .decide(&root, tools.clone(), defaults(), &waiting.id, &low)
            .is_err(),
        "no raise"
    );
    let stray = PackDecision::Keep {
        key: "stove".into(),
    };
    assert!(
        packs
            .decide(&root, tools.clone(), defaults(), &waiting.id, &stray)
            .is_err(),
        "nothing to keep"
    );
    let keep = PackDecision::Keep { key: "rake".into() };
    let done = packs
        .decide(&root, tools.clone(), defaults(), &waiting.id, &keep)
        .expect("keep");
    assert_eq!(done.state, PackState::Done);
    assert_eq!(done.items[1].state, PackItemState::Kept);
    assert_eq!(
        *heard.lock().expect("heard"),
        [PackState::Zipping, PackState::Done]
    );
    assert_eq!(
        record::load(&root, &waiting.id).expect("load").state,
        PackState::Done
    );
    assert!(
        packs.zip(&root, &waiting.id, ZipChoice::All).is_err(),
        "done already"
    );
    assert!(
        packs
            .decide(&root, tools, defaults(), &waiting.id, &keep)
            .is_err()
    );
}

#[test]
fn zipping_a_waiting_pack_takes_what_passes() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path().join("ws");
    let waiting = done_record(dir.path());
    record::save(&root, &waiting).expect("save");
    let (packs, _) = service();
    let done = packs
        .zip(&root, &waiting.id, ZipChoice::Passing)
        .expect("zip");
    assert_eq!(done.state, PackState::Done);
    assert_eq!(
        done.message.as_deref(),
        Some("2 models left out of the pack.")
    );
}

#[test]
fn a_pack_the_app_closed_on_reads_as_stopped_and_goes_on_from_there() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path().join("ws");
    let mut closed = done_record(dir.path());
    closed.state = PackState::Running;
    closed.items[1].state = PackItemState::Running;
    closed.items[2].state = PackItemState::Queued;
    record::save(&root, &closed).expect("save");
    let (packs, _) = service();
    let read = packs.get(&root, &closed.id).expect("get");
    assert_eq!(read.state, PackState::Stopped);
    assert_eq!(
        read.message.as_deref(),
        Some("Maquette closed with 1 of 3 done, model 2 is next")
    );
    assert_eq!(read.items[1].state, PackItemState::Queued);
    assert_eq!(packs.recent(&root)[0].state, PackState::Stopped);
    assert!(packs.stop(&closed.id).is_err(), "not running");
    let err = packs
        .resume(&root, Toolbox::default(), defaults(), &closed.id)
        .expect_err("no blender");
    assert!(err.to_string().contains("Blender"));
}

#[test]
fn a_model_sent_back_is_the_next_one_the_stop_message_names() {
    let dir = tempfile::tempdir().expect("dir");
    let mut record = done_record(dir.path());
    record.items[0].state = PackItemState::Queued;
    assert_eq!(
        super::decide::stopped_at(&record, "Stopped"),
        "Stopped with 2 of 3 done, model 1 is next"
    );
}

#[test]
fn a_pack_the_app_closed_on_while_zipping_waits_for_the_zip_again() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path().join("ws");
    let mut closed = done_record(dir.path());
    closed.state = PackState::Zipping;
    record::save(&root, &closed).expect("save");
    let (packs, _) = service();
    let read = packs.get(&root, &closed.id).expect("get");
    assert_eq!(read.state, PackState::Waiting);
    assert_eq!(
        read.message.as_deref(),
        Some("Maquette closed while finishing the pack. Zip it again.")
    );
    let done = packs
        .zip(&root, &closed.id, ZipChoice::Passing)
        .expect("zip");
    assert_eq!(done.state, PackState::Done);
}

/// A pack stopped with models still queued and one warning to answer.
fn stopped_record(dir: &Path) -> PackRecord {
    let mut record = done_record(dir);
    record.state = PackState::Stopped;
    record.items[1].state = PackItemState::Queued;
    record
}

#[test]
fn keeping_a_warning_on_a_stopped_pack_leaves_it_stopped() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path().join("ws");
    let stopped = stopped_record(dir.path());
    record::save(&root, &stopped).expect("save");
    let (packs, heard) = service();
    let keep = PackDecision::Keep { key: "rake".into() };
    // Without Blender a restart would fail loudly, so none is tried.
    let read = packs
        .decide(&root, Toolbox::default(), defaults(), &stopped.id, &keep)
        .expect("keep");
    assert_eq!(read.state, PackState::Stopped);
    assert_eq!(read.items[2].state, PackItemState::Kept);
    assert_eq!(read.items[1].state, PackItemState::Queued, "still waits");
    assert_eq!(
        read.message.as_deref(),
        Some("Stopped with 2 of 3 done, model 2 is next")
    );
    assert_eq!(*heard.lock().expect("heard"), [PackState::Stopped]);
    packs.wait();
    let saved = record::load(&root, &stopped.id).expect("load");
    assert_eq!(saved.state, PackState::Stopped);
    assert_eq!(saved.items[2].state, PackItemState::Kept);
    assert!(packs.stop(&stopped.id).is_err(), "nothing runs");
}
