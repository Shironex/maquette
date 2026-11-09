//! Pins in a real, headless Blender: the fixture model takes the fixture
//! clip, whose planted left foot slides about 5 cm and back, with that foot
//! pinned where it stands, and its left hand pinned to a prop blank the
//! layout places in front of it. Read back from the staged model, the foot
//! and its toes stay put and the hand stays on the prop while each pin holds
//! whole, and the review's drift agrees. Skipped where no Blender is
//! installed.

use std::path::Path;

use super::job::model_bone;
use super::tests::{Project, project_on};
use super::tests_blender::baked;
use super::tests_blender_joined::{Glb, fps_of, world};
use super::*;
use crate::animations::list;
use crate::animations::tests_blender::{Setup, fixture, import_fixture, setup};
use crate::recipes::pins::{held, spans};
use crate::recipes::timeline::timeline;
use crate::recipes::{BlankKind, LayerKind, LayerSettings, RecipeBlank, RecipeLayer};
use crate::workspace::WorkspaceConfig;

/// Where the layout stands the prop, from the model at the origin (glTF
/// axes). The fixture's body box is 1 m up, so the prop's middle sits at
/// (0.4, 1.2, 0.25): ahead of the model, left of its chest, within reach.
const PROP_AT: [f32; 3] = [0.4, 0.2, 0.25];
const PROP_MIDDLE: [f64; 3] = [0.4, 1.2, 0.25];

fn pin(id: &str, bone: &str, target: Option<&str>) -> RecipeLayer {
    RecipeLayer::new(
        id,
        LayerKind::Pin,
        bone,
        LayerSettings {
            start: 0.1,
            end: 0.9,
            target: target.map(str::to_string),
            ..LayerSettings::default()
        },
    )
}

/// A project in `s` on the sliding foot clip, with `layers` and a prop
/// blank "rod" the layout places at [`PROP_AT`].
fn project(s: &Setup, layers: Vec<RecipeLayer>) -> Project {
    let config = WorkspaceConfig::default();
    if list(s.root.path()).expect("clips").is_empty() {
        import_fixture(s, &config, "slide", "Foot Slide.fbx");
    }
    let clip = list(s.root.path()).expect("clips").remove(0).id;
    let file = s.files.join("model.glb");
    fixture(&s.blender, &s.root.path().join("scripts"), "model", &file);
    let bytes = std::fs::read(&file).expect("model");
    let mut p = project_on(s.root.path(), Some(&clip), bytes.clone());
    std::fs::write(p.dir.join("models").join("rod_v1.glb"), &bytes).expect("prop");
    crate::project::modify(p.root(), &p.config.layout, &p.id, |m| {
        let asset = crate::assets::add(
            m,
            crate::assets::NewAsset {
                name: "Rod".into(),
                kind: crate::assets::AssetKind::Prop,
                stage: crate::assets::Stage::Blender,
            },
        )?;
        crate::assets::add_version(m, &asset.id, "models/rod_v1.glb".into(), None).map(|_| ())
    })
    .expect("prop asset");
    crate::layout::update(&p.dir, |l| {
        let mut next = l.clone();
        next.models
            .insert("elsheria".into(), crate::layout::Placement::default());
        next.models.insert(
            "rod".into(),
            crate::layout::Placement {
                position: PROP_AT,
                ..crate::layout::Placement::default()
            },
        );
        Ok(next)
    })
    .expect("layout");
    p.change(
        layers,
        vec![RecipeBlank {
            id: "rod".into(),
            label: "The rod the hand stays on".into(),
            kind: BlankKind::Prop,
            prop: Some("rod".into()),
            point: None,
        }],
    );
    p
}

fn apart(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Bakes `p` and answers the result, the staged model, the node of each
/// VRM bone asked for and the frames each pin holds whole.
fn bake_of(
    s: &Setup,
    p: &Project,
    vrm: &[&str],
) -> (RecipeBakeResult, Glb, Vec<usize>, Vec<[u32; 2]>) {
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let record = baked(s, p);
    assert_eq!(record.state, RecipeBakeState::Done, "{:?}", record.error);
    let result = record.result.expect("result");
    let glb = Glb::read(Path::new(&result.file));
    let nodes = vrm
        .iter()
        .map(|v| glb.node(&model_bone(v, &prepared.map, &prepared.model.joints).expect("bone")))
        .collect();
    let recipe = &p.recipe.recipe;
    let whole = spans(recipe, &timeline(recipe))
        .iter()
        .filter_map(held)
        .collect();
    (result, glb, nodes, whole)
}

/// The most node `node` moves from where it is at frame `from`, over frames
/// `from` to `to`.
fn moves(glb: &Glb, result: &RecipeBakeResult, node: usize, [from, to]: [u32; 2]) -> f64 {
    let fps = fps_of(glb, result);
    let at = |f: u32| world(glb, &result.clip, node, f64::from(f) / fps);
    let spot = at(from);
    (from..=to).map(|f| apart(at(f), spot)).fold(0.0, f64::max)
}

#[test]
fn a_pinned_foot_stays_put_and_a_pinned_hand_stays_on_its_prop() {
    let Some(s) = setup() else { return };
    let bones = ["leftFoot", "leftToes", "leftHand"];

    // The clip alone: its planted left foot slides.
    let nudge = RecipeLayer::new(
        "l-9",
        LayerKind::Nudge,
        "head",
        LayerSettings {
            rotation: Some([4.0, 0.0, 0.0]),
            ..LayerSettings::default()
        },
    );
    let plain = project(&s, vec![nudge]);
    let (base, base_glb, base_nodes, _) = bake_of(&s, &plain, &bones);
    assert!(base.pins.is_empty());

    let pinned = project(
        &s,
        vec![
            pin("l-1", "leftFoot", None),
            pin("l-2", "leftHand", Some("rod")),
        ],
    );
    let (result, glb, nodes, whole) = bake_of(&s, &pinned, &bones);
    assert_eq!(result.frames, base.frames, "a pin keeps the length");
    let [foot_whole, hand_whole] = whole[..] else {
        panic!("{whole:?}")
    };
    let slid = moves(&base_glb, &base, base_nodes[0], foot_whole);
    eprintln!("the clip's left foot moves {slid} m while the pin would hold it");
    assert!(
        slid > 0.02,
        "the clip slides too little to tell a pin ({slid} m)"
    );

    let foot = moves(&glb, &result, nodes[0], foot_whole);
    let toes = moves(&glb, &result, nodes[1], foot_whole);
    eprintln!("pinned: the foot moves {foot} m, its toes {toes} m");
    assert!(foot < 0.01, "the pinned foot moves {foot} m");
    assert!(
        toes < 0.015,
        "toe and heel stay flat: the toes move {toes} m"
    );

    let fps = fps_of(&glb, &result);
    let hand_gap = (hand_whole[0]..=hand_whole[1])
        .map(|f| {
            apart(
                world(&glb, &result.clip, nodes[2], f64::from(f) / fps),
                PROP_MIDDLE,
            )
        })
        .fold(0.0, f64::max);
    eprintln!("pinned: the hand is at most {hand_gap} m from the prop");
    assert!(hand_gap < 0.01, "the hand leaves its prop by {hand_gap} m");

    let [foot_run, hand_run] = &result.pins[..] else {
        panic!("{:?}", result.pins)
    };
    assert_eq!(
        (foot_run.layer.as_str(), hand_run.layer.as_str()),
        ("l-1", "l-2")
    );
    let drift = foot_run.drift_cm.expect("foot drift");
    assert!(drift < PIN_DRIFT_LIMIT_CM, "{foot_run:?}");
    assert!(
        foot_run.base_drift_cm.expect("base drift") > 2.0,
        "without the pin the foot slides: {foot_run:?}"
    );
    assert!(
        hand_run.drift_cm.expect("hand drift") < PIN_DRIFT_LIMIT_CM,
        "{hand_run:?}"
    );
    let review = review_of(&result);
    assert!(
        review
            .lines
            .iter()
            .filter(|l| l.label == "Pin drift")
            .all(|l| l.status == RecipeReviewStatus::Pass),
        "{review:?}"
    );
}
