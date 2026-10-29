//! Matching a download by its picture, end to end: a red cube made and
//! rendered by a real headless Blender against a red block ref and a blue
//! disc ref. Skipped where no Blender is installed. Nothing talks to a
//! provider or reads a real Downloads folder.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::super::tests::{Rig, rig};
use super::*;
use crate::inbox::fixture;
use crate::inbox::images::write_png;
use crate::pipeline::runner::NullSink;
use crate::providers::jobs::Provider;
use crate::providers::presets::PresetVariant;
use crate::providers::tests::add_objects;
use crate::test_blender;

const GROUND: [u8; 3] = [212, 212, 216];

/// A 600 px ref: studio grey with `colour` where `inside` says.
fn drawn(colour: [u8; 3], inside: impl Fn(f64, f64) -> bool) -> Vec<u8> {
    let mut rgb = Vec::new();
    for y in 0..600u32 {
        for x in 0..600u32 {
            let (fx, fy) = (f64::from(x) / 600.0, f64::from(y) / 600.0);
            rgb.extend_from_slice(if inside(fx, fy) { &colour } else { &GROUND });
        }
    }
    write_png(600, 600, &rgb)
}

/// A red cube as a GLB, made by headless Blender.
fn red_cube(blender: &Path, out: &Path) {
    let expr = "import bpy, sys\n\
        bpy.ops.wm.read_factory_settings(use_empty=True)\n\
        bpy.ops.mesh.primitive_cube_add(size=1)\n\
        m = bpy.data.materials.new('paint')\n\
        m.diffuse_color = (0.8, 0.05, 0.04, 1)\n\
        m.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value = (0.8, 0.05, 0.04, 1)\n\
        bpy.context.active_object.data.materials.append(m)\n\
        bpy.ops.export_scene.gltf(filepath=sys.argv[-1], export_format='GLB')";
    let out = out.display().to_string();
    test_blender::run(blender, &["--python-expr", expr, "--", &out]);
}

/// Two sends waiting: the red crate (a red block ref) and the blue ball
/// (a blue disc ref).
fn two_sends(rig: &Rig) -> (String, String) {
    let ids = add_objects(&rig.world, &["Blue ball"]);
    let refs = rig.world.project_dir().join("refs");
    let block = drawn([196, 40, 36], |x, y| {
        (0.25..0.75).contains(&x) && (0.25..0.75).contains(&y)
    });
    fs::write(refs.join("front.png"), block).expect("red ref");
    let disc = drawn([40, 70, 200], |x, y| {
        ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt() < 0.3
    });
    fs::write(refs.join("ball.png"), disc).expect("blue ref");
    let red = rig.by_hand(&rig.world.asset.clone());
    let mut request = rig
        .world
        .request(Provider::ByHand, PresetVariant::Prop, &["ball.png"]);
    request.asset = ids[0].clone();
    let blue = rig.jobs.send(&request, false, false).expect("send");
    (red.id, blue.id)
}

fn settled(rig: &Rig, id: &str) -> Arrival {
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        let found = rig
            .arrivals
            .status()
            .arrivals
            .into_iter()
            .find(|a| a.id == id)
            .expect("listed");
        if found.picture != PictureState::Queued || Instant::now() > deadline {
            return found;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn without_a_renderer_the_sends_are_ranked_by_name_alone() {
    let rig = rig();
    let (red, _) = two_sends(&rig);
    let file = rig.download("stone lantern 3d model.glb", &fixture::clean_prop());
    let arrival = rig.arrivals.take(&file).expect("seen");
    assert_eq!(arrival.picture, PictureState::Skipped);
    assert_eq!(arrival.suggestions.len(), 2, "every waiting send");
    assert_eq!(arrival.suggestions[0].job, red, "the closer name first");
    assert!(arrival.suggestions.iter().all(|s| s.picture.is_none()));
}

#[test]
fn a_red_cube_download_is_suggested_for_the_red_crate_and_never_attached() {
    let Some(blender) = test_blender::blender() else {
        return;
    };
    let rig = rig();
    let tools = test_blender::toolbox(&blender);
    rig.arrivals.set_renderer(PictureRenderer::new(
        PipelineRunner::new(Arc::new(NullSink)),
        move || tools.clone(),
    ));
    let (red, blue) = two_sends(&rig);
    let file = rig.download("model (3).glb", b"");
    red_cube(&blender, &file);

    let arrival = rig.arrivals.take(&file).expect("seen");
    assert_eq!(arrival.state, ArrivalState::Unmatched, "two sends, no name");
    assert_eq!(arrival.picture, PictureState::Queued);
    let done = settled(&rig, &arrival.id);
    assert_eq!(done.picture, PictureState::Ready, "{done:?}");
    let order: Vec<&str> = done.suggestions.iter().map(|s| s.job.as_str()).collect();
    assert_eq!(order, [red.as_str(), blue.as_str()]);
    let best = &done.suggestions[0];
    let worst = &done.suggestions[1];
    eprintln!("red crate {best:?}\nblue ball {worst:?}");
    assert!(best.picture.expect("scored") > worst.picture.expect("scored") + 0.2);
    assert_eq!(done.state, ArrivalState::Review, "waits for a look");
    assert_eq!(done.job.as_deref(), Some(red.as_str()));
    assert!(
        done.reason.starts_with("Looks like Stone lantern ("),
        "{}",
        done.reason
    );
    assert_eq!(done.inbox_file, None, "never attached by itself");
    let render = done.render.clone().expect("render");
    let cache = rig
        .world
        .root
        .join(&rig.world.config.layout.cache)
        .join(PICTURES_DIR);
    assert!(Path::new(&render).starts_with(&cache), "{render}");
    // The app heard of it twice: when it came, and when its picture did.
    let heard = rig.seen.lock().expect("lock").len();
    assert_eq!(heard, 2);
}
