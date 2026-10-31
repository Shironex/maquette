//! The humanoid route: Mixamo names, one skinned primitive.

use std::path::Path;

use serde_json::Value;

use super::defects::{self, count, num};
use super::run::Run;
use super::{
    DefectKind, JointMove, JointMoveState, MAX_ROUNDS, RigRecord, RigSheet, RigStat, RigState,
    RigStep, clips, steps, verdict,
};
use crate::error::{CoreError, CoreResult};

const ROUTE: &str = "humanoid";
/// The wider seam of the second round, for a model one unit tall.
const ROUND2_BAND: &str = "0.08";

pub(super) fn arg(path: &Path) -> String {
    path.display().to_string()
}

pub(super) fn read_json(path: &Path) -> CoreResult<Value> {
    let text = std::fs::read_to_string(path).map_err(|e| CoreError::io(path, e))?;
    serde_json::from_str(&text).map_err(|e| CoreError::json(path, e))
}

/// Two frames from each of the model's first clips, as rendertest names them.
fn clip_frames(case: &RigRecord) -> Vec<String> {
    let names: Vec<String> = case
        .model
        .clips
        .iter()
        .filter(|c| !c.contains([',', ':']))
        .take(2)
        .map(|c| format!("{c}:0.5"))
        .collect();
    if names.is_empty() {
        Vec::new()
    } else {
        vec!["--clips".to_string(), names.join(",")]
    }
}

struct Render<'a> {
    glbs: Vec<(&'a Path, &'a str)>,
    views: &'a str,
    out: &'a str,
    rows: &'a str,
    xray: bool,
}

fn render(run: &Run, name: &str, job: &Render<'_>) -> CoreResult<f64> {
    let case = run.snapshot();
    let mut args = Vec::new();
    for (glb, _) in &job.glbs {
        args.extend(["--glb".to_string(), arg(glb)]);
    }
    let labels: Vec<&str> = job.glbs.iter().map(|(_, l)| *l).collect();
    let out = run.dir.join(job.out);
    args.extend([
        "--out".to_string(),
        arg(&out),
        "--labels".to_string(),
        labels.join(","),
        "--views".to_string(),
        job.views.to_string(),
        "--rows".to_string(),
        arg(&run.dir.join(job.rows)),
    ]);
    args.extend(clip_frames(&case));
    if job.xray {
        args.extend(["--xray".to_string(), "--bones".to_string()]);
    }
    let poses = 7 + clip_frames(&case).len().min(1) * case.model.clips.len().min(2);
    let total = poses * job.views.split(',').count() * job.glbs.len();
    let cells = run.dir.join(format!("{}_cells", job.out));
    let started = std::time::Instant::now();
    run.watched(
        (name, u32::try_from(total).unwrap_or(u32::MAX)),
        (&cells, ""),
        || run.script(ROUTE, "rendertest.py", &args),
    )?;
    Ok(started.elapsed().as_secs_f64())
}

/// The pose test before: probe, the fix's plan (which chains are off their
/// mesh) and the sheets.
pub(super) fn test(run: &Run) -> CoreResult<()> {
    let input = run.working_copy()?;
    let one_island = run.snapshot().model.one_island;
    let probe_file = run.dir.join("before.probe.json");
    let mut probe = Value::Null;
    run.stage("Probe", || {
        run.script(
            ROUTE,
            "probe.py",
            &[arg(&input), "--json".to_string(), arg(&probe_file)],
        )?;
        probe = read_json(&probe_file)?;
        Ok(format!(
            "{} bones, {} islands",
            num(&probe, "joints"),
            num(&probe, "islands")
        ))
    })?;
    let plan = if one_island {
        None
    } else {
        let plan_out = run.dir.join("plan.glb");
        run.script(
            ROUTE,
            "fixweights.py",
            &[arg(&input), arg(&plan_out), "--report-only".to_string()],
        )?;
        Some(read_json(&run.dir.join("plan.glb.report.json"))?)
    };
    let mut before = RigSheet::default();
    run.stage("Pose test before", || {
        before.seconds = render(
            run,
            "Pose test before",
            &Render {
                glbs: vec![(&input, "before")],
                views: "front,side,back",
                out: "before.png",
                rows: "before.rows.json",
                xray: false,
            },
        )?;
        let xray = render(
            run,
            "X-ray bones",
            &Render {
                glbs: vec![(&input, "before")],
                views: "front",
                out: "before_xray.png",
                rows: "before_xray.rows.json",
                xray: true,
            },
        );
        let x_rows = xray
            .ok()
            .and_then(|_| read_json(&run.dir.join("before_xray.rows.json")).ok());
        let data = read_json(&run.dir.join("before.rows.json"))?;
        before.file = arg(&run.dir.join("before.png"));
        before.rows = defects::rows(&data, "before", x_rows.as_ref());
        Ok(format!("{} rows", before.rows.len()))
    })?;
    let found = defects::humanoid(&probe, plan.as_ref(), one_island);
    verdict::mark_rows(&mut before.rows, &found);
    let far =
        |key: &str| num(&probe, &format!("Left_{key}")) + num(&probe, &format!("Right_{key}"));
    let mut stats = vec![
        stat(
            "Hand weight far from the hand",
            None,
            count(far("hand_joint")),
            far("hand_joint") > 0,
        ),
        stat(
            "Cloth on arm bones",
            None,
            count(far("arm_far")),
            far("arm_far") > 0,
        ),
    ];
    stats.extend(steps::before_sums(&before.rows));
    run.update(|case| {
        case.before = Some(before);
        case.defects = found;
        case.stats = stats;
        case.state = RigState::Tested;
        case.step = RigStep::Before;
    });
    Ok(())
}

pub(super) fn stat(label: &str, before: Option<String>, after: String, bad: bool) -> RigStat {
    RigStat {
        label: label.to_string(),
        before,
        after,
        bad,
    }
}

/// The fix, its checks, the pose test after, and the moved joints' offer.
pub(super) fn fix(run: &Run) -> CoreResult<()> {
    let case = run.snapshot();
    let input = run.dir.join("in.glb");
    let draft = run.dir.join("draft.glb");
    let last_round = case.round >= MAX_ROUNDS;
    run.update(|c| c.after = None);
    let mut report = Value::Null;
    run.stage("Fix", || {
        if case.model.one_island {
            run.script(ROUTE, "narrow.py", &[arg(&input), arg(&draft)])?;
        } else {
            let mut args = vec![arg(&input), arg(&draft)];
            if case.round >= MAX_ROUNDS {
                args.extend(["--band".to_string(), ROUND2_BAND.to_string()]);
            }
            run.script(ROUTE, "fixweights.py", &args)?;
        }
        report = read_json(&run.dir.join("draft.glb.report.json"))?;
        Ok(format!(
            "{} vertices changed",
            count(num(&report, "changed"))
        ))
    })?;
    run.log(
        "fixweights.py",
        format!("{} vertices changed", count(num(&report, "changed"))),
    );
    if num(&report, "changed") == 0 && defects::misplaced(&report).is_none() {
        let v = case.source.version.unwrap_or(1);
        run.nothing_changed(format!(
            "No vertex's weights moved. No draft was made and v{v} stays."
        ));
        return Ok(());
    }
    let verify_file = run.dir.join("verify.json");
    run.update(|c| c.step = RigStep::Verify);
    run.stage("Verify", || {
        run.script(
            ROUTE,
            "verify.py",
            &[arg(&input), arg(&draft), arg(&verify_file)],
        )?;
        Ok("Only joints and weights".to_string())
    })?;
    let verify = read_json(&verify_file)?;
    let checks = steps::checks(&verify);
    let after_probe = run.dir.join("after.probe.json");
    run.script(
        ROUTE,
        "probe.py",
        &[arg(&draft), "--json".to_string(), arg(&after_probe)],
    )?;
    let after = read_json(&after_probe)?;
    let before = read_json(&run.dir.join("before.probe.json"))?;
    let mut sheet = RigSheet::default();
    run.stage("Pose test after", || {
        sheet.seconds = render(
            run,
            "Pose test after",
            &Render {
                glbs: vec![(&input, "before"), (&draft, "after")],
                views: "front,side",
                out: "before_after.png",
                rows: "after.rows.json",
                xray: false,
            },
        )?;
        let data = read_json(&run.dir.join("after.rows.json"))?;
        sheet.file = arg(&run.dir.join("before_after.png"));
        sheet.rows = defects::rows(&data, "after", None);
        Ok(format!("{} rows", sheet.rows.len()))
    })?;
    let before_sheet = case.before.clone().unwrap_or_default();
    let worst = verdict::mark_after(&mut sheet, &before_sheet);
    let sums = steps::sums(&before_sheet.rows, &sheet.rows);
    let joint_move = moved_offer(run, &report, &draft, &case)?;
    let far =
        |v: &Value, key: &str| num(v, &format!("Left_{key}")) + num(v, &format!("Right_{key}"));
    let stats = vec![
        stat(
            "Hand weight far from the hand",
            Some(count(far(&before, "hand_joint"))),
            count(far(&after, "hand_joint")),
            far(&after, "hand_joint") > 0,
        ),
        stat(
            "Cloth on arm bones",
            Some(count(far(&before, "arm_far"))),
            count(far(&after, "arm_far")),
            far(&after, "arm_far") > 0,
        ),
        sums.stretch,
        sums.past,
        sums.gaps,
        stat("Seam band", None, steps::band_text(&case), false),
        stat(
            "New weights",
            None,
            format!("{} v", count(num(&report, "changed"))),
            false,
        ),
    ];
    let fix_steps = steps::humanoid(&case, &report);
    let stop = steps::stop_of(&case.name, &checks, last_round, worst.as_ref());
    let draft_path = arg(&draft);
    run.update(|c| {
        c.after = Some(sheet);
        c.stats = stats;
        c.checks = checks;
        c.fix_steps = fix_steps;
        c.draft = Some(draft_path);
        c.joint_move = joint_move;
        for d in c.defects.iter_mut() {
            match d.kind {
                DefectKind::WeightsOnly => {
                    let key = if d.title.contains("arm bones") {
                        "arm_far"
                    } else {
                        "hand_joint"
                    };
                    let left = far(&after, key);
                    d.fixed = left == 0;
                    d.outcome = Some(if left == 0 {
                        "fixed".to_string()
                    } else {
                        format!("{} v still there", count(left))
                    });
                }
                DefectKind::NeedsYes => {
                    d.outcome = Some("Weights mirrored, joints wait for a yes".to_string());
                }
                DefectKind::Note => {
                    d.fixed = true;
                    d.outcome = Some("narrow reweight ran".to_string());
                }
            }
        }
        c.state = if stop.is_some() {
            RigState::Stopped
        } else {
            RigState::Fixed
        };
        c.step = RigStep::Land;
        c.stop = stop;
    });
    Ok(())
}

/// The joint move's offer: ready and rendered, but asked, never used by
/// default. It names the clips the move would break.
fn moved_offer(
    run: &Run,
    report: &Value,
    draft: &Path,
    case: &RigRecord,
) -> CoreResult<Option<JointMove>> {
    let Some((limb, off_cm, joints, names)) = defects::misplaced(report) else {
        return Ok(None);
    };
    let moved = run.dir.join("draft_moved.glb");
    let report_file = run.dir.join("draft.glb.report.json");
    run.script(
        ROUTE,
        "rigmove.py",
        &[arg(draft), arg(&report_file), arg(&moved)],
    )?;
    let seconds = render(
        run,
        "Joints moved",
        &Render {
            glbs: vec![(&moved, "moved")],
            views: "front,side",
            out: "moved.png",
            rows: "moved.rows.json",
            xray: false,
        },
    )?;
    let data = read_json(&run.dir.join("moved.rows.json"))?;
    let breaks_clips = clips::clips_on_joints(Path::new(&case.input), &names)?;
    Ok(Some(JointMove {
        state: JointMoveState::Asked,
        limb,
        off_cm,
        joints,
        breaks_clips,
        draft: arg(&moved),
        sheet: Some(RigSheet {
            file: arg(&run.dir.join("moved.png")),
            rows: defects::rows(&data, "moved", None),
            seconds,
        }),
    }))
}
