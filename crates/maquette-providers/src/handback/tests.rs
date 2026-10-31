//! Tests for reading handed back models and matching downloads to sends.

use super::*;
use std::io::Write as _;
use std::time::{Duration, SystemTime};

use crate::jobs::CancelMode;
use crate::presets::{PresetVariant, ProviderId};

fn glb() -> Vec<u8> {
    let mut bytes = b"glTF".to_vec();
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&20u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let file = File::create(path).expect("zip");
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        zip.start_file(*name, options).expect("entry");
        zip.write_all(bytes).expect("write");
    }
    zip.finish().expect("finish");
}

fn job(id: &str, asset_name: &str, created_at_ms: f64) -> ProviderJob {
    ProviderJob {
        id: id.into(),
        project: "landing".into(),
        asset: id.rsplit_once('-').map_or(id, |(a, _)| a).into(),
        asset_name: asset_name.into(),
        provider: Provider::ByHand,
        service: ProviderId::Tripo,
        variant: PresetVariant::Prop,
        images: vec!["refs/a.png".into()],
        views: Vec::new(),
        tripo: None,
        meshy: None,
        state: JobState::Waiting,
        phase: None,
        progress: 0.0,
        remote_id: None,
        credits: None,
        created_at_ms,
        started_at_ms: None,
        finished_at_ms: None,
        message: None,
        inbox_file: None,
        handoff: None,
        card: None,
        version: None,
        via_agent: false,
        cancel: CancelMode::StopWaiting,
        batch: None,
        prompt: None,
        redo_of: None,
        estimate: None,
        split: None,
        studio: None,
        retexture: None,
    }
}

fn touch(path: &Path, bytes: &[u8], at: SystemTime) {
    fs::write(path, bytes).expect("write");
    File::options()
        .write(true)
        .open(path)
        .expect("open")
        .set_modified(at)
        .expect("mtime");
}

fn ms(at: SystemTime) -> f64 {
    at.duration_since(UNIX_EPOCH).expect("epoch").as_millis() as f64
}

#[test]
fn a_glb_is_read_as_it_is() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("lamp.glb");
    fs::write(&path, glb()).expect("glb");
    let model = read_model(&path).expect("read");
    assert_eq!(model.bytes, glb());
    assert!(model.inner.is_none());
    fs::write(&path, b"not a model at all").expect("junk");
    let err = read_model(&path).expect_err("junk");
    assert!(err.to_string().contains("not a GLB or a zip"), "{err}");
}

#[test]
fn a_zip_with_one_glb_is_unzipped_and_left_in_place() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("tripo_download.zip");
    write_zip(
        &path,
        &[
            ("__MACOSX/._desk_lamp.glb", b"junk"),
            ("textures/basecolor.png", b"png"),
            ("model/desk_lamp.glb", &glb()),
            (".hidden.glb", b"junk"),
        ],
    );
    let before = fs::read(&path).expect("zip bytes");
    let model = read_model(&path).expect("read");
    assert_eq!(model.bytes, glb());
    assert_eq!(model.inner.as_deref(), Some("model/desk_lamp.glb"));
    assert_eq!(fs::read(&path).expect("still there"), before);

    let none = dir.path().join("none.zip");
    write_zip(&none, &[("readme.txt", b"hi")]);
    assert!(
        read_model(&none)
            .expect_err("none")
            .to_string()
            .contains("has no .glb")
    );

    let two = dir.path().join("two.zip");
    write_zip(&two, &[("a.glb", &glb()), ("b.glb", &glb())]);
    let err = read_model(&two).expect_err("two").to_string();
    assert!(err.contains("holds 2 .glb files (a.glb, b.glb)"), "{err}");

    let fake = dir.path().join("fake.zip");
    write_zip(&fake, &[("lamp.glb", b"not a glb")]);
    assert!(
        read_model(&fake)
            .expect_err("fake")
            .to_string()
            .contains("is not a GLB")
    );
}

#[test]
fn downloads_match_by_name_then_time_and_skip_old_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let now = SystemTime::now();
    let handoff = now - Duration::from_secs(3600);
    let jobs = vec![
        job("desk-lamp-1", "Desk lamp", ms(handoff)),
        job(
            "plaster-plinth-1",
            "Plaster plinth with clay figure",
            ms(handoff),
        ),
        job("brush-cup-1", "Brush cup", ms(handoff)),
        {
            let mut done = job("laptop-1", "Laptop", ms(handoff));
            done.state = JobState::Inbox;
            done
        },
    ];
    let d = dir.path();
    touch(
        &d.join("Desk_Lamp_textured.glb"),
        &glb(),
        now - Duration::from_secs(600),
    );
    touch(
        &d.join("old_lamp.glb"),
        &glb(),
        handoff - Duration::from_secs(7200),
    );
    touch(&d.join("notes.txt"), b"hi", now);
    touch(&d.join(".partial.glb"), &glb(), now);
    let zip = d.join("tripo_c0ffee.zip");
    write_zip(&zip, &[("plaster_plinth_clay.glb", &glb())]);
    File::options()
        .write(true)
        .open(&zip)
        .expect("open")
        .set_modified(now - Duration::from_secs(300))
        .expect("mtime");
    touch(
        &d.join("a1b2c3d4.glb"),
        &glb(),
        now - Duration::from_secs(60),
    );

    let scan = check_downloads(d, &jobs);
    assert_eq!(scan.folder, d.display().to_string());
    assert_eq!(scan.since_ms, Some(ms(handoff)));
    assert_eq!(scan.waiting.len(), 3, "the Inbox send waits for nothing");
    let names: Vec<&str> = scan.files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["a1b2c3d4.glb", "tripo_c0ffee.zip", "Desk_Lamp_textured.glb"]
    );
    let by = |name: &str| scan.files.iter().find(|f| f.name == name).expect(name);
    assert_eq!(
        by("Desk_Lamp_textured.glb").job.as_deref(),
        Some("desk-lamp-1")
    );
    let zipped = by("tripo_c0ffee.zip");
    assert_eq!(zipped.job.as_deref(), Some("plaster-plinth-1"));
    assert_eq!(zipped.inner.as_deref(), Some("plaster_plinth_clay.glb"));
    let unnamed = by("a1b2c3d4.glb");
    assert_eq!(unnamed.job.as_deref(), Some("brush-cup-1"));
    assert_eq!(
        unnamed.reason.as_deref(),
        Some("The only new model since its handoff")
    );
    // Nothing was moved.
    assert!(d.join("Desk_Lamp_textured.glb").is_file() && zip.is_file());
}

#[test]
fn unnamed_files_stay_unmatched_when_several_sends_could_own_them() {
    let dir = tempfile::tempdir().expect("tempdir");
    let now = SystemTime::now();
    let handoff = now - Duration::from_secs(600);
    let jobs = vec![
        job("desk-lamp-1", "Desk lamp", ms(handoff)),
        job("laptop-1", "Laptop", ms(handoff)),
    ];
    touch(&dir.path().join("model_1.glb"), &glb(), now);
    let scan = check_downloads(dir.path(), &jobs);
    assert_eq!(scan.files.len(), 1);
    assert_eq!(scan.files[0].job, None);
    // The exact return name always wins.
    touch(&dir.path().join("laptop-1.glb"), &glb(), now);
    let scan = check_downloads(dir.path(), &jobs);
    let exact = scan
        .files
        .iter()
        .find(|f| f.name == "laptop-1.glb")
        .expect("exact");
    assert_eq!(exact.job.as_deref(), Some("laptop-1"));
    assert_eq!(exact.reason.as_deref(), Some("Named like Laptop"));
    // A send made after the file came down does not claim it.
    let late = vec![job("desk-lamp-2", "Desk lamp", ms(now) + 3_600_000.0)];
    assert!(check_downloads(dir.path(), &late).files.is_empty());
    // No waiting sends, nothing to look for.
    assert!(check_downloads(dir.path(), &[]).files.is_empty());
}
