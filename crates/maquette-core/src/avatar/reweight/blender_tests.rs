//! Reweighting in a real, headless Blender, then normalising the approved
//! result. Ignored by default, so CI never needs Blender: run with
//! `--ignored`. `MAQUETTE_AVATAR_PROBE` names a rigged `.glb` to reweight
//! (the box humanoid with a strip of cloth on its arm otherwise), and
//! `MAQUETTE_AVATAR_OUT` a folder to copy the outputs into for a look.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::avatar::normalize::tests::world_with;
use crate::avatar::normalize::{self, Elbows, Fingers, NormalizeRequest};
use crate::inbox::fixture::{self, HumanoidSpec};
use crate::test_blender;

#[test]
#[ignore = "runs Blender; MAQUETTE_AVATAR_PROBE picks the model"]
fn reweight_then_normalise_a_model_in_blender() {
    let Some(blender) = test_blender::blender() else {
        return;
    };
    let model = match std::env::var_os("MAQUETTE_AVATAR_PROBE") {
        Some(path) => fs::read(PathBuf::from(path)).expect("read the probe model"),
        None => fixture::humanoid(&HumanoidSpec {
            arm_drop: 55.0,
            cape_strip: true,
            animation: false,
            ..HumanoidSpec::default()
        }),
    };
    let w = world_with(&model);
    let none = BoneOverrides::new();
    let planned = plan(&ReweightRequest {
        workspace_root: &w.root,
        config: &w.config,
        project: &w.project,
        asset: &w.asset,
        version: None,
        overrides: &none,
    })
    .expect("plan");
    let runner = test_blender::runner();
    let tools = test_blender::toolbox(&blender);
    let found = run(&runner, &tools, &planned).expect("reweight");
    println!(
        "{}",
        serde_json::to_string_pretty(&found.report).expect("json")
    );
    assert_eq!(found.state, NormalizeState::Pending);
    assert!(found.report.moved > 0, "{:#?}", found.report);
    assert_eq!(found.report.weights.unweighted, 0);
    assert!(found.report.weights.max_influences <= 4);
    for sheet in [&found.sheets.before, &found.sheets.after] {
        assert!(fs::metadata(sheet).is_ok_and(|m| m.len() > 1000), "{sheet}");
    }
    // Only the weights change: the reweighted model reads the same bones.
    let after = vrm::readiness(&fs::read(&found.glb).expect("glb"), &planned.dir, &none)
        .expect("readiness");
    assert_eq!(after.bones, planned.readiness.bones);

    approve(&planned.dir).expect("approve");
    let lifted = normalize::plan(&NormalizeRequest {
        workspace_root: &w.root,
        config: &w.config,
        project: &w.project,
        asset: &w.asset,
        version: None,
        overrides: &none,
        elbows: Elbows::Straighten,
        fingers: Fingers::Straighten,
    })
    .expect("normalise plan");
    assert_eq!(lifted.source.display().to_string(), found.glb);
    let normalised = normalize::run(&runner, &tools, &lifted).expect("normalise");
    assert_eq!(
        normalised.reweight.as_ref().map(|r| r.moved),
        Some(found.report.moved)
    );

    if let Some(out) = std::env::var_os("MAQUETTE_AVATAR_OUT") {
        let out = PathBuf::from(out);
        fs::create_dir_all(&out).expect("out");
        for name in [OUTPUT_FILE, REPORT_FILE, SHEET_BEFORE, SHEET_AFTER] {
            fs::copy(planned.dir.join(name), out.join(format!("reweight_{name}"))).expect("copy");
        }
        for name in ["normalized.glb", "sheet_after.png"] {
            fs::copy(lifted.dir.join(name), out.join(format!("normalize_{name}"))).expect("copy");
        }
    }
}
