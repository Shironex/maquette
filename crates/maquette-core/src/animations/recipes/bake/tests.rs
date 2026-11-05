use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::job::{self, JOB_FILE, REPORT_FILE, TargetSpec, limb_of, mixamo_name, model_bone};
use super::result::{read_run, result_of};
use super::*;
use crate::animations::recipes::{self as library_recipes, RecipeDoc};
use crate::animations::retarget::{BonePair, PairMatch};
use crate::animations::tests::library_clip;
use crate::animations::{CLIP_GLB, clip_dir, store};
use crate::base::ports::{CANCELLED, ScriptRunner, Watch};
use crate::error::{CoreError, CoreResult};
use crate::inbox::fixture::{self, HumanoidSpec};
use crate::pipeline::tools::Toolbox;
use crate::recipes::{BlankKind, LayerKind, LayerSettings, RecipeBlank, RecipeChange, RecipeLayer};
use crate::workspace::WorkspaceConfig;

#[test]
fn vrm_bones_read_as_the_humanoid_names_the_bone_map_lists() {
    assert_eq!(mixamo_name("rightHand").as_deref(), Some("RightHand"));
    assert_eq!(mixamo_name("leftUpperLeg").as_deref(), Some("LeftUpLeg"));
    assert_eq!(mixamo_name("chest").as_deref(), Some("Spine1"));
    assert_eq!(
        mixamo_name("leftIndexProximal").as_deref(),
        Some("LeftHandIndex1")
    );
    assert_eq!(
        mixamo_name("rightLittleDistal").as_deref(),
        Some("RightHandPinky3")
    );
    assert_eq!(
        mixamo_name("leftThumbMetacarpal").as_deref(),
        Some("LeftHandThumb1")
    );
    assert_eq!(mixamo_name("tail"), None);
    assert_eq!(
        limb_of("rightHand"),
        Some(("rightUpperArm", "rightLowerArm"))
    );
    assert_eq!(limb_of("head"), None);
}

#[test]
fn a_bone_comes_from_the_map_and_else_from_a_joint_of_that_name() {
    let map = vec![BonePair {
        bone: "RightHand".into(),
        source: None,
        target: Some("hand.R".into()),
        how: PairMatch::Saved,
    }];
    let joints = vec![
        "hand.R".to_string(),
        "mixamorig:RightHandIndex1".to_string(),
    ];
    assert_eq!(
        model_bone("rightHand", &map, &joints).as_deref(),
        Some("hand.R")
    );
    assert_eq!(
        model_bone("rightIndexProximal", &map, &joints).as_deref(),
        Some("mixamorig:RightHandIndex1")
    );
    assert_eq!(model_bone("leftHand", &map, &joints), None);
}

pub(super) struct Project {
    _tmp: Option<tempfile::TempDir>,
    root: PathBuf,
    pub(super) config: WorkspaceConfig,
    pub(super) dir: PathBuf,
    pub(super) id: String,
    pub(super) recipe: RecipeDoc,
}

impl Project {
    pub(super) fn root(&self) -> &std::path::Path {
        &self.root
    }

    pub(super) fn request(&self) -> RecipeBakeRequest {
        RecipeBakeRequest {
            project: self.id.clone(),
            recipe: self.recipe.id.clone(),
            asset: "elsheria".into(),
            expected: None,
        }
    }

    pub(super) fn change(&mut self, layers: Vec<RecipeLayer>, blanks: Vec<RecipeBlank>) {
        let manifest =
            crate::project::load(self.root(), &self.config.layout, &self.id).expect("manifest");
        self.recipe = library_recipes::update(
            self.root(),
            &self.dir,
            &manifest,
            &self.recipe.id,
            RecipeChange {
                layers: Some(layers),
                blanks: Some(blanks),
                ..RecipeChange::default()
            },
        )
        .expect("change");
    }
}

/// Frostveil with Elsheria (a fixture humanoid, `model`) and a recipe
/// "Reach out" on a Mixamo clip of the library, with no changes yet.
pub(super) fn project_with(model: Vec<u8>) -> Project {
    let tmp = tempfile::tempdir().expect("temp");
    let root = tmp.path();
    store(
        root,
        &library_clip("still", "Standing Still", "Singles", "h1"),
    )
    .expect("store");
    fs::write(clip_dir(root, "still").expect("dir").join(CLIP_GLB), b"glb").expect("clip");
    let mut p = project_on(root, "still", model);
    p._tmp = Some(tmp);
    p
}

/// As [`project_with`], in the workspace at `root` on library clip `clip`.
pub(super) fn project_on(root: &std::path::Path, clip: &str, model: Vec<u8>) -> Project {
    let config = WorkspaceConfig::default();
    let made = crate::project::create(
        root,
        &config.layout,
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
    let dir = crate::project::dir(root, &config.layout, &made.id).expect("dir");
    fs::create_dir_all(dir.join("models")).expect("models");
    fs::write(dir.join("models").join("elsheria_v1.glb"), model).expect("model");
    crate::project::modify(root, &config.layout, &made.id, |m| {
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
    let recipe = library_recipes::create(root, &dir, clip, Some("Reach out")).expect("recipe");
    Project {
        _tmp: None,
        root: root.to_path_buf(),
        config,
        dir,
        id: made.id,
        recipe,
    }
}

pub(super) fn humanoid() -> Vec<u8> {
    fixture::humanoid(&HumanoidSpec::default())
}

pub(super) fn reach(id: &str, bone: &str, target: &str) -> RecipeLayer {
    RecipeLayer {
        id: id.into(),
        kind: LayerKind::Reach,
        bone: bone.into(),
        settings: LayerSettings {
            target: Some(target.into()),
            ..LayerSettings::default()
        },
    }
}

pub(super) fn point(id: &str, at: Option<[f32; 3]>) -> RecipeBlank {
    RecipeBlank {
        id: id.into(),
        label: id.into(),
        kind: BlankKind::Point,
        prop: None,
        point: at,
    }
}

pub(super) fn reaching() -> Project {
    let mut p = project_with(humanoid());
    p.change(
        vec![reach("l-1", "rightHand", "b-1")],
        vec![point("b-1", Some([-0.3, 1.2, 0.35]))],
    );
    p
}

fn refusal(p: &Project, request: &RecipeBakeRequest) -> CoreError {
    match prepare(p.root(), &p.config, request) {
        Ok(_) => panic!("the bake was not refused"),
        Err(err) => err,
    }
}

#[test]
fn a_recipe_with_open_blanks_or_no_changes_is_refused_naming_what_to_fix() {
    let mut p = project_with(humanoid());
    let none = refusal(&p, &p.request()).to_string();
    assert!(none.contains("no changes yet"), "{none}");

    p.change(
        vec![
            reach("l-1", "rightHand", "b-1"),
            reach("l-2", "leftHand", "b-2"),
        ],
        vec![point("b-1", None), point("b-2", None)],
    );
    assert_eq!(open_blanks(&p.recipe), ["b-1", "b-2"]);
    let open = refusal(&p, &p.request());
    assert!(matches!(open, CoreError::Invalid(_)));
    assert!(open.to_string().contains("b-1, b-2"), "{open}");
}

#[test]
fn a_recipe_that_changed_since_it_was_read_is_refused_as_moved() {
    let p = reaching();
    let mut request = p.request();
    request.expected = Some("an older version".into());
    assert!(matches!(refusal(&p, &request), CoreError::Moved(_)));
    request.expected = Some(p.recipe.version.clone());
    assert!(prepare(p.root(), &p.config, &request).is_ok());
}

#[test]
fn a_base_clip_gone_from_the_library_or_a_model_with_no_rig_is_refused() {
    let p = reaching();
    let mut request = p.request();
    request.asset = "nobody".into();
    assert!(prepare(p.root(), &p.config, &request).is_err());

    fs::write(
        p.dir.join("models").join("elsheria_v1.glb"),
        fixture::clean_prop(),
    )
    .expect("prop");
    let unrigged = refusal(&p, &p.request()).to_string();
    assert!(unrigged.contains("no rig"), "{unrigged}");

    fs::remove_dir_all(clip_dir(p.root(), "still").expect("dir")).expect("gone");
    let gone = refusal(&p, &p.request()).to_string();
    assert!(gone.contains("library"), "{gone}");
}

#[test]
fn a_bake_gathers_the_bones_targets_and_clip_name_it_needs() {
    let p = reaching();
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    assert_eq!(prepared.setting.name, "reach_out");
    assert!(prepared.doc.mixamo, "the base clip is Mixamo's");
    assert!(prepared.hips.ends_with("Hips"), "{}", prepared.hips);
    for vrm in [
        "rightHand",
        "rightUpperArm",
        "rightLowerArm",
        "leftUpperArm",
    ] {
        assert!(prepared.bones.contains_key(vrm), "{vrm}");
    }
    assert_eq!(
        prepared.targets["b-1"],
        TargetSpec::Point {
            point: [-0.3, 1.2, 0.35]
        }
    );
}

#[test]
fn every_clip_this_recipe_made_is_replaced_whatever_its_name() {
    use crate::recipes::baked::RecipeBakeUse;
    let bake = |recipe: &str, clip: &str| RecipeBakeUse {
        recipe: recipe.into(),
        recipe_version: "v".into(),
        base_clip: "idle".into(),
        clip: clip.into(),
        mixamo: false,
        version: 2,
    };
    let clips = vec!["Idle".to_string(), "wave".to_string(), "wave_2".to_string()];
    // No earlier bake: the model's clips stay as they are.
    let free = job::clip_name(&clips, &[], "wave");
    assert_eq!(
        (free.taken.clone(), free.replace),
        (clips.clone(), Vec::new())
    );
    // This recipe made wave before: replaced, and the name is free again.
    let ours = job::clip_name(&clips, &[bake("wave", "wave")], "wave");
    assert_eq!(ours.replace, ["wave"]);
    assert_eq!(ours.taken, ["Idle", "wave_2"]);
    // It made wave_2 because wave was someone else's: wave_2 is replaced,
    // so a rebake takes wave_2 again instead of wave_3.
    let second = job::clip_name(
        &clips,
        &[bake("other", "wave"), bake("wave", "wave_2")],
        "wave",
    );
    assert_eq!(second.replace, ["wave_2"]);
    assert_eq!(second.taken, ["Idle", "wave"]);
    // Both are its own: both go.
    let both = job::clip_name(
        &clips,
        &[bake("wave", "wave"), bake("wave", "wave_2")],
        "wave",
    );
    assert_eq!(both.replace, ["wave", "wave_2"]);
    assert_eq!(both.taken, ["Idle"]);
    // Another recipe's clip, or one the model no longer has: nothing goes.
    let kept = job::clip_name(
        &clips,
        &[bake("other", "wave"), bake("wave", "gone")],
        "wave",
    );
    assert!(kept.replace.is_empty());
    assert_eq!(kept.taken, clips);
}

#[test]
fn a_rebake_onto_its_own_clip_replaces_it_and_another_clip_of_that_name_stays() {
    // The fixture's clip, lower case, so a recipe called Idle clashes with it.
    let mut model = humanoid();
    let at = model
        .windows(6)
        .position(|w| w == b"\"Idle\"")
        .expect("clip name");
    model[at + 1] = b'i';
    let mut p = project_with(model);
    let manifest = crate::project::load(p.root(), &p.config.layout, &p.id).expect("manifest");
    library_recipes::update(
        p.root(),
        &p.dir,
        &manifest,
        &p.recipe.id,
        RecipeChange {
            name: Some("Idle".into()),
            ..RecipeChange::default()
        },
    )
    .expect("rename");
    p.change(
        vec![reach("l-1", "rightHand", "b-1")],
        vec![point("b-1", Some([-0.3, 1.2, 0.35]))],
    );

    // The model's own idle stays; the bake takes idle_2 and says why.
    let kept = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    assert_eq!(kept.setting.name, "idle_2");
    assert!(kept.replace.is_empty());
    let note = kept.clip_note.clone().expect("note");
    assert!(note.contains("already has a clip named idle"), "{note}");

    // Once an earlier bake of this recipe made idle, a new bake replaces it.
    crate::recipes::baked::record(
        &p.dir,
        "elsheria",
        crate::recipes::baked::RecipeBakeUse {
            recipe: p.recipe.id.clone(),
            recipe_version: "older".into(),
            base_clip: "still".into(),
            clip: "idle".into(),
            mixamo: true,
            version: 1,
        },
    )
    .expect("record");
    let ours = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    assert_eq!(ours.setting.name, "idle");
    assert_eq!(ours.replace, ["idle"]);
    assert!(
        ours.clip_note
            .as_deref()
            .is_some_and(|n| n.starts_with("Replaces idle"))
    );
    let work = p.root().join("work");
    job::stage(p.root(), &ours, &work).expect("stage");
    let job: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join(JOB_FILE)).expect("job")).expect("json");
    assert_eq!(job["retarget"]["replace"], serde_json::json!(["idle"]));
}

#[test]
fn a_prop_with_no_place_in_the_layout_takes_the_preview_slot() {
    let mut p = project_with(humanoid());
    crate::project::modify(p.root(), &p.config.layout, &p.id, |m| {
        for name in ["Lantern", "Cup"] {
            crate::assets::add(
                m,
                crate::assets::NewAsset {
                    name: name.into(),
                    kind: crate::assets::AssetKind::Prop,
                    stage: crate::assets::Stage::Blender,
                },
            )?;
        }
        Ok(())
    })
    .expect("props");
    let prop = |id: &str, asset: &str| RecipeBlank {
        id: id.into(),
        label: id.into(),
        kind: BlankKind::Prop,
        prop: Some(asset.into()),
        point: None,
    };
    p.change(
        vec![
            reach("l-1", "rightHand", "b-1"),
            reach("l-2", "leftHand", "b-2"),
        ],
        vec![prop("b-1", "lantern"), prop("b-2", "cup")],
    );
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    assert_eq!(prepared.targets["b-1"], TargetSpec::Slot { slot: 0 });
    assert_eq!(prepared.targets["b-2"], TargetSpec::Slot { slot: 1 });
    assert!(prepared.props.is_empty());
}

#[test]
fn staging_writes_the_job_the_scripts_and_the_inputs() {
    let p = reaching();
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let work = p.root().join("work");
    job::stage(p.root(), &prepared, &work).expect("stage");
    for file in [
        "retarget_clips.py",
        "surface.py",
        "in/model.glb",
        "in/clips/still.glb",
    ] {
        assert!(work.join(file).is_file(), "{file}");
    }
    let job: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(work.join(JOB_FILE)).expect("job")).expect("json");
    assert_eq!(job["out"], STAGED_FILE);
    assert_eq!(job["report"], REPORT_FILE);
    assert_eq!(job["retarget"]["model"], "in/model.glb");
    assert_eq!(job["retarget"]["clips"][0]["name"], "reach_out");
    assert_eq!(job["recipe"]["clip"], "still");
    assert_eq!(job["recipe"]["layers"][0]["kind"], "reach");
    assert_eq!(
        job["recipe"]["targets"]["b-1"]["point"],
        serde_json::json!([-0.3, 1.2, 0.35])
    );
    assert!(job["recipe"]["bones"]["rightHand"].is_string());
    assert_eq!(job["recipe"]["stretchFrames"], 10);
}

pub(super) const REPORT: &str = r#"{
  "clip": "reach_out", "frames": 31, "facing": [0, 0, 1],
  "feet": {}, "baseFeet": {},
  "stretch": {"mm": 3.24, "frame": 12, "base": 1.04},
  "reaches": [
    {"layer": "l-1", "bone": "rightHand", "gap": 0.01234, "frame": 30, "flipFrame": null},
    {"layer": "l-2", "bone": "leftHand", "gap": 0.05, "frame": 30, "flipFrame": 20},
    {"layer": "l-3", "bone": "leftFoot", "gap": null, "frame": null}
  ],
  "seconds": 2.456
}"#;

#[test]
fn the_report_reads_as_the_result_the_review_shows() {
    let p = reaching();
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let work = p.root().join("work");
    fs::create_dir_all(&work).expect("work");
    fs::write(work.join(REPORT_FILE), REPORT).expect("report");
    let run = read_run(&work).expect("run");
    let result = result_of(&run, &prepared, &work, 3.0);
    assert_eq!(result.clip, "reach_out");
    assert_eq!(
        (result.recipe.as_str(), result.base_clip.as_str()),
        ("reach-out", "still")
    );
    assert_eq!(result.recipe_version, p.recipe.version);
    assert!(result.mixamo);
    assert_eq!(
        (result.asset.as_str(), result.from_version),
        ("elsheria", 1)
    );
    assert_eq!((result.slide_cm, result.slide_frames), (0.0, None));
    assert_eq!(
        (result.stretch_mm, result.stretch_frame),
        (Some(3.2), Some(12))
    );
    assert_eq!(result.base_stretch_mm, Some(1.0));
    assert_eq!(result.blender_seconds, 2.46);
    let [hand, other, foot] = &result.reaches[..] else {
        panic!("{:?}", result.reaches)
    };
    assert_eq!(
        (hand.gap_cm, hand.reached, hand.flip_frame),
        (Some(1.23), true, None)
    );
    assert_eq!(
        (other.gap_cm, other.reached, other.flip_frame),
        (Some(5.0), false, Some(20))
    );
    assert_eq!((foot.gap_cm, foot.reached), (None, false));
    assert!(result.file.ends_with(STAGED_FILE));

    fs::write(work.join(REPORT_FILE), "{ not json").expect("broken");
    assert!(read_run(&work).is_err());
}

/// The run folders in a bake's folder.
fn runs_in(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut runs: Vec<PathBuf> = fs::read_dir(dir)
        .expect("dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("run-"))
        })
        .collect();
    runs.sort();
    runs
}

/// A runner that writes what the bake script would, or that waits to be
/// cancelled.
struct Fake {
    hold: bool,
    panics: bool,
}

impl ScriptRunner for Fake {
    fn run_script(
        &self,
        _: &Toolbox,
        work: &std::path::Path,
        script: (&str, &str),
        extra: &[String],
    ) -> CoreResult<()> {
        assert_eq!(script.0, "bake_recipe.py");
        assert_eq!(extra, ["--job", JOB_FILE]);
        fs::write(
            work.join("progress.json"),
            r#"{"label": "Solving frame 31 of 31", "done": 31, "total": 31}"#,
        )
        .expect("progress");
        fs::write(work.join(STAGED_FILE), humanoid()).expect("staged");
        fs::write(work.join(REPORT_FILE), REPORT).expect("report");
        Ok(())
    }

    fn run_watched(
        &self,
        tools: &Toolbox,
        work: &std::path::Path,
        (name, script): (&str, (&str, &str)),
        extra: &[String],
        watch: &mut Watch<'_>,
    ) -> CoreResult<()> {
        assert!(!self.panics, "a bug in the bake");
        if !self.hold {
            self.run_step(tools, work, name, script, extra)?;
            (watch.tick)();
            return Ok(());
        }
        let started = Instant::now();
        while !watch.cancel.load(Ordering::SeqCst) {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "never cancelled"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        Err(CoreError::Invalid(CANCELLED.to_string()))
    }
}

pub(super) fn bakes(hold: bool) -> (RecipeBakes, Arc<Mutex<Vec<RecipeBakeRecord>>>) {
    bakes_on(Fake {
        hold,
        panics: false,
    })
}

fn bakes_on(fake: Fake) -> (RecipeBakes, Arc<Mutex<Vec<RecipeBakeRecord>>>) {
    let told = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&told);
    let bakes = RecipeBakes::new(Arc::new(fake), move |r: &RecipeBakeRecord| {
        sink.lock().expect("told").push(r.clone());
    });
    (bakes, told)
}

pub(super) fn settled(bakes: &RecipeBakes, p: &Project) -> RecipeBakeRecord {
    let started = Instant::now();
    loop {
        let record = bakes
            .get(p.root(), &p.config, &p.id, &p.recipe.id)
            .expect("get")
            .expect("record");
        if !record.running() {
            return record;
        }
        assert!(started.elapsed() < Duration::from_secs(10), "{record:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_bake_runs_as_a_job_and_stages_its_model_outside_the_project_and_library() {
    let p = reaching();
    let (bakes, told) = self::bakes(false);
    assert!(
        bakes
            .get(p.root(), &p.config, &p.id, &p.recipe.id)
            .expect("get")
            .is_none()
    );
    let started = bakes
        .start(
            p.root(),
            &p.config,
            Toolbox::default(),
            &p.request(),
            RecipeBakeStarter::Agent,
        )
        .expect("start");
    assert!(started.running());
    assert_eq!(started.started_by, RecipeBakeStarter::Agent);
    let done = settled(&bakes, &p);
    assert_eq!(done.state, RecipeBakeState::Done, "{:?}", done.error);
    let result = done.result.expect("result");
    assert!(result.mixamo);
    let staged = PathBuf::from(&result.file);
    assert!(staged.is_file());
    // The staged model carries its Mixamo mark in its own GLB.
    let mark = mark_of(&staged).expect("mark");
    assert_eq!(mark, mark_for(&result));
    assert!(mark.mixamo && marked_mixamo(&staged));
    let cache = p.root().join(&p.config.layout.cache);
    assert!(staged.starts_with(&cache), "{}", staged.display());
    let runs = runs_in(staged.parent().expect("dir"));
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert!(!runs[0].join("in").exists(), "the copies are gone");
    // Nothing was added to the model or to the clip library.
    let manifest = crate::project::load(p.root(), &p.config.layout, &p.id).expect("manifest");
    assert_eq!(
        crate::assets::find(&manifest, "elsheria")
            .expect("asset")
            .versions
            .len(),
        1
    );
    let clips = crate::animations::list(p.root()).expect("clips");
    assert_eq!(clips.len(), 1);

    let states: Vec<RecipeBakeState> = told.lock().expect("told").iter().map(|r| r.state).collect();
    assert_eq!(states.first(), Some(&RecipeBakeState::Running));
    assert_eq!(states.last(), Some(&RecipeBakeState::Done));
    assert!(
        told.lock()
            .expect("told")
            .iter()
            .any(|r| r.progress.as_ref().is_some_and(|g| g.done == 31)),
        "the script's progress was told"
    );

    // The record outlives the service, as across a restart.
    let (again, _) = self::bakes(false);
    let read = again
        .get(p.root(), &p.config, &p.id, &p.recipe.id)
        .expect("get");
    assert_eq!(read.map(|r| r.state), Some(RecipeBakeState::Done));
}

#[test]
fn a_bake_is_cancelled_part_way_and_one_runs_at_a_time() {
    let p = reaching();
    let (bakes, _) = bakes(true);
    assert!(
        bakes.cancel(&p.id, &p.recipe.id).is_err(),
        "nothing is baking"
    );
    bakes
        .start(
            p.root(),
            &p.config,
            Toolbox::default(),
            &p.request(),
            RecipeBakeStarter::Person,
        )
        .expect("start");
    let twice = bakes.start(
        p.root(),
        &p.config,
        Toolbox::default(),
        &p.request(),
        RecipeBakeStarter::Person,
    );
    assert!(
        twice
            .expect_err("twice")
            .to_string()
            .contains("baking already")
    );
    bakes.cancel(&p.id, &p.recipe.id).expect("cancel");
    let stopped = settled(&bakes, &p);
    assert_eq!(stopped.state, RecipeBakeState::Cancelled);
    assert!(stopped.result.is_none() && stopped.error.is_none());
}

#[test]
fn a_bake_the_app_closed_on_reads_as_failed() {
    let p = reaching();
    let (bakes, _) = bakes(true);
    bakes
        .start(
            p.root(),
            &p.config,
            Toolbox::default(),
            &p.request(),
            RecipeBakeStarter::Person,
        )
        .expect("start");
    // Another service, as after a restart, finds the record still running.
    let (after, _) = self::bakes(false);
    let read = after
        .get(p.root(), &p.config, &p.id, &p.recipe.id)
        .expect("get")
        .expect("record");
    assert_eq!(read.state, RecipeBakeState::Failed);
    assert!(read.error.expect("error").contains("app closed"));
    bakes.cancel(&p.id, &p.recipe.id).expect("cancel");
    let last = settled(&bakes, &p);
    assert_eq!(last.state, RecipeBakeState::Cancelled, "{:?}", last.error);
}

#[test]
fn a_bake_of_another_project_or_recipe_name_is_refused() {
    let p = reaching();
    let (bakes, _) = bakes(false);
    let mut request = p.request();
    request.recipe = "../escape".into();
    assert!(
        bakes
            .start(
                p.root(),
                &p.config,
                Toolbox::default(),
                &request,
                RecipeBakeStarter::Agent
            )
            .is_err()
    );
    assert!(bakes.get(p.root(), &p.config, "..", "x").is_err());
}

#[test]
fn a_bake_reads_as_out_of_date_once_the_recipe_or_the_model_moves_on() {
    let p = reaching();
    let (bakes, _) = self::bakes(false);
    bakes
        .start(
            p.root(),
            &p.config,
            Toolbox::default(),
            &p.request(),
            RecipeBakeStarter::Person,
        )
        .expect("start");
    let done = settled(&bakes, &p);
    assert_eq!(done.state, RecipeBakeState::Done, "{:?}", done.error);
    assert_eq!(done.stale, None, "a fresh bake is current");

    // The model gets a newer version.
    fs::write(p.dir.join("models").join("elsheria_v2.glb"), humanoid()).expect("v2");
    crate::project::modify(p.root(), &p.config.layout, &p.id, |m| {
        crate::assets::add_version(m, "elsheria", "models/elsheria_v2.glb".into(), None).map(|_| ())
    })
    .expect("v2");
    let newer = settled(&bakes, &p).stale.expect("stale");
    assert!(newer.contains("v2") && newer.contains("v1"), "{newer}");

    // The recipe changes: that is said first.
    let mut p = p;
    p.change(
        vec![reach("l-1", "leftHand", "b-1")],
        vec![point("b-1", Some([0.3, 1.2, 0.35]))],
    );
    assert_eq!(settled(&bakes, &p).stale.as_deref(), Some(RECIPE_CHANGED));
    // Never saved: the record on disk holds no reason.
    let saved = fs::read_to_string(
        p.root()
            .join(&p.config.layout.cache)
            .join(WORK_DIR)
            .join(&p.id)
            .join(&p.recipe.id)
            .join("bake.json"),
    )
    .expect("saved");
    assert!(
        saved.contains("\"stale\": null") || !saved.contains("stale"),
        "{saved}"
    );
}

#[test]
fn a_bake_that_panics_ends_as_failed_and_the_recipe_can_bake_again() {
    let p = reaching();
    let (broken, _) = bakes_on(Fake {
        hold: false,
        panics: true,
    });
    broken
        .start(
            p.root(),
            &p.config,
            Toolbox::default(),
            &p.request(),
            RecipeBakeStarter::Person,
        )
        .expect("start");
    let failed = settled(&broken, &p);
    assert_eq!(failed.state, RecipeBakeState::Failed);
    assert_eq!(failed.error.as_deref(), Some(super::service::BROKE));
    // It left the running ones: the same service starts it again.
    let started = Instant::now();
    loop {
        match broken.start(
            p.root(),
            &p.config,
            Toolbox::default(),
            &p.request(),
            RecipeBakeStarter::Person,
        ) {
            Ok(_) => break,
            Err(err) => {
                assert!(err.to_string().contains("baking already"), "{err}");
                assert!(
                    started.elapsed() < Duration::from_secs(10),
                    "stuck as baking"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
    assert_eq!(settled(&broken, &p).state, RecipeBakeState::Failed);
}

#[test]
fn each_run_bakes_in_a_folder_of_its_own() {
    let p = reaching();
    let (bakes, _) = self::bakes(false);
    let bake = || {
        bakes
            .start(
                p.root(),
                &p.config,
                Toolbox::default(),
                &p.request(),
                RecipeBakeStarter::Person,
            )
            .expect("start");
        settled(&bakes, &p)
    };
    let dir = PathBuf::from(bake().result.expect("result").file)
        .parent()
        .expect("dir")
        .to_path_buf();
    let first = runs_in(&dir);
    // A Blender left over from that run writing on finds its folder gone,
    // never the next run's.
    assert_eq!(bake().state, RecipeBakeState::Done);
    let second = runs_in(&dir);
    assert_eq!((first.len(), second.len()), (1, 1));
    assert_ne!(first, second, "a new run takes a new folder");
    assert!(!first[0].exists(), "the earlier run's folder is cleared");
}
