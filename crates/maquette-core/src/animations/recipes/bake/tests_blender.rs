//! The recipe bake in a real, headless Blender: the fixture model, facing
//! glTF's +Z or turned to +X, takes a library clip with a reach laid over it. Skipped where no Blender is
//! installed.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::tests::{Project, point, project_on, reach};
use super::*;
use crate::animations::list;
use crate::animations::tests_blender::{Setup, fixture, import_fixture, setup};
use crate::pipeline::runner::{NullSink, PipelineRunner};
use crate::test_blender;
use crate::workspace::WorkspaceConfig;

/// The reach's target, in the model's own frame: in front of it, below and
/// inside its right shoulder, well within the arm's length.
const TARGET: [f32; 3] = [-0.3, 1.25, 0.3];

/// A project in `s` on fixture model `which`, with a recipe reaching its
/// right hand to [`TARGET`] on a library clip.
fn reaching(s: &Setup, which: &str) -> Project {
    let config = WorkspaceConfig::default();
    if list(s.root.path()).expect("clips").is_empty() {
        import_fixture(s, &config, "slide", "Foot Slide.fbx");
    }
    let clip = list(s.root.path()).expect("clips").remove(0).id;
    let file = s.files.join(format!("{which}.glb"));
    fixture(&s.blender, &s.root.path().join("scripts"), which, &file);
    let model = std::fs::read(&file).expect("model");
    let mut p = project_on(s.root.path(), Some(&clip), model);
    p.change(
        vec![reach("l-1", "rightHand", "b-1")],
        vec![point("b-1", Some(TARGET))],
    );
    p
}

fn baked(s: &Setup, p: &Project) -> RecipeBakeRecord {
    let runner = Arc::new(PipelineRunner::new(Arc::new(NullSink)));
    let bakes = RecipeBakes::new(runner, |_: &RecipeBakeRecord| {});
    bakes
        .start(
            p.root(),
            &p.config,
            s.tools.clone(),
            &p.request(),
            RecipeBakeStarter::Person,
        )
        .expect("start");
    let started = Instant::now();
    loop {
        let record = bakes
            .get(p.root(), &p.config, &p.id, &p.recipe.id)
            .expect("get")
            .expect("record");
        if !record.running() {
            return record;
        }
        assert!(started.elapsed() < Duration::from_secs(300), "{record:?}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn clip_names(file: &Path) -> Vec<String> {
    test_blender::glb_json(file)["animations"]
        .as_array()
        .expect("animations")
        .iter()
        .filter_map(|a| a["name"].as_str().map(String::from))
        .collect()
}

fn check(s: &Setup, which: &str, facing: [f32; 3]) -> RecipeBakeResult {
    let p = reaching(s, which);
    let record = baked(s, &p);
    assert_eq!(record.state, RecipeBakeState::Done, "{:?}", record.error);
    let result = record.result.expect("result");
    let hand = &result.reaches[0];
    assert!(hand.reached, "{which}: {hand:?}");
    assert!(hand.gap_cm.expect("gap") < GAP_LIMIT_CM, "{hand:?}");
    for (read, want) in result.facing.iter().zip(facing) {
        assert!(
            (read - want).abs() < 0.05,
            "{which} faces {:?}",
            result.facing
        );
    }
    assert!(result.mixamo, "a bake of a Mixamo clip counts as Mixamo's");
    // The library counts a clip's frames as its last key's frame (keys 0 to
    // N), so the bake reports the same number as the clip it started from.
    let base = list(s.root.path()).expect("clips").remove(0);
    assert_eq!(
        result.frames, base.frames,
        "the bake is as long as its base"
    );
    let file = Path::new(&result.file);
    assert!(file.starts_with(p.root().join(&p.config.layout.cache)));
    let mut names = clip_names(file);
    names.sort();
    assert_eq!(names, ["idle", result.clip.as_str()], "one new clip");
    assert_eq!(result.clip, "reach_out");
    assert!(marked_mixamo(file), "the staged model carries its mark");
    result
}

#[test]
fn a_reach_bakes_within_two_centimetres_whichever_way_the_model_faces() {
    let Some(s) = setup() else { return };
    let front = check(&s, "model", [0.0, 0.0, 1.0]);
    let side = check(&s, "model_x", [1.0, 0.0, 0.0]);
    let (a, b) = (&front.reaches[0], &side.reaches[0]);
    assert_eq!(a.frame, b.frame);
    let gaps = (a.gap_cm.expect("gap"), b.gap_cm.expect("gap"));
    assert!((gaps.0 - gaps.1).abs() < 0.5, "{gaps:?}");
    assert_eq!(a.flip_frame.is_some(), b.flip_frame.is_some());
    assert_eq!(
        list(s.root.path()).expect("clips").len(),
        1,
        "nothing went to the library"
    );
}

#[test]
fn a_reach_on_the_rest_pose_bakes_with_no_base_clip() {
    let Some(s) = setup() else { return };
    let file = s.files.join("model.glb");
    fixture(&s.blender, &s.root.path().join("scripts"), "model", &file);
    let mut p = project_on(s.root.path(), None, std::fs::read(&file).expect("model"));
    p.change(
        vec![reach("l-1", "rightHand", "b-1")],
        vec![point("b-1", Some(TARGET))],
    );
    let record = baked(&s, &p);
    assert_eq!(record.state, RecipeBakeState::Done, "{:?}", record.error);
    let result = record.result.expect("result");
    assert!(result.reaches[0].reached, "{:?}", result.reaches);
    assert_eq!(result.frames, crate::recipes::REST_FRAMES);
    assert!(!result.mixamo, "the rest pose is nobody's clip");
    assert!(result.base_clip.is_empty());
    assert!(result.base_slide_cm < 0.01, "the rest pose stands still");
    let mut names = clip_names(Path::new(&result.file));
    names.sort();
    assert_eq!(names, ["idle", "reach_out"], "one new clip");
}

/// The Blender processes running a bake script whose working folder is
/// under `root` (the script is passed by a relative path, so the folder is
/// the only mark of whose bake it is). Where the folder cannot be read
/// (Windows) every bake script's process counts; only call this where
/// [`can_see_processes`]. Empty once Blender is gone.
/// Whether this machine can say which processes belong to the test: `lsof`
/// reads a process's folder on macOS and Linux. Windows has no such lookup
/// here, so the "Blender is gone" check is left out there.
fn can_see_processes() -> bool {
    !cfg!(windows)
        && std::process::Command::new("lsof")
            .arg("-v")
            .output()
            .is_ok()
}

fn bake_processes(root: &Path) -> Vec<String> {
    let folder = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        (
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Process | ForEach-Object { \"$($_.ProcessId) $($_.CommandLine)\" }",
            ],
        )
    } else {
        ("ps", &["-axo", "pid=,command="])
    };
    let out = std::process::Command::new(program)
        .args(args)
        .output()
        .expect("list the processes");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.contains("bake_recipe.py"))
        .filter(|l| {
            if cfg!(windows) {
                return true;
            }
            let pid = l.split_whitespace().next().unwrap_or_default();
            let cwd = std::process::Command::new("lsof")
                .args(["-a", "-d", "cwd", "-Fn", "-p", pid])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                .unwrap_or_default();
            cwd.contains(&folder)
        })
        .map(str::to_string)
        .collect()
}

/// Cancelling a bake in a real Blender while it solves ends it: the record
/// reads cancelled, with no result, and the Blender process is gone.
///
/// The recipe is long on purpose (a rest pose at the 600 frame cap with
/// four reaches) so that Cancel has time to land mid-solve. By hand, the
/// same recipe makes a slow bake to try Cancel in the app.
#[test]
fn cancelling_a_long_bake_mid_solve_kills_blender_and_reads_cancelled() {
    let Some(s) = setup() else { return };
    let seeing = can_see_processes();
    if !seeing {
        eprintln!("no lsof here: the check that Blender is gone is skipped");
    }
    let file = s.files.join("model.glb");
    fixture(&s.blender, &s.root.path().join("scripts"), "model", &file);
    let mut p = project_on(s.root.path(), None, std::fs::read(&file).expect("model"));
    // Four reaches on each hand and foot, the most the cap lets the solver
    // chew on; the bake is quick, so Cancel has to land at once.
    let ends = [(0.0_f32, 0.0_f32), (0.01, 0.02), (0.02, 0.0), (0.0, 0.01)];
    let limbs = [
        ("rightHand", [-0.3, 1.25, 0.3]),
        ("leftHand", [0.3, 1.25, 0.3]),
        ("rightFoot", [-0.15, 0.1, 0.25]),
        ("leftFoot", [0.15, 0.1, 0.25]),
    ];
    let mut layers = Vec::new();
    let mut blanks = Vec::new();
    for (n, (bone, at)) in limbs.iter().enumerate() {
        for (m, (dx, dz)) in ends.iter().enumerate() {
            let id = format!("l-{}", n * ends.len() + m + 1);
            let blank = format!("b-{id}");
            layers.push(reach(&id, bone, &blank));
            blanks.push(point(&blank, Some([at[0] + dx, at[1], at[2] + dz])));
        }
    }
    p.change(layers, blanks);
    // The recipe runs for the longest a rest pose does.
    let file = crate::recipes::path(&p.dir, &p.recipe.id).expect("path");
    let mut recipe = crate::recipes::load_file(&file).expect("recipe");
    recipe.base = crate::recipes::RecipeBase::rest_pose(crate::recipes::MAX_REST_FRAMES);
    crate::json::write_json(&file, &recipe).expect("write");

    let runner = Arc::new(PipelineRunner::new(Arc::new(NullSink)));
    let bakes = RecipeBakes::new(runner, |_: &RecipeBakeRecord| {});
    bakes
        .start(
            p.root(),
            &p.config,
            s.tools.clone(),
            &p.request(),
            RecipeBakeStarter::Person,
        )
        .expect("start");
    let get = || {
        bakes
            .get(p.root(), &p.config, &p.id, &p.recipe.id)
            .expect("get")
            .expect("record")
    };
    // Wait until Blender is solving, not just starting.
    let started = Instant::now();
    loop {
        let record = get();
        assert!(
            record.running(),
            "ended before it could be cancelled: {record:?}"
        );
        if record
            .progress
            .as_ref()
            .is_some_and(|g| g.label.starts_with("Solving"))
        {
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(120), "{record:?}");
        std::thread::sleep(Duration::from_millis(5));
    }
    let solving_since = Instant::now();
    assert!(
        !seeing || !bake_processes(p.root()).is_empty(),
        "the bake's Blender should be running before Cancel"
    );

    bakes.cancel(&p.id, &p.recipe.id).expect("cancel");
    let cancelled = Instant::now();
    let record = loop {
        let record = get();
        if !record.running() {
            break record;
        }
        assert!(cancelled.elapsed() < Duration::from_secs(60), "{record:?}");
        std::thread::sleep(Duration::from_millis(100));
    };
    eprintln!(
        "cancelled {:?} after it began solving ({:?} seen)",
        solving_since.elapsed(),
        record.progress.as_ref().map(|g| (g.done, g.total))
    );
    assert_eq!(record.state, RecipeBakeState::Cancelled, "{record:?}");
    assert!(record.result.is_none() && record.error.is_none());
    // The process is gone, not just forgotten.
    let gone = Instant::now();
    while seeing && !bake_processes(p.root()).is_empty() {
        assert!(
            gone.elapsed() < Duration::from_secs(15),
            "Blender still runs: {:?}",
            bake_processes(p.root())
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}
