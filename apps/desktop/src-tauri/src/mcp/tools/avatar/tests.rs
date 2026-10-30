use std::path::Path;

use maquette_core::avatar::export::AvatarExport;
use maquette_core::avatar::normalize::{NormalizeReport, NormalizeSheets, NormalizeState};
use maquette_core::inbox::fixture::{self, HumanoidSpec};
use maquette_core::inbox::vrm::{self, BoneOverrides, MaterialMode, RoundTrip};

use super::*;
use crate::mcp::hub::ApprovalKind;

fn pending() -> Normalized {
    let readiness = vrm::readiness(
        &fixture::humanoid(&HumanoidSpec::default()),
        Path::new("."),
        &BoneOverrides::new(),
    )
    .expect("readiness");
    Normalized {
        project: "pets".into(),
        asset: "hero".into(),
        source_version: 2,
        state: NormalizeState::Pending,
        glb: "/cache/avatar/pets/hero/v2/normalized.glb".into(),
        sheets: NormalizeSheets {
            before: "/cache/avatar/pets/hero/v2/sheet_before.png".into(),
            after: "/cache/avatar/pets/hero/v2/sheet_after.png".into(),
        },
        report: NormalizeReport {
            turned_deg: -90.0,
            ..NormalizeReport::default()
        },
        readiness,
        key: "k".into(),
        approved_at: None,
    }
}

#[test]
fn elbows_default_to_straighten_and_bad_words_are_refused() {
    assert_eq!(parse_elbows(None), Ok(Elbows::Straighten));
    assert_eq!(parse_elbows(Some("Keep")), Ok(Elbows::Keep));
    assert!(
        parse_elbows(Some("bend"))
            .expect_err("bad")
            .contains("straighten or keep")
    );
}

#[test]
fn a_waiting_build_asks_a_person_once_and_shows_the_sheets() {
    let hub = Hub::new(|| {});
    let built = AvatarBuild::Waiting {
        normalized: pending(),
    };
    let first = build_answer(&hub, "Hero", &built);
    assert_eq!(first["status"], WAITING);
    assert_eq!(
        first["pose"]["sheets"]["after"],
        "/cache/avatar/pets/hero/v2/sheet_after.png"
    );
    let second = build_answer(&hub, "Hero", &built);
    assert_eq!(first["approval"], second["approval"], "one per asset");
    let approvals = hub.approvals();
    assert_eq!(approvals.len(), 1);
    let waiting = &approvals[0];
    assert_eq!(waiting.kind, ApprovalKind::RestPose);
    assert_eq!(
        (waiting.asset.as_deref(), waiting.version),
        (Some("hero"), Some(2))
    );
    assert_eq!(waiting.detail.as_deref(), Some("v2, turned 90 degrees"));
    assert!(hub.rest_pose_for("pets", "hero").is_some());

    let denied = build_answer(
        &hub,
        "Hero",
        &AvatarBuild::Denied {
            normalized: pending(),
        },
    );
    assert_eq!(denied["status"], "denied");
}

#[test]
fn a_written_build_names_its_report_and_round_trip() {
    let hub = Hub::new(|| {});
    let export = AvatarExport {
        path: "/p/exports/hero.vrm".into(),
        bytes: 1024.0,
        source_version: 2,
        normalized: false,
        material: MaterialMode::MtoonSoft,
        unresolved: Vec::new(),
        written_at: "2026-10-30T12:00:00Z".into(),
        exists: true,
        report: Some("/p/exports/hero.vrm.report.json".into()),
        round_trip: Some(RoundTrip {
            passed: true,
            problems: Vec::new(),
            bones: 52,
        }),
    };
    let built = AvatarBuild::Written {
        export,
        normalized: None,
    };
    let answer = build_answer(&hub, "Hero", &built);
    assert_eq!(answer["status"], "written");
    assert_eq!(answer["report"], "/p/exports/hero.vrm.report.json");
    assert_eq!(answer["roundTrip"]["passed"], true);
    assert_eq!(answer["roundTrip"]["bones"], 52);
    assert!(hub.approvals().is_empty());
}
