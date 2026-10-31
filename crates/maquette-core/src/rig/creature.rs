//! The creature route: its own bone names, chains read from the bone tree.

use serde_json::Value;

use super::defects::{self, count, num};
use super::humanoid::{arg, read_json, stat};
use super::run::Run;
use super::{
    DefectKind, MAX_ROUNDS, RigChain, RigCreature, RigRow, RigSheet, RigState, RigStep, TEAR_MM,
    steps,
};
use crate::error::CoreResult;

const ROUTE: &str = "creature";
/// Files a posetest run writes per chain.
const TILES: usize = 2;

fn chains_of(table: &Value) -> Vec<RigChain> {
    table["chains"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| RigChain {
            chain: c["chain"].as_str().unwrap_or_default().to_string(),
            bones: u32::try_from(num(c, "bones")).unwrap_or(0),
            side: c["side"].as_str().unwrap_or_default().to_string(),
            read_as: c["read_as"].as_str().unwrap_or("chain").to_string(),
            note: c["note"].as_str().map(str::to_string),
        })
        .collect()
}

/// The sheet of a posetest run.
fn sheet(run: &Run, tag: &str) -> CoreResult<RigSheet> {
    let report = read_json(&run.dir.join(format!("{tag}_report.json")))?;
    let rows = report
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(chain, _)| !chain.starts_with("junk"))
        .map(|(chain, r)| {
            let tile = |kind: &str| {
                let file = run.dir.join(format!("{tag}_{chain}_{kind}.png"));
                file.is_file().then(|| arg(&file))
            };
            let rigid = r["rigid"].as_array().map_or(0, Vec::len);
            let stretch = r["stretch_mm"].as_f64().unwrap_or(0.0);
            let bend = r["rigid"][0]["err"].as_f64().unwrap_or(0.0) * 1000.0;
            let note = if stretch >= TEAR_MM {
                Some(format!("tears {stretch:.0} mm"))
            } else if rigid > 0 {
                Some(format!(
                    "{rigid} piece{} bend {bend:.0} mm",
                    if rigid == 1 { "" } else { "s" }
                ))
            } else {
                None
            };
            RigRow {
                id: chain.clone(),
                label: format!("{chain} bent"),
                note,
                defects: Vec::new(),
                stretch_mm: stretch,
                seam_gap_mm: r["seam_gap"].as_f64().unwrap_or(0.0) * 1000.0,
                front: tile("pose"),
                side: tile("rest"),
                back: None,
                xray: None,
            }
        })
        .collect();
    Ok(RigSheet {
        file: arg(&run.dir.join(format!("{tag}_sheet.png"))),
        rows,
        seconds: 0.0,
    })
}

fn posetest(
    run: &Run,
    name: &str,
    glb: &std::path::Path,
    tag: &str,
    chains: u64,
) -> CoreResult<()> {
    let total = usize::try_from(chains).unwrap_or(0) * TILES;
    let args = [arg(glb), arg(&run.dir), tag.to_string()];
    let started = std::time::Instant::now();
    run.watched(
        (name, u32::try_from(total).unwrap_or(u32::MAX)),
        (&run.dir, &format!("{tag}_")),
        || run.script(ROUTE, "posetest.py", &args),
    )?;
    run.log(
        "posetest.py",
        format!("{tag} in {:.0} s", started.elapsed().as_secs_f64()),
    );
    Ok(())
}

/// Reads the chains from the bones, then bends every chain.
pub(super) fn test(run: &Run) -> CoreResult<()> {
    let input = run.working_copy()?;
    let scripts = run.dir.join("scripts").join(ROUTE);
    let mut table = Value::Null;
    run.stage("Chains and weight map", || {
        run.script(ROUTE, "autoconfig.py", &[arg(&input), arg(&scripts)])?;
        table = read_json(&scripts.join("rigconfig.json"))?;
        run.script(
            ROUTE,
            "weightmap.py",
            &[arg(&input), arg(&run.dir.join("map"))],
        )?;
        Ok(format!(
            "{} bones, {} chains",
            num(&table, "bones"),
            num(&table, "chains_to_test")
        ))
    })?;
    let map = ["map_persp.png", "map_side.png", "map_front.png"]
        .iter()
        .map(|f| run.dir.join(f))
        .find(|f| f.is_file())
        .map(|f| arg(&f));
    let creature = RigCreature {
        front: table["front"].as_str().unwrap_or_default().to_string(),
        legs: u32::try_from(num(&table, "legs")).unwrap_or(0),
        swapped: u32::try_from(num(&table, "swapped")).unwrap_or(0),
        junk: defects::names(&table["junk"]),
        rigid_pieces: u32::try_from(num(&table, "rigid_pieces")).unwrap_or(0),
        one_shell: table["one_shell"] == true,
        chains: chains_of(&table),
        weight_map: map,
    };
    run.update(|c| c.creature = Some(creature));
    run.stage("Pose test before", || {
        posetest(
            run,
            "Pose test before",
            &input,
            "before",
            num(&table, "chains_to_test"),
        )?;
        Ok(format!("{} chains", num(&table, "chains_to_test")))
    })?;
    let mut before = sheet(run, "before")?;
    let found = defects::creature(&table, &before);
    for row in &mut before.rows {
        if row.note.is_some() {
            row.defects.push(1);
        }
    }
    let worst = before.rows.iter().map(|r| r.stretch_mm).fold(0.0, f64::max);
    let stats = vec![
        stat(
            "Worst stretch",
            None,
            format!("{worst:.0} mm"),
            worst >= TEAR_MM,
        ),
        stat(
            "Stray vertices",
            None,
            count(
                table["stray"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|s| num(s, "vertices"))
                    .sum(),
            ),
            false,
        ),
        stat(
            "Rigid pieces",
            None,
            count(num(&table, "rigid_pieces")),
            false,
        ),
        stat("Chains", None, count(num(&table, "chains_to_test")), false),
    ];
    run.update(|c| {
        c.before = Some(before);
        c.defects = found;
        c.stats = stats;
        c.state = RigState::Tested;
        c.step = RigStep::Before;
    });
    Ok(())
}

/// The fix (dry run first, as the skill asks), verify and the pose test after.
pub(super) fn fix(run: &Run) -> CoreResult<()> {
    let case = run.snapshot();
    let input = run.dir.join("in.glb");
    let draft = run.dir.join("draft.glb");
    let scripts = run.dir.join("scripts").join(ROUTE);
    let last_round = case.round >= MAX_ROUNDS;
    // Installing the scripts put the example config back: read the chains again.
    run.script(ROUTE, "autoconfig.py", &[arg(&input), arg(&scripts)])?;
    if last_round {
        // The last round: tighter stray cut, larger rigid pieces.
        let file = scripts.join("rigconfig.py");
        let mut text =
            std::fs::read_to_string(&file).map_err(|e| crate::error::CoreError::io(&file, e))?;
        text.push_str("\nSTRAY_DIST = {k: v * 0.75 for k, v in STRAY_DIST.items()}\nRIGID_EXT = RIGID_EXT * 1.3\n");
        std::fs::write(&file, text).map_err(|e| crate::error::CoreError::io(&file, e))?;
    }
    run.update(|c| c.after = None);
    let mut report = Value::Null;
    run.stage("Fix, dry run checked", || {
        run.script(
            ROUTE,
            "fix.py",
            &[arg(&input), arg(&draft), "dry".to_string()],
        )?;
        run.script(ROUTE, "fix.py", &[arg(&input), arg(&draft)])?;
        report = read_json(&run.dir.join("draft.glb.report.json"))?;
        Ok(format!(
            "{} vertices changed",
            count(num(&report, "changed"))
        ))
    })?;
    run.log(
        "fix.py",
        format!(
            "{} vertices changed, joints and weights patched",
            count(num(&report, "changed"))
        ),
    );
    run.update(|c| c.step = RigStep::Verify);
    let verify_file = run.dir.join("verify.json");
    run.stage("Verify", || {
        run.script(
            ROUTE,
            "verify.py",
            &[arg(&input), arg(&draft), arg(&verify_file)],
        )?;
        Ok("Rest pose and bones checked".to_string())
    })?;
    let verify = read_json(&verify_file)?;
    let checks = steps::checks(&verify);
    run.log(
        "verify.py",
        format!(
            "{} accessors changed, max {}",
            verify["changed"].as_array().map_or(0, Vec::len),
            num(&verify, "max_influences")
        ),
    );
    let table = read_json(&scripts.join("rigconfig.json"))?;
    run.stage("Pose test after", || {
        posetest(
            run,
            "Pose test after",
            &draft,
            "after",
            num(&table, "chains_to_test"),
        )?;
        Ok(format!("{} chains", num(&table, "chains_to_test")))
    })?;
    let mut after = sheet(run, "after")?;
    let both = run.dir.join("before_after.png");
    run.script(
        ROUTE,
        "sheet.py",
        &[
            arg(&both),
            arg(&run.dir.join("before_sheet.png")),
            arg(&run.dir.join("after_sheet.png")),
        ],
    )?;
    after.file = arg(&both);
    let before = case.before.clone().unwrap_or_default();
    let worst = defects::mark_after(&mut after, &before);
    let worst_before = before.rows.iter().map(|r| r.stretch_mm).fold(0.0, f64::max);
    let worst_after = after.rows.iter().map(|r| r.stretch_mm).fold(0.0, f64::max);
    let stats_of = |rows: &[RigRow]| {
        rows.iter()
            .filter(|r| {
                r.note
                    .as_deref()
                    .is_some_and(|n| n.contains("bend") || n.contains("tears"))
            })
            .count() as u64
    };
    let stats = vec![
        stat(
            "Worst stretch",
            Some(format!("{worst_before:.0} mm")),
            format!("{worst_after:.0} mm"),
            worst_after >= TEAR_MM,
        ),
        stat(
            "Chains flagged",
            Some(count(stats_of(&before.rows))),
            count(worst.as_ref().map_or(0, |_| 1)),
            worst.is_some(),
        ),
        stat(
            "New weights",
            None,
            format!("{} v", count(num(&report, "changed"))),
            false,
        ),
    ];
    let fix_steps = steps::creature(&report, defects::names(&table["junk"]).len());
    let stop = steps::stop_of(&case.name, &checks, last_round, worst.as_ref());
    let draft_path = arg(&draft);
    run.update(|c| {
        c.after = Some(after);
        c.stats = stats;
        c.checks = checks;
        c.fix_steps = fix_steps;
        c.draft = Some(draft_path);
        for d in c
            .defects
            .iter_mut()
            .filter(|d| d.kind == DefectKind::WeightsOnly)
        {
            d.fixed = true;
            d.outcome = Some("fixed".to_string());
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
