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

/// Imports fixture `which` into the library as `file`.
fn import_fixture(s: &Setup, config: &WorkspaceConfig, which: &str, file: &str) {
    let path = s.files.join(file);
    fixture(&s.blender, &s.root.path().join("scripts"), which, &path);
    let paths = [path.display().to_string()];
    let plan = import::read(s.root.path(), config, &s.runner, &s.tools, &paths).expect("read");
    let request = ImportRequest {
        token: plan.token,
        ..ImportRequest::default()
    };
    import::commit(s.root.path(), config, &request).expect("import");
}

/// Frostveil with Elsheria (the fixture model, v1) and the dodge clip in
/// the library.
pub(crate) fn elsheria_with_a_dodge(s: &Setup, config: &WorkspaceConfig) -> PathBuf {
    let scripts = s.root.path().join("scripts");
    import_fixture(s, config, "clip", "Dodge Left.fbx");

    let layout = &config.layout;
    let made = crate::project::create(
        s.root.path(),
        layout,
        crate::project::NewProject {
            name: "Frostveil".into(),
            template: crate::project::ProjectTemplate::FloatingDiorama,
            target: crate::game::ProjectTarget::Website,
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let dir = crate::project::dir(s.root.path(), layout, &made.id).expect("dir");
    fixture(
        &s.blender,
        &scripts,
        "model",
        &dir.join("models").join("elsheria_v1.glb"),
    );
    crate::project::modify(s.root.path(), layout, &made.id, |m| {
        let asset = crate::assets::add(
            m,
            crate::assets::NewAsset {
                name: "Elsheria".into(),
                kind: crate::assets::AssetKind::Character,
                stage: crate::assets::Stage::Blender,
            },
        )?;
        crate::assets::add_version(m, &asset.id, "models/elsheria_v1.glb".into(), None).map(|_| ())
    })
    .expect("asset");
    dir
}

#[test]
fn library_clips_bake_onto_a_rigged_model_as_its_next_version() {
    use super::retarget::{self, ReviewStatus, UseClips};
    let Some(s) = setup() else { return };
    let config = WorkspaceConfig::default();
    let dir = elsheria_with_a_dodge(&s, &config);
    let target = ("frostveil", "elsheria");
    let clips = ["dodge-left".to_string()];
    let plan = retarget::plan(s.root.path(), &config, target, &clips, &[]).expect("plan");
    assert_eq!((plan.version, plan.joints), (1, 22));
    assert_eq!(plan.model_clips, ["idle"]);
    assert_eq!(plan.source_rig, Some(SourceRig::Mixamo));
    assert_eq!(plan.settings[0].name, "dodge_left");
    assert!(plan.needs_look, "the fixture's spine names need a look");

    let request = UseClips {
        project: target.0.into(),
        asset: target.1.into(),
        settings: plan.settings.clone(),
        map: Vec::new(),
        save_map: true,
        follow: Vec::new(),
    };
    let tools = (&s.runner, &s.tools);
    let draft = retarget::draft(s.root.path(), &config, tools, &request).expect("draft");
    let review = &draft.review.clips[0];
    assert_ne!(review.status, ReviewStatus::Failed, "{review:?}");
    assert_eq!(review.frames, 31, "frames 0 to 30 of the clip");
    // The fixture moves only its hips, so its planted feet slide with them.
    assert!(review.slide_cm > 50.0, "{review:?}");
    assert!(
        (review.travel.unwrap_or(0.0) - 0.9).abs() < 0.05,
        "{review:?}"
    );
    assert!(draft.review.driven >= 20);
    let json = test_blender::glb_json(Path::new(&draft.file));
    let names: Vec<&str> = json["animations"]
        .as_array()
        .expect("animations")
        .iter()
        .filter_map(|a| a["name"].as_str())
        .collect();
    assert!(
        names.contains(&"idle") && names.contains(&"dodge_left"),
        "{names:?}"
    );
    // The clip is 30 frames at 30 fps: it bakes at its own rate and keeps
    // its real time length, not 25% longer at Blender's default 24 fps.
    let seconds = test_blender::animation_seconds(&json, "dodge_left");
    assert!((seconds - 1.0).abs() < 0.01, "dodge_left lasts {seconds} s");
    let idle = |file: &Path| test_blender::animation_seconds(&test_blender::glb_json(file), "idle");
    let model_idle = idle(&dir.join("models").join("elsheria_v1.glb"));
    assert!(
        (idle(Path::new(&draft.file)) - model_idle).abs() < 0.01,
        "the model's own clip keeps its length"
    );

    let baked = retarget::bake(s.root.path(), &config, &draft.token).expect("bake");
    assert_eq!(baked.version.version, 2);
    assert!(dir.join(&baked.version.file).is_file());
    let entry = retarget::uses::of_asset(&dir, "elsheria").expect("record");
    assert_eq!(entry.baked[0].setting.name, "dodge_left");
    assert_eq!((entry.baked[0].version, entry.review_version), (2, Some(2)));
    assert!(!entry.map.is_empty(), "the map was saved");
    let uses = retarget::uses::uses(
        s.root.path(),
        &config.layout,
        &list(s.root.path()).expect("clips"),
    )
    .expect("uses");
    assert_eq!(uses["dodge-left"][0].asset_name, "Elsheria");
    assert!(
        retarget::bake(s.root.path(), &config, &draft.token).is_err(),
        "a draft bakes once"
    );

    // Baking it again replaces its own clip rather than adding dodge_left_2.
    let again = retarget::plan(s.root.path(), &config, target, &clips, &[]).expect("again");
    assert_eq!(again.settings[0].name, "dodge_left");
    assert!(!again.needs_look, "the saved map is reused");
}

#[test]
fn pin_the_feet_holds_a_sliding_foot_and_reports_before_and_after() {
    use super::retarget::{self, ReviewStatus, UseClips};
    let Some(s) = setup() else { return };
    let config = WorkspaceConfig::default();
    elsheria_with_a_dodge(&s, &config);
    import_fixture(&s, &config, "slide", "Foot Slide.fbx");
    let target = ("frostveil", "elsheria");
    let clips = ["foot-slide".to_string()];
    let plan = retarget::plan(s.root.path(), &config, target, &clips, &[]).expect("plan");
    assert!(!plan.settings[0].pin_feet, "off until asked");
    let mut request = UseClips {
        project: target.0.into(),
        asset: target.1.into(),
        settings: plan.settings.clone(),
        map: Vec::new(),
        save_map: false,
        follow: Vec::new(),
    };
    let tools = (&s.runner, &s.tools);
    let draft = retarget::draft(s.root.path(), &config, tools, &request).expect("draft");
    let before = draft.review.clips[0].clone();
    assert_eq!(before.status, ReviewStatus::Look, "{before:?}");
    assert!(
        before.note.starts_with("The left foot slides"),
        "{before:?}"
    );
    assert!(before.unpinned_cm.is_none());

    request.settings[0].pin_feet = true;
    let pinned = retarget::draft(s.root.path(), &config, tools, &request).expect("pinned");
    let after = &pinned.review.clips[0];
    assert_eq!(after.status, ReviewStatus::Good, "{after:?}");
    assert!(after.slide_cm < 0.5, "{after:?}");
    let was = after.unpinned_cm.expect("before");
    assert!((was - before.slide_cm).abs() < 0.01, "{was} {before:?}");
    assert!(
        after
            .note
            .starts_with("Feet pinned, the slide is down from"),
        "{after:?}"
    );
    assert!(pinned.settings[0].pin_feet, "kept with the clip settings");
}
