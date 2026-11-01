use std::fs;

use serde_json::json;

use super::*;
use crate::layout::{LayoutPatch, LayoutStop, PlacementPatch, apply, load, path, update};

fn stop(id: &str) -> LayoutStop {
    LayoutStop {
        id: id.to_string(),
        name: id.to_string(),
        position: [0.0, 1.0, 2.0],
        target: [0.0, 0.0, 0.0],
        fov: 35.0,
        models: None,
    }
}

fn cable() -> ScenePath {
    ScenePath {
        id: "main-feed".into(),
        name: "Main feed".into(),
        curve: PathCurve::CatmullRom,
        points: vec![[0.0, 0.0, 1.0], [0.5, 0.0, 0.5], [0.5, 1.0, 0.0]],
        start: Some("trench".into()),
        end: Some("panel".into()),
        width: DEFAULT_WIDTH,
        color: DEFAULT_COLOR.into(),
        glow: DEFAULT_GLOW.into(),
    }
}

fn lit() -> PathTrack {
    PathTrack {
        easing: Easing::Linear,
        keys: vec![
            PathKey {
                stop: "porch".into(),
                amount: 0.0,
            },
            PathKey {
                stop: "canopy".into(),
                amount: 1.0,
            },
        ],
    }
}

fn wired() -> SceneLayout {
    let mut layout = SceneLayout {
        stops: vec![stop("porch"), stop("canopy")],
        paths: vec![cable()],
        ..SceneLayout::default()
    };
    for asset in ["trench", "panel"] {
        layout.models.insert(asset.into(), Placement::default());
    }
    layout.timeline.paths.insert("main-feed".into(), lit());
    layout
}

fn known() -> Vec<String> {
    vec!["trench".into(), "panel".into()]
}

#[test]
fn paths_save_and_load_and_older_files_load_without_them() {
    let dir = tempfile::tempdir().expect("tempdir");
    update(dir.path(), |_| Ok(wired())).expect("save");
    let back = load(dir.path()).expect("load");
    assert_eq!(back.paths, wired().paths);
    assert_eq!(back.timeline.paths, wired().timeline.paths);
    let text = fs::read_to_string(path(dir.path())).expect("read");
    assert!(text.contains("\"catmull-rom\""), "{text}");

    fs::write(
        path(dir.path()),
        json!({ "version": 1, "models": {}, "stops": [], "timeline": { "explode": {} } })
            .to_string(),
    )
    .expect("old");
    let old = load(dir.path()).expect("old");
    assert!(old.paths.is_empty());
    assert!(old.timeline.paths.is_empty());
}

#[test]
fn validation_refuses_paths_the_site_could_not_draw() {
    assert!(crate::layout::validate(&wired()).is_ok());
    let refused = |change: &dyn Fn(&mut SceneLayout), needle: &str| {
        let mut layout = wired();
        change(&mut layout);
        let err = crate::layout::validate(&layout)
            .expect_err(needle)
            .to_string();
        assert!(err.contains(needle), "{err}");
    };
    refused(&|l| l.paths[0].name = " ".into(), "Give every path a name");
    refused(&|l| l.paths[0].id = "Main Feed".into(), "lowercase letters");
    refused(&|l| l.paths[0].points.truncate(1), "Give it 2 to");
    refused(&|l| l.paths[0].points[1][0] = f32::NAN, "not a number");
    refused(&|l| l.paths[0].points[1][2] = 20_000.0, "from the middle");
    refused(&|l| l.paths[0].width = 0.0, "m wide");
    refused(&|l| l.paths[0].color = "yellow".into(), "Use #rrggbb");
    refused(&|l| l.paths[0].glow = "#ffe08".into(), "Use #rrggbb");
    refused(
        &|l| l.paths.push(cable()),
        "Two paths share the id main-feed",
    );
    refused(
        &|l| {
            l.timeline.paths.insert("gone".into(), lit());
        },
        "lights the path gone",
    );
    refused(
        &|l| {
            let track = l.timeline.paths.get_mut("main-feed").expect("track");
            track.keys[1].amount = 1.5;
        },
        "Use 0 to 1",
    );
    refused(
        &|l| {
            let track = l.timeline.paths.get_mut("main-feed").expect("track");
            track.keys[1].stop = "gone".into();
        },
        "not in the layout",
    );
}

#[test]
fn an_anchored_end_follows_its_model() {
    let layout = wired();
    let patch = LayoutPatch {
        models: [(
            "panel".to_string(),
            Some(PlacementPatch {
                position: Some([1.0, 0.5, 0.0]),
                ..PlacementPatch::default()
            }),
        )]
        .into(),
        ..LayoutPatch::default()
    };
    let next = apply(&layout, &patch, &known()).expect("apply");
    let points = &next.paths[0].points;
    assert_eq!(points[0], [0.0, 0.0, 1.0], "the trench did not move");
    assert_eq!(points[1], [0.5, 0.0, 0.5], "a middle point stays");
    assert_eq!(points[2], [1.5, 1.5, 0.0], "the end moved with the panel");

    let mut loose = wired();
    loose.paths[0].end = None;
    let next = apply(&loose, &patch, &known()).expect("apply");
    assert_eq!(next.paths[0].points[2], [0.5, 1.0, 0.0]);
}

#[test]
fn patches_add_change_and_remove_paths_and_their_tracks() {
    let layout = wired();
    let patch: LayoutPatch = serde_json::from_value(json!({
        "paths": [
            { "id": "main-feed", "name": "Feed", "curve": "polyline", "end": null },
            { "name": "Earth line", "points": [[0, 0, 0], [1, 0, 0]], "color": "#8FA58C" }
        ],
        "pathTracks": { "earth-line": { "easing": "ease-in", "keys": [{ "stop": "porch", "amount": 1 }] } }
    }))
    .expect("patch");
    let next = apply(&layout, &patch, &known()).expect("apply");
    assert_eq!(next.paths[0].name, "Feed");
    assert_eq!(next.paths[0].curve, PathCurve::Polyline);
    assert_eq!(next.paths[0].end, None);
    assert_eq!(next.paths[0].start.as_deref(), Some("trench"));
    let earth = &next.paths[1];
    assert_eq!(earth.id, "earth-line");
    assert_eq!(earth.color, "#8fa58c");
    assert!((earth.width - DEFAULT_WIDTH).abs() < f32::EPSILON);
    assert_eq!(next.timeline.paths["earth-line"].easing, Easing::EaseIn);

    let gone: LayoutPatch =
        serde_json::from_value(json!({ "paths": [{ "id": "main-feed", "remove": true }] }))
            .expect("patch");
    let next = apply(&layout, &gone, &known()).expect("apply");
    assert!(next.paths.is_empty());
    assert!(next.timeline.paths.is_empty(), "its track went with it");

    let refused = |patch: serde_json::Value, needle: &str| {
        let patch: LayoutPatch = serde_json::from_value(patch).expect("patch");
        let err = apply(&layout, &patch, &known())
            .expect_err(needle)
            .to_string();
        assert!(err.contains(needle), "{err}");
    };
    refused(
        json!({ "paths": [{ "points": [[0, 0, 0], [1, 0, 0]] }] }),
        "needs a name",
    );
    refused(
        json!({ "paths": [{ "name": "Loose" }] }),
        "needs its points",
    );
    refused(
        json!({ "paths": [{ "id": "nope", "remove": true }] }),
        "no path nope",
    );
    refused(
        json!({ "paths": [{ "id": "main-feed", "start": "ghost" }] }),
        "no model for the asset ghost",
    );
}

#[test]
fn removing_a_stop_drops_its_path_keys() {
    let patch: LayoutPatch =
        serde_json::from_value(json!({ "stops": [{ "id": "canopy", "remove": true }] }))
            .expect("patch");
    let next = apply(&wired(), &patch, &known()).expect("apply");
    let keys = &next.timeline.paths["main-feed"].keys;
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].stop, "porch");
}

#[test]
fn the_editor_replaces_the_paths_whole() {
    let current = wired();
    let refused = with_paths(&current, &["trench".into()], vec![cable()], BTreeMap::new())
        .expect_err("panel has no model")
        .to_string();
    assert!(refused.contains("anchored to panel"), "{refused}");

    let mut other = cable();
    other.id = "spare".into();
    other.start = None;
    other.end = None;
    let next = with_paths(
        &current,
        &known(),
        vec![other],
        current.timeline.paths.clone(),
    )
    .expect("save");
    assert_eq!(next.paths.len(), 1);
    assert!(next.timeline.paths.is_empty(), "main-feed's track is gone");
    assert_eq!(path_id(&next.paths, "Spare"), "spare-2");
}
