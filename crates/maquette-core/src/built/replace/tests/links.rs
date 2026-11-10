//! What names the old model or its parts: it all stays in the layout, and
//! what the new model cannot resolve is reported, never dropped.

use std::collections::BTreeMap;

use super::*;
use crate::assets::AssetOrigin;
use crate::assets::remove::Sent;
use crate::layout::{
    ClipSettings, Easing, ExplodeKey, ExplodeTrack, ObjectState, ObjectStates, PartPose, ScenePath,
    StateTrigger, TriggerKind,
};
use crate::recipes::baked::{self, RecipeBakeUse};

fn state(id: &str, name: &str, parts: &[&str]) -> ObjectState {
    ObjectState {
        id: id.into(),
        name: name.into(),
        parts: parts
            .iter()
            .map(|p| ((*p).to_string(), PartPose::default()))
            .collect(),
    }
}

/// States that move a part of the scan and a part the built model has too,
/// a click on the scan's part, an explode track, a clip choice and a path
/// that ends at the asset.
fn motion(w: &World, id: &str) {
    layout::update(&w.dir, |current| {
        let mut next = current.clone();
        next.states.insert(
            id.to_string(),
            ObjectStates {
                states: vec![
                    state("rest", "Rest", &[]),
                    state("open", "Open", &["Crate", "Part_1"]),
                ],
                transitions: Vec::new(),
                triggers: vec![StateTrigger {
                    kind: TriggerKind::Click,
                    to: "open".into(),
                    stop: None,
                    part: Some("Crate".into()),
                    start: None,
                    end: None,
                }],
            },
        );
        next.timeline.explode.insert(
            id.to_string(),
            ExplodeTrack {
                distance: 0.25,
                easing: Easing::EaseInOut,
                keys: vec![ExplodeKey {
                    stop: "desk".into(),
                    amount: 1.0,
                }],
            },
        );
        next.clips.insert(
            id.to_string(),
            BTreeMap::from([("Idle".to_string(), ClipSettings { ship: false })]),
        );
        next.paths.push(ScenePath {
            id: "cable".into(),
            name: "Cable".into(),
            curve: Default::default(),
            points: vec![[0.0, 0.0, 0.0], [0.4, 0.0, -0.1]],
            start: None,
            end: Some(id.to_string()),
            width: 0.01,
            color: "#202020".into(),
            glow: "#ffcc66".into(),
        });
        Ok(next)
    })
    .expect("motion");
}

#[test]
fn states_tracks_and_paths_stay_and_what_no_longer_resolves_is_reported() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.place(&planner.id);
    motion(&w, &planner.id);
    let before = w.read("layout.json");

    let report = w.replace(&planner.id, &desk.id).report;

    assert_eq!(w.read("layout.json"), before, "nothing leaves the layout");
    assert_eq!(
        report.unresolved,
        [
            "The state Open moves the part Crate, which the built model does not have.",
            "A trigger to the state open listens on the part Crate, which the built model does not have.",
            "The clip choice for Idle names a clip the built model does not have.",
        ]
    );
    let kept: Vec<&str> = report
        .settings
        .iter()
        .filter(|s| s.action == SwapAction::Kept)
        .map(|s| s.what.as_str())
        .collect();
    assert_eq!(
        kept,
        [
            "Layout placement: position 0.4, 0, -0.1, turn 0, 35, 0, scale 1",
            "Camera stops that list it: Desk",
            "Path ends that follow it: 1",
            "Object states: 2 states, 1 trigger",
            "Explode track, 1 key",
            "Clip choices, 1 clip",
        ]
    );
}

#[test]
fn an_explode_track_on_a_model_of_one_part_is_reported() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    // The built model is one mesh this time.
    w.add_model(&desk.id, &fixture::repainted_merged(3));
    w.mark_built(&desk.id);
    w.place(&planner.id);
    motion(&w, &planner.id);

    let report = plan(&w.env(), &planner.id, &desk.id).expect("plan");

    assert!(
        report.unresolved.contains(
            &"The explode track spreads parts apart, and the built model has 1 part.".to_string()
        ),
        "{:?}",
        report.unresolved
    );
}

#[test]
fn a_library_origin_goes_and_recipe_bakes_stay() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    project::modify(&w.root, &w.config.layout, &w.project, |m| {
        assets::find_mut(m, &planner.id)?.origin = Some(AssetOrigin {
            entry: "planner".into(),
            version: 2,
        });
        Ok(())
    })
    .expect("origin");
    baked::record(
        &w.dir,
        &planner.id,
        RecipeBakeUse {
            recipe: "wave".into(),
            recipe_version: "v1".into(),
            base_clip: "idle".into(),
            clip: "wave".into(),
            mixamo: true,
            version: 1,
        },
    )
    .expect("bake");
    let bakes = w.read("recipe-bakes.json");

    let report = w.replace(&planner.id, &desk.id).report;

    assert_eq!(w.asset("spiral-planner").origin, None);
    let origin = report
        .settings
        .iter()
        .find(|s| s.what == "Library origin, planner v2")
        .expect("origin line");
    assert_eq!(origin.action, SwapAction::Reset);
    assert_eq!(w.read("recipe-bakes.json"), bakes, "the records stay");
    let kept = report
        .settings
        .iter()
        .find(|s| s.what == "Recipe bake records, 1 bake")
        .expect("bakes line");
    assert_eq!(kept.action, SwapAction::Kept);
    assert!(kept.why.contains("Mixamo"), "{}", kept.why);
    // The origin is kept for the undo.
    let record = load(&w.dir).expect("built").assets["spiral-planner"]
        .replaced
        .clone()
        .expect("record");
    assert_eq!(record.origin.map(|o| o.version), Some(2));
}

#[test]
fn a_send_still_open_for_the_asset_is_named() {
    let mut w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.sends = Stub(vec![Sent {
        id: "spiral-planner-2".into(),
        asset: planner.id.clone(),
        images: Vec::new(),
        provider: "By hand",
        state: SentState::Waiting,
    }]);

    let report = plan(&w.env(), &planner.id, &desk.id).expect("plan");

    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.starts_with("A send is still open for Spiral planner: spiral-planner-2.")),
        "{:?}",
        report.warnings
    );
}
