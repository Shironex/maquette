use std::path::Path;

use maquette_core::avatar::export::AvatarExport;
use maquette_core::avatar::export::{NoSprings, SpringCarried, SpringsChange, SpringsView};
use maquette_core::avatar::highlights::{HighlightsReport, ImageHighlights};
use maquette_core::avatar::normalize::{NormalizeReport, NormalizeSheets, NormalizeState};
use maquette_core::avatar::reweight::{ReweightBridge, ReweightReport};
use maquette_core::avatar::springs::{
    ChainBuilt, ChainReport, Chained, FoundSprings, SpringCandidate,
};
use maquette_core::avatar::{BridgeFaces, Highlights};
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
            bridge_faces: 17,
            bridges: vec![ReweightBridge {
                side: "right".into(),
                faces: 17,
                at: [0.1, 0.98, 0.08],
                length: 0.48,
            }],
            hidden_faces: 17,
            ..ReweightReport::default()
        },
        bridges: BridgeFaces::Hide,
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
    assert_eq!(first["reweight"]["bridgeFaces"], "hide");
    assert_eq!(first["reweight"]["hiddenFaces"], 17);
    assert_eq!(first["reweight"]["bridges"][0]["side"], "right");
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
    let built = AvatarBuild::SpringsWaiting {
        chained: chained(),
        carried: Vec::new(),
    };
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
    assert!(first.get("carried").is_none());

    let carried = AvatarBuild::SpringsWaiting {
        chained: chained(),
        carried: vec![SpringCarried {
            from: "cloth-1".into(),
            to: "hair-1".into(),
            root_bone: "head".into(),
            overlap: 1.0,
        }],
    };
    let answer = build_answer(&hub, "Hero", &carried);
    assert_eq!(answer["carried"][0]["from"], "cloth-1");
    assert_eq!(answer["carried"][0]["to"], "hair-1");
    assert!(
        answer["message"]
            .as_str()
            .is_some_and(|m| m.contains("carried over"))
    );
}

#[test]
fn a_build_the_springs_stop_says_what_to_pick_and_writes_nothing() {
    let hub = Hub::new(|| {});
    let found = view().found.expect("found");
    let unpicked = build_answer(
        &hub,
        "Hero",
        &AvatarBuild::SpringsUnpicked {
            found: found.clone(),
        },
    );
    assert_eq!(unpicked["status"], "candidates waiting for a pick");
    assert_eq!(unpicked["springs"]["candidates"][0]["id"], "hair-1");
    assert!(
        unpicked["message"]
            .as_str()
            .is_some_and(|m| m.contains("\"none\""))
    );
    let stale = build_answer(
        &hub,
        "Hero",
        &AvatarBuild::SpringsStale {
            found,
            stale: vec!["cloth-1".into()],
        },
    );
    assert_eq!(stale["status"], "springs no longer match");
    assert_eq!(stale["stale"][0], "cloth-1");
    assert!(
        stale["message"]
            .as_str()
            .is_some_and(|m| m.starts_with("The confirmed springs no longer match; pick again."))
    );
    assert!(hub.approvals().is_empty());
}

#[test]
fn springs_none_confirms_none_and_an_empty_list_clears_the_picks() {
    let view = view();
    let none: springs::SpringsArg = serde_json::from_value(json!("None")).expect("word");
    assert_eq!(
        springs::change(&view, &none),
        Ok(SpringsChange::None(NoSprings::None))
    );
    let cleared: springs::SpringsArg = serde_json::from_value(json!([])).expect("list");
    assert_eq!(
        springs::change(&view, &cleared),
        Ok(SpringsChange::Picked(Vec::new()))
    );
    let picked: springs::SpringsArg =
        serde_json::from_value(json!([{ "id": "hair-1" }])).expect("list");
    assert!(matches!(
        springs::change(&view, &picked),
        Ok(SpringsChange::Picked(list)) if list.len() == 1
    ));
    let other: springs::SpringsArg = serde_json::from_value(json!("all")).expect("word");
    assert!(
        springs::change(&view, &other)
            .expect_err("unknown word")
            .contains("\"none\"")
    );
}

/// A springs view with one candidate, the ponytail.
fn view() -> SpringsView {
    SpringsView {
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
                small: false,
            }],
            sheet: "/c/candidates.png".into(),
            input: "/c/normalized.glb".into(),
            height: 1.7,
        }),
        chained: None,
    }
}

#[test]
fn springs_are_confirmed_by_candidate_id_and_tuned_on_top() {
    let view = view();
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

#[test]
fn a_small_piece_is_confirmed_by_id_with_its_short_chain() {
    let mut view = view();
    if let Some(found) = view.found.as_mut() {
        found.candidates.push(SpringCandidate {
            id: "charm-1".into(),
            kind: SpringKind::Accessory,
            root_bone: "hips".into(),
            line: vec![[0.1, 0.95, 0.1], [0.1, 0.85, 0.1]],
            length: 0.1,
            drop: 0.1,
            vertices: 40,
            confidence: 0.8,
            joints: 3,
            color: "#3fa7e8".into(),
            small: true,
        });
    }
    let picked: springs::SpringsArg =
        serde_json::from_value(json!([{ "id": "charm-1" }, { "id": "hair-1" }])).expect("list");
    let Ok(SpringsChange::Picked(specs)) = springs::change(&view, &picked) else {
        panic!("picked");
    };
    assert_eq!(specs[0].root_bone, "hips");
    assert_eq!(specs[0].joints, Some(3));
    assert_eq!(specs[0].preset(), SpringPreset::Stiff);
    assert_eq!(
        specs[1].joints, None,
        "a long part keeps its joints by length"
    );
    let found = view.found.as_ref().expect("found");
    let summary = springs::candidates_summary(found);
    assert_eq!(summary["candidates"][1]["small"], true);
    assert_eq!(summary["candidates"][0]["small"], false);
}

fn avatar_view() -> AvatarView {
    let readiness = vrm::readiness(
        &fixture::humanoid(&HumanoidSpec::default()),
        Path::new("."),
        &BoneOverrides::new(),
    )
    .expect("readiness");
    AvatarView {
        project: "pets".into(),
        asset: "hero".into(),
        name: "Hero".into(),
        kind: maquette_core::AssetKind::Character,
        version: Some(1),
        model: Some("/p/hero.glb".into()),
        spec: AvatarSpec::default(),
        stored: true,
        readiness: Some(readiness),
        readiness_error: None,
        needs_normalize: false,
        needs_reweight: false,
        reweighted: None,
        normalized: None,
        springs: view(),
        export: None,
        export_path: "/p/exports/hero.vrm".into(),
        meta_problem: Some("The avatar needs a name.".into()),
    }
}

#[test]
fn get_avatar_is_short_until_bones_and_joints_are_asked_for() {
    let view = avatar_view();
    let short = view_summary(&view, false, Include::default());
    let readiness = &short["readiness"];
    assert!(readiness.get("bones").is_none() && readiness.get("joints").is_none());
    assert!(readiness["jointCount"].as_u64().is_some());
    assert_eq!(readiness["missingRequired"], json!([]));
    assert_eq!(short["waitingApproval"], json!(false));
    assert_eq!(short["springs"]["found"]["candidates"][0]["id"], "hair-1");

    let words = ["Bones".to_string(), "joints".to_string()];
    let full = view_summary(&view, true, Include::parse(&words).expect("include"));
    assert!(
        full["readiness"]["bones"]
            .as_array()
            .is_some_and(|b| b.len() == 55)
    );
    assert!(full["readiness"]["joints"].is_array());
    assert!(short.to_string().len() < full.to_string().len() / 2);
    assert!(Include::parse(&["spring".to_string()]).is_err());
}

#[test]
fn set_avatar_answers_what_changed_the_spec_and_the_problems() {
    let saved = avatar_view();
    let mut before = saved.spec.clone();
    before.material = MaterialMode::Pbr;
    before.highlights = Highlights::Keep;
    let answer = set_answer(&before, &saved);
    assert_eq!(answer["status"], "saved");
    assert_eq!(answer["changed"], json!(["highlights", "material"]));
    assert_eq!(answer["problems"], json!(["The avatar needs a name."]));
    assert_eq!(answer["ready"], json!(false));
    assert!(answer.get("readiness").is_none() && answer.get("springs").is_none());

    let same = set_answer(&saved.spec, &saved);
    assert_eq!(same["status"], "unchanged");
    assert_eq!(same["changed"], json!([]));
}
