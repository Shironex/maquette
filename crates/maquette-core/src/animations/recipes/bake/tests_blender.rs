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
    let mut p = project_on(s.root.path(), &clip, model);
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
