use std::collections::BTreeMap;
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

fn cabinet() -> ObjectStates {
    let open = PartPose {
        position: [0.0; 3],
        rotation: [0.0, -105.0, 0.0],
    };
    ObjectStates {
        states: vec![
            ObjectState {
                id: "closed".into(),
                name: "Closed".into(),
                parts: BTreeMap::new(),
            },
            ObjectState {
                id: "door-open".into(),
                name: "Door open".into(),
                parts: BTreeMap::from([("Door".to_string(), open)]),
            },
        ],
        transitions: vec![StateTransition {
            from: "closed".into(),
            to: "door-open".into(),
            duration: 0.6,
            delay: 0.0,
            easing: Easing::EaseOut,
            stagger: 0.0,
        }],
        triggers: vec![StateTrigger {
            kind: TriggerKind::Stop,
            to: "door-open".into(),
            stop: Some("switch-room".into()),
            part: None,
            start: None,
            end: None,
        }],
    }
}

fn with_cabinet() -> SceneLayout {
    let mut layout = SceneLayout::default();
    layout.stops.push(stop("switch-room"));
    layout.states.insert("cabinet".into(), cabinet());
    layout
}

#[test]
fn a_layout_written_before_states_loads_with_none() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        path(dir.path()),
        r#"{ "version": 1, "models": { "lamp": { "position": [1, 0, 0], "rotation": [0, 0, 0], "scale": 1 } }, "stops": [], "desk": null }"#,
    )
    .expect("old file");
    let layout = load(dir.path()).expect("loads");
    assert!(layout.states.is_empty());
    assert!(layout.clips.is_empty());
    assert_eq!(layout.models["lamp"].position, [1.0, 0.0, 0.0]);
}

#[test]
fn states_and_clip_choices_round_trip_and_clips_that_ship_are_left_out() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut layout = with_cabinet();
    layout.clips.insert(
        "figure".into(),
        BTreeMap::from([
            ("Sculpt loop".to_string(), ClipSettings { ship: false }),
            ("Walk loop".to_string(), ClipSettings { ship: true }),
        ]),
    );
    save(dir.path(), &layout).expect("save");
    let text = fs::read_to_string(path(dir.path())).expect("read");
    let value: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(
        value["version"],
        json!(1),
        "older builds still read the file"
    );
    assert_eq!(
        value["states"]["cabinet"]["states"][1]["parts"]["Door"]["rotation"][1],
        json!(-105.0)
    );
    assert_eq!(
        value["states"]["cabinet"]["transitions"][0]["easing"],
        json!("ease-out")
    );
    assert_eq!(
        value["clips"]["figure"]["Sculpt loop"]["ship"],
        json!(false)
    );
    assert!(value["clips"]["figure"].get("Walk loop").is_none());
    let back = load(dir.path()).expect("load");
    assert_eq!(back.states, layout.states);
    assert_eq!(clips_off(&back, "figure"), ["Sculpt loop"]);
    assert!(clips_off(&back, "lamp").is_empty());
}

#[test]
fn validation_refuses_states_that_point_at_nothing() {
    assert!(super::validate(&with_cabinet()).is_ok());
    let refused = |change: &dyn Fn(&mut ObjectStates), needle: &str| {
        let mut layout = with_cabinet();
        let object = layout.states.get_mut("cabinet").expect("cabinet");
        change(object);
        let err = super::validate(&layout).expect_err(needle).to_string();
        assert!(err.contains(needle), "{needle}: {err}");
    };
    refused(&|o| o.states[1].name = " ".into(), "name");
    refused(&|o| o.states[1].id = "Door Open".into(), "id of lowercase");
    refused(&|o| o.states[1].id = "closed".into(), "share the id");
    refused(&|o| o.transitions[0].to = "gone".into(), "not there");
    refused(&|o| o.transitions[0].to = "closed".into(), "to itself");
    refused(&|o| o.transitions[0].duration = 90.0, "duration");
    refused(&|o| o.transitions.push(o.transitions[0].clone()), "twice");
    refused(
        &|o| o.triggers[0].stop = Some("nope".into()),
        "not in the layout",
    );
    refused(&|o| o.triggers[0].stop = None, "needs the camera stop");
    refused(
        &|o| {
            o.triggers[0].kind = TriggerKind::Scroll;
            o.triggers[0].start = Some(60.0);
            o.triggers[0].end = Some(40.0);
        },
        "scroll trigger",
    );
    refused(
        &|o| {
            if let Some(pose) = o.states[1].parts.get_mut("Door") {
                pose.rotation[1] = f32::NAN;
            }
        },
        "not a number",
    );
    refused(&|o| o.states.clear(), "no states");
}

#[test]
fn removing_a_stop_takes_its_triggers_and_states_patch_whole_per_asset() {
    let known = vec!["cabinet".to_string(), "figure".to_string()];
    let layout = with_cabinet();
    let patch: LayoutPatch = serde_json::from_value(json!({
        "stops": [{ "id": "switch-room", "remove": true }],
        "clips": { "figure": { "Wave": { "ship": false } } }
    }))
    .expect("patch");
    let next = apply(&layout, &patch, &known).expect("apply");
    assert!(next.states["cabinet"].triggers.is_empty());
    assert_eq!(next.states["cabinet"].states.len(), 2);
    assert_eq!(clips_off(&next, "figure"), ["Wave"]);

    let gone: LayoutPatch =
        serde_json::from_value(json!({ "states": { "cabinet": null } })).expect("patch");
    assert!(
        apply(&next, &gone, &known)
            .expect("apply")
            .states
            .is_empty()
    );

    let unknown: LayoutPatch = serde_json::from_value(json!({
        "states": { "nope": { "states": [], "transitions": [], "triggers": [] } }
    }))
    .expect("patch");
    let err = apply(&next, &unknown, &known)
        .expect_err("unknown")
        .to_string();
    assert!(err.contains("no model for the asset nope"), "{err}");

    let back_on: LayoutPatch =
        serde_json::from_value(json!({ "clips": { "figure": { "Wave": { "ship": true } } } }))
            .expect("patch");
    assert!(
        apply(&next, &back_on, &known)
            .expect("apply")
            .clips
            .is_empty()
    );
}

#[test]
fn prune_counts_what_it_drops() {
    let mut layout = with_cabinet();
    assert_eq!(prune_stops(&mut layout), 0);
    layout.stops.clear();
    assert_eq!(prune_stops(&mut layout), 1);
    assert!(super::validate(&layout).is_ok());
}
