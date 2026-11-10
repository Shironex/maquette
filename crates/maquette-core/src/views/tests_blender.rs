//! The standard views of real models in a real, headless Blender. Skipped
//! where no Blender is found. Set `MAQUETTE_KEEP_RENDERS` to a folder to
//! keep the pictures there and look.

use std::fs;
use std::path::Path;

use super::cameras::VIEW_IDS;
use super::*;
use crate::inbox::fixture;
use crate::pipeline::tools::Toolbox;
use crate::refs::png::{self, Pixels};

/// Small pictures, so the test stays quick: the cameras do not depend on it.
const EDGE: u32 = 256;

fn keep(set: &ViewSet, tag: &str) {
    let Some(dir) = std::env::var_os("MAQUETTE_KEEP_RENDERS") else {
        return;
    };
    let dir = Path::new(&dir).join(tag);
    fs::create_dir_all(&dir).expect("keep");
    for file in spec::files_of(set) {
        if let Some(name) = file.file_name() {
            let _ = fs::copy(&file, dir.join(name));
        }
    }
}

fn picture(file: &str) -> Pixels {
    png::decode(&fs::read(file).expect("read")).expect("png")
}

/// Which of red, green and blue leads at a point given in shares of the
/// picture from its top left, or `None` for a grey.
fn leads(pixels: &Pixels, x: f32, y: f32) -> Option<usize> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "pixel positions"
    )]
    let at = |share: f32, edge: usize| (share * edge as f32) as usize;
    let p = pixels.at(at(x, pixels.width), at(y, pixels.height));
    let top = (0..3).max_by_key(|c| p[*c])?;
    let rest = (0..3).filter(|c| *c != top).map(|c| p[c]).max()?;
    (p[top] > rest.saturating_add(30)).then_some(top)
}

const RED: Option<usize> = Some(0);
const GREEN: Option<usize> = Some(1);
const BLUE: Option<usize> = Some(2);

fn render(views: &Views, asked: &ViewsRequest, tools: &Toolbox) -> ViewSet {
    assert_eq!(
        views.request(asked, tools).expect("start"),
        Asked::Rendering
    );
    assert!(views.is_running(&asked.dir));
    // A second ask joins the run.
    assert_eq!(
        views.request(asked, tools).expect("joined"),
        Asked::Rendering
    );
    views.wait();
    match views.request(asked, tools).expect("finished") {
        Asked::Ready(set) => *set,
        Asked::Rendering => panic!("the set rendered twice"),
    }
}

#[test]
fn the_standard_set_of_a_prop_shows_its_front_its_right_and_its_top() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let model = dir.path().join("signpost_v1.glb");
    fs::write(&model, fixture::signpost()).expect("model");
    let reference = dir.path().join("ref.png");
    fs::write(&reference, fixture::ref_card()).expect("ref");
    // A ref Blender cannot read leaves the sheet and stops nothing.
    let broken = dir.path().join("broken.png");
    fs::write(&broken, b"not a picture").expect("broken ref");
    let asked = ViewsRequest {
        dir: dir.path().join("project/exports/renders/signpost/views/v1"),
        work: dir.path().join("cache/views/signpost/v1"),
        models: vec![ViewModel {
            id: "v1".into(),
            version: 1,
            file: model,
        }],
        views: standard_set(None, &[ViewTarget::Node("right".into())]).expect("set"),
        refs: vec![reference, broken],
        size: EDGE,
    };
    let views = Views::default();
    let tools = crate::test_blender::toolbox(&blender);
    let set = render(&views, &asked, &tools);
    keep(&set, "signpost");

    // Every picture is there, at the size asked for, in the folder asked for.
    assert_eq!(set.size, EDGE);
    let shots = &set.models[0].shots;
    assert_eq!(
        shots.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        VIEW_IDS
    );
    let edge = EDGE as usize;
    for shot in shots {
        let file = Path::new(&shot.file);
        assert_eq!(file.parent(), Some(asked.dir.as_path()), "{}", shot.file);
        assert!(
            fs::metadata(file).expect("picture").len() > 2000,
            "{}",
            shot.file
        );
        let pixels = picture(&shot.file);
        assert_eq!((pixels.width, pixels.height), (edge, edge), "{}", shot.id);
    }
    // The sheet holds the ref and the five views in one row.
    let sheet = picture(&set.sheet);
    assert_eq!(
        (sheet.width, sheet.height),
        (6 * edge + 7 * 8, edge + 2 * 8)
    );
    assert!(asked.dir.join(RECORD_FILE).is_file());
    assert!(
        !asked.work.join("out").exists(),
        "the scratch pictures are gone"
    );

    // The model is measured as its file has it, in glTF's axes.
    let seen = &set.models[0];
    assert_eq!(seen.triangles, 48);
    let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|k| (a[k] - b[k]).abs() < 1e-3);
    assert!(near(seen.min, [-0.2, 0.0, -0.1]), "{:?}", seen.min);
    assert!(near(seen.max, [0.26, 0.36, 0.16]), "{:?}", seen.max);
    assert_eq!(
        (set.light.sun, set.light.fill, set.light.exposure),
        (3.0, 0.18, 1.15)
    );

    // The front view stands on +Z and looks at the green marker, with the
    // red one (+X) on the right of the picture and no top face in sight.
    let front = picture(&shots[1].file);
    assert_eq!(
        leads(&front, 0.5, 0.52),
        GREEN,
        "the front marker is in the middle"
    );
    assert_eq!(
        leads(&front, 0.8, 0.52),
        RED,
        "+X is the right of the front view"
    );
    assert!(shots[1].position[2] > 0.5 && shots[1].position[0].abs() < 0.05);
    // From above the blue marker is in the middle, the front below it and
    // +X to the right.
    let top = picture(&shots[2].file);
    assert_eq!(
        leads(&top, 0.5, 0.5),
        BLUE,
        "the top marker is in the middle"
    );
    assert_eq!(
        leads(&top, 0.5, 0.66),
        GREEN,
        "the front is at the bottom of the top view"
    );
    assert_eq!(
        leads(&top, 0.8, 0.5),
        RED,
        "+X is the right of the top view"
    );
    assert!(shots[2].position[1] > 0.5);
    // The three quarter view stands front right and above: x, y and z all positive.
    assert!(
        shots[0].position.iter().all(|v| *v > 0.1),
        "{:?}",
        shots[0].position
    );
    assert_eq!(shots[0].frames, "the whole model");
    // The close ups: the part asked for, then the half that was left.
    assert_eq!(shots[3].frames, "the part right");
    let close = picture(&shots[3].file);
    assert_eq!(
        leads(&close, 0.5, 0.5),
        RED,
        "the close up is on the right marker"
    );
    assert_eq!(shots[4].frames, "the left half");

    // The same request answers from the folder, with no Blender at all.
    let again = views.request(&asked, &Toolbox::default()).expect("cached");
    assert_eq!(again, Asked::Ready(Box::new(set)));
}

#[test]
fn two_versions_side_by_side_share_every_camera() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let rebuilt = dir.path().join("signpost_v2.glb");
    fs::write(&rebuilt, fixture::signpost()).expect("v2");
    let scan = dir.path().join("signpost_v1.glb");
    fs::write(&scan, fixture::clean_prop()).expect("v1");
    let asked = ViewsRequest {
        dir: dir.path().join("views/v2-vs-v1"),
        work: dir.path().join("work"),
        models: vec![
            ViewModel {
                id: "v2".into(),
                version: 2,
                file: rebuilt,
            },
            ViewModel {
                id: "v1".into(),
                version: 1,
                file: scan,
            },
        ],
        views: standard_set(None, &[]).expect("set")[..2].to_vec(),
        refs: Vec::new(),
        size: EDGE,
    };
    let views = Views::default();
    let tools = crate::test_blender::toolbox(&blender);
    let set = render(&views, &asked, &tools);
    keep(&set, "side-by-side");
    assert_eq!(set.models.len(), 2);
    let (new, old) = (&set.models[0], &set.models[1]);
    assert_eq!((new.id.as_str(), old.id.as_str()), ("v2", "v1"));
    // The cameras are the same, shot for shot; the pictures are not.
    for (a, b) in new.shots.iter().zip(&old.shots) {
        assert_eq!((a.position, a.target, a.fov), (b.position, b.target, b.fov));
        assert!(a.file.ends_with(&format!("v2_{}.png", a.id)), "{}", a.file);
        assert!(b.file.ends_with(&format!("v1_{}.png", b.id)), "{}", b.file);
        assert_ne!(fs::read(&a.file).expect("a"), fs::read(&b.file).expect("b"));
    }
    // Each is measured on its own: the crate is half a metre, the signpost not.
    assert!(
        (old.max[0] - old.min[0] - 0.5).abs() < 1e-3,
        "{:?}",
        old.max
    );
    assert!(
        (new.max[0] - new.min[0] - 0.46).abs() < 1e-3,
        "{:?}",
        new.max
    );
    // A row per model on the sheet.
    let sheet = picture(&set.sheet);
    let edge = EDGE as usize;
    assert_eq!(
        (sheet.width, sheet.height),
        (2 * edge + 3 * 8, 2 * edge + 3 * 8)
    );
}

#[test]
fn a_render_that_fails_says_why_once_and_a_cancelled_one_says_nothing() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let model = dir.path().join("signpost_v1.glb");
    fs::write(&model, fixture::signpost()).expect("model");
    let mut asked = ViewsRequest {
        dir: dir.path().join("views/v1"),
        work: dir.path().join("work"),
        models: vec![ViewModel {
            id: "v1".into(),
            version: 1,
            file: model,
        }],
        views: standard_set(None, &[ViewTarget::Node("handle".into())]).expect("set"),
        refs: Vec::new(),
        size: EDGE,
    };
    let views = Views::default();
    let tools = crate::test_blender::toolbox(&blender);
    assert_eq!(
        views.request(&asked, &tools).expect("start"),
        Asked::Rendering
    );
    views.wait();
    // No part is called handle: the script names the ones there are.
    let failed = views.request(&asked, &tools).expect_err("failed");
    assert!(failed.to_string().contains("did not render"), "{failed}");
    assert!(!asked.dir.join(RECORD_FILE).exists());

    // A cancelled run leaves nothing behind and no complaint.
    asked.views = standard_set(None, &[]).expect("set");
    assert_eq!(
        views.request(&asked, &tools).expect("start"),
        Asked::Rendering
    );
    assert!(views.cancel(&asked.dir));
    views.wait();
    assert!(!asked.dir.join(RECORD_FILE).exists());
    assert_eq!(
        views.request(&asked, &tools).expect("again"),
        Asked::Rendering
    );
    views.wait();
    assert!(matches!(
        views.request(&asked, &tools).expect("done"),
        Asked::Ready(_)
    ));
}
