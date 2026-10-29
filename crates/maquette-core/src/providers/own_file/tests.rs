use std::fs;
use std::path::Path;

use super::*;
use crate::inbox::{self, fixture};
use crate::providers::http::Timing;
use crate::providers::jobs::{JobState, Provider};
use crate::providers::presets::PresetVariant;
use crate::providers::tests::World;
use crate::test_blender;

fn studio(world: &World) -> ProviderJobs {
    world.jobs_at("http://127.0.0.1:9", Timing::instant())
}

fn own<'a>(world: &'a World, file: &'a Path) -> OwnFile<'a> {
    OwnFile {
        root: &world.root,
        config: &world.config,
        project: &world.project,
        asset: &world.asset,
        file,
    }
}

fn tools() -> Toolbox {
    Toolbox::default()
}

/// A script runner for files that need no Blender: a `.glb` is copied as it
/// is, so a run here fails the test.
struct NoBlender;

impl ScriptRunner for NoBlender {
    fn run_script(
        &self,
        _tools: &Toolbox,
        _work: &Path,
        (file, _): (&str, &str),
        _extra: &[String],
    ) -> CoreResult<()> {
        panic!("a .glb needs no Blender, yet {file} ran");
    }
}

#[test]
fn only_real_model_files_outside_the_versions_are_taken() {
    let world = World::new();
    let outside = tempfile::tempdir().expect("tempdir");
    let good = outside.path().join("Fixed lantern.GLB");
    fs::write(&good, fixture::clean_prop()).expect("write");
    let (real, ext) = check(&world.root, &world.config, &good).expect("a glb outside");
    assert_eq!(ext, "glb");
    assert_eq!(real, fs::canonicalize(&good).expect("real"));

    let refused = |file: &Path| check(&world.root, &world.config, file).expect_err("refused");
    assert!(
        refused(Path::new("lantern.glb"))
            .to_string()
            .contains("absolute")
    );
    let notes = outside.path().join("notes.txt");
    fs::write(&notes, "hello").expect("write");
    assert!(refused(&notes).to_string().contains("not a model file"));
    assert!(
        refused(&outside.path().join("missing.fbx"))
            .to_string()
            .contains("missing.fbx")
    );
    let folder = outside.path().join("looks.glb");
    fs::create_dir_all(&folder).expect("dir");
    assert!(refused(&folder).to_string().contains("not a file"));

    let inbox_dir = inbox::inbox_dir(&world.root, &world.config);
    fs::create_dir_all(&inbox_dir).expect("inbox");
    let in_inbox = inbox_dir.join("dropped.glb");
    fs::write(&in_inbox, fixture::clean_prop()).expect("write");
    assert!(refused(&in_inbox).to_string().contains("inbox"));

    let models = world.project_dir().join("models");
    fs::create_dir_all(&models).expect("models");
    let version = models.join("stone-lantern_v1.glb");
    fs::write(&version, fixture::clean_prop()).expect("write");
    assert!(refused(&version).to_string().contains("version already"));
    // A file elsewhere in the project, like an export folder, is fine.
    let export = world.project_dir().join("tools").join("fixed.glb");
    fs::create_dir_all(export.parent().expect("parent")).expect("dir");
    fs::write(&export, fixture::clean_prop()).expect("write");
    assert!(check(&world.root, &world.config, &export).is_ok());
}

#[test]
fn a_glb_lands_in_the_inbox_aimed_at_the_asset_and_stays_where_it_was() {
    let world = World::new();
    let jobs = studio(&world);
    let outside = tempfile::tempdir().expect("tempdir");
    let file = outside.path().join("lantern fixed.glb");
    let bytes = fixture::clean_prop();
    fs::write(&file, &bytes).expect("write");
    let runner = NoBlender;

    let added = add(&jobs, &runner, &tools(), &own(&world, &file)).expect("added");
    assert_eq!(added.job, None, "no send waits for it");
    assert_eq!(added.asset, world.asset);
    assert_eq!(
        fs::read(&file).expect("still there"),
        bytes,
        "copied, never moved"
    );
    let item = inbox::item(&world.root, &world.config, &added.inbox_file).expect("in the inbox");
    let aim = item.aim.as_ref().expect("aimed");
    assert_eq!(
        (aim.project.as_str(), aim.asset.as_str()),
        (world.project.as_str(), world.asset.as_str())
    );

    let unknown = OwnFile {
        asset: "no-such-asset",
        ..own(&world, &file)
    };
    assert!(add(&jobs, &runner, &tools(), &unknown).is_err());
}

#[test]
fn a_waiting_by_hand_send_takes_the_file_as_its_model() {
    let world = World::new();
    let jobs = studio(&world);
    let request = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    let job = jobs.send(&request, true, false).expect("send");
    assert_eq!(job.state, JobState::Waiting);
    let outside = tempfile::tempdir().expect("tempdir");
    let file = outside.path().join("my fix.glb");
    fs::write(&file, fixture::clean_prop()).expect("write");
    let runner = NoBlender;

    let added = add(&jobs, &runner, &tools(), &own(&world, &file)).expect("added");
    assert_eq!(added.job.as_deref(), Some(job.id.as_str()));
    assert_eq!(jobs.get(&job.id).expect("job").state, JobState::Inbox);
}

#[test]
fn an_obj_becomes_a_glb_without_touching_the_original() {
    let Some(blender) = test_blender::blender() else {
        return;
    };
    let world = World::new();
    let jobs = studio(&world);
    let outside = tempfile::tempdir().expect("tempdir");
    let file = outside.path().join("crate.obj");
    let obj = "o crate\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n";
    fs::write(&file, obj).expect("write");
    let runner = test_blender::runner();

    let added = add(
        &jobs,
        &runner,
        &test_blender::toolbox(&blender),
        &own(&world, &file),
    )
    .expect("converted");
    assert_eq!(fs::read_to_string(&file).expect("kept"), obj);
    let landed = inbox::inbox_dir(&world.root, &world.config).join(&added.inbox_file);
    let json = test_blender::glb_json(&landed);
    assert_eq!(json["meshes"].as_array().map(Vec::len), Some(1));
    let work = world.root.join(&world.config.layout.cache).join(WORK_DIR);
    let left = fs::read_dir(&work).map_or(0, Iterator::count);
    assert_eq!(left, 0, "the scratch folder is cleaned up");
}
