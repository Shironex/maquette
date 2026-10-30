use std::fs;

use serde_json::json;

use super::*;
use crate::avatar::normalize::tests::{FakeBlender, World, tools, world};
use crate::inbox::fixture::{self, HumanoidSpec};
use crate::inbox::vrm::VRMC_VRM;

fn request(w: &World) -> AvatarRequest<'_> {
    AvatarRequest {
        workspace_root: &w.root,
        config: &w.config,
        project: &w.project,
        asset: &w.asset,
        version: None,
        elbows: Elbows::Straighten,
    }
}

fn project_dir(w: &World) -> PathBuf {
    project::dir(&w.root, &w.config.layout, &w.project).expect("dir")
}

/// Stores a spec with a name and an author, so the meta can be written.
fn named(w: &World) {
    let mut spec = w.config.avatar.spec("Hero");
    spec.meta.authors = vec!["Me".into()];
    super::super::set(&project_dir(w), &w.asset, Some(spec)).expect("spec");
}

fn versions(w: &World) -> usize {
    let manifest = project::load(&w.root, &w.config.layout, &w.project).expect("manifest");
    assets::find(&manifest, &w.asset)
        .expect("asset")
        .versions
        .len()
}

#[test]
fn the_view_reads_defaults_readiness_and_where_the_file_goes() {
    let w = world();
    let v = view(&request(&w)).expect("view");
    assert_eq!(
        (v.name.as_str(), v.version, v.stored),
        ("Hero", Some(1), false)
    );
    assert_eq!(v.spec.meta.name, "Hero");
    assert!(v.needs_normalize);
    let r = v.readiness.expect("readiness");
    assert!(r.findings.iter().any(|f| f.id == "arms"));
    assert!(v.normalized.is_none() && v.export.is_none());
    let tail = std::path::Path::new("exports").join(format!("{}.vrm", w.asset));
    assert!(std::path::Path::new(&v.export_path).ends_with(tail));
    assert_eq!(v.meta_problem.as_deref(), Some("Name at least one author."));
}

#[test]
fn a_build_waits_for_a_yes_then_writes_the_export_and_no_version() {
    let w = world();
    let blender = FakeBlender::good();
    let refused = build(&blender, &tools(), &request(&w)).expect_err("no author");
    assert!(refused.to_string().contains("author"), "{refused}");
    assert_eq!(blender.runs.get(), 0);

    named(&w);
    let planned = plan(&request(&w)).expect("plan");
    assert!(planned.normalize && !planned.cached && planned.state.is_none());
    assert_eq!(planned.meta_problem, None);
    let AvatarBuild::Waiting { normalized } =
        build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("waits for a yes");
    };
    assert_eq!(normalized.state, NormalizeState::Pending);
    let again = plan(&request(&w)).expect("plan");
    assert_eq!(
        (again.cached, again.state),
        (true, Some(NormalizeState::Pending))
    );
    // Asking again reuses the result and still waits.
    assert!(matches!(
        build(&blender, &tools(), &request(&w)).expect("build"),
        AvatarBuild::Waiting { .. }
    ));
    assert_eq!(blender.runs.get(), 1);

    assert_eq!(
        decide(&request(&w), true).expect("approve").state,
        NormalizeState::Approved
    );
    let AvatarBuild::Written { export, normalized } =
        build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("written");
    };
    assert!(normalized.is_some() && export.normalized && export.exists);
    assert_eq!(export.source_version, 1);
    let file = vrm_file(&project_dir(&w), &w.asset);
    assert_eq!(export.path, file.display().to_string());
    let bytes = fs::read(&file).expect("vrm");
    assert!((export.bytes - bytes.len() as f64).abs() < 0.5);
    let gltf = gltf::Gltf::from_slice(&bytes).expect("glTF");
    assert!(gltf.extensions_used().any(|e| e == VRMC_VRM));
    // A derived export: the asset gains no version.
    assert_eq!(versions(&w), 1);
    let v = view(&request(&w)).expect("view");
    assert_eq!(v.export, Some(export));
    assert_eq!(
        v.normalized.map(|n| n.state),
        Some(NormalizeState::Approved)
    );
}

#[test]
fn a_no_stops_the_build_until_the_options_change() {
    let w = world();
    named(&w);
    let blender = FakeBlender::good();
    build(&blender, &tools(), &request(&w)).expect("build");
    decide(&request(&w), false).expect("deny");
    assert!(matches!(
        build(&blender, &tools(), &request(&w)).expect("build"),
        AvatarBuild::Denied { .. }
    ));
    let keep = AvatarRequest {
        elbows: Elbows::Keep,
        ..request(&w)
    };
    assert!(matches!(
        build(&blender, &tools(), &keep).expect("build"),
        AvatarBuild::Waiting { .. }
    ));
    assert!(!vrm_file(&project_dir(&w), &w.asset).exists());
}

#[test]
fn a_model_already_in_the_vrm_rest_pose_is_written_without_blender() {
    let w = world();
    named(&w);
    let dir = project_dir(&w);
    fs::write(
        dir.join("models/hero_v2.glb"),
        fixture::humanoid(&HumanoidSpec::default()),
    )
    .expect("model");
    project::modify(&w.root, &w.config.layout, &w.project, |m| {
        assets::add_version(m, &w.asset, "models/hero_v2.glb".into(), None).map(drop)
    })
    .expect("version");
    let planned = plan(&request(&w)).expect("plan");
    assert_eq!((planned.version, planned.normalize), (2, false));
    let blender = FakeBlender::good();
    let AvatarBuild::Written { export, normalized } =
        build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("written");
    };
    assert_eq!(blender.runs.get(), 0);
    assert!(normalized.is_none() && !export.normalized);
    assert_eq!(export.source_version, 2);
    assert!(
        export
            .unresolved
            .iter()
            .all(|f| f.level != vrm::FindingLevel::Blocker)
    );
}

#[test]
fn a_patch_changes_only_what_it_names() {
    let defaults = WorkspaceConfig::default().avatar.spec("Hero");
    let current = AvatarSpec {
        material: MaterialMode::Pbr,
        ..defaults.clone()
    };
    let patch: AvatarPatch = serde_json::from_value(json!({
        "meta": { "authors": ["Me"], "commercialUsage": "corporation", "version": "1" },
        "bones": { "leftEye": "Eye_L", "jaw": null },
    }))
    .expect("patch");
    let next = apply_patch(&current, &defaults, &patch).expect("apply");
    assert_eq!(next.meta.authors, ["Me"]);
    assert_eq!(
        next.meta.commercial_usage,
        vrm::CommercialUsage::Corporation
    );
    assert_eq!(next.material, MaterialMode::Pbr);
    assert_eq!(next.bones.get("leftEye"), Some(&Some("Eye_L".to_string())));
    assert_eq!(next.bones.get("jaw"), Some(&None));

    let clear: AvatarPatch = serde_json::from_value(json!({
        "meta": { "version": null },
        "clearBones": ["jaw"],
        "material": "mtoon-soft",
    }))
    .expect("patch");
    let cleared = apply_patch(&next, &defaults, &clear).expect("apply");
    assert_eq!(cleared.meta.version, None);
    assert!(!cleared.bones.contains_key("jaw"));
    assert_eq!(cleared.material, MaterialMode::MtoonSoft);

    let reset: AvatarPatch = serde_json::from_value(json!({ "reset": true })).expect("patch");
    assert_eq!(
        apply_patch(&cleared, &defaults, &reset).expect("reset"),
        defaults
    );

    for (bad, says) in [
        (
            json!({ "meta": { "colour": "red" } }),
            "not a VRM meta field",
        ),
        (json!({ "meta": { "authors": "Me" } }), "does not read"),
        (
            json!({ "bones": { "tail": "Tail_1" } }),
            "not a VRM humanoid bone",
        ),
        (
            json!({ "meta": { "licenseUrl": "https://example.com" } }),
            "licence URL",
        ),
    ] {
        let patch: AvatarPatch = serde_json::from_value(bad).expect("patch");
        let err = apply_patch(&current, &defaults, &patch).expect_err("refused");
        assert!(err.to_string().contains(says), "{err}");
    }
}

#[test]
fn a_written_vrm_has_a_report_beside_it_and_reads_back() {
    let w = world();
    named(&w);
    let blender = FakeBlender::good();
    build(&blender, &tools(), &request(&w)).expect("build");
    decide(&request(&w), true).expect("approve");
    let AvatarBuild::Written { export, .. } =
        build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("written");
    };
    let file = vrm_file(&project_dir(&w), &w.asset);
    let path = report::report_file(&file);
    assert!(path.ends_with(format!("{}.vrm.report.json", w.asset)));
    assert_eq!(export.report, Some(path.display().to_string()));
    let trip = export.round_trip.clone().expect("round trip");
    assert!(trip.passed, "{:?}", trip.problems);
    assert!(
        export.unresolved.iter().all(|f| f.id != "round-trip"),
        "{:?}",
        export.unresolved
    );

    let text = fs::read_to_string(&path).expect("report");
    let found: report::AvatarReport = serde_json::from_str(&text).expect("parse");
    assert_eq!(found.file, file.display().to_string());
    assert_eq!(found.writer, report::WRITER);
    assert_eq!(found.source.version, 1);
    let source = project_dir(&w).join("models").join("hero_v1.glb");
    assert_eq!(found.source.sha256, hash_file(&source).expect("hash"));
    assert!(found.normalized);
    assert_eq!(found.elbows, Some(Elbows::Straighten));
    assert!(!found.before.ready || !found.before.findings.is_empty());
    assert!(found.after.is_some());
    assert_eq!(found.meta.name, "Hero");
    assert_eq!(found.meta.authors, ["Me"]);
    assert!(found.round_trip.passed);
    let hips = found.bones.iter().find(|b| b.bone == "hips").expect("hips");
    assert_eq!(hips.source, Some(report::BoneSource::Convention));
    assert_eq!(found.unresolved, export.unresolved);

    // The view keeps the report while it is there, and drops it once gone.
    assert_eq!(view(&request(&w)).expect("view").export, Some(export));
    fs::remove_file(&path).expect("remove");
    let gone = view(&request(&w)).expect("view").export.expect("export");
    assert_eq!(gone.report, None);
}

#[test]
fn a_bone_set_by_hand_reads_as_an_override_in_the_report() {
    let w = world();
    named(&w);
    let dir = project_dir(&w);
    fs::write(
        dir.join("models/hero_v2.glb"),
        fixture::humanoid(&HumanoidSpec::default()),
    )
    .expect("model");
    project::modify(&w.root, &w.config.layout, &w.project, |m| {
        assets::add_version(m, &w.asset, "models/hero_v2.glb".into(), None).map(drop)
    })
    .expect("version");
    let mut spec = super::super::load(&dir)
        .expect("avatars")
        .avatars
        .values()
        .next()
        .cloned()
        .expect("spec");
    spec.bones
        .insert("leftToes".into(), Some("mixamorig:LeftToeBase".into()));
    super::super::set(&dir, &w.asset, Some(spec)).expect("spec");
    let AvatarBuild::Written { export, .. } =
        build(&FakeBlender::good(), &tools(), &request(&w)).expect("build")
    else {
        panic!("written");
    };
    let text = fs::read_to_string(export.report.expect("report")).expect("report");
    let found: report::AvatarReport = serde_json::from_str(&text).expect("parse");
    assert_eq!((found.normalized, found.elbows), (false, None));
    assert_eq!(found.source.version, 2);
    let toes = found
        .bones
        .iter()
        .find(|b| b.bone == "leftToes")
        .expect("toes");
    assert_eq!(toes.source, Some(report::BoneSource::Override));
    let eye = found
        .bones
        .iter()
        .find(|b| b.bone == "leftEye")
        .expect("eye");
    assert_eq!((eye.joint.as_deref(), eye.source), (None, None));
}

#[test]
fn a_failed_round_trip_is_a_blocker() {
    let failed = RoundTrip {
        passed: false,
        problems: vec!["The meta has no name.".into(), "No animations.".into()],
        bones: 52,
    };
    let finding = report::round_trip_finding(&failed).expect("blocker");
    assert_eq!(
        (finding.id.as_str(), finding.level),
        ("round-trip", vrm::FindingLevel::Blocker)
    );
    assert_eq!(
        finding.detail.as_deref(),
        Some("The meta has no name. No animations.")
    );
    let passed = RoundTrip {
        passed: true,
        problems: Vec::new(),
        bones: 52,
    };
    assert_eq!(report::round_trip_finding(&passed), None);
}
