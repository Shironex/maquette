use std::path::Path;

use super::*;
use crate::inbox::{analyse::analyse_bytes, fixture};

/// The walk row of a character turned `yaw` degrees whose walk carries its
/// hips `walk` metres along its own front.
fn walk_row(yaw: f32, walk: f32) -> Option<Check> {
    let bytes = fixture::humanoid(&fixture::HumanoidSpec {
        yaw,
        walk: Some(walk),
        ..fixture::HumanoidSpec::default()
    });
    let report = analyse_bytes(&bytes, Path::new(".")).expect("analyse");
    let list = run(
        &report,
        AssetKind::Character,
        &WorkspaceConfig::default(),
        SendNotes::default(),
        None,
    );
    list.into_iter().find(|c| c.id == "walk-facing")
}

#[test]
fn a_walk_that_goes_the_way_the_model_faces_passes() {
    let row = walk_row(90.0, 0.8).expect("walk row");
    assert_eq!(row.status, CheckStatus::Pass);
    assert_eq!(row.title, "Walks the way it faces");
    assert_eq!(
        row.detail.as_deref(),
        Some("Walk moves 0.80 m along +X, the way the model faces (+X).")
    );
    let row = walk_row(0.0, 0.8).expect("walk row");
    assert_eq!(row.status, CheckStatus::Pass);
}

#[test]
fn a_walk_that_goes_against_the_face_is_flagged() {
    let row = walk_row(90.0, -0.8).expect("walk row");
    assert_eq!(row.status, CheckStatus::Warning);
    assert_eq!(row.title, "Walk travels against the face");
    let detail = row.detail.as_deref().unwrap_or_default();
    assert!(
        detail.starts_with("Walk moves 0.80 m along -X while the model faces +X"),
        "{detail}"
    );
    // A model facing +Z that walks towards -Z, too.
    let row = walk_row(0.0, -0.8).expect("walk row");
    assert_eq!(row.status, CheckStatus::Warning);
    assert!(row.detail.unwrap_or_default().contains("along -Z"));
}

#[test]
fn a_clip_that_plays_in_place_has_no_walk_row() {
    // The fixture's idle sways its hips 2 cm: no travel, no row.
    let config = WorkspaceConfig::default();
    let report = analyse_bytes(&fixture::character(false), Path::new(".")).expect("analyse");
    let list = run(
        &report,
        AssetKind::Character,
        &config,
        SendNotes::default(),
        None,
    );
    assert!(list.iter().all(|c| c.id != "walk-facing"), "{list:#?}");
    // A walk too short to travel is a sway, not a walk.
    assert_eq!(walk_row(90.0, 0.01), None);
}
