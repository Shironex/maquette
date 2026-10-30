use std::cell::{Cell, RefCell};
use std::fs;
use std::path::Path;

use super::*;
use crate::avatar::normalize::tests::{World, tools, world};
use crate::avatar::normalize::{self, Elbows, Fingers, NormalizeRequest};
use crate::inbox::fixture::{self, HumanoidSpec};

fn request<'a>(w: &'a World, overrides: &'a BoneOverrides) -> ReweightRequest<'a> {
    ReweightRequest {
        workspace_root: &w.root,
        config: &w.config,
        project: &w.project,
        asset: &w.asset,
        version: None,
        overrides,
    }
}

/// A script runner standing in for Blender: it writes the outputs the
/// reweight script would, with `output` as the reweighted model.
pub(crate) struct FakeReweighter {
    output: Vec<u8>,
    pub(crate) runs: Cell<u32>,
    job: RefCell<Option<serde_json::Value>>,
    unweighted: u32,
}

impl FakeReweighter {
    pub(crate) fn good() -> Self {
        Self {
            // Any model other than the source's bytes will do.
            output: fixture::humanoid(&HumanoidSpec {
                arm_drop: 60.0,
                yaw: 90.0,
                offset: [0.0, 0.2],
                animation: false,
                ..HumanoidSpec::default()
            }),
            runs: Cell::new(0),
            job: RefCell::new(None),
            unweighted: 0,
        }
    }
}

impl ScriptRunner for FakeReweighter {
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
        fs::write(work.join(OUTPUT_FILE), &self.output).expect("glb");
        fs::write(work.join(SHEET_BEFORE), b"png before").expect("sheet");
        fs::write(work.join(SHEET_AFTER), b"png after").expect("sheet");
        let report = serde_json::json!({
            "vertices": 1000, "armWeighted": 300, "moved": 120, "weightMoved": 80.5,
            "keptAsSleeve": 150, "band": 40,
            "fromBones": { "mixamorig:LeftArm": 100, "mixamorig:LeftForeArm": 20 },
            "toBones": { "mixamorig:Spine2": 120, "mixamorig:LeftShoulder": 12 },
            "weights": { "vertices": 1000, "maxInfluences": 4, "unweighted": self.unweighted, "offSum": 0 },
            "sides": { "left": { "moved": 120, "keptAsSleeve": 80, "limbRadius": [0.02, 0.015] } },
        });
        fs::write(work.join(REPORT_FILE), report.to_string()).expect("report");
        Ok(())
    }
}

#[test]
fn a_run_makes_a_pending_reweight_beside_the_rest_pose() {
    let w = world();
    let none = BoneOverrides::new();
    let plan = plan(&request(&w, &none)).expect("plan");
    let version_dir = w
        .root
        .join(&w.config.layout.cache)
        .join("avatar")
        .join(&w.project)
        .join(&w.asset)
        .join("v1");
    assert_eq!(plan.dir, version_dir.join("reweight"));
    assert!(!plan.cached());

    let source_before = fs::read(&plan.source).expect("source");
    let blender = FakeReweighter::good();
    let found = run(&blender, &tools(), &plan).expect("run");
    assert_eq!(blender.runs.get(), 1);
    assert_eq!(found.state, NormalizeState::Pending);
    assert_eq!(found.source_version, 1);
    assert_eq!(found.glb, plan.dir.join(OUTPUT_FILE).display().to_string());
    assert_eq!(found.report.moved, 120);
    assert_eq!(found.report.kept_as_sleeve, 150);
    assert_eq!(found.report.from_bones["mixamorig:LeftArm"], 100);
    assert_eq!(found.report.sides["left"].limb_radius, [0.02, 0.015]);

    let job = blender.job.borrow().clone().expect("job");
    assert_eq!(job["bones"]["leftUpperArm"], "mixamorig:LeftArm");
    assert_eq!(job["input"], plan.source.display().to_string());
    assert_eq!(job["output"], OUTPUT_FILE);

    // The version is only read.
    assert_eq!(fs::read(&plan.source).expect("source"), source_before);
    assert_eq!(load(&plan.dir).expect("load"), Some(found));

    // Asking again with nothing changed skips Blender.
    assert!(plan.cached());
    run(&blender, &tools(), &plan).expect("again");
    assert_eq!(blender.runs.get(), 1);
}

#[test]
fn normalising_lifts_the_approved_reweight_only() {
    let w = world();
    let none = BoneOverrides::new();
    let normalize_request = NormalizeRequest {
        workspace_root: &w.root,
        config: &w.config,
        project: &w.project,
        asset: &w.asset,
        version: None,
        overrides: &none,
        elbows: Elbows::Keep,
        fingers: Fingers::Keep,
    };
    let plain = normalize::plan(&normalize_request).expect("plan");
    assert!(plain.reweight.is_none());

    let plan = plan(&request(&w, &none)).expect("plan");
    let found = run(&FakeReweighter::good(), &tools(), &plan).expect("run");
    // Pending is not used.
    assert_eq!(normalize::plan(&normalize_request).expect("plan"), plain);

    approve(&plan.dir).expect("approve");
    let lifted = normalize::plan(&normalize_request).expect("plan");
    assert_eq!(lifted.source.display().to_string(), found.glb);
    assert_eq!(lifted.reweight.as_ref().map(|r| r.moved), Some(120));
    assert_ne!(lifted.key, plain.key, "another source is another run");
    assert_eq!(lifted.readiness, plain.readiness, "read on the source");

    // A no puts the source back.
    deny(&plan.dir).expect("deny");
    assert_eq!(normalize::plan(&normalize_request).expect("plan"), plain);

    // A yes holds for the bytes it was given for.
    approve(&plan.dir).expect("approve again");
    fs::write(plan.dir.join(OUTPUT_FILE), b"changed").expect("tamper");
    assert_eq!(normalize::plan(&normalize_request).expect("plan"), plain);
    assert!(
        approve(&plan.dir).is_err(),
        "changed outputs are not a result"
    );
}

#[test]
fn a_reweight_an_older_version_wrote_is_a_miss_and_runs_again() {
    let w = world();
    let none = BoneOverrides::new();
    let plan = plan(&request(&w, &none)).expect("plan");
    let blender = FakeReweighter::good();
    run(&blender, &tools(), &plan).expect("run");
    let file = plan.dir.join(RESULT_FILE);
    let mut old: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&file).expect("result")).expect("json");
    old.as_object_mut().expect("object").remove("report");
    fs::write(&file, old.to_string()).expect("old shape");
    assert_eq!(load(&plan.dir).expect("a miss, not an error"), None);
    let again = run(&blender, &tools(), &plan).expect("runs again");
    assert_eq!(blender.runs.get(), 2);
    assert_eq!(load(&plan.dir).expect("load"), Some(again));
}

#[test]
fn weights_left_out_are_an_error() {
    let w = world();
    let none = BoneOverrides::new();
    let plan = plan(&request(&w, &none)).expect("plan");
    let blender = FakeReweighter {
        unweighted: 3,
        ..FakeReweighter::good()
    };
    let err = run(&blender, &tools(), &plan).expect_err("not whole");
    assert!(
        err.to_string().contains("3 vertices without weight"),
        "{err}"
    );
    assert_eq!(load(&plan.dir).expect("load"), None);
}

#[test]
fn only_cloth_on_the_arms_needs_a_reweight() {
    let read = |spec: &HumanoidSpec| {
        vrm::readiness(
            &fixture::humanoid(spec),
            Path::new("."),
            &BoneOverrides::new(),
        )
        .expect("readiness")
    };
    assert!(!needs_reweight(&read(&HumanoidSpec::default())));
    assert!(needs_reweight(&read(&HumanoidSpec {
        cape: true,
        ..HumanoidSpec::default()
    })));
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
        "copy src/avatar/avatar_reweight.py over {SKILL_COPY}"
    );
}
