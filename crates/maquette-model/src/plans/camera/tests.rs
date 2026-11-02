use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::plans::ops::{self, Merge, PlanOp};
use crate::plans::{PLAN_VERSION, PLAN_VERSION_PLAIN, create, get, remove, save};

pub(super) fn stop(id: &str, x: f32) -> LayoutStop {
    let mut name = id.to_string();
    name[..1].make_ascii_uppercase();
    LayoutStop {
        id: id.into(),
        name,
        position: [x, 1.0, 3.0],
        target: [0.0, 0.5, 0.0],
        fov: 35.0,
        models: None,
    }
}

pub(super) fn tour(ids: &[&str]) -> Vec<LayoutStop> {
    ids.iter()
        .zip(0..)
        .map(|(id, i)| stop(id, f32::from(i16::try_from(i).unwrap_or(0))))
        .collect()
}

pub(super) struct Site {
    _dir: tempfile::TempDir,
    pub(super) root: PathBuf,
}

/// A project folder whose `layout.json` has `ids` as its stops.
pub(super) fn site(ids: &[&str]) -> Site {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("moon-island");
    fs::create_dir_all(&root).expect("project");
    set_stops(&root, tour(ids));
    Site { _dir: dir, root }
}

pub(super) fn set_stops(root: &Path, stops: Vec<LayoutStop>) {
    layout::update(root, |l| {
        let mut next = l.clone();
        next.stops = stops;
        Ok(next)
    })
    .expect("layout");
}

/// The camera plan's main line as stop ids.
pub(super) fn main_ids(plan: &PlanFile) -> Vec<String> {
    let chain = main_chain(plan).expect("a main line");
    chain_stops(plan, &chain)
        .into_iter()
        .map(|s| s.id)
        .collect()
}

#[test]
fn the_camera_plan_is_built_from_layout_json_in_tour_order() {
    let s = site(&["porch", "schedule", "browser", "moon", "pullback"]);
    let made = ensure(&s.root, None).expect("camera plan");
    assert_eq!(made.id, CAMERA_PLAN_ID);
    let plan = &made.plan;
    assert_eq!(plan.version, PLAN_VERSION);
    assert_eq!(
        main_ids(plan),
        ["porch", "schedule", "browser", "moon", "pullback"]
    );
    assert_eq!(plan.edges.len(), 4);
    assert!(plan.edges.iter().all(|e| e.kind == EdgeKind::LeadsTo));
    assert_eq!(
        plan.camera.as_ref().expect("base").stops,
        tour(&["porch", "schedule", "browser", "moon", "pullback"])
    );
    let file = fs::read_to_string(s.root.join("plans/camera-plan.plan.json")).expect("file");
    assert!(file.contains("\"version\": 3"), "{file}");
    // Opening it again reads the file and writes nothing.
    let again = ensure(&s.root, None).expect("again");
    assert_eq!(again.plan, made.plan);
    assert_eq!(
        fs::read_to_string(s.root.join("plans/camera-plan.plan.json")).expect("file"),
        file
    );
    // It is built in: it can not be removed.
    let trash = s.root.join("../trash");
    let refused = remove(&s.root, &trash, CAMERA_PLAN_ID).expect_err("built in");
    assert!(refused.to_string().contains("built in"), "{refused}");
}

#[test]
fn a_canvas_without_stops_stays_version_2_and_holds_no_stops() {
    let s = site(&["porch"]);
    let made = create(&s.root, "Ideas", PlanPreset::Brainstorm).expect("canvas");
    assert_eq!(made.plan.version, PLAN_VERSION_PLAIN);
    let file = fs::read_to_string(s.root.join(&made.file)).expect("file");
    assert!(
        file.contains("\"version\": 2") && !file.contains("camera") && !file.contains("stop"),
        "{file}"
    );
    let mut with_stop = made.plan.clone();
    with_stop
        .nodes
        .push(stop_node(&stop("porch", 0.0), 0, 0, None));
    let refused = save(&s.root, &made.id, &with_stop).expect_err("a stop outside the camera plan");
    assert!(
        refused.to_string().contains("Only the camera plan"),
        "{refused}"
    );
}

#[test]
fn a_stop_node_is_checked_like_a_layout_stop() {
    let s = site(&["porch", "moon"]);
    let made = ensure(&s.root, None).expect("camera plan");
    let mut plan = made.plan.clone();
    plan.nodes[0].stop.as_mut().expect("stop").fov = 400.0;
    assert!(
        save(&s.root, CAMERA_PLAN_ID, &plan).is_err(),
        "a field of view past 170"
    );
    let mut plan = made.plan.clone();
    plan.nodes[0].title = "Porch".into();
    assert!(
        save(&s.root, CAMERA_PLAN_ID, &plan).is_err(),
        "a stop keeps no title"
    );
}

#[test]
fn chains_find_the_main_line_proposals_and_problems() {
    let s = site(&["porch", "schedule", "moon"]);
    let mut plan = ensure(&s.root, None).expect("camera plan").plan;
    let (branch, links) = line_of(
        &tour(&["porch", "moon", "schedule"]),
        0,
        400,
        Some("Camera agent"),
    );
    plan.nodes.extend(branch.clone());
    plan.edges.extend(links);
    let all = chains(&plan);
    assert_eq!(all.len(), 2);
    assert_eq!(main_ids(&plan), ["porch", "schedule", "moon"]);
    let proposal = chain_of(&plan, &branch[1].id).expect("the proposal");
    assert_eq!(proposal.agent.as_deref(), Some("Camera agent"));
    assert_eq!(
        chain_stops(&plan, &proposal)
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        ["porch", "moon", "schedule"]
    );

    // A fork is a problem, named for the person.
    let mut forked = plan.clone();
    let first = forked
        .nodes
        .iter()
        .find(|n| n.by.is_none() && n.x == 0)
        .expect("porch")
        .id
        .clone();
    let last = forked
        .nodes
        .iter()
        .find(|n| n.by.is_none() && n.x == 2 * STEP_X)
        .expect("moon")
        .id
        .clone();
    forked.edges.push(lead(&first, &last, None));
    let main = main_chain(&forked).expect("main");
    assert!(
        main.problem
            .as_deref()
            .is_some_and(|p| p.contains("Porch leads to more than one")),
        "{main:?}"
    );

    // A loop has no first stop.
    let mut looped = plan;
    looped.edges.push(lead(&last, &first, None));
    let main = main_chain(&looped).expect("main");
    assert!(
        main.problem.as_deref().is_some_and(|p| p.contains("loop")),
        "{main:?}"
    );
}

#[test]
fn the_base_moves_only_by_its_own_operation_and_turns_around() {
    let s = site(&["porch", "moon"]);
    let plan = ensure(&s.root, None).expect("camera plan").plan;
    let mut moved = plan.clone();
    moved.camera = Some(CameraBase {
        stops: tour(&["moon", "porch"]),
    });
    let ops = ops::diff(&plan, &moved);
    assert!(
        matches!(ops.as_slice(), [PlanOp::SetCamera { .. }]),
        "{ops:?}"
    );
    assert_eq!(ops::apply(&plan, &ops, Merge::Mine).plan, moved);
    assert_eq!(
        ops::apply(&moved, &ops::invert(&ops), Merge::Careful).plan,
        plan
    );
    // Careful: a base that moved since stays.
    let mut other = plan.clone();
    other.camera = Some(CameraBase {
        stops: tour(&["porch"]),
    });
    assert_eq!(
        ops::apply(&other, &ops::invert(&ops), Merge::Careful).skipped,
        1
    );
    // A person's save never moves it, even when the view sends another base.
    let mut sent = plan.clone();
    sent.camera = Some(CameraBase { stops: Vec::new() });
    let saved = save(&s.root, CAMERA_PLAN_ID, &sent).expect("save");
    assert_eq!(saved.plan.camera, plan.camera);
    assert_eq!(
        get(&s.root, CAMERA_PLAN_ID).expect("get").plan.camera,
        plan.camera
    );
}

#[test]
fn the_diff_names_every_kind_of_change_and_shows_a_moved_stop() {
    let before = tour(&["porch", "schedule", "browser", "moon", "pullback"]);
    let mut after = vec![
        before[0].clone(),
        before[3].clone(),
        stop("lantern", 9.0),
        before[1].clone(),
        before[2].clone(),
    ];
    after[3].fov = 50.0;
    let diff = diff_stops(&before, &after, &[]);
    assert_eq!(
        (diff.added, diff.removed, diff.moved, diff.changed),
        (1, 1, 1, 1)
    );
    let moon = diff.stops.iter().find(|s| s.id == "moon").expect("moon");
    assert!(
        moon.moved && moon.from == Some(4) && moon.to == Some(2),
        "{moon:?}"
    );
    let schedule = diff
        .stops
        .iter()
        .find(|s| s.id == "schedule")
        .expect("schedule");
    assert!(
        !schedule.moved && schedule.fields[0].field == "fov",
        "{schedule:?}"
    );
    assert_eq!(
        (
            schedule.fields[0].before.as_str(),
            schedule.fields[0].after.as_str()
        ),
        ("35", "50")
    );
    assert!(diff.stops.iter().any(|s| s.id == "lantern" && s.added));
    assert!(diff.stops.iter().any(|s| s.id == "pullback" && s.removed));
    // The moon leaves its old line and comes in at its new one.
    let notes: Vec<(DiffKind, Option<&str>, bool)> = diff
        .lines
        .iter()
        .map(|l| (l.kind, l.note.as_deref(), l.text.contains("\"moon\"")))
        .collect();
    assert!(
        notes.contains(&(DiffKind::Remove, Some("moves to 2"), true)),
        "{notes:?}"
    );
    assert!(
        notes.contains(&(DiffKind::Add, Some("moved from 4"), true)),
        "{notes:?}"
    );
    assert!(
        notes
            .iter()
            .any(|(k, n, _)| *k == DiffKind::Remove && *n == Some("removed"))
    );
    assert!(
        notes
            .iter()
            .any(|(k, n, _)| *k == DiffKind::Add && *n == Some("new"))
    );
    assert!(
        notes
            .iter()
            .any(|(k, n, _)| *k == DiffKind::Add && *n == Some("changed"))
    );
    assert_eq!(
        diff.counts_line(),
        "1 stop added · 1 moved · 1 removed · 1 changed"
    );
    assert!(diff_stops(&before, &before, &[]).is_empty());
}

#[test]
fn an_agent_proposes_its_own_line_and_never_touches_the_main_line_or_layout() {
    use crate::plans::agent::Agent;
    let s = site(&["porch", "schedule", "moon"]);
    let agent = Agent {
        name: "Camera agent",
        run: Some("r-1"),
    };
    let want = |id: &str| ProposedStop {
        id: id.into(),
        ..ProposedStop::default()
    };
    let lantern = ProposedStop {
        name: Some("Lantern close-up".into()),
        position: Some([-2.1, 1.3, 3.4]),
        target: Some([-2.6, 1.0, 1.9]),
        fov: Some(32.0),
        ..want("lantern")
    };
    let first = propose(
        &s.root,
        &agent,
        &[
            want("porch"),
            want("moon"),
            lantern.clone(),
            want("schedule"),
        ],
    )
    .expect("propose");
    assert_eq!(first.nodes.len(), 4);
    assert_eq!((first.review.diff.added, first.review.diff.moved), (1, 1));
    let plan = get(&s.root, CAMERA_PLAN_ID).expect("plan").plan;
    assert_eq!(
        main_ids(&plan),
        ["porch", "schedule", "moon"],
        "the main line stays"
    );
    assert_eq!(
        layout::load(&s.root).expect("layout").stops,
        tour(&["porch", "schedule", "moon"])
    );
    let entry = first.changed.entry.as_ref().expect("logged");
    assert_eq!(entry.by.as_deref(), Some("Camera agent"));

    // A second proposal replaces the first.
    let second = propose(&s.root, &agent, &[want("moon"), want("porch")]).expect("again");
    let plan = second.changed.doc.plan;
    let agents = plan
        .nodes
        .iter()
        .filter(|n| n.by.as_deref() == Some("Camera agent"))
        .count();
    assert_eq!(agents, 2);

    // A new stop needs its camera.
    let refused = propose(&s.root, &agent, &[want("dock")]).expect_err("no camera");
    assert!(
        refused
            .to_string()
            .contains("needs a name, a position and a target"),
        "{refused}"
    );
}

#[test]
fn the_camera_plan_id_is_kept_for_the_built_in_plan() {
    let s = site(&["porch", "moon"]);
    // A canvas a person or agent names "Camera plan" never takes the id.
    let named =
        create(&s.root, "Camera plan", crate::plans::PlanPreset::Brainstorm).expect("create");
    assert_eq!(named.id, "camera-plan-2");

    // A file from before the id was kept: opening the camera plan moves it, with its log.
    let old = PlanFile::new("Camera plan", crate::plans::PlanPreset::Story);
    fs::write(
        s.root.join("plans/camera-plan.plan.json"),
        serde_json::to_string(&old).expect("json"),
    )
    .expect("old canvas");
    fs::write(s.root.join("plans/camera-plan.log.jsonl"), "").expect("old log");
    let made = ensure(&s.root, None).expect("camera plan");
    assert!(made.plan.camera.is_some());
    let moved = get(&s.root, "camera-plan-3").expect("moved canvas");
    assert_eq!(moved.plan.preset, crate::plans::PlanPreset::Story);
    let lines = crate::plans::log::lines(&s.root, "camera-plan-3", 5).expect("log");
    assert!(
        lines.iter().any(|l| l.summary.contains("make room")),
        "{lines:?}"
    );
    // The camera plan's own log starts fresh.
    let own = crate::plans::log::lines(&s.root, CAMERA_PLAN_ID, 5).expect("log");
    assert!(own.iter().all(|l| !l.summary.contains("make room")));
}

#[test]
fn a_log_left_without_its_canvas_counts_as_taken() {
    let s = site(&["porch"]);
    fs::create_dir_all(s.root.join("plans")).expect("plans");
    fs::write(s.root.join("plans/camera-plan-2.log.jsonl"), "{}\n").expect("orphan log");
    let made =
        create(&s.root, "Camera plan", crate::plans::PlanPreset::Brainstorm).expect("create");
    assert_eq!(made.id, "camera-plan-3");
    assert_eq!(
        fs::read_to_string(s.root.join("plans/camera-plan-2.log.jsonl")).expect("log"),
        "{}\n",
        "the orphan log is left alone"
    );
}
