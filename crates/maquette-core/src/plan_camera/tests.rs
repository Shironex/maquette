use std::fs;
use std::path::Path;

use serde_json::json;

use super::*;
use crate::layout::LayoutStop;
use crate::plans::camera::{CAMERA_PLAN_ID, line_of};

fn stop(id: &str, name: &str, x: f32) -> LayoutStop {
    LayoutStop {
        id: id.into(),
        name: name.into(),
        position: [x, 1.0, 3.0],
        target: [0.0, 0.5, 0.0],
        fov: 35.0,
        models: None,
    }
}

fn set_stops(root: &Path, stops: Vec<LayoutStop>) {
    layout::update(root, |l| {
        let mut next = l.clone();
        next.stops = stops;
        Ok(next)
    })
    .expect("layout");
}

#[test]
fn the_view_makes_the_plan_shows_posters_and_what_changed_outside() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("moon-island");
    fs::create_dir_all(root.join("web/posters")).expect("posters");
    fs::write(root.join("web/posters/stop_01_porch.webp"), b"webp").expect("poster");
    fs::write(
        root.join("web/posters/stops.json"),
        json!({ "stops": [{ "camera": "stop_01_porch", "label": "Porch", "file": "stop_01_porch.webp" }] })
            .to_string(),
    )
    .expect("stops file");
    set_stops(
        &root,
        vec![stop("porch", "Porch", 0.0), stop("moon", "Moon", 1.0)],
    );

    let first = view(&root).expect("view");
    assert_eq!(first.doc.id, CAMERA_PLAN_ID);
    assert_eq!(first.main.len(), 2);
    assert!(!first.stale);
    assert!(first.outside.is_empty());
    assert_eq!(first.posters.len(), 1);
    assert_eq!(first.posters[0].stop, "porch");
    assert!(first.posters[0].poster.ends_with("stop_01_porch.webp"));

    set_stops(
        &root,
        vec![stop("moon", "Moon", 1.0), stop("porch", "Porch", 0.0)],
    );
    let behind = view(&root).expect("view");
    assert!(behind.stale);
    assert_eq!(behind.outside.moved, 1);
}

#[test]
fn an_agents_line_shows_beside_the_main_line() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("moon-island");
    fs::create_dir_all(&root).expect("project");
    set_stops(&root, vec![stop("porch", "Porch", 0.0)]);
    let mut plan = view(&root).expect("view").doc.plan;
    let (nodes, edges) = line_of(
        &[stop("lantern", "Lantern", 2.0)],
        0,
        400,
        Some("Camera agent"),
    );
    plan.nodes.extend(nodes);
    plan.edges.extend(edges);
    crate::plans::save(&root, CAMERA_PLAN_ID, &plan).expect("save");
    let shown = view(&root).expect("view");
    assert_eq!(shown.lines.len(), 1);
    assert_eq!(shown.lines[0].agent.as_deref(), Some("Camera agent"));
}
