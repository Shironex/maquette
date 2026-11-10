//! What a replace refuses, writing nothing.

use super::*;
use crate::assets::remove::Sent;

fn refusal(w: &World, asset: &str, built: &str) -> String {
    let before = files(&w.dir);
    let err = replace(&w.env(), asset, built).expect_err("refused");
    assert_eq!(files(&w.dir), before, "a refusal writes nothing");
    err.to_string()
}

#[test]
fn only_a_built_asset_takes_the_place_of_one_that_is_not() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let lamp = w.scan("Desk lamp");
    let desk = w.built("Planner built", "planner.py");
    let stack = w.built("Book stack", "books.py");

    let same = refusal(&w, &desk.id, &desk.id);
    assert!(same.contains("Pick two assets"), "{same}");
    let both_built = refusal(&w, &stack.id, &desk.id);
    assert!(
        both_built.contains("Book stack is already a built asset"),
        "{both_built}"
    );
    let not_built = refusal(&w, &planner.id, &lamp.id);
    assert!(
        not_built.contains("Desk lamp is not a built asset"),
        "{not_built}"
    );
    let missing = refusal(&w, "nope", &desk.id);
    assert!(missing.contains("nope"), "{missing}");
}

#[test]
fn an_asset_without_a_model_has_nothing_to_replace() {
    let w = World::website();
    let desk = w.built("Planner built", "planner.py");
    project::modify(&w.root, &w.config.layout, &w.project, |m| {
        assets::add(
            m,
            NewAsset {
                name: "Sketch".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )
        .map(|_| ())
    })
    .expect("asset");

    let why = refusal(&w, "sketch", &desk.id);

    assert!(why.contains("Sketch has no model yet"), "{why}");
}

#[test]
fn the_built_model_has_to_be_what_its_script_makes() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");

    // The script changed since the build.
    fs::write(w.dir.join("built/planner.py"), "# edited\n").expect("edit");
    let stale = refusal(&w, &planner.id, &desk.id);
    assert!(
        stale.contains("Planner built has a script that changed since its last build"),
        "{stale}"
    );

    // A helper is gone.
    fs::remove_file(w.dir.join("built/common.py")).expect("remove");
    let missing = refusal(&w, &planner.id, &desk.id);
    assert!(
        missing.contains("Planner built is missing built/common.py"),
        "{missing}"
    );
}

#[test]
fn a_built_asset_that_never_built_or_failed_is_refused() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    built::update(&w.dir, |file| {
        let entry = file.assets.get_mut(&desk.id).expect("entry");
        if let Some(last) = entry.last.as_mut() {
            last.error = Some("NameError: boom".into());
        }
        Ok(())
    })
    .expect("fail");
    let failed = refusal(&w, &planner.id, &desk.id);
    assert!(
        failed.contains("Planner built did not build: NameError: boom"),
        "{failed}"
    );

    built::update(&w.dir, |file| {
        file.assets.get_mut(&desk.id).expect("entry").last = None;
        Ok(())
    })
    .expect("new");
    let new = refusal(&w, &planner.id, &desk.id);
    assert!(new.contains("Planner built has not built yet"), "{new}");
}

#[test]
fn a_run_or_a_build_going_refuses_the_replace() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    let building = [desk.id.clone()];

    let env = ReplaceEnv {
        busy: Busy {
            run: Some("study-room-run-3"),
            building: &[],
        },
        ..w.env()
    };
    let running = replace(&env, &planner.id, &desk.id)
        .expect_err("run going")
        .to_string();
    assert!(running.contains("A pipeline run"), "{running}");

    let env = ReplaceEnv {
        busy: Busy {
            run: None,
            building: &building,
        },
        ..w.env()
    };
    let in_blender = replace(&env, &planner.id, &desk.id)
        .expect_err("building")
        .to_string();
    assert!(
        in_blender.contains("Planner built is building in Blender now"),
        "{in_blender}"
    );
}

#[test]
fn a_send_making_the_built_assets_model_refuses_the_replace() {
    let mut w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.sends = Stub(vec![Sent {
        id: "planner-built-1".into(),
        asset: desk.id.clone(),
        images: Vec::new(),
        provider: "Tripo",
        state: SentState::Running,
    }]);

    let why = refusal(&w, &planner.id, &desk.id);

    assert!(
        why.contains("Tripo is making a model of Planner built now"),
        "{why}"
    );
}
