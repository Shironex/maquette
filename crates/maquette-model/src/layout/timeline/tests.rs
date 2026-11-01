use std::fs;

use serde_json::json;

use super::*;
use crate::layout::{LayoutPatch, LayoutStop, apply, load, path, save};

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

fn key(stop: &str, amount: f32) -> ExplodeKey {
    ExplodeKey {
        stop: stop.to_string(),
        amount,
    }
}

fn spread() -> SceneLayout {
    let mut layout = SceneLayout {
        stops: vec![stop("porch"), stop("canopy")],
        ..SceneLayout::default()
    };
    layout.timeline.explode.insert(
        "cabinet".into(),
        ExplodeTrack {
            keys: vec![key("porch", 0.0), key("canopy", 1.0)],
            ..ExplodeTrack::default()
        },
    );
    layout
}

#[test]
fn a_timeline_saves_and_loads_and_older_files_load_without_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    update(dir.path(), |_| Ok(spread())).expect("save");
    let back = load(dir.path()).expect("load");
    assert_eq!(back.timeline, spread().timeline);
    let text = fs::read_to_string(path(dir.path())).expect("read");
    assert!(text.contains("\"explode\""), "{text}");

    fs::write(
        path(dir.path()),
        json!({ "version": 1, "models": {}, "stops": [] }).to_string(),
    )
    .expect("old");
    assert!(load(dir.path()).expect("old").timeline.explode.is_empty());
}

#[test]
fn validation_refuses_keys_at_missing_stops_and_amounts_out_of_range() {
    assert!(crate::layout::validate(&spread()).is_ok());
    let refused = |change: &dyn Fn(&mut ExplodeTrack), needle: &str| {
        let mut layout = spread();
        change(layout.timeline.explode.get_mut("cabinet").expect("track"));
        let err = crate::layout::validate(&layout)
            .expect_err(needle)
            .to_string();
        assert!(err.contains(needle), "{err}");
    };
    refused(&|t| t.keys.push(key("gone", 0.5)), "not in the layout");
    refused(
        &|t| t.keys.push(key("porch", 0.5)),
        "two keys at the stop porch",
    );
    refused(&|t| t.keys[1].amount = 1.5, "Use 0 to 1");
    refused(&|t| t.keys[1].amount = f32::NAN, "Use 0 to 1");
    refused(&|t| t.distance = 0.0, "more than 0");
    refused(&|t| t.distance = 11.0, "at most 10");
}

#[test]
fn removing_a_stop_drops_its_keys_and_a_patch_sets_or_removes_a_track() {
    let mut layout = spread();
    assert_eq!(prune_stops(&mut layout), 0);
    layout.stops.retain(|s| s.id != "canopy");
    assert_eq!(prune_stops(&mut layout), 1);
    assert_eq!(layout.timeline.explode["cabinet"].keys, [key("porch", 0.0)]);

    let known = vec!["cabinet".to_string()];
    let removed: LayoutPatch = serde_json::from_value(json!({
        "stops": [{ "id": "canopy", "remove": true }]
    }))
    .expect("patch");
    let next = apply(&spread(), &removed, &known).expect("apply");
    assert_eq!(next.timeline.explode["cabinet"].keys.len(), 1);

    let set: LayoutPatch = serde_json::from_value(json!({
        "explode": { "cabinet": { "distance": 0.5, "easing": "linear",
            "keys": [{ "stop": "canopy", "amount": 0.5 }] } }
    }))
    .expect("patch");
    let next = apply(&spread(), &set, &known).expect("apply");
    let track = &next.timeline.explode["cabinet"];
    assert!((track.distance - 0.5).abs() < f32::EPSILON);
    assert_eq!(track.easing, Easing::Linear);

    let gone: LayoutPatch =
        serde_json::from_value(json!({ "explode": { "cabinet": null } })).expect("patch");
    assert!(
        apply(&spread(), &gone, &known)
            .expect("apply")
            .timeline
            .explode
            .is_empty()
    );

    let unknown: LayoutPatch = serde_json::from_value(
        json!({ "explode": { "lamp": { "distance": 0.2, "easing": "linear", "keys": [] } } }),
    )
    .expect("patch");
    assert!(apply(&spread(), &unknown, &known).is_err());
}

#[test]
fn the_style_and_text_tracks_keep_to_the_stops() {
    let dir = tempfile::tempdir().expect("dir");
    let base = SceneLayout {
        stops: vec![stop("porch"), stop("canopy")],
        ..SceneLayout::default()
    };
    let patch: LayoutPatch = serde_json::from_value(json!({
        "styleTrack": { "look": "blueprint", "easing": "linear",
            "keys": [{ "stop": "porch", "amount": 0 }, { "stop": "canopy", "amount": 1 }] },
        "textTrack": { "easing": "linear",
            "keys": [{ "stop": "porch", "amount": 1 }, { "stop": "canopy", "amount": 0 }] }
    }))
    .expect("patch");
    let next = apply(&base, &patch, &[]).expect("applies");
    update(dir.path(), |_| Ok(next.clone())).expect("saves");
    let back = load(dir.path()).expect("loads");
    assert_eq!(
        back.timeline.style.as_ref().map(|t| t.look),
        Some(TrackLook::Blueprint)
    );
    assert_eq!(back.timeline.text.as_ref().map(|t| t.keys.len()), Some(2));

    // A removed stop takes its keys along, and null removes a track.
    let gone: LayoutPatch = serde_json::from_value(json!({
        "stops": [{ "id": "canopy", "remove": true }], "textTrack": null
    }))
    .expect("patch");
    let after = apply(&back, &gone, &[]).expect("applies");
    assert_eq!(after.timeline.style.as_ref().map(|t| t.keys.len()), Some(1));
    assert!(after.timeline.text.is_none());

    // A key at a stop that is not there is refused.
    let bad = SceneLayout {
        timeline: Timeline {
            text: Some(PathTrack {
                easing: Easing::Linear,
                keys: vec![PathKey {
                    stop: "nowhere".into(),
                    amount: 1.0,
                }],
            }),
            ..Timeline::default()
        },
        ..base
    };
    assert!(update(dir.path(), |_| Ok(bad.clone())).is_err());
}

#[test]
fn a_layout_without_the_new_tracks_still_loads() {
    let dir = tempfile::tempdir().expect("dir");
    fs::write(
        path(dir.path()),
        r#"{ "version": 1, "timeline": { "explode": {} } }"#,
    )
    .expect("write");
    let layout = load(dir.path()).expect("loads");
    assert!(layout.timeline.style.is_none() && layout.timeline.text.is_none());
}
