use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::assets::{AssetKind, NewAsset, Stage};
use crate::inbox::fixture::{self, HumanoidSpec};
use crate::inbox::vrm::{VRMC_VRM, VrmMeta, WriteSpec};
use crate::project::{NewProject, ProjectTemplate};
use crate::workspace;

struct World {
    _dir: tempfile::TempDir,
    root: PathBuf,
    config: WorkspaceConfig,
    project: String,
    asset: String,
}

/// A rigged character in an A-pose facing +X, as its first version.
fn world() -> World {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Pets".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    let models = project_dir.join("models");
    fs::create_dir_all(&models).expect("models");
    fs::write(
        models.join("hero_v1.glb"),
        fixture::humanoid(&HumanoidSpec {
            arm_drop: 60.0,
            yaw: 90.0,
            offset: [0.0, 0.2],
            ..HumanoidSpec::default()
        }),
    )
    .expect("model");
    let mut asset = String::new();
    project::modify(&root, &config.layout, &manifest.id, |m| {
        let added = assets::add(
            m,
            NewAsset {
                name: "Hero".into(),
                kind: AssetKind::Character,
                stage: Stage::Blender,
            },
        )?;
        assets::add_version(m, &added.id, "models/hero_v1.glb".into(), None)?;
        asset = added.id;
        Ok(())
    })
    .expect("asset");
    World {
        _dir: dir,
        root,
        config,
        project: manifest.id,
        asset,
    }
}

fn request<'a>(w: &'a World, overrides: &'a BoneOverrides, elbows: Elbows) -> NormalizeRequest<'a> {
    NormalizeRequest {
        workspace_root: &w.root,
        config: &w.config,
        project: &w.project,
        asset: &w.asset,
        version: None,
        overrides,
        elbows,
    }
}

/// A script runner standing in for Blender: it writes the outputs the
/// script would, with `output` as the normalised model.
struct FakeBlender {
    output: Vec<u8>,
    runs: Cell<u32>,
    job: RefCell<Option<serde_json::Value>>,
    fail: bool,
}

impl FakeBlender {
    fn new(output: Vec<u8>) -> Self {
        Self {
            output,
            runs: Cell::new(0),
            job: RefCell::new(None),
            fail: false,
        }
    }

    /// A clean T-pose facing +Z, as a good run leaves it.
    fn good() -> Self {
        Self::new(fixture::humanoid(&HumanoidSpec::default()))
    }
}

impl ScriptRunner for FakeBlender {
    fn run_script(
        &self,
        _tools: &Toolbox,
        work: &Path,
        (file, script): (&str, &str),
        extra: &[String],
    ) -> CoreResult<()> {
        self.runs.set(self.runs.get() + 1);
        assert_eq!(file, SCRIPT_FILE);
        assert_eq!(script, SCRIPT);
        assert_eq!(extra, ["--job", JOB_FILE]);
        let job: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(work.join(JOB_FILE)).expect("job"))
                .expect("job json");
        *self.job.borrow_mut() = Some(job);
        if self.fail {
            return Err(invalid("Blender stopped: the bone map has no hips"));
        }
        fs::write(work.join(OUTPUT_FILE), &self.output).expect("glb");
        fs::write(work.join(SHEET_BEFORE), b"png before").expect("sheet");
        fs::write(work.join(SHEET_AFTER), b"png after").expect("sheet");
        let report = serde_json::json!({
            "before": { "facingDeg": 90.0, "armLeftDeg": 60.0, "armRightDeg": 60.0, "hips": [0.0, 1.0, 0.2] },
            "after": { "facingDeg": 0.0, "armLeftDeg": 0.0, "armRightDeg": 0.0, "hips": [0.0, 1.0, 0.0] },
            "turnedDeg": -90.0,
            "moved": [0.0, 0.0, -0.2],
            "elbows": "keep",
            "lifted": { "left": { "shoulderDeg": 60.0 }, "right": { "shoulderDeg": 60.0 } },
            "weights": { "vertices": 100, "maxInfluences": 4 },
        });
        fs::write(work.join(REPORT_FILE), report.to_string()).expect("report");
        Ok(())
    }
}

fn tools() -> Toolbox {
    Toolbox::default()
}

#[test]
fn a_run_makes_a_pending_result_in_the_cache_and_no_new_version() {
    let w = world();
    let none = BoneOverrides::new();
    let plan = plan(&request(&w, &none, Elbows::Keep)).expect("plan");
    assert_eq!(plan.version, 1);
    let expected = w
        .root
        .join(&w.config.layout.cache)
        .join("avatar")
        .join(&w.project)
        .join(&w.asset)
        .join("v1");
    assert_eq!(plan.dir, expected);
    // The source's own findings are what normalising fixes.
    let blockers: Vec<&str> = plan
        .readiness
        .findings
        .iter()
        .filter(|f| f.level == FindingLevel::Blocker)
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(blockers, ["facing", "arms"]);

    let source_before = fs::read(&plan.source).expect("source");
    let blender = FakeBlender::good();
    let found = run(&blender, &tools(), &plan).expect("run");
    assert_eq!(blender.runs.get(), 1);
    assert_eq!(found.state, NormalizeState::Pending);
    assert_eq!(found.source_version, 1);
    assert_eq!(found.glb, expected.join(OUTPUT_FILE).display().to_string());
    assert_eq!(
        found.sheets.after,
        expected.join(SHEET_AFTER).display().to_string()
    );
    assert!((found.report.before.arm_left_deg - 60.0).abs() < 1e-9);
    assert_eq!(found.report.lifted["left"].shoulder_deg, 60.0);
    assert!(found.readiness.ready, "{:#?}", found.readiness.findings);

    // The job carries the bone roles by VRM name and the source by path.
    let job = blender.job.borrow().clone().expect("job");
    assert_eq!(job["bones"]["leftUpperArm"], "mixamorig:LeftArm");
    assert_eq!(job["bones"]["hips"], "mixamorig:Hips");
    assert_eq!(job["input"], plan.source.display().to_string());
    assert_eq!(job["elbows"], "keep");

    // The version is only read, and stays the asset's only one.
    assert_eq!(fs::read(&plan.source).expect("source"), source_before);
    let manifest = project::load(&w.root, &w.config.layout, &w.project).expect("load");
    assert_eq!(
        assets::find(&manifest, &w.asset)
            .expect("asset")
            .versions
            .len(),
        1
    );
    assert_eq!(load(&plan.dir).expect("load"), Some(found));
}

#[test]
fn the_same_plan_comes_from_the_cache_and_another_runs_again() {
    let w = world();
    let none = BoneOverrides::new();
    let blender = FakeBlender::good();
    let first = plan(&request(&w, &none, Elbows::Keep)).expect("plan");
    run(&blender, &tools(), &first).expect("run");
    let again = plan(&request(&w, &none, Elbows::Keep)).expect("plan");
    assert_eq!(again.key, first.key);
    run(&blender, &tools(), &again).expect("cached");
    assert_eq!(
        blender.runs.get(),
        1,
        "nothing changed, so Blender did not run"
    );

    // Another option is another key.
    let straight = plan(&request(&w, &none, Elbows::Straighten)).expect("plan");
    assert_ne!(straight.key, first.key);
    run(&blender, &tools(), &straight).expect("run");
    assert_eq!(blender.runs.get(), 2);
    let job = blender.job.borrow().clone().expect("job");
    assert_eq!(job["elbows"], "straighten");

    // An output touched since the run is not trusted.
    fs::write(first.dir.join(SHEET_AFTER), b"edited").expect("edit");
    assert_eq!(load(&first.dir).expect("load"), None);
    run(&blender, &tools(), &straight).expect("run");
    assert_eq!(blender.runs.get(), 3);
}

#[test]
fn the_writer_path_refuses_until_a_person_approves() {
    let w = world();
    let none = BoneOverrides::new();
    let plan = plan(&request(&w, &none, Elbows::Keep)).expect("plan");
    run(&FakeBlender::good(), &tools(), &plan).expect("run");
    let spec = WriteSpec {
        meta: VrmMeta {
            name: "Hero".into(),
            authors: vec!["Me".into()],
            ..VrmMeta::default()
        },
        ..WriteSpec::default()
    };
    let refused = write_vrm(&plan.dir, &spec).expect_err("pending");
    assert!(
        refused.to_string().contains("waits for a person's yes"),
        "{refused}"
    );
    assert!(approved_glb(&plan.dir).is_err());

    let approved = approve(&plan.dir).expect("approve");
    assert_eq!(approved.state, NormalizeState::Approved);
    assert!(approved.approved_at.is_some());
    assert_eq!(
        load(&plan.dir).expect("load").map(|n| n.state),
        Some(NormalizeState::Approved)
    );
    let written = write_vrm(&plan.dir, &spec).expect("write");
    let gltf = gltf::Gltf::from_slice(&written.bytes).expect("valid glTF");
    assert!(gltf.extensions_used().any(|e| e == VRMC_VRM));
    assert!(
        written
            .unresolved
            .iter()
            .all(|f| f.level != FindingLevel::Blocker)
    );

    // A changed file loses the yes.
    let glb = plan.dir.join(OUTPUT_FILE);
    let mut bytes = fs::read(&glb).expect("glb");
    bytes.push(0);
    fs::write(&glb, bytes).expect("tamper");
    assert!(write_vrm(&plan.dir, &spec).is_err());
    assert!(approve(&plan.dir).is_err(), "nothing intact to approve");
}

#[test]
fn a_new_run_clears_the_old_yes() {
    let w = world();
    let none = BoneOverrides::new();
    let blender = FakeBlender::good();
    let keep = plan(&request(&w, &none, Elbows::Keep)).expect("plan");
    run(&blender, &tools(), &keep).expect("run");
    approve(&keep.dir).expect("approve");
    // The same key from the cache keeps it.
    let cached = run(&blender, &tools(), &keep).expect("cached");
    assert_eq!(cached.state, NormalizeState::Approved);
    // Another key in the same folder makes a new output, pending again.
    let straight = plan(&request(&w, &none, Elbows::Straighten)).expect("plan");
    let fresh = run(&blender, &tools(), &straight).expect("run");
    assert_eq!(fresh.state, NormalizeState::Pending);
    assert!(approved_glb(&straight.dir).is_err());
}

#[test]
fn an_output_off_the_vrm_rest_pose_is_an_error() {
    let w = world();
    let none = BoneOverrides::new();
    let plan = plan(&request(&w, &none, Elbows::Keep)).expect("plan");
    let lazy = FakeBlender::new(fixture::humanoid(&HumanoidSpec {
        arm_drop: 5.0,
        yaw: 30.0,
        ..HumanoidSpec::default()
    }));
    let err = run(&lazy, &tools(), &plan).expect_err("off");
    let text = err.to_string();
    assert!(
        text.contains("the left arm is 5.0 degrees off level"),
        "{text}"
    );
    assert!(
        text.contains("the front is turned 30.0 degrees from +Z"),
        "{text}"
    );
    assert_eq!(load(&plan.dir).expect("load"), None, "nothing recorded");

    // Within the slack it passes.
    let close = FakeBlender::new(fixture::humanoid(&HumanoidSpec {
        arm_drop: 2.5,
        ..HumanoidSpec::default()
    }));
    run(&close, &tools(), &plan).expect("within 3 degrees");
}

#[test]
fn a_failed_script_or_a_missing_output_says_so() {
    let w = world();
    let none = BoneOverrides::new();
    let plan = plan(&request(&w, &none, Elbows::Keep)).expect("plan");
    let mut broken = FakeBlender::good();
    broken.fail = true;
    let err = run(&broken, &tools(), &plan).expect_err("failed");
    assert!(err.to_string().contains("no hips"), "{err}");
    assert_eq!(load(&plan.dir).expect("load"), None);

    struct Silent;
    impl ScriptRunner for Silent {
        fn run_script(
            &self,
            _: &Toolbox,
            _: &Path,
            _: (&str, &str),
            _: &[String],
        ) -> CoreResult<()> {
            Ok(())
        }
    }
    let err = run(&Silent, &tools(), &plan).expect_err("nothing written");
    assert!(
        err.to_string().contains("without writing normalized.glb"),
        "{err}"
    );
}

#[test]
fn a_rig_normalising_cannot_fix_is_refused_before_blender() {
    let w = world();
    let mut overrides = BoneOverrides::new();
    overrides.insert("hips".into(), None);
    let err = plan(&request(&w, &overrides, Elbows::Keep)).expect_err("no hips");
    assert!(err.to_string().contains("cannot fix this rig"), "{err}");

    let none = BoneOverrides::new();
    let missing = NormalizeRequest {
        version: Some(4),
        ..request(&w, &none, Elbows::Keep)
    };
    assert!(
        plan(&missing)
            .expect_err("no v4")
            .to_string()
            .contains("version 4")
    );
}

#[test]
fn the_rest_pose_bar_reads_the_readiness() {
    let read = |spec: &HumanoidSpec| {
        vrm::readiness(
            &fixture::humanoid(spec),
            Path::new("."),
            &BoneOverrides::new(),
        )
        .expect("readiness")
    };
    assert!(check_rest_pose(&read(&HumanoidSpec::default())).is_ok());
    let turned = read(&HumanoidSpec {
        yaw: 180.0,
        ..HumanoidSpec::default()
    });
    assert!(check_rest_pose(&turned).is_err());
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
        "copy src/avatar/avatar_normalize.py over {SKILL_COPY}"
    );
}
