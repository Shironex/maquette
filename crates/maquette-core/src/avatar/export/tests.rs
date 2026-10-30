use std::fs;

use serde_json::json;

use super::*;
use crate::avatar::normalize::tests::{FakeBlender, World, tools, world, world_with};
use crate::avatar::reweight::tests::FakeReweighter;
use crate::base::ports::ScriptRunner;
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
        reweight: ReweightMode::Auto,
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

/// Blender for a whole build: the reweight script and the normalise script
/// each go to their own stand in.
struct BothSteps {
    reweight: FakeReweighter,
    normalize: FakeBlender,
}

impl ScriptRunner for BothSteps {
    fn run_script(
        &self,
        tools: &Toolbox,
        work: &Path,
        script: (&str, &str),
        extra: &[String],
    ) -> CoreResult<()> {
        if script.0 == "avatar_reweight.py" {
            self.reweight.run_script(tools, work, script, extra)
        } else {
            self.normalize.run_script(tools, work, script, extra)
        }
    }
}

/// A character in an A-pose facing +X with a cape weighted to its arm.
fn caped() -> World {
    let w = world_with(&fixture::humanoid(&HumanoidSpec {
        arm_drop: 60.0,
        yaw: 90.0,
        offset: [0.0, 0.2],
        cape: true,
        ..HumanoidSpec::default()
    }));
    named(&w);
    w
}

#[test]
fn cloth_on_the_arms_is_freed_first_and_waits_for_its_own_yes() {
    let w = caped();
    let v = view(&request(&w)).expect("view");
    assert!(v.needs_reweight && v.needs_normalize && v.reweighted.is_none());
    let planned = plan(&request(&w)).expect("plan");
    assert!(planned.reweight && planned.reweight_state.is_none());

    let blender = BothSteps {
        reweight: FakeReweighter::good(),
        normalize: FakeBlender::good(),
    };
    let AvatarBuild::ReweightWaiting { reweighted } =
        build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("the reweight waits first");
    };
    assert_eq!(reweighted.state, NormalizeState::Pending);
    assert_eq!(
        (blender.reweight.runs.get(), blender.normalize.runs.get()),
        (1, 0)
    );
    let again = plan(&request(&w)).expect("plan");
    assert_eq!(again.reweight_state, Some(NormalizeState::Pending));
    assert_eq!(
        view(&request(&w)).expect("view").reweighted,
        Some(reweighted.clone())
    );

    // A yes moves on to the rest pose, lifted from the reweighted model.
    decide_reweight(&request(&w), true).expect("approve");
    let AvatarBuild::Waiting { normalized } =
        build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("the rest pose waits next");
    };
    assert_eq!(blender.reweight.runs.get(), 1, "cached");
    assert_eq!(normalized.reweight.as_ref().map(|r| r.moved), Some(120));
    decide(&request(&w), true).expect("approve the pose");
    let AvatarBuild::Written { export, .. } =
        build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("written");
    };
    assert!(export.reweighted && export.normalized);
    let report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(export.report.as_deref().expect("report")).expect("read"),
    )
    .expect("json");
    assert_eq!(report["reweight"]["moved"], 120);
    assert_eq!(report["reweight"]["keptAsSleeve"], 150);
    assert_eq!(report["reweight"]["fromBones"]["mixamorig:LeftArm"], 100);
}

#[test]
fn a_no_to_the_weights_or_reweight_off_lifts_the_source_as_it_is() {
    let w = caped();
    let blender = BothSteps {
        reweight: FakeReweighter::good(),
        normalize: FakeBlender::good(),
    };
    let off = AvatarRequest {
        reweight: ReweightMode::Off,
        ..request(&w)
    };
    assert!(!plan(&off).expect("plan").reweight);
    let AvatarBuild::Waiting { normalized } = build(&blender, &tools(), &off).expect("build")
    else {
        panic!("straight to the rest pose");
    };
    assert!(normalized.reweight.is_none());
    assert_eq!(blender.reweight.runs.get(), 0);

    assert!(matches!(
        build(&blender, &tools(), &request(&w)).expect("build"),
        AvatarBuild::ReweightWaiting { .. }
    ));
    let denied = decide_reweight(&request(&w), false).expect("deny");
    assert_eq!(denied.state, NormalizeState::Denied);
    let AvatarBuild::Waiting { normalized } =
        build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("the rest pose, from the source");
    };
    assert!(normalized.reweight.is_none());
    assert_eq!(
        blender.normalize.runs.get(),
        1,
        "the same run as with reweight off"
    );
}

#[test]
fn after_a_reweight_the_cloth_left_on_the_arms_is_a_note_not_an_offer() {
    let w = caped();
    // The normalised model still has the cape on its arm: what the
    // reweight kept as sleeve.
    let blender = BothSteps {
        reweight: FakeReweighter::good(),
        normalize: FakeBlender::new(fixture::humanoid(&HumanoidSpec {
            cape: true,
            ..HumanoidSpec::default()
        })),
    };
    build(&blender, &tools(), &request(&w)).expect("reweight waits");
    decide_reweight(&request(&w), true).expect("approve the weights");
    let AvatarBuild::Waiting { normalized } =
        build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("the rest pose waits next");
    };
    let left = |findings: &[VrmFinding]| {
        findings
            .iter()
            .find(|f| f.id == "arm-weights")
            .cloned()
            .expect("arm weights")
    };
    let after = left(&normalized.readiness.findings);
    assert_eq!(after.level, vrm::FindingLevel::Note);
    assert_eq!(
        after.title,
        "24 vertices still follow the arms after the reweight, cloth sewn to the sleeves"
    );
    assert!(
        !after
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("Free them first")
    );

    decide(&request(&w), true).expect("approve the pose");
    let AvatarBuild::Written { export, .. } =
        build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("written");
    };
    assert_eq!(left(&export.unresolved), after);
    let report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(export.report.as_deref().expect("report")).expect("read"),
    )
    .expect("json");
    for list in [&report["unresolved"], &report["after"]["findings"]] {
        let found = list
            .as_array()
            .expect("findings")
            .iter()
            .find(|f| f["id"] == "arm-weights")
            .expect("arm weights");
        assert_eq!(found["level"], "note", "{found}");
        assert_eq!(found["title"], after.title.as_str());
    }
    // The source's own readiness still offers the reweight.
    let before = report["before"]["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .find(|f| f["id"] == "arm-weights")
        .expect("arm weights");
    assert_eq!(before["level"], "warning");
}
