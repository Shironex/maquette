use std::path::Path;

use maquette_core::avatar::Highlights;
use maquette_core::avatar::export::AvatarExport;
use maquette_core::avatar::export::SpringsView;
use maquette_core::avatar::highlights::{HighlightsReport, ImageHighlights};
use maquette_core::avatar::normalize::{NormalizeReport, NormalizeSheets, NormalizeState};
use maquette_core::avatar::reweight::ReweightReport;
use maquette_core::avatar::springs::{
    ChainBuilt, ChainReport, Chained, FoundSprings, SpringCandidate,
};
use maquette_core::inbox::fixture::{self, HumanoidSpec};
use maquette_core::inbox::vrm::{self, BoneOverrides, MaterialMode, RoundTrip};
use maquette_core::inbox::vrm::{SpringKind, SpringPreset};

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
        reweight: None,
    }
}

fn freed() -> Reweighted {
    Reweighted {
        project: "pets".into(),
        asset: "hero".into(),
        source_version: 2,
        state: NormalizeState::Pending,
        glb: "/cache/avatar/pets/hero/v2/reweight/reweighted.glb".into(),
        sheets: NormalizeSheets {
            before: "/cache/avatar/pets/hero/v2/reweight/sheet_before.png".into(),
            after: "/cache/avatar/pets/hero/v2/reweight/sheet_after.png".into(),
        },
        report: ReweightReport {
            moved: 2314,
            kept_as_sleeve: 2410,
            ..ReweightReport::default()
        },
        key: "r".into(),
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
fn fingers_default_to_straighten_and_bad_words_are_refused() {
    assert_eq!(parse_fingers(None), Ok(Fingers::Straighten));
    assert_eq!(parse_fingers(Some("KEEP")), Ok(Fingers::Keep));
    assert!(
        parse_fingers(Some("curl"))
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
fn reweight_defaults_to_auto_and_bad_words_are_refused() {
    assert_eq!(parse_reweight(None), Ok(ReweightMode::Auto));
    assert_eq!(parse_reweight(Some("Off")), Ok(ReweightMode::Off));
    assert!(
        parse_reweight(Some("maybe"))
            .expect_err("bad")
            .contains("auto or off")
    );
}

#[test]
fn a_waiting_reweight_asks_a_person_apart_from_the_rest_pose() {
    let hub = Hub::new(|| {});
    let built = AvatarBuild::ReweightWaiting {
        reweighted: freed(),
    };
    let first = build_answer(&hub, "Hero", &built);
    assert_eq!(first["status"], WAITING);
    assert_eq!(first["step"], "reweight");
    assert_eq!(first["reweight"]["moved"], 2314);
    assert_eq!(
        first["reweight"]["sheets"]["after"],
        "/cache/avatar/pets/hero/v2/reweight/sheet_after.png"
    );
    let second = build_answer(&hub, "Hero", &built);
    assert_eq!(first["approval"], second["approval"], "one per asset");
    let waiting = hub.reweight_for("pets", "hero").expect("waits");
    assert_eq!(waiting.kind, ApprovalKind::Reweight);
    assert_eq!(
        waiting.detail.as_deref(),
        Some("v2, 2314 vertices freed from the arms, 2410 kept as sleeve")
    );
    assert!(hub.rest_pose_for("pets", "hero").is_none());

    // The rest pose that follows is an approval of its own.
    let pose = build_answer(
        &hub,
        "Hero",
        &AvatarBuild::Waiting {
            normalized: pending(),
        },
    );
    assert_eq!(pose["step"], "rest pose");
    assert_ne!(pose["approval"], first["approval"]);
    assert_eq!(hub.approvals().len(), 2);
}

#[test]
fn a_written_build_names_its_report_and_round_trip() {
    let hub = Hub::new(|| {});
    let export = AvatarExport {
        path: "/p/exports/hero.vrm".into(),
        bytes: 1024.0,
        source_version: 2,
        normalized: false,
        reweighted: false,
        material: MaterialMode::MtoonSoft,
        highlights: Some(Box::new(HighlightsReport {
            mode: Highlights::Soften,
            strength: 0.75,
            images: vec![ImageHighlights {
                image: 1,
                width: 2048,
                height: 2048,
                coverage: 0.03,
                dark_coverage: 0.032,
                softened: true,
                note: None,
            }],
            note: None,
        })),
        unresolved: Vec::new(),
        written_at: "2026-10-30T12:00:00Z".into(),
        exists: true,
        report: Some("/p/exports/hero.vrm.report.json".into()),
        round_trip: Some(RoundTrip {
            passed: true,
            problems: Vec::new(),
            bones: 52,
            springs: 0,
        }),
        springs: 2,
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
    assert_eq!(answer["springs"], 2);
    assert_eq!(answer["highlights"]["mode"], "soften");
    assert_eq!(answer["highlights"]["images"][0]["softened"], true);
    assert!(hub.approvals().is_empty());
}

fn chained() -> Chained {
    Chained {
        project: "pets".into(),
        asset: "hero".into(),
        source_version: 2,
        state: NormalizeState::Pending,
        glb: "/cache/avatar/pets/hero/v2/springs/chain/springs.glb".into(),
        sheets: NormalizeSheets {
            before: "/cache/avatar/pets/hero/v2/springs/chain/sheet_before.png".into(),
            after: "/cache/avatar/pets/hero/v2/springs/chain/sheet_after.png".into(),
        },
        report: ChainReport {
            chains: vec![ChainBuilt {
                id: "hair-1".into(),
                root_bone: "head".into(),
                joints: vec!["spring_hair-1_0".into(), "spring_hair-1_1".into()],
                ..ChainBuilt::default()
            }],
            changed: 640,
            ..ChainReport::default()
        },
        key: "s".into(),
        approved_at: None,
    }
}

#[test]
fn waiting_chains_ask_a_person_once_with_the_sway_sheet() {
    let hub = Hub::new(|| {});
    let built = AvatarBuild::SpringsWaiting { chained: chained() };
    let first = build_answer(&hub, "Hero", &built);
    assert_eq!(
        (first["status"].as_str(), first["step"].as_str()),
        (Some(WAITING), Some("springs"))
    );
    assert_eq!(first["springs"]["chains"][0]["id"], "hair-1");
    assert_eq!(first["springs"]["chains"][0]["joints"], 2);
    assert!(
        first["springs"]["sheets"]["after"]
            .as_str()
            .is_some_and(|p| p.ends_with("sheet_after.png"))
    );
    let again = build_answer(&hub, "Hero", &built);
    assert_eq!(again["approval"], first["approval"]);
    let approvals = hub.approvals();
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].kind, ApprovalKind::Springs);
    assert!(
        approvals[0]
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("hair-1"))
    );
    assert!(hub.springs_for("pets", "hero").is_some());
}

#[test]
fn springs_are_confirmed_by_candidate_id_and_tuned_on_top() {
    let view = SpringsView {
        ready: true,
        found: Some(FoundSprings {
            candidates: vec![SpringCandidate {
                id: "hair-1".into(),
                kind: SpringKind::Hair,
                root_bone: "head".into(),
                line: vec![[0.0, 1.6, -0.1], [0.0, 1.0, -0.2]],
                length: 0.6,
                drop: 0.6,
                vertices: 900,
                confidence: 0.9,
                joints: 6,
                color: "#e8553f".into(),
            }],
            sheet: "/c/candidates.png".into(),
            input: "/c/normalized.glb".into(),
            height: 1.7,
        }),
        chained: None,
    };
    let arg = |id: &str| springs::SpringArg {
        id: id.into(),
        kind: None,
        root_bone: None,
        line: None,
        joints: None,
        preset: None,
        stiffness: None,
        gravity_power: None,
        drag_force: None,
        hit_radius: None,
    };
    let tuned = springs::SpringArg {
        joints: Some(4),
        preset: Some("Heavy".into()),
        drag_force: Some(0.7),
        ..arg("hair-1")
    };
    let specs = springs::confirmed(&view, &[tuned]).expect("confirmed");
    assert_eq!(specs[0].root_bone, "head");
    assert_eq!(specs[0].line.len(), 2);
    assert_eq!(specs[0].joints, Some(4));
    assert_eq!(specs[0].preset, Some(SpringPreset::Heavy));
    assert_eq!(specs[0].settings().drag_force, 0.7);
    let err = springs::confirmed(&view, &[arg("cape-9")]).expect_err("unknown");
    assert!(err.contains("No candidate is called cape-9"), "{err}");
    assert_eq!(springs::parse_springs(Some("OFF")), Ok(SpringsMode::Off));
    assert!(springs::parse_springs(Some("on")).is_err());
}
