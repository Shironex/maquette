//! A part bound rigidly to one body bone: a wing, a cape or a braid that an
//! auto rig weighted to the arms follows the torso instead. The parts are
//! what the arm chains hold away from the arm, read on the pose test; the
//! bind runs on a copy of the draft and is measured against it. Anyone may
//! ask for it, a person or an agent; only a person uses the result, through
//! [`RigDoctors::decide_rigid`], which no agent tool reaches, and the same
//! call puts the draft before it back.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::defects::{self, count, num};
use super::humanoid::{Render, arg, read_json, render};
use super::run::Run;
use super::{
    RigCheck, RigDoctors, RigRecord, RigRoute, RigRun, RigSheet, RigStarter, RigState, RigStep,
    steps, verdict,
};
use crate::error::{CoreError, CoreResult};
use crate::inbox::analyse::bare_joint_name;
use crate::pipeline::tools::Toolbox;
use crate::workspace::WorkspaceConfig;

/// A patch of surface an arm chain holds away from the arm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigPart {
    /// Its number, largest first from 1.
    pub part: u32,
    pub vertices: u32,
    /// left, right or both.
    pub side: String,
    /// The bones holding most of it, without the rig's prefix.
    pub bones: Vec<String>,
    /// Its longest side, cm at a one metre tall model.
    pub across_cm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RigidState {
    /// The bind and its pose test run.
    Making,
    /// Measured; waits for a person. No agent can answer it.
    Asked,
    /// The draft is the bound one.
    Approved,
    /// Turned down, or undone: the draft is the fix's.
    Declined,
}

/// A bind of parts to one bone, and what it did to the poses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RigRigid {
    pub parts: Vec<u32>,
    /// The body bone they follow, its whole name.
    pub bone: String,
    pub state: RigidState,
    /// Who asked for it.
    pub asked_by: RigStarter,
    /// The vertices that follow the bone alone.
    pub vertices: u32,
    /// The draft with the parts bound.
    pub file: Option<String>,
    /// The fix's draft it was made from, which an undo puts back.
    pub earlier: Option<String>,
    /// The bound draft against the version, each row's change against the
    /// fix's draft.
    pub sheet: Option<RigSheet>,
    pub checks: Vec<RigCheck>,
    /// The pose the fix's draft stretches most in, and its worst stretch
    /// there before and after the bind, mm at a one metre tall model.
    pub row: Option<String>,
    pub stretch_before: Option<f64>,
    pub stretch_after: Option<f64>,
}

/// The bone a part follows unless a person picks another.
pub const RIGID_BONE: &str = "Spine2";

fn refuse<T>(why: &str) -> CoreResult<T> {
    Err(CoreError::Invalid(why.to_string()))
}

/// Why a bind of `parts` to `bone` cannot start on `case`, or `None`.
pub(super) fn cannot_bind(case: &RigRecord, parts: &[u32], bone: &str) -> Option<String> {
    let why = if case.route != Some(RigRoute::Humanoid) {
        "A creature's rigid pieces already follow one bone each."
    } else if case.state != RigState::Fixed || case.draft.is_none() {
        "Run the fix first: the parts are bound on its draft."
    } else if case
        .rigid
        .as_ref()
        .is_some_and(|r| r.state == RigidState::Approved)
    {
        "Undo the bound parts first: a new bind starts from the fix's draft."
    } else if parts.is_empty() {
        "Pick at least one part."
    } else if let Some(p) = parts
        .iter()
        .find(|p| !case.parts.iter().any(|x| x.part == **p))
    {
        return Some(format!("This model has no part {p}."));
    } else if !case.body_bones.iter().any(|b| b == bone) {
        return Some(format!("{bone} is not a body bone of this model."));
    } else {
        return None;
    };
    Some(why.to_string())
}

/// The body bone a bind uses when none is named: Spine2, else the first.
pub fn default_bone(case: &RigRecord) -> Option<String> {
    case.body_bones
        .iter()
        .find(|b| b.ends_with(RIGID_BONE))
        .or_else(|| case.body_bones.first())
        .cloned()
}

impl RigDoctors {
    /// Starts binding `parts` rigidly to `bone` on a copy of the draft, with
    /// its pose test; answers at once. Its result waits for a person.
    ///
    /// # Errors
    ///
    /// Fails for an unknown case, one with no fixed draft, a part or bone
    /// the model lacks, a bind already in the draft, and a run going.
    pub fn bind_rigid(
        &self,
        (root, config, tools): (&Path, &WorkspaceConfig, Toolbox),
        id: &str,
        (parts, bone): (&[u32], Option<&str>),
        asked_by: RigStarter,
    ) -> CoreResult<RigRecord> {
        let mut case = self.get(root, config, id)?;
        let bone = match bone {
            Some(name) => case
                .body_bones
                .iter()
                .find(|b| *b == name || bare_joint_name(b) == name)
                .cloned()
                .unwrap_or_else(|| name.to_string()),
            None => default_bone(&case).unwrap_or_default(),
        };
        let mut parts = parts.to_vec();
        parts.sort_unstable();
        parts.dedup();
        if let Some(why) = cannot_bind(&case, &parts, &bone) {
            return refuse(&why);
        }
        if self.active().contains_key(id) {
            return refuse("This rig doctor case is already running.");
        }
        case.rigid = Some(RigRigid {
            parts,
            bone,
            state: RigidState::Making,
            asked_by,
            vertices: 0,
            file: None,
            earlier: case.draft.clone(),
            sheet: None,
            checks: Vec::new(),
            row: None,
            stretch_before: None,
            stretch_after: None,
        });
        self.publish(&mut case)?;
        self.run(root, config, tools, id, RigRun::Rigid)
    }

    /// A person's answer to a bind: yes makes the bound file the draft, no
    /// keeps the fix's; no after a yes is the undo, and puts the fix's
    /// draft back. There is no agent path to it.
    ///
    /// # Errors
    ///
    /// Fails for an unknown case, one with no bind to answer, and a yes to
    /// a bind whose checks did not hold.
    pub fn decide_rigid(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        id: &str,
        approve: bool,
    ) -> CoreResult<RigRecord> {
        let mut case = self.get(root, config, id)?;
        if case.state != RigState::Fixed {
            return refuse("The bind can be answered once the case is fixed and not running.");
        }
        let Some(rigid) = case.rigid.as_mut() else {
            return refuse("No bind waits for an answer.");
        };
        match (rigid.state, approve) {
            (RigidState::Asked, true) => {
                if rigid.checks.iter().any(|c| !c.ok) {
                    return refuse("A check of the bound draft did not hold.");
                }
                let Some(file) = rigid.file.clone() else {
                    return refuse("The bound draft is missing.");
                };
                rigid.earlier.clone_from(&case.draft);
                rigid.state = RigidState::Approved;
                case.draft = Some(file);
            }
            (RigidState::Asked, false) => rigid.state = RigidState::Declined,
            (RigidState::Approved, false) => {
                rigid.state = RigidState::Declined;
                case.draft.clone_from(&rigid.earlier);
            }
            _ => return refuse("No bind waits for that answer."),
        }
        case.step = RigStep::Land;
        self.publish(&mut case)?;
        Ok(case)
    }
}

/// The parts and body bones `rigid.py --parts` read, into the case.
pub(super) fn read_parts(run: &Run, input: &Path) -> CoreResult<()> {
    let file = run.dir.join("parts.json");
    run.script(
        "humanoid",
        "rigid.py",
        &[arg(input), "--parts".to_string(), arg(&file)],
    )?;
    let found = read_json(&file)?;
    let parts: Vec<RigPart> = found["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| RigPart {
            part: u32::try_from(num(p, "part")).unwrap_or(0),
            vertices: u32::try_from(num(p, "vertices")).unwrap_or(0),
            side: p["side"].as_str().unwrap_or_default().to_string(),
            bones: defects::names(&p["bones"]),
            across_cm: p["across_cm"].as_f64().unwrap_or(0.0),
        })
        .collect();
    let bones = defects::names(&found["bones"]);
    run.update(|c| {
        c.parts = parts;
        c.body_bones = bones;
    });
    Ok(())
}

/// The bind run: the parts bound on a copy of the draft, verified and
/// tested. The case goes back to where it stood whatever happens; a bind
/// that fails says so and leaves the draft alone.
pub(super) fn bind(run: &Run) -> CoreResult<()> {
    let outcome = made(run);
    let (state, stop) = (run.earlier().state, run.earlier().stop.clone());
    run.update(|c| {
        c.state = state;
        c.stop = stop;
        c.step = RigStep::Land;
        match outcome {
            Ok(done) => c.rigid = Some(done),
            Err(error) => {
                c.rigid = None;
                c.error = Some(format!("The rigid bind did not finish. {error}"));
            }
        }
    });
    Ok(())
}

fn made(run: &Run) -> CoreResult<RigRigid> {
    let case = run.snapshot();
    let mut ask = case
        .rigid
        .clone()
        .ok_or_else(|| CoreError::Invalid("No bind was asked.".to_string()))?;
    let input = run.dir.join("in.glb");
    let base = ask
        .earlier
        .clone()
        .or_else(|| case.draft.clone())
        .map(PathBuf::from)
        .ok_or_else(|| CoreError::Invalid("There is no draft to bind on.".to_string()))?;
    let out = run.dir.join("rigid.glb");
    let parts: Vec<String> = ask.parts.iter().map(u32::to_string).collect();
    let mut report = Value::Null;
    run.stage("Bind parts", || {
        run.script(
            "humanoid",
            "rigid.py",
            &[
                arg(&input),
                arg(&base),
                arg(&out),
                "--part".to_string(),
                parts.join(","),
                "--bone".to_string(),
                ask.bone.clone(),
            ],
        )?;
        report = read_json(&run.dir.join("rigid.glb.report.json"))?;
        Ok(format!(
            "{} vertices follow {}",
            count(num(&report, "vertices")),
            bare_joint_name(&ask.bone)
        ))
    })?;
    let verify_file = run.dir.join("verify_rigid.json");
    run.script(
        "humanoid",
        "verify.py",
        &[arg(&input), arg(&out), arg(&verify_file)],
    )?;
    ask.checks = steps::checks(&read_json(&verify_file)?);
    let mut sheet = RigSheet::default();
    run.stage("Pose test bound", || {
        sheet.seconds = render(
            run,
            "Pose test bound",
            &Render {
                glbs: vec![(&input, "before"), (&out, "after")],
                views: "front,side",
                out: "rigid.png",
                rows: "rigid.rows.json",
                xray: false,
            },
        )?;
        let data = read_json(&run.dir.join("rigid.rows.json"))?;
        sheet.file = arg(&run.dir.join("rigid.png"));
        sheet.rows = defects::rows(&data, "after", None);
        Ok(format!("{} rows", sheet.rows.len()))
    })?;
    let draft = case.after.clone().unwrap_or_default();
    verdict::mark_after(&mut sheet, &draft);
    if let Some(worst) = draft
        .rows
        .iter()
        .max_by(|a, b| a.stretch_mm.total_cmp(&b.stretch_mm))
    {
        ask.row = Some(worst.label.clone());
        ask.stretch_before = Some(worst.stretch_mm);
        ask.stretch_after = sheet
            .rows
            .iter()
            .find(|r| r.id == worst.id)
            .map(|r| r.stretch_mm);
    }
    ask.vertices = u32::try_from(num(&report, "vertices")).unwrap_or(0);
    ask.file = Some(arg(&out));
    ask.sheet = Some(sheet);
    ask.state = RigidState::Asked;
    Ok(ask)
}
