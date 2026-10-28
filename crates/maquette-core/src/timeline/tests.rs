use std::fs;

use super::*;
use crate::assets::{Asset, AssetKind, AssetStatus, AssetVersion, Stage};
use crate::inbox::fixture;
use crate::layout::{ExplodeKey, ExplodeTrack, ObjectState};
use crate::project::{PROJECT_VERSION, ProjectTemplate};

fn manifest(dir: &Path) -> ProjectManifest {
    let models = dir.join("models");
    fs::create_dir_all(&models).expect("models");
    fs::write(models.join("crate_v1.glb"), fixture::clean_prop()).expect("crate");
    let at = "2026-10-26T09:30:00.000Z".to_string();
    ProjectManifest {
        version: PROJECT_VERSION,
        id: "maquette".into(),
        name: "Maquette".into(),
        template: ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: None,
        site_repo_name: None,
        site_repo_state: Default::default(),
        source_folder: None,
        created_at: at.clone(),
        assets: vec![Asset {
            id: "crate".into(),
            name: "Crate".into(),
            kind: AssetKind::Prop,
            stage: Stage::Blender,
            status: AssetStatus::Review,
            versions: vec![AssetVersion {
                version: 1,
                file: "models/crate_v1.glb".into(),
                created_at: at.clone(),
                stats: None,
            }],
            thumbnail: None,
            created_at: at.clone(),
            updated_at: at,
            origin: None,
            refs: Vec::new(),
        }],
    }
}

fn trigger(kind: TriggerKind, to: &str) -> StateTrigger {
    StateTrigger {
        kind,
        to: to.into(),
        stop: (kind == TriggerKind::Stop).then(|| "porch".into()),
        part: None,
        start: (kind == TriggerKind::Scroll).then_some(10.0),
        end: (kind == TriggerKind::Scroll).then_some(30.0),
    }
}

fn layout_with_states() -> SceneLayout {
    let mut layout = SceneLayout::default();
    layout.stops.push(LayoutStop {
        id: "porch".into(),
        name: "Porch".into(),
        position: [0.0, 1.0, 2.0],
        target: [0.0; 3],
        fov: 35.0,
        models: None,
    });
    let state = |id: &str| ObjectState {
        id: id.into(),
        name: id.into(),
        parts: BTreeMap::new(),
    };
    layout.states.insert(
        "crate".into(),
        ObjectStates {
            states: vec![state("shut"), state("open")],
            transitions: Vec::new(),
            triggers: vec![
                trigger(TriggerKind::Stop, "open"),
                trigger(TriggerKind::Click, "open"),
            ],
        },
    );
    layout
}

#[test]
fn the_view_lists_every_model_with_its_parts_and_the_stops() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = manifest(dir.path());
    layout::save(dir.path(), &layout_with_states()).expect("layout");
    let view = view(dir.path(), &manifest).expect("view");
    assert_eq!(view.stops.len(), 1);
    assert_eq!(view.models.len(), 1);
    assert!(!view.models[0].parts.is_empty(), "the crate draws a mesh");
    assert!(view.timeline.explode.is_empty());
}

#[test]
fn a_save_replaces_the_scroll_driven_triggers_and_keeps_the_rest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = manifest(dir.path());
    layout::save(dir.path(), &layout_with_states()).expect("layout");
    let triggers = BTreeMap::from([(
        "crate".to_string(),
        vec![trigger(TriggerKind::Scroll, "open")],
    )]);
    let timeline = Timeline {
        explode: BTreeMap::from([(
            "crate".to_string(),
            ExplodeTrack {
                keys: vec![ExplodeKey {
                    stop: "porch".into(),
                    amount: 1.0,
                }],
                ..ExplodeTrack::default()
            },
        )]),
        ..Timeline::default()
    };
    let saved = save(dir.path(), &manifest, &triggers, timeline).expect("save");
    let kinds: Vec<TriggerKind> = saved.states["crate"]
        .triggers
        .iter()
        .map(|t| t.kind)
        .collect();
    assert_eq!(kinds, [TriggerKind::Click, TriggerKind::Scroll]);
    assert_eq!(saved.timeline.explode["crate"].keys.len(), 1);
    assert_eq!(
        layout::load(dir.path()).expect("load").timeline,
        saved.timeline
    );
}

#[test]
fn a_save_refuses_click_triggers_assets_without_states_and_unknown_models() {
    let current = layout_with_states();
    let known = vec!["crate".to_string()];
    let click = BTreeMap::from([(
        "crate".to_string(),
        vec![trigger(TriggerKind::Click, "open")],
    )]);
    let err = with_timeline(&current, &known, &click, Timeline::default()).expect_err("click");
    assert!(
        err.to_string().contains("stop and scroll triggers only"),
        "{err}"
    );

    let stateless = BTreeMap::from([("lamp".to_string(), Vec::new())]);
    let err = with_timeline(&current, &known, &stateless, Timeline::default()).expect_err("lamp");
    assert!(err.to_string().contains("has no states yet"), "{err}");

    let mut timeline = Timeline::default();
    timeline
        .explode
        .insert("lamp".into(), ExplodeTrack::default());
    let err = with_timeline(&current, &known, &BTreeMap::new(), timeline).expect_err("unknown");
    assert!(
        err.to_string().contains("no model for the asset lamp"),
        "{err}"
    );

    let bad = BTreeMap::from([(
        "crate".to_string(),
        vec![StateTrigger {
            start: Some(50.0),
            end: Some(20.0),
            ..trigger(TriggerKind::Scroll, "open")
        }],
    )]);
    assert!(with_timeline(&current, &known, &bad, Timeline::default()).is_err());
}
