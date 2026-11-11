//! What Ship does after a replace: the new file name by itself, the same
//! name on a hand-built site, the same game file, and the swap listed for
//! the person who approves the next Ship.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::*;
use crate::game::{DEFAULT_MODELS_DIR, GameFormat, GameModel, NewGame, ProjectTarget};
use crate::pipeline::history::{RunRecord, RunState, StepRecord, StepState};
use crate::ship::{AssetShip, ShipChoice, ShipSettingsChange, ShipStyle};

fn ship_file(w: &World, asset: &str) -> String {
    let ship = crate::ship::load(&w.dir).expect("ship");
    crate::site::ship_index(&w.manifest(), &ship).models[asset]
        .file
        .clone()
}

#[test]
fn ship_names_the_new_file_and_never_the_scan_again() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.place(&planner.id);
    assert_eq!(ship_file(&w, &planner.id), "spiral_planner_v1.glb");

    w.replace(&planner.id, &desk.id);

    // The ship index is what the Ship step writes the site's model list from.
    let ship = crate::ship::load(&w.dir).expect("ship");
    let index = crate::site::ship_index(&w.manifest(), &ship);
    assert_eq!(
        index.models.len(),
        1,
        "the built asset's card is not a second model"
    );
    assert_eq!(index.models["spiral-planner"].file, "spiral_planner_v2.glb");
    assert_eq!(index.models["spiral-planner"].version, 2);
    assert_eq!(index.origin, "scene", "exported where the layout puts it");
    // Build scene imports the newest version of each asset: the built one.
    let newest = w.asset("spiral-planner").latest().expect("newest").clone();
    assert_eq!(newest.file, "models/spiral_planner_v2.glb");
}

#[test]
fn a_hand_built_site_keeps_its_file_name_and_says_what_holds_a_file_back() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    crate::ship::update(
        &w.dir,
        ShipSettingsChange {
            style: Some(ShipStyle::Models),
            assets: BTreeMap::from([(
                planner.id.clone(),
                AssetShip {
                    choice: ShipChoice::Auto,
                    file: Some("planner".into()),
                },
            )]),
            ..ShipSettingsChange::default()
        },
    )
    .expect("ship settings");
    let before = w.read("ship.json");

    let report = w.replace(&planner.id, &desk.id).report;

    assert_eq!(w.read("ship.json"), before, "the choice and the name stay");
    assert_eq!(report.old.ships_as.as_deref(), Some("planner.glb"));
    assert_eq!(report.new.ships_as.as_deref(), Some("planner.glb"));
    assert_eq!(ship_file(&w, "spiral-planner"), "planner.glb");
    assert!(
        report
            .settings
            .iter()
            .any(|s| s.what == "Ship file name planner.glb" && s.action == SwapAction::Kept),
        "{:?}",
        report.settings
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("hand-built") && w.contains("holds it back")),
        "{:?}",
        report.warnings
    );
}

#[test]
fn an_asset_set_to_never_ship_says_so() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    crate::ship::update(
        &w.dir,
        ShipSettingsChange {
            assets: BTreeMap::from([(
                planner.id.clone(),
                AssetShip {
                    choice: ShipChoice::Never,
                    file: None,
                },
            )]),
            ..ShipSettingsChange::default()
        },
    )
    .expect("ship settings");

    let report = plan(&w.env(), &planner.id, &desk.id).expect("plan");

    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("Spiral planner is set to never ship")),
        "{:?}",
        report.warnings
    );
}

#[test]
fn a_game_project_keeps_the_game_file() {
    let w = World::new(NewProject {
        name: "Crypt".into(),
        template: ProjectTemplate::FloatingDiorama,
        target: ProjectTarget::Game,
        site_repo: None,
        source_folder: None,
        game: Some(NewGame {
            format: GameFormat::Glb,
            models_dir: DEFAULT_MODELS_DIR.into(),
            template: None,
        }),
    });
    let brazier = w.scan("Brazier");
    let desk = w.built("Brazier built", "brazier.py");
    // The scan came in from the game repo as props/brazier.glb.
    project::modify(&w.root, &w.config.layout, &w.project, |m| {
        let game = m.game.as_mut().expect("game");
        game.models.insert(
            brazier.id.clone(),
            GameModel {
                file: "props/brazier.glb".into(),
                imported_version: Some(1),
            },
        );
        Ok(())
    })
    .expect("game model");

    let report = w.replace(&brazier.id, &desk.id).report;

    assert_eq!(report.old.ships_as.as_deref(), Some("props/brazier.glb"));
    assert_eq!(report.new.ships_as.as_deref(), Some("props/brazier.glb"));
    assert!(
        !report.bakes,
        "a game bakes only when its Bake setting is on"
    );
    let manifest = w.manifest();
    let game = crate::game::settings_of(&manifest).expect("game");
    assert_eq!(game.models["brazier"].file, "props/brazier.glb");
    // Game export reads the index: the built model, to the same game file,
    // and no longer the untouched import.
    let prep = prepare::load(&w.dir).expect("prepare");
    let index = crate::game::index::build(&w.dir, &manifest, game, &prep);
    assert_eq!(index.models.len(), 1);
    assert_eq!(index.models[0].source, "models/brazier_v2.glb");
    assert_eq!(index.models[0].target, "props/brazier.glb");
    assert!(!index.models[0].as_is);
    assert!(
        report
            .settings
            .iter()
            .any(|s| s.what == "Game file props/brazier.glb" && s.action == SwapAction::Kept),
        "{:?}",
        report.settings
    );
}

pub(super) fn run(number: u32, finished: SystemTime, ship: Option<StepState>) -> RunRecord {
    let ms = finished
        .duration_since(UNIX_EPOCH)
        .expect("after 1970")
        .as_secs_f64()
        * 1000.0;
    RunRecord {
        id: format!("study-room-run-{number}"),
        number,
        project: "study-room".into(),
        from: None,
        force: false,
        build_only: ship.is_none(),
        state: RunState::Done,
        queued_at_ms: ms,
        started_at_ms: Some(ms),
        finished_at_ms: Some(ms),
        steps: ship
            .into_iter()
            .map(|state| StepRecord {
                id: "ship".into(),
                name: "Ship".into(),
                state,
                started_at_ms: Some(ms),
                duration_ms: Some(1.0),
                exit_code: Some(0),
                message: None,
                progress: None,
            })
            .collect(),
        message: None,
        not_shipped: None,
        repo_for_run: None,
    }
}

#[test]
fn the_swap_is_listed_until_a_ship_writes_it() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    assert!(
        swaps(&w.dir, &w.manifest(), &[]).is_empty(),
        "nothing replaced yet"
    );
    let earlier = SystemTime::now() - Duration::from_secs(3600);

    w.replace(&planner.id, &desk.id);

    let later = SystemTime::now() + Duration::from_secs(3600);
    let manifest = w.manifest();
    let listed = swaps(&w.dir, &manifest, &[]);
    assert_eq!(listed.len(), 1);
    let swap = &listed[0];
    assert_eq!(swap.asset, "spiral-planner");
    assert_eq!(swap.name, "Spiral planner");
    assert_eq!(swap.script, "built/planner.py");
    assert_eq!(swap.old_file.as_deref(), Some("spiral_planner_v1.glb"));
    assert_eq!(swap.new_file.as_deref(), Some("spiral_planner_v2.glb"));
    assert_eq!((swap.old_version, swap.new_version), (1, 2));
    assert_eq!(
        (swap.old_triangles, swap.new_triangles),
        (Some(12), Some(36))
    );
    assert!(swap.old_bytes.is_some() && swap.new_bytes.is_some());

    // A ship from before the replace, a build only run and a run whose Ship
    // failed do not count; a run that shipped after it does.
    let before = [run(1, earlier, Some(StepState::Done))];
    assert_eq!(swaps(&w.dir, &manifest, &before).len(), 1);
    let build_only = [run(2, later, None)];
    assert_eq!(swaps(&w.dir, &manifest, &build_only).len(), 1);
    let failed = [run(3, later, Some(StepState::Failed))];
    assert_eq!(swaps(&w.dir, &manifest, &failed).len(), 1);
    let shipped = [run(4, later, Some(StepState::Done))];
    assert!(swaps(&w.dir, &manifest, &shipped).is_empty());
}

/// A run that shipped after the replace, with its Build step as `build`.
pub(super) fn shipped_after(number: u32, build: StepState) -> RunRecord {
    let mut record = run(
        number,
        SystemTime::now() + Duration::from_secs(3600),
        Some(StepState::Done),
    );
    let mut step = record.steps[0].clone();
    step.id = "build".into();
    step.name = "Build scene".into();
    step.state = build;
    record.steps.insert(0, step);
    record
}

#[test]
fn only_a_run_that_built_and_shipped_into_the_projects_repo_writes_the_swap() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.replace(&planner.id, &desk.id);
    let manifest = w.manifest();

    // Run from Ship: it ships the export from before the replace.
    let mut from_ship = shipped_after(1, StepState::Skipped);
    from_ship.from = Some("ship".into());
    assert_eq!(swaps(&w.dir, &manifest, &[from_ship]).len(), 1);
    // A run that shipped into another repo for this run only.
    let mut elsewhere = shipped_after(2, StepState::Done);
    elsewhere.repo_for_run = Some("worktrees/try".into());
    assert_eq!(swaps(&w.dir, &manifest, &[elsewhere]).len(), 1);
    // A run that built (or found the build up to date) and shipped home.
    assert!(swaps(&w.dir, &manifest, &[shipped_after(3, StepState::Done)]).is_empty());
    assert!(swaps(&w.dir, &manifest, &[shipped_after(4, StepState::Cached)]).is_empty());
}
