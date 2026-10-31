//! A real pack in a real, headless Blender: a textured crate and a rigged
//! character with a Mixamo clip, written as OBJ (and a GLB for the rig),
//! opened again, rendered and zipped. Skipped where no Blender is found.
//! Set `MAQUETTE_KEEP_PACK` to a folder to keep the pack there and look.

use std::fs;
use std::io::Read as _;
use std::path::Path;
use std::sync::Arc;

use super::record::{self, PackItemState, PackState};
use super::tests::{entry, stats};
use super::*;
use crate::inbox::fixture;
use crate::library::LibrarySource;
use crate::pipeline::CompressDefaults;
use crate::pipeline::runner::{NullSink, PipelineRunner};
use crate::workspace::{FolderLayout, MeshoptLevel, TextureCodec};

fn by_hand(root: &Path, id: &str) {
    let path = root.join("library").join(id).join("entry.json");
    let text = fs::read_to_string(&path).expect("entry");
    let mut entry: crate::library::LibraryEntry = serde_json::from_str(&text).expect("json");
    entry.versions[0].source = LibrarySource::ByHand;
    entry.versions[0].prep = None;
    crate::json::write_json(&path, &entry).expect("write");
}

fn read_zip(zip: &Path) -> (Vec<String>, serde_json::Value) {
    let mut archive = zip::ZipArchive::new(fs::File::open(zip).expect("zip")).expect("archive");
    let mut names: Vec<String> = (0..archive.len())
        .map(|i| {
            archive
                .by_index(i)
                .expect("entry")
                .name()
                .expect("name")
                .to_string()
        })
        .collect();
    names.sort();
    let mut text = String::new();
    archive
        .by_name("village/manifest.json")
        .expect("manifest")
        .read_to_string(&mut text)
        .expect("read");
    (names, serde_json::from_str(&text).expect("json"))
}

#[test]
fn a_real_pack_is_written_checked_rendered_and_zipped() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    entry(
        root,
        "crate",
        "Crate",
        AssetKind::Prop,
        &fixture::clean_prop(),
        stats(12, 0, 0),
    );
    entry(
        root,
        "hero",
        "Hero",
        AssetKind::Character,
        &fixture::character(false),
        stats(4_000, 65, 1),
    );
    by_hand(root, "hero");
    let settings = PackSettings {
        name: "village".into(),
        ..PackSettings::default()
    };
    let source = PackSource::Entries {
        entries: vec!["crate".into(), "hero".into()],
    };
    let no_source = |_: &Path, _: &crate::assets::Asset, _: u32| (LibrarySource::Imported, None);
    let plan = plan(
        root,
        &FolderLayout::default(),
        &source,
        &settings,
        &no_source,
    )
    .expect("plan");
    assert!(
        plan.models[1].mixamo,
        "a by hand model on Mixamo's rig keeps no clips"
    );
    let packs = Packs::new(PipelineRunner::new(Arc::new(NullSink)), |_: &PackRecord| {});
    let request = PackRequest {
        source,
        settings,
        out: None,
        preset: Some("plain-obj".into()),
    };
    let defaults = CompressDefaults {
        texture_size: 2048,
        codec: TextureCodec::Etc1s,
        meshopt: MeshoptLevel::Medium,
    };
    let tools = crate::test_blender::toolbox(&blender);
    let started = packs
        .start(root, tools, defaults, &request, plan, false)
        .expect("start");
    packs.wait();
    let record = packs.get(root, &started.id).expect("record");
    for item in &record.items {
        eprintln!(
            "{}: {:?} {:?} {:?}",
            item.model.key, item.state, item.note, item.files
        );
    }
    assert_eq!(record.state, PackState::Done, "{:?}", record.message);
    let crate_item = &record.items[0];
    assert_eq!(crate_item.state, PackItemState::Pass);
    assert_eq!(crate_item.triangles, Some(12));
    let height = crate_item.height_m.expect("height");
    assert!((height - 0.92).abs() < 0.01, "real size: {height}");
    assert!(
        crate_item
            .picture
            .as_deref()
            .is_some_and(|p| Path::new(p).is_file()),
        "rendered"
    );
    let hero = &record.items[1];
    assert!(hero.clips_left_out >= 1, "the Mixamo clip is taken out");
    assert!(
        hero.files.iter().any(|f| f == "hero/hero.glb"),
        "the rig gets a GLB"
    );
    let pack = Path::new(&record.dir);
    let glb = crate::test_blender::glb_json(&pack.join("hero").join("hero.glb"));
    assert!(
        glb["skins"].as_array().is_some_and(|s| !s.is_empty()),
        "bones kept"
    );
    assert!(
        glb["animations"].as_array().is_none_or(Vec::is_empty),
        "no clips"
    );
    let mtl = fs::read_to_string(pack.join("crate").join("crate.mtl")).expect("mtl");
    assert!(mtl.contains("newmtl body"), "{mtl}");
    let (names, manifest) = read_zip(Path::new(record.zip.as_deref().expect("zip")));
    for wanted in [
        "village/README.txt",
        "village/contact-sheet.png",
        "village/crate/crate.obj",
        "village/crate/crate.mtl",
        "village/hero/hero.glb",
    ] {
        assert!(names.iter().any(|n| n == wanted), "{wanted} in {names:?}");
    }
    assert!(
        names
            .iter()
            .any(|n| n.starts_with("village/crate/") && n.ends_with(".png"))
    );
    assert!(
        !names
            .iter()
            .any(|n| n.contains(".work") || n.contains(".maquette"))
    );
    assert_eq!(manifest["models"][0]["tris"], 12);
    assert_eq!(manifest["models"][1]["source"], "By hand");
    assert!(
        manifest["notes"][0]
            .as_str()
            .is_some_and(|n| n.starts_with("Mixamo"))
    );
    if let Some(keep) = std::env::var_os("MAQUETTE_KEEP_PACK") {
        let keep = Path::new(&keep);
        fs::create_dir_all(keep).expect("keep");
        for name in ["README.txt", "manifest.json", "contact-sheet.png"] {
            fs::copy(pack.join(name), keep.join(name)).expect("copy");
        }
        fs::copy(
            record.zip.as_deref().expect("zip"),
            keep.join("village.zip"),
        )
        .expect("zip");
    }
}

fn start_props(packs: &Packs, root: &Path, names: &[&str], blender: &Path) -> String {
    for name in names {
        entry(
            root,
            name,
            name,
            AssetKind::Prop,
            &fixture::clean_prop(),
            stats(12, 0, 0),
        );
    }
    let settings = PackSettings {
        name: "village".into(),
        contact_sheet: false,
        ..PackSettings::default()
    };
    let source = PackSource::Entries {
        entries: names.iter().map(ToString::to_string).collect(),
    };
    let no_source = |_: &Path, _: &crate::assets::Asset, _: u32| (LibrarySource::Imported, None);
    let plan = plan(
        root,
        &FolderLayout::default(),
        &source,
        &settings,
        &no_source,
    )
    .expect("plan");
    let request = PackRequest {
        source,
        settings,
        out: None,
        preset: None,
    };
    let tools = crate::test_blender::toolbox(blender);
    packs
        .start(root, tools, pack_defaults(), &request, plan, false)
        .expect("start")
        .id
}

fn pack_defaults() -> CompressDefaults {
    CompressDefaults {
        texture_size: 2048,
        codec: TextureCodec::Etc1s,
        meshopt: MeshoptLevel::Medium,
    }
}

/// Stops a real pack mid run, then reads it as if the app closed on the
/// next model: nothing that ran runs again, nothing is skipped, and the
/// model cut short is written and checked from the start.
#[test]
fn a_real_pack_stops_goes_on_and_rechecks_the_model_it_closed_on() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    let packs = Packs::new(PipelineRunner::new(Arc::new(NullSink)), |_: &PackRecord| {});
    let id = start_props(&packs, root, &["one", "two", "three", "four"], &blender);
    while packs.get(root, &id).expect("get").done() == 0 {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    packs.stop(&id).expect("stop");
    packs.wait();
    let mut closed = packs.get(root, &id).expect("stopped");
    assert_eq!(closed.state, PackState::Stopped);
    let done = closed.done();
    assert!((1..4).contains(&done), "{done} done");
    assert_eq!(
        closed.message.as_deref(),
        Some(format!("Stopped with {done} of 4 done, model {} is next", done + 1).as_str())
    );
    assert!(closed.items[..done].iter().all(|i| i.took_ms > 0.0));
    assert!(
        closed.items[done..].iter().all(|i| i.took_ms == 0.0),
        "a model still queued has no time yet"
    );

    closed.state = PackState::Running;
    closed.items[done].state = PackItemState::Running;
    let cut = Path::new(&closed.dir).join(&closed.items[done].model.key);
    fs::create_dir_all(&cut).expect("folder");
    fs::write(cut.join("half-written.obj"), "v 0 0").expect("junk");
    record::save(root, &closed).expect("save");
    let before: Vec<f64> = closed.items[..done].iter().map(|i| i.took_ms).collect();

    let reopened = Packs::new(PipelineRunner::new(Arc::new(NullSink)), |_: &PackRecord| {});
    let read = reopened.get(root, &id).expect("read");
    assert_eq!(read.state, PackState::Stopped);
    assert_eq!(read.items[done].state, PackItemState::Queued);
    let tools = crate::test_blender::toolbox(&blender);
    reopened
        .resume(root, tools, pack_defaults(), &id)
        .expect("go on");
    reopened.wait();
    let finished = reopened.get(root, &id).expect("finished");
    assert_eq!(finished.state, PackState::Done, "{:?}", finished.message);
    assert!(
        finished
            .items
            .iter()
            .all(|i| i.state == PackItemState::Pass && i.took_ms > 0.0)
    );
    let after: Vec<f64> = finished.items[..done].iter().map(|i| i.took_ms).collect();
    assert_eq!(before, after, "what ran is not run again");
    assert!(
        !cut.join("half-written.obj").exists(),
        "written from the start"
    );
}

/// Stop during the last model has nothing left to stop before: the pack
/// finishes, it does not read as stopped with nothing to go on with.
#[test]
fn a_real_pack_stopped_on_its_last_model_finishes() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    let packs = Packs::new(PipelineRunner::new(Arc::new(NullSink)), |_: &PackRecord| {});
    let id = start_props(&packs, root, &["only"], &blender);
    while packs.get(root, &id).expect("get").items[0].state != PackItemState::Running {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    packs.stop(&id).expect("stop");
    packs.wait();
    let record = packs.get(root, &id).expect("record");
    assert_eq!(record.state, PackState::Done, "{:?}", record.message);
}
