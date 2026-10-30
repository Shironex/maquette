use std::cell::{Cell, RefCell};
use std::fs;
use std::path::Path;

use super::*;
use crate::avatar::export::{self, AvatarBuild, AvatarRequest};
use crate::avatar::normalize::tests::{World, tools, world_with};
use crate::avatar::normalize::{Elbows, Fingers, NormalizeState};
use crate::avatar::reweight::ReweightMode;
use crate::avatar::{SpringSpec, set};
use crate::base::ports::ScriptRunner;
use crate::error::CoreResult;
use crate::inbox::fixture::{self, HumanoidSpec};
use crate::inbox::vrm::{SpringKind, SpringPreset, VRMC_SPRING_BONE};
use crate::pipeline::tools::Toolbox;
use crate::project;

/// A script runner standing in for Blender: in find mode it proposes one
/// strand of hair from the head, in chain mode it writes `output` as the
/// chained model with a chain down the spine (joints the box humanoid
/// has, so the writer finds them).
pub(crate) struct FakeSpringer {
    output: Vec<u8>,
    pub(crate) finds: Cell<u32>,
    pub(crate) chains: Cell<u32>,
    job: RefCell<Option<serde_json::Value>>,
    unweighted: u32,
}

impl FakeSpringer {
    pub(crate) fn good(output: &[u8]) -> Self {
        Self {
            output: output.to_vec(),
            finds: Cell::new(0),
            chains: Cell::new(0),
            job: RefCell::new(None),
            unweighted: 0,
        }
    }
}

const SPINE: [&str; 3] = ["mixamorig:Spine", "mixamorig:Spine1", "mixamorig:Spine2"];

impl ScriptRunner for FakeSpringer {
    fn run_script(
        &self,
        _tools: &Toolbox,
        work: &Path,
        (file, script): (&str, &str),
        extra: &[String],
    ) -> CoreResult<()> {
        if crate::avatar::highlights::tests::leaves_as_is(work, file) {
            return Ok(());
        }
        assert_eq!(file, SCRIPT_FILE);
        assert_eq!(script, SCRIPT);
        assert_eq!(extra, ["--job", JOB_FILE]);
        let job: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(work.join(JOB_FILE)).expect("job"))
                .expect("job json");
        *self.job.borrow_mut() = Some(job.clone());
        if job["mode"] == "find" {
            self.finds.set(self.finds.get() + 1);
            let found = serde_json::json!({
                "height": 1.8, "vertices": 900,
                "candidates": [{
                    "id": "hair-1", "kind": "hair", "rootBone": "head",
                    "line": [[0.0, 1.6, -0.1], [0.0, 1.2, -0.1]],
                    "length": 0.4, "drop": 0.4, "vertices": 66,
                    "confidence": 0.9, "joints": 5, "color": "#e0a040",
                }],
            });
            fs::write(work.join("candidates.json"), found.to_string()).expect("found");
            fs::write(work.join("candidates.png"), b"png").expect("sheet");
            return Ok(());
        }
        self.chains.set(self.chains.get() + 1);
        fs::write(work.join("springs.glb"), &self.output).expect("glb");
        fs::write(work.join("sheet_before.png"), b"png before").expect("sheet");
        fs::write(work.join("sheet_after.png"), b"png after").expect("sheet");
        let report = serde_json::json!({
            "height": 1.8,
            "chains": [{
                "id": "hair-1", "kind": "hair", "rootBone": "hips",
                "rootJoint": "mixamorig:Hips", "joints": SPINE,
                "length": 0.4, "vertices": 66, "blended": 12,
                "heads": [[0.0, 1.0, 0.0], [0.0, 1.1, 0.0], [0.0, 1.2, 0.0]],
                "radius": 0.02, "colliderGroups": ["Head", "Chest"],
            }],
            "colliders": [{
                "group": "Head", "bone": "head", "joint": "mixamorig:Head",
                "shape": { "sphere": { "offset": [0.0, 0.09, 0.0], "radius": 0.1 } },
            }],
            "weights": { "vertices": 900, "maxInfluences": 4, "unweighted": self.unweighted, "offSum": 0 },
            "changed": 66,
        });
        fs::write(work.join("report.json"), report.to_string()).expect("report");
        Ok(())
    }
}

/// A character in a T-pose facing VRM's way, with a strip hanging from
/// its head: its rest pose needs no normalising.
fn tailed() -> (World, Vec<u8>) {
    let model = fixture::humanoid(&HumanoidSpec {
        tail: true,
        animation: false,
        ..HumanoidSpec::default()
    });
    (world_with(&model), model)
}

fn request(w: &World) -> AvatarRequest<'_> {
    AvatarRequest {
        workspace_root: &w.root,
        config: &w.config,
        project: &w.project,
        asset: &w.asset,
        version: None,
        elbows: Elbows::Straighten,
        reweight: ReweightMode::Auto,
        fingers: Fingers::Straighten,
        springs: SpringsMode::Auto,
    }
}

fn hair() -> SpringSpec {
    SpringSpec {
        id: "hair-1".to_string(),
        kind: SpringKind::Hair,
        root_bone: "head".to_string(),
        line: vec![[0.0, 1.6, -0.1], [0.0, 1.2, -0.1]],
        joints: Some(3),
        preset: None,
        stiffness: None,
        gravity_power: None,
        drag_force: None,
        hit_radius: None,
    }
}

/// Stores a spec that can be written, with `springs` confirmed.
fn confirm(w: &World, springs: Vec<SpringSpec>) {
    let dir = project::dir(&w.root, &w.config.layout, &w.project).expect("dir");
    let mut spec = w.config.avatar.spec("Hero");
    spec.meta.authors = vec!["Me".into()];
    spec.springs = springs;
    set(&dir, &w.asset, Some(spec)).expect("spec");
}

fn input(w: &World, glb: &Path) -> SpringsInput {
    SpringsInput {
        project: w.project.clone(),
        asset: w.asset.clone(),
        version: 1,
        dir: w.root.join("springs-work"),
        glb: glb.to_path_buf(),
        bones: BTreeMap::from([("head".to_string(), "mixamorig:Head".to_string())]),
        cache_root: w.root.join("hashes"),
    }
}

fn source(w: &World) -> std::path::PathBuf {
    let view = export::view(&request(w)).expect("view");
    std::path::PathBuf::from(view.model.expect("model"))
}

/// The JSON chunk of a `.glb`.
fn glb_json(bytes: &[u8]) -> serde_json::Value {
    let len = u32::from_le_bytes(bytes[12..16].try_into().expect("chunk length"));
    let end = 20 + usize::try_from(len).expect("length");
    serde_json::from_slice(&bytes[20..end]).expect("json chunk")
}

#[test]
fn the_finder_runs_once_per_model_and_reads_back() {
    let (w, model) = tailed();
    let input = input(&w, &source(&w));
    assert_eq!(find::found(&input).expect("found"), None);
    let blender = FakeSpringer::good(&model);
    let found = find::find(&blender, &tools(), &input).expect("find");
    assert_eq!(found.candidates.len(), 1);
    assert_eq!(found.candidates[0].kind, SpringKind::Hair);
    assert_eq!(found.candidates[0].root_bone, "head");
    assert!(found.sheet.ends_with("candidates.png"));
    let job = blender.job.borrow().clone().expect("job");
    assert_eq!(job["bones"]["head"], "mixamorig:Head");
    assert_eq!(job["input"], input.glb.display().to_string());

    find::find(&blender, &tools(), &input).expect("again");
    assert_eq!(blender.finds.get(), 1, "cached");
    assert_eq!(find::found(&input).expect("found"), Some(found));
}

#[test]
fn chains_wait_for_a_yes_and_key_on_the_part_not_the_swing() {
    let (w, model) = tailed();
    let input = input(&w, &source(&w));
    assert!(chain::plan(&input, &[]).is_err());
    let plan = chain::plan(&input, &[hair()]).expect("plan");
    assert_eq!(plan.dir, input.dir.join("chain"));
    let blender = FakeSpringer::good(&model);
    let found = chain::run(&blender, &tools(), &plan).expect("run");
    assert_eq!(found.state, NormalizeState::Pending);
    assert_eq!(found.report.chains[0].joints, SPINE);
    assert_eq!(found.report.colliders.len(), 1);
    let job = blender.job.borrow().clone().expect("job");
    assert_eq!(job["springs"][0]["rootBone"], "head");
    assert_eq!(job["springs"][0]["joints"], 3);
    assert!(job["springs"][0].get("stiffness").is_none());
    assert!(chain::approved_glb(&plan.dir).is_err(), "pending");

    // Softer hair is the same chain; more joints is another.
    let softer = SpringSpec {
        stiffness: Some(0.2),
        preset: Some(SpringPreset::Heavy),
        ..hair()
    };
    assert_eq!(chain::plan(&input, &[softer]).expect("plan").key, plan.key);
    let longer = SpringSpec {
        joints: Some(6),
        ..hair()
    };
    assert_ne!(chain::plan(&input, &[longer]).expect("plan").key, plan.key);

    chain::approve(&plan.dir).expect("approve");
    assert_eq!(chain::approved_glb(&plan.dir).expect("bytes"), model);
    chain::run(&blender, &tools(), &plan).expect("cached");
    assert_eq!(blender.chains.get(), 1);
    let denied = chain::deny(&plan.dir).expect("deny");
    assert_eq!(denied.state, NormalizeState::Denied);
    assert!(chain::approved_glb(&plan.dir).is_err());
}

#[test]
fn a_chain_that_leaves_weights_out_or_moves_the_rig_is_refused() {
    let (w, model) = tailed();
    let input = input(&w, &source(&w));
    let plan = chain::plan(&input, &[hair()]).expect("plan");
    let holes = FakeSpringer {
        unweighted: 2,
        ..FakeSpringer::good(&model)
    };
    let err = chain::run(&holes, &tools(), &plan).expect_err("not whole");
    assert!(
        err.to_string().contains("2 vertices without weight"),
        "{err}"
    );

    let other = fixture::humanoid(&HumanoidSpec {
        naming: fixture::Naming::Unreal,
        animation: false,
        ..HumanoidSpec::default()
    });
    let err = chain::run(&FakeSpringer::good(&other), &tools(), &plan).expect_err("moved");
    assert!(
        err.to_string().contains("humanoid bones differently"),
        "{err}"
    );
    assert_eq!(chain::load(&plan.dir).expect("load"), None);
}

#[test]
fn spring_bones_take_the_spec_settings_or_the_kind_preset() {
    let report = ChainReport {
        chains: vec![
            ChainBuilt {
                id: "hair-1".to_string(),
                kind: Some(SpringKind::Hair),
                joints: SPINE.iter().map(ToString::to_string).collect(),
                ..ChainBuilt::default()
            },
            ChainBuilt {
                id: "accessory-1".to_string(),
                kind: Some(SpringKind::Accessory),
                ..ChainBuilt::default()
            },
        ],
        ..ChainReport::default()
    };
    let tuned = SpringSpec {
        stiffness: Some(0.9),
        ..hair()
    };
    let bones = chain::spring_bones(&report, &[tuned]);
    assert_eq!(bones.springs[0].settings.stiffness, 0.9);
    assert_eq!(
        bones.springs[0].settings.drag_force,
        SpringKind::Hair.preset().settings().drag_force
    );
    assert_eq!(
        bones.springs[1].settings,
        SpringKind::Accessory.preset().settings()
    );
}

#[test]
fn a_build_finds_first_then_chains_the_confirmed_springs_and_writes_them() {
    let (w, model) = tailed();
    confirm(&w, Vec::new());
    let blender = FakeSpringer::good(&model);
    let view = export::view(&request(&w)).expect("view");
    assert!(!view.needs_normalize, "{:?}", view.readiness);
    assert!(view.springs.ready && view.springs.found.is_none());

    // Nothing confirmed: the finder runs, the file has no springs.
    let AvatarBuild::Written { export: plain, .. } =
        export::build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("written without springs");
    };
    assert_eq!((plain.springs, blender.finds.get()), (0, 1));
    let found = export::view(&request(&w)).expect("view").springs.found;
    assert_eq!(found.map(|f| f.candidates.len()), Some(1));

    // Confirmed: the chains wait for their own yes.
    confirm(&w, vec![hair()]);
    assert_eq!(export::plan(&request(&w)).expect("plan").springs, 1);
    let AvatarBuild::SpringsWaiting { chained, .. } =
        export::build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("the chains wait");
    };
    assert_eq!(chained.state, NormalizeState::Pending);
    assert_eq!(
        export::view(&request(&w)).expect("view").springs.chained,
        Some(chained)
    );

    export::decide_springs(&request(&w), true).expect("approve");
    let AvatarBuild::Written { export: sprung, .. } =
        export::build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("written with springs");
    };
    assert_eq!(sprung.springs, 1);
    assert_eq!(blender.chains.get(), 1);
    let round_trip = sprung.round_trip.expect("round trip");
    assert!(round_trip.passed, "{:?}", round_trip.problems);
    assert_eq!(round_trip.springs, 1);
    let bytes = fs::read(&sprung.path).expect("vrm");
    let json = glb_json(&bytes);
    let ext = &json["extensions"][VRMC_SPRING_BONE];
    assert_eq!(ext["springs"][0]["name"], "hair-1");
    assert_eq!(
        ext["springs"][0]["joints"].as_array().map(Vec::len),
        Some(3)
    );
    let report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(sprung.report.as_deref().expect("report")).expect("read"),
    )
    .expect("json");
    assert_eq!(report["springs"]["springs"][0]["id"], "hair-1");
    assert_eq!(report["springs"]["springs"][0]["kind"], "hair");
    assert_eq!(report["springs"]["colliders"], 1);

    // A no writes the file without springs again; off skips them.
    export::decide_springs(&request(&w), false).expect("deny");
    let AvatarBuild::Written { export: denied, .. } =
        export::build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("written");
    };
    assert_eq!(denied.springs, 0);
    let off = AvatarRequest {
        springs: SpringsMode::Off,
        ..request(&w)
    };
    assert_eq!(export::plan(&off).expect("plan").springs, 0);
}

/// The stored spec's picks and whether it confirms none.
fn stored(w: &World) -> (Vec<SpringSpec>, bool) {
    let dir = project::dir(&w.root, &w.config.layout, &w.project).expect("dir");
    let spec = crate::avatar::load(&dir).expect("load").avatars[&w.asset].clone();
    (spec.springs, spec.no_springs)
}

/// Builds with `hair-1` confirmed, approves the chains and writes the VRM
/// with its spring.
fn written_with_a_spring(w: &World, blender: &FakeSpringer) {
    confirm(w, vec![hair()]);
    export::build(blender, &tools(), &request(w)).expect("chains");
    export::decide_springs(&request(w), true).expect("approve");
    let AvatarBuild::Written { export, .. } =
        export::build(blender, &tools(), &request(w)).expect("build")
    else {
        panic!("written");
    };
    assert_eq!(export.springs, 1);
}

#[test]
fn cleared_picks_wait_for_a_pick_and_only_none_writes_without_springs() {
    let (w, model) = tailed();
    let blender = FakeSpringer::good(&model);
    written_with_a_spring(&w, &blender);
    let before = fs::read(export::view(&request(&w)).expect("view").export_path).expect("vrm");

    // The picks cleared after an export with springs: the candidates wait,
    // nothing is written over the file.
    confirm(&w, Vec::new());
    let AvatarBuild::SpringsUnpicked { found } =
        export::build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("the candidates wait for a pick");
    };
    assert_eq!(found.candidates[0].id, "hair-1");
    let view = export::view(&request(&w)).expect("view");
    assert_eq!(fs::read(&view.export_path).expect("vrm"), before);
    assert_eq!(view.export.expect("export").springs, 1);

    // A person confirms none: written without springs.
    let dir = project::dir(&w.root, &w.config.layout, &w.project).expect("dir");
    let mut spec = crate::avatar::load(&dir).expect("load").avatars[&w.asset].clone();
    spec.no_springs = true;
    set(&dir, &w.asset, Some(spec)).expect("none");
    let AvatarBuild::Written { export, .. } =
        export::build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("written without springs");
    };
    assert_eq!(export.springs, 0);
    // With no springs in the last export, cleared picks write as before.
    confirm(&w, Vec::new());
    assert!(matches!(
        export::build(&blender, &tools(), &request(&w)).expect("build"),
        AvatarBuild::Written { .. }
    ));
}

#[test]
fn a_pick_the_finder_reads_differently_now_is_carried_over_and_saved() {
    let (w, model) = tailed();
    let blender = FakeSpringer::good(&model);
    // Picked when the finder read the strip as cloth from the upper chest;
    // it reads hair-1 from the head now, on the same line.
    let old = SpringSpec {
        id: "cloth-1".to_string(),
        kind: SpringKind::Cloth,
        root_bone: "upperChest".to_string(),
        preset: Some(SpringPreset::Heavy),
        ..hair()
    };
    confirm(&w, vec![old]);
    let AvatarBuild::SpringsWaiting { chained, carried } =
        export::build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("carried over, the chains wait");
    };
    assert_eq!(chained.state, NormalizeState::Pending);
    assert_eq!(carried.len(), 1);
    assert_eq!(
        (carried[0].from.as_str(), carried[0].to.as_str()),
        ("cloth-1", "hair-1")
    );
    let job = blender.job.borrow().clone().expect("job");
    assert_eq!(job["springs"][0]["id"], "hair-1");
    assert_eq!(job["springs"][0]["rootBone"], "head");
    let (picks, none) = stored(&w);
    assert!(!none);
    assert_eq!(picks.len(), 1);
    assert_eq!(
        (picks[0].id.as_str(), picks[0].root_bone.as_str()),
        ("hair-1", "head")
    );
    // The person's kind, joints and preset stay.
    assert_eq!(picks[0].kind, SpringKind::Cloth);
    assert_eq!(picks[0].joints, Some(3));
    assert_eq!(picks[0].preset, Some(SpringPreset::Heavy));
}

#[test]
fn a_pick_with_no_part_left_stops_the_build_without_writing() {
    let (w, model) = tailed();
    let blender = FakeSpringer::good(&model);
    let gone = SpringSpec {
        id: "cloth-2".to_string(),
        line: vec![[0.5, 1.0, 0.3], [0.5, 0.4, 0.3]],
        ..hair()
    };
    confirm(&w, vec![gone.clone()]);
    let AvatarBuild::SpringsStale { found, stale } =
        export::build(&blender, &tools(), &request(&w)).expect("build")
    else {
        panic!("the picks no longer match");
    };
    assert_eq!(stale, ["cloth-2"]);
    assert_eq!(found.candidates.len(), 1);
    assert_eq!(blender.chains.get(), 0);
    assert_eq!(stored(&w).0, vec![gone]);
    let view = export::view(&request(&w)).expect("view");
    assert!(view.export.is_none());
}

#[test]
fn the_skill_keeps_the_same_script() {
    let copy = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(SKILL_COPY);
    let text = fs::read_to_string(&copy).expect("the skill's copy");
    assert_eq!(
        text.replace("\r\n", "\n"),
        SCRIPT.replace("\r\n", "\n"),
        "copy src/avatar/avatar_springs.py over {SKILL_COPY}"
    );
}
