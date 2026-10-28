//! The clip reader in a real, headless Blender, on tiny rigs made by
//! `fixtures/clip_rigs.py`. Skipped where no Blender is installed.

use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::import::{self, ClipStatus, ImportRequest};
use super::save::{self, ClipsFrom, SaveClips};
use super::*;
use crate::pipeline::runner::{NullSink, PipelineRunner};
use crate::pipeline::tools::{ResolvedTool, Toolbox};
use crate::test_blender;
use crate::toolchain::{HostOs, ToolId};
use crate::workspace::WorkspaceConfig;

const FIXTURES: &str = include_str!("fixtures/clip_rigs.py");

pub(crate) fn toolbox(blender: &Path) -> Toolbox {
    let mut tools = Toolbox {
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        os: Some(HostOs::current()),
        ..Toolbox::default()
    };
    tools.tools.insert(
        ToolId::Blender,
        ResolvedTool {
            program: blender.to_path_buf(),
            prefix: Vec::new(),
            version: "test".into(),
        },
    );
    tools
}

/// Makes fixture `which` at `out` with the fixture script.
pub(crate) fn fixture(blender: &Path, scripts: &Path, which: &str, out: &Path) {
    let script = scripts.join("clip_rigs.py");
    if !script.is_file() {
        fs::create_dir_all(scripts).expect("scripts");
        fs::write(&script, FIXTURES).expect("fixture script");
    }
    let script = script.display().to_string();
    let out = out.display().to_string();
    test_blender::run(blender, &["--python", &script, "--", which, &out]);
}

fn zip_of(path: &Path, files: &[(&str, &Path)]) {
    let mut zip = zip::ZipWriter::new(File::create(path).expect("zip"));
    let options = zip::write::SimpleFileOptions::default();
    for (name, file) in files {
        zip.start_file(*name, options).expect("entry");
        zip.write_all(&fs::read(file).expect("fbx")).expect("write");
    }
    zip.finish().expect("finish");
}

pub(crate) struct Setup {
    pub root: tempfile::TempDir,
    pub files: PathBuf,
    pub tools: Toolbox,
    pub runner: PipelineRunner,
    pub blender: PathBuf,
}

pub(crate) fn setup() -> Option<Setup> {
    let blender = test_blender::blender()?;
    let root = tempfile::tempdir().expect("tempdir");
    let files = root.path().join("drop");
    Some(Setup {
        tools: toolbox(&blender),
        runner: PipelineRunner::new(Arc::new(NullSink)),
        files,
        root,
        blender,
    })
}

#[test]
fn a_pack_and_single_files_import_skeleton_only() {
    let Some(s) = setup() else { return };
    let scripts = s.root.path().join("scripts");
    let dodge = s.files.join("Dodge Left.fbx");
    let still = s.files.join("still.fbx");
    let custom = s.files.join("heroine_sway.fbx");
    fixture(&s.blender, &scripts, "clip", &dodge);
    fixture(&s.blender, &scripts, "still", &still);
    fixture(&s.blender, &scripts, "custom", &custom);
    let pack = s.files.join("Frost Pack.zip");
    zip_of(
        &pack,
        &[
            ("Frost Pack/Dodge Left.fbx", &dodge),
            ("Frost Pack/pose.fbx", &still),
        ],
    );
    let config = WorkspaceConfig::default();
    let paths = [&pack, &dodge, &custom].map(|p| p.display().to_string());
    let plan = import::read(s.root.path(), &config, &s.runner, &s.tools, &paths).expect("read");

    let zip = &plan.files[0];
    assert_eq!(
        (zip.pack.as_str(), zip.fbx, zip.bones),
        ("Frost Pack", 2, 22)
    );
    assert_eq!(zip.rig, Some(SourceRig::Mixamo));
    assert_eq!(zip.prefix.as_deref(), Some("mixamorig:"));
    assert_eq!(zip.skinned, 2);
    assert_eq!(zip.left_out[0].file, "pose.fbx");
    let clip = &zip.clips[0];
    assert_eq!(clip.name, "Dodge Left");
    assert_eq!((clip.frames, clip.fps), (30, 30.0));
    let root = clip.root_motion.as_ref().expect("root motion");
    assert!((root.metres - 0.9).abs() < 0.02, "{root:?}");
    assert_eq!(root.direction, "left");
    // The single file has the pack's keys.
    assert_eq!(plan.files[1].clips[0].status, ClipStatus::Duplicate);
    assert_eq!(plan.files[2].rig, Some(SourceRig::Custom));
    assert!((plan.files[2].clips[0].fps - 24.0).abs() < 0.01);

    let request = ImportRequest {
        token: plan.token.clone(),
        ..ImportRequest::default()
    };
    let done = import::commit(s.root.path(), &config, &request).expect("commit");
    assert_eq!(done.added.len(), 2);
    let folder = clip_dir(s.root.path(), "dodge-left").expect("dir");
    let json = test_blender::glb_json(&folder.join(CLIP_GLB));
    assert!(json.get("meshes").is_none(), "the skin was dropped");
    assert_eq!(json["animations"].as_array().map(Vec::len), Some(1));
    assert_eq!(json["animations"][0]["name"], "Dodge Left");
    let loaded = load(s.root.path(), "dodge-left").expect("clip");
    assert_eq!(loaded.rig.bones.len(), 22);
    assert!((loaded.rig.hips_height - 1.0).abs() < 0.01);
}

#[test]
fn clips_save_from_a_rigged_model_in_the_inbox() {
    let Some(s) = setup() else { return };
    let config = WorkspaceConfig::default();
    let inbox = crate::inbox::inbox_dir(s.root.path(), &config);
    let model = inbox.join("elsheria_rig.glb");
    fixture(&s.blender, &s.root.path().join("scripts"), "model", &model);
    let from = ClipsFrom {
        inbox: Some("elsheria_rig.glb".into()),
        ..ClipsFrom::default()
    };
    let plan = save::plan(s.root.path(), &config, &from, true, &[]).expect("plan");
    assert_eq!((plan.rig, plan.bones), (SourceRig::Tripo, 22));
    assert_eq!(plan.clips.len(), 1);
    let request = SaveClips {
        from,
        clips: vec![plan.clips[0].name.clone()],
        pack: "Tripo animations".into(),
        tags: vec!["idle".into()],
    };
    let tools = (&s.runner, &s.tools);
    let saved = save::save(s.root.path(), &config, tools, &request, true, &[]).expect("save");
    assert_eq!(saved.added.len(), 1);
    let clip = &saved.added[0];
    assert_eq!(clip.pack, "Tripo animations");
    assert_eq!(clip.origin.source, ClipSource::Tripo);
    let folder = clip_dir(s.root.path(), &clip.id).expect("dir");
    assert!(
        test_blender::glb_json(&folder.join(CLIP_GLB))
            .get("meshes")
            .is_none()
    );
    let again = save::save(s.root.path(), &config, tools, &request, true, &[]).expect("again");
    assert_eq!((again.added.len(), again.skipped.len()), (0, 1));
}
