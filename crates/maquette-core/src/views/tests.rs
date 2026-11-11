//! Tests for the standard views: the camera set, the names of the pictures
//! and the service's answers without Blender.

use std::fs;
use std::path::{Path, PathBuf};

use super::cameras::{
    CLOSE_FILL, FRONT, MAX_ELEVATION, THREE_QUARTER, TOP, VIEW_IDS, WHOLE_FILL, standard_set,
};
use super::*;
use crate::pipeline::tools::Toolbox;

fn ids(set: &[StandardView]) -> Vec<&str> {
    set.iter().map(|v| v.id.as_str()).collect()
}

#[test]
fn the_set_is_three_quarter_front_top_and_two_close_ups() {
    let set = standard_set(None, &[]).expect("set");
    assert_eq!(ids(&set), VIEW_IDS);
    assert_eq!(
        (set[0].azimuth, set[0].elevation),
        (THREE_QUARTER.azimuth, THREE_QUARTER.elevation)
    );
    assert_eq!(
        (set[1].azimuth, set[1].elevation),
        (FRONT.azimuth, FRONT.elevation)
    );
    assert_eq!(
        (set[2].azimuth, set[2].elevation),
        (TOP.azimuth, TOP.elevation)
    );
    for whole in &set[..3] {
        assert_eq!(whole.target, ViewTarget::Whole);
        assert_eq!(whole.fill, WHOLE_FILL);
    }
    // The close ups look from the three quarter side at one half each.
    assert_eq!(set[3].target, ViewTarget::Half(0));
    assert_eq!(set[4].target, ViewTarget::Half(1));
    for close in &set[3..] {
        assert_eq!(
            (close.azimuth, close.elevation),
            (set[0].azimuth, set[0].elevation)
        );
        assert_eq!(close.fill, CLOSE_FILL);
    }
}

#[test]
fn the_three_quarter_stands_where_the_thumbnail_and_the_refs_do() {
    // The Board's thumbnail camera, in the script's own numbers.
    let thumb = crate::thumbs::SCRIPT;
    assert!(thumb.contains(&format!(
        "AZIMUTH = math.radians({})",
        THREE_QUARTER.azimuth
    )));
    assert!(thumb.contains(&format!(
        "ELEVATION = math.radians({})",
        THREE_QUARTER.elevation
    )));
    assert!(thumb.contains(&format!(
        "FRONT_ELEVATION = math.radians({})",
        FRONT.elevation
    )));
    assert!(thumb.contains(&format!("FILL = {WHOLE_FILL}")));
    // Object refs are asked for from the same side.
    let set = standard_set(None, &[]).expect("set");
    assert_eq!(
        set[0].stands(),
        "38 degrees round from the front towards +X, the right of the front view, 22 degrees up"
    );
    assert_eq!(
        set[1].stands(),
        "straight in front, on the model's +Z side, 6 degrees up"
    );
    assert!(set[2].stands().starts_with("straight above, looking down"));
}

#[test]
fn an_angle_moves_the_three_quarter_and_the_close_ups_only() {
    let angle = ViewAngle {
        azimuth: -30.0,
        elevation: 35.0,
    };
    let set = standard_set(Some(angle), &[]).expect("set");
    assert_eq!((set[0].azimuth, set[0].elevation), (-30.0, 35.0));
    assert_eq!((set[3].azimuth, set[4].azimuth), (-30.0, -30.0));
    assert_eq!(
        (set[1].azimuth, set[1].elevation),
        (FRONT.azimuth, FRONT.elevation)
    );
    assert_eq!(set[2].elevation, TOP.elevation);
    assert!(
        set[0]
            .stands()
            .contains("30 degrees round from the front towards -X")
    );

    for bad in [
        ViewAngle {
            azimuth: 200.0,
            elevation: 20.0,
        },
        ViewAngle {
            azimuth: 0.0,
            elevation: MAX_ELEVATION + 1.0,
        },
        ViewAngle {
            azimuth: 0.0,
            elevation: -5.0,
        },
        ViewAngle {
            azimuth: f32::NAN,
            elevation: 20.0,
        },
    ] {
        let err = standard_set(Some(bad), &[]).expect_err("refused");
        assert!(
            err.to_string().contains("azimuth from -180 to 180"),
            "{err}"
        );
    }
}

#[test]
fn close_ups_frame_a_part_a_box_or_the_half_that_is_left() {
    let mug = ViewTarget::Node(" mug ".to_string());
    let set = standard_set(None, std::slice::from_ref(&mug)).expect("set");
    assert_eq!(set[3].target, ViewTarget::Node("mug".to_string()));
    assert_eq!(set[4].target, ViewTarget::Half(0));
    // A named half leaves the other one for the second close up.
    let set = standard_set(None, &[ViewTarget::Half(0)]).expect("set");
    assert_eq!(set[4].target, ViewTarget::Half(1));
    let corner = ViewTarget::Box([[0.0, 0.5, 0.0], [0.5, 1.0, 1.0]]);
    let set = standard_set(None, &[corner.clone(), mug]).expect("set");
    assert_eq!(set[3].target, corner);
    assert_eq!(ids(&set), VIEW_IDS);

    let refused = |targets: &[ViewTarget]| {
        standard_set(None, targets)
            .expect_err("refused")
            .to_string()
    };
    let three = [
        ViewTarget::Half(0),
        ViewTarget::Half(1),
        ViewTarget::Half(0),
    ];
    assert!(refused(&three).contains("2 close ups"));
    assert!(refused(&[ViewTarget::Half(2)]).contains("0 or 1"));
    assert!(refused(&[ViewTarget::Node("  ".into())]).contains("object's name"));
    assert!(refused(&[ViewTarget::Whole]).contains("not the whole model"));
    for flat in [
        [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
        [[0.0, 0.0, 0.0], [1.5, 1.0, 1.0]],
    ] {
        assert!(refused(&[ViewTarget::Box(flat)]).contains("shares of the bounding box"));
    }
}

#[test]
fn pictures_are_named_after_their_view_and_side_by_side_after_their_model() {
    assert_eq!(file_name(1, "v3", "three-quarter"), "three-quarter.png");
    assert_eq!(file_name(2, "v3", "close-up-1"), "v3_close-up-1.png");
    let project = Path::new("/ws/projects/frostveil");
    let standard = standard_set(None, &[]).expect("set");
    let dir = set_dir(project, "book-stack", 3, None, false, &standard);
    assert_eq!(dir, project.join("exports/renders/book-stack/views/v3"));
    let side = |as_is| set_dir(project, "book-stack", 3, Some(2), as_is, &standard);
    assert!(side(false).ends_with("views/v3-vs-v2"));
    assert!(side(true).ends_with("views/v3-vs-v2-as-is"));
    assert!(set_dir(project, "book-stack", 1, None, true, &standard).ends_with("views/v1-as-is"));
}

#[test]
fn a_set_with_other_cameras_keeps_a_folder_of_its_own() {
    let project = Path::new("/ws/projects/frostveil");
    let folder = |views: &[StandardView]| {
        let dir = set_dir(project, "book-stack", 3, None, false, views);
        dir.file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned()
    };
    let standard = folder(&standard_set(None, &[]).expect("set"));
    let turned = ViewAngle {
        azimuth: -60.0,
        elevation: 22.0,
    };
    let other = folder(&standard_set(Some(turned), &[]).expect("turned"));
    let part = [ViewTarget::Node("mug".to_string())];
    let closer = folder(&standard_set(None, &part).expect("close ups"));
    assert_eq!(standard, "v3");
    for name in [&other, &closer] {
        assert_eq!(name.len(), "v3-".len() + 8, "{name}");
        assert!(name.starts_with("v3-"), "{name}");
    }
    assert_ne!(other, closer);
    // The same cameras always land in the same folder.
    assert_eq!(
        other,
        folder(&standard_set(Some(turned), &[]).expect("again"))
    );
}

fn request(dir: &Path, models: &[(&str, u32)]) -> ViewsRequest {
    let folder = dir.join("models");
    fs::create_dir_all(&folder).expect("models");
    ViewsRequest {
        dir: dir.join("out"),
        work: dir.join("work"),
        models: models
            .iter()
            .map(|(id, version)| {
                let file = folder.join(format!("book_{id}.glb"));
                fs::write(&file, crate::inbox::fixture::signpost()).expect("model");
                ViewModel {
                    id: (*id).to_string(),
                    version: *version,
                    file,
                }
            })
            .collect(),
        views: standard_set(None, &[]).expect("set"),
        refs: Vec::new(),
        size: SIZE,
    }
}

#[test]
fn the_spec_gives_every_view_a_file_per_model_and_the_sheet_its_refs() {
    let dir = tempfile::tempdir().expect("dir");
    let mut asked = request(dir.path(), &[("v3", 3), ("v2", 2)]);
    asked.refs = vec![PathBuf::from("/ws/refs/book.png")];
    let spec = spec::spec_json(&asked, &asked.work);
    assert_eq!(spec["size"], SIZE);
    assert_eq!(spec["models"][1]["id"], "v2");
    let views = spec["views"].as_array().expect("views");
    assert_eq!(views.len(), 5);
    let out = asked.work.join("out");
    assert_eq!(
        views[1]["outs"]["v3"],
        serde_json::json!(out.join("v3_front.png"))
    );
    assert_eq!(
        views[1]["outs"]["v2"],
        serde_json::json!(out.join("v2_front.png"))
    );
    assert_eq!(views[3]["half"], 0);
    assert_eq!(views[4]["half"], 1);
    assert!(views[0].get("half").is_none() && views[0].get("node").is_none());
    assert_eq!(spec["sheet"]["refs"][0], "/ws/refs/book.png");
    assert_eq!(
        spec["sheet"]["out"],
        serde_json::json!(out.join(SHEET_FILE))
    );
}

#[test]
fn the_key_follows_the_models_the_views_and_the_size() {
    let dir = tempfile::tempdir().expect("dir");
    let asked = request(dir.path(), &[("v3", 3)]);
    let key = spec::key_of(&asked).expect("key");
    assert_eq!(key, spec::key_of(&asked).expect("again"));
    let mut turned = asked.clone();
    turned.views = standard_set(
        Some(ViewAngle {
            azimuth: -38.0,
            elevation: 22.0,
        }),
        &[],
    )
    .expect("set");
    assert_ne!(key, spec::key_of(&turned).expect("turned"));
    let mut small = asked.clone();
    small.size = 256;
    assert_ne!(key, spec::key_of(&small).expect("small"));
    fs::write(&asked.models[0].file, crate::inbox::fixture::clean_prop()).expect("rebuilt");
    assert_ne!(key, spec::key_of(&asked).expect("rebuilt"));
}

#[test]
fn a_request_is_checked_before_blender_is_asked_for() {
    let dir = tempfile::tempdir().expect("dir");
    let views = Views::default();
    let tools = Toolbox::default();
    let refused = |asked: &ViewsRequest| views.request(asked, &tools).expect_err("refused");

    let good = request(dir.path(), &[("v3", 3)]);
    let none = refused(&good);
    assert_eq!(none.code(), "unavailable");
    assert!(none.to_string().contains("Blender was not found"), "{none}");

    let mut twice = request(dir.path(), &[("v3", 3), ("v2", 2)]);
    twice.models[1].id = "v3".into();
    assert!(
        refused(&twice)
            .to_string()
            .contains("two different versions")
    );
    let mut three = request(dir.path(), &[("v3", 3), ("v2", 2)]);
    three.models.push(three.models[0].clone());
    assert!(refused(&three).to_string().contains("one model, or two"));
    let mut blend = good.clone();
    blend.models[0].file = dir.path().join("scene.blend");
    assert!(
        refused(&blend)
            .to_string()
            .contains("not a .glb, .gltf or .obj")
    );
    let mut gone = good.clone();
    gone.models[0].file = dir.path().join("models/gone.glb");
    assert_eq!(refused(&gone).code(), "not-found");
    let mut huge = good.clone();
    huge.size = 9000;
    assert!(refused(&huge).to_string().contains("64 to 4096 px"));
    assert!(!views.is_running(&good.dir));
    assert!(!views.cancel(&good.dir));
}

#[test]
fn a_set_that_is_there_answers_without_blender_until_something_changes() {
    let dir = tempfile::tempdir().expect("dir");
    let asked = request(dir.path(), &[("v3", 3)]);
    let key = spec::key_of(&asked).expect("key");
    fs::create_dir_all(&asked.dir).expect("out");
    let shot = |id: &str| ViewShot {
        id: id.to_string(),
        file: asked.dir.join(file_name(1, "v3", id)).display().to_string(),
        azimuth: 0.0,
        elevation: 6.0,
        frames: "the whole model".to_string(),
        position: [0.0, 0.1, 1.0],
        target: [0.0, 0.1, 0.0],
        fov: 33.4,
    };
    let set = ViewSet {
        version: RECORD_VERSION,
        key,
        dir: asked.dir.display().to_string(),
        size: SIZE,
        models: vec![ViewedModel {
            id: "v3".into(),
            version: 3,
            file: asked.models[0].file.display().to_string(),
            min: [-0.2, 0.0, -0.1],
            max: [0.26, 0.36, 0.16],
            triangles: 48,
            shots: VIEW_IDS.iter().map(|id| shot(id)).collect(),
        }],
        sheet: asked.dir.join(SHEET_FILE).display().to_string(),
        light: PageLight {
            sun: 3.0,
            fill: 0.18,
            tone: "aces-filmic".into(),
            exposure: 1.15,
        },
        seconds: 12.5,
        rendered_at: "2026-11-10T21:00:00.000Z".into(),
    };
    crate::json::write_json(&asked.dir.join(RECORD_FILE), &set).expect("record");
    let views = Views::default();
    let tools = Toolbox::default();
    // The record alone is not a set: its pictures have to be there.
    assert_eq!(
        views
            .request(&asked, &tools)
            .expect_err("no pictures")
            .code(),
        "unavailable"
    );
    for id in VIEW_IDS {
        fs::write(asked.dir.join(file_name(1, "v3", id)), b"png").expect("picture");
    }
    fs::write(asked.dir.join(SHEET_FILE), b"png").expect("sheet");
    assert_eq!(
        views.request(&asked, &tools).expect("cached"),
        Asked::Ready(Box::new(set.clone()))
    );
    assert_eq!(stored(&asked.dir), Some(set));
    // Another model under the same name is another set.
    fs::write(&asked.models[0].file, crate::inbox::fixture::clean_prop()).expect("rebuilt");
    assert_eq!(
        views.request(&asked, &tools).expect_err("stale").code(),
        "unavailable"
    );
}

#[cfg(unix)]
#[test]
fn a_stopped_render_reads_as_stopped_even_with_an_older_set_in_the_folder() {
    use crate::pipeline::tools::ResolvedTool;
    use crate::toolchain::{HostOs, ToolId};

    let dir = tempfile::tempdir().expect("dir");
    let asked = request(dir.path(), &[("v3", 3)]);
    // An older set of other cameras stays in the folder.
    fs::create_dir_all(&asked.dir).expect("out");
    let old = ViewSet {
        version: RECORD_VERSION,
        key: "an-older-set".into(),
        dir: asked.dir.display().to_string(),
        size: SIZE,
        models: Vec::new(),
        sheet: asked.dir.join(SHEET_FILE).display().to_string(),
        light: PageLight {
            sun: 3.0,
            fill: 0.18,
            tone: "aces-filmic".into(),
            exposure: 1.15,
        },
        seconds: 1.0,
        rendered_at: "2026-11-10T21:00:00.000Z".into(),
    };
    crate::json::write_json(&asked.dir.join(RECORD_FILE), &old).expect("record");
    // A Blender stand in that never finishes on its own.
    let program = dir.path().join("blender");
    fs::write(&program, "#!/bin/sh\nsleep 30\n").expect("script");
    let mut perms = fs::metadata(&program).expect("meta").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    fs::set_permissions(&program, perms).expect("chmod");
    let mut tools = Toolbox {
        path_env: std::ffi::OsString::from("/bin:/usr/bin"),
        os: Some(HostOs::current()),
        ..Toolbox::default()
    };
    tools.tools.insert(
        ToolId::Blender,
        ResolvedTool {
            program,
            prefix: Vec::new(),
            version: "5.2.2".into(),
        },
    );
    let views = Views::default();
    assert_eq!(
        views.request(&asked, &tools).expect("asked"),
        Asked::Rendering
    );
    assert!(views.cancel(&asked.dir));
    views.wait();
    assert_eq!(views.outcome(&asked).expect("stopped"), None);
    // Asked once; after that the older set answers as before.
    assert!(views.outcome(&asked).is_err());
}
