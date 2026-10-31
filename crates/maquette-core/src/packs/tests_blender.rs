//! A real pack in a real, headless Blender: a textured crate and a rigged
//! character with a Mixamo clip, written as OBJ (and a GLB for the rig),
//! opened again, rendered and zipped. Skipped where no Blender is found.
//! Set `MAQUETTE_KEEP_PACK` to a folder to keep the pack there and look.

use std::fs;
use std::io::Read as _;
use std::path::Path;
use std::sync::Arc;

use super::record::{PackItemState, PackState};
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
