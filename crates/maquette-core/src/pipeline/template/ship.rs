//! The Ship step: copies the shipped models and posters into the site and
//! writes the site's scene data (see [`crate::site`]). Without a site repo
//! in the project settings the runner leaves the step out and the run says
//! nothing shipped. It writes outside the pipeline root, so it has no outputs
//! and runs every time.

use std::path::Path;

use super::s;
use crate::pipeline::{PipelineDef, StepDef, StepKind};

pub(super) const SCRIPT: &str = "tools/ship.mjs";

/// The args of the Ship starter before it wrote the site's data.
const LEGACY_ARGS: &[&str] = &[
    "{script}",
    "--models",
    "web/dist",
    "--posters",
    "web/posters",
    "--site",
    "{siteRepo}",
    "--models-to",
    "public/models",
    "--posters-to",
    "public/posters",
];

const ARGS: &[&str] = &[
    "{script}",
    "--models",
    "web/dist",
    "--posters",
    "web/posters",
    "--site",
    "{siteRepo}",
    "--index",
    "{shipIndex}",
    "--layout",
    "{project}/layout.json",
    "--materials",
    "{project}/materials.json",
];

pub(super) fn ship(deps: &[&str]) -> StepDef {
    StepDef {
        id: "ship".into(),
        name: "Ship".into(),
        kind: StepKind::Shell,
        command: Some("node".into()),
        script: Some(SCRIPT.into()),
        args: s(ARGS),
        inputs: s(&[
            "web/dist/*.glb",
            "web/posters/*.png",
            "web/posters/stops.json",
        ]),
        outputs: Vec::new(),
        depends_on: s(deps),
        compress: None,
    }
}

/// Gives an untouched Ship step from before the site data the args of the
/// current starter, once the project's script reads them (the old script
/// refuses the new flags). Steps whose args were changed by hand are left
/// alone. Returns true when `def` changed.
pub fn fit_ship_args(def: &mut PipelineDef, project_dir: &Path) -> bool {
    let root = def.root_dir(project_dir);
    let reads_index =
        std::fs::read_to_string(root.join(SCRIPT)).is_ok_and(|text| text.contains("{shipIndex}"));
    if !reads_index {
        return false;
    }
    let mut changed = false;
    for step in &mut def.steps {
        if step.script.as_deref() == Some(SCRIPT) && step.args == s(LEGACY_ARGS) {
            step.args = s(ARGS);
            changed = true;
        }
    }
    changed
}

/// The placeholder the Export GLB step's `--origin` takes: `scene` or
/// `own`, from the project's ship settings.
pub(super) const ORIGIN_VAR: &str = "{exportOrigin}";
const EXPORT_SCRIPT: &str = "tools/export_web.py";
const EXPORT_ARGS: &[&str] = &["--bake", "blender/bake", "--out", "web/raw"];

/// Gives an untouched Export GLB step the `--origin {exportOrigin}` args
/// once the project's `export_web.py` reads them (the older script refuses
/// the flag). Steps whose args were changed by hand are left alone, so they
/// export as the scene has it. Returns true when `def` changed.
pub fn fit_export_origin(def: &mut PipelineDef, project_dir: &Path) -> bool {
    let root = def.root_dir(project_dir);
    let reads = std::fs::read_to_string(root.join(EXPORT_SCRIPT))
        .is_ok_and(|text| text.contains(crate::ship::EXPORT_ORIGIN_MARKER));
    if !reads {
        return false;
    }
    let mut changed = false;
    for step in &mut def.steps {
        if step.script.as_deref() != Some(EXPORT_SCRIPT)
            || step.args.iter().any(|a| a == "--origin")
        {
            continue;
        }
        let tail = &step.args[step.args.len().saturating_sub(EXPORT_ARGS.len())..];
        if tail == s(EXPORT_ARGS).as_slice() {
            step.args.extend(s(&["--origin", ORIGIN_VAR]));
            changed = true;
        }
    }
    changed
}
