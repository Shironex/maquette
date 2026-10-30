//! The add many plan on real .glb headers and PNG pictures, and adding
//! into a chosen entry.

use std::fs;

use super::tests::{add_lamp, fixture};
use super::tests_batch::{BLUE, RED, bar, block, glb, model, to_library};
use super::*;
use crate::assets::{NewAsset, Stage};
use crate::project;

#[test]
fn adding_into_a_chosen_entry_makes_the_model_its_next_version() {
    let f = fixture();
    add_lamp(&f, &[]);
    let other = model(&f, "study-room", "Lamp", &glb("another lamp"), None);
    let request = AddToLibrary {
        project: "study-room".into(),
        asset: other.clone(),
        name: None,
        tags: None,
        into: Some("desk-lamp".into()),
    };
    let added = add(f.root(), &f.layout, &request, &[]).expect("into");
    assert_eq!(
        (added.entry.id.as_str(), added.version, added.added),
        ("desk-lamp", 2, true)
    );
    let lamp = added
        .project
        .assets
        .iter()
        .find(|a| a.id == other)
        .expect("asset");
    assert_eq!(
        lamp.origin.as_ref().map(|o| o.entry.as_str()),
        Some("desk-lamp")
    );
    assert_eq!(list(f.root()).expect("list").len(), 1);
    let again = add(f.root(), &f.layout, &request, &[]).expect("again");
    assert!(!again.added, "the same model adds nothing");
    let missing = AddToLibrary {
        into: Some("nope".into()),
        ..request
    };
    assert!(add(f.root(), &f.layout, &missing, &[]).is_err());
}

#[test]
fn the_add_many_plan_says_what_each_asset_would_become() {
    let f = fixture();
    let crate_id = model(&f, "moon-island", "Crate", &glb("crate"), Some(&block(RED)));
    to_library(&f, "moon-island", &crate_id);
    let barrel = model(&f, "moon-island", "Barrel", &glb("barrel"), None);
    to_library(&f, "moon-island", &barrel);
    model(&f, "moon-island", "Barrel", &glb("barrel 2"), None);
    let stool = model(&f, "study-room", "Stool", &glb("stool"), Some(&bar(BLUE)));
    to_library(&f, "study-room", &stool);
    model(
        &f,
        "moon-island",
        "Stool 2",
        &glb("other stool"),
        Some(&bar(BLUE)),
    );
    model(&f, "moon-island", "Vase", &glb("vase"), Some(&block(RED)));
    model(&f, "moon-island", "Rope", &glb("rope"), None);
    let broken = model(&f, "moon-island", "Broken", &glb("broken"), None);
    let mut cut = glb("broken");
    cut.truncate(cut.len() - 2);
    fs::write(f.moon.join(format!("models/{broken}_v1.glb")), cut).expect("cut");
    let ghost = model(&f, "moon-island", "Ghost", &glb("ghost"), None);
    fs::remove_file(f.moon.join(format!("models/{ghost}_v1.glb"))).expect("gone");
    project::modify(f.root(), &f.layout, "moon-island", |m| {
        crate::assets::add(
            m,
            NewAsset {
                name: "Bench".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )
        .map(|_| ())
    })
    .expect("bench");

    let plan = plan_add_many(f.root(), &f.layout, "moon-island").expect("plan");
    assert_eq!(
        (plan.tag.as_str(), plan.check_on_add),
        ("moon-island", true)
    );
    let state = |name: &str| {
        let found = plan.assets.iter().find(|a| a.name == name).expect(name);
        (found.state, found.entry.clone())
    };
    let names: Vec<&str> = plan.assets.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Desk lamp",
            "Crate",
            "Barrel",
            "Stool 2",
            "Vase",
            "Rope",
            "Broken",
            "Ghost"
        ],
        "the project's order, the bench without a model left out"
    );
    assert_eq!(state("Desk lamp").0, PlannedState::Unreadable, "not a glb");
    assert_eq!(
        state("Crate"),
        (PlannedState::SameFile, Some("crate".into()))
    );
    assert_eq!(
        state("Barrel"),
        (PlannedState::NextVersion, Some("barrel".into()))
    );
    assert_eq!(
        state("Stool 2"),
        (PlannedState::NameClash, Some("stool".into()))
    );
    let clash = plan
        .assets
        .iter()
        .find(|a| a.name == "Stool 2")
        .expect("stool");
    assert!(clash.looks.is_some_and(|l| l > 0.9), "{:?}", clash.looks);
    assert_eq!(
        state("Vase"),
        (PlannedState::LooksLike, Some("crate".into()))
    );
    assert_eq!(state("Rope"), (PlannedState::New, None));
    let reason = |name: &str| {
        plan.assets
            .iter()
            .find(|a| a.name == name)
            .and_then(|a| a.reason.clone())
            .expect("reason")
    };
    assert_eq!(state("Broken").0, PlannedState::Unreadable);
    assert!(reason("Broken").contains("cut short"));
    assert!(reason("Ghost").contains("missing"));
    assert_eq!(plan.assets[1].triangles, Some(1200));

    // With the check off, a look-alike is just new.
    save_settings(
        f.root(),
        &LookAlikeSettings {
            check_on_add: false,
            ..LookAlikeSettings::default()
        },
    )
    .expect("settings");
    let plan = plan_add_many(f.root(), &f.layout, "moon-island").expect("plan");
    let vase = plan.assets.iter().find(|a| a.name == "Vase").expect("vase");
    assert_eq!(vase.state, PlannedState::New);
}
