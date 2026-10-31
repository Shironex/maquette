//! The creature route: its own bone names, chains read from the bone tree.

use serde_json::Value;

use super::defects::{self, count, num};
use super::humanoid::{arg, read_json, stat};
use super::run::Run;
use super::{
    MAX_ROUNDS, RigChain, RigCreature, RigRow, RigSheet, RigState, RigStep, chains, steps, verdict,
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
            key: c["key"].as_str().unwrap_or_default().to_string(),
            bones: u32::try_from(num(c, "bones")).unwrap_or(0),
            side: c["side"].as_str().unwrap_or_default().to_string(),
            read_as: c["read_as"].as_str().unwrap_or("chain").to_string(),
            note: c["note"].as_str().map(str::to_string),
        })
        .collect()
}

/// The most rigid pieces any one chain's bend deforms in a posetest run.
fn bent_pieces(run: &Run, tag: &str) -> CoreResult<usize> {
    let report = read_json(&run.dir.join(format!("{tag}_report.json")))?;
    Ok(report
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(chain, _)| !chain.starts_with("junk"))
        .map(|(_, r)| r["rigid"].as_array().map_or(0, Vec::len))
        .max()
        .unwrap_or(0))
}

/// The sheet of a posetest run. Its seconds are the caller's to set.
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
            let bend = r["rigid"][0]["bend_mm"].as_f64().unwrap_or(0.0);
            let mut row = RigRow {
                id: chain.clone(),
                label: format!("{chain} bent"),
                stretch_mm: stretch,
                beyond_body_mm: r["beyond_body_mm"].as_f64(),
                seam_gap_mm: r["seam_gap_mm"].as_f64().unwrap_or(0.0),
                front: tile("pose"),
                side: tile("rest"),
                ..RigRow::default()
            };
            row.note = verdict::problem(&row).or_else(|| {
                (rigid > 0).then(|| {
                    format!(
                        "{rigid} piece{} bend {bend:.0} mm",
                        if rigid == 1 { "" } else { "s" }
                    )
                })
            });
            row
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
) -> CoreResult<f64> {
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
    Ok(started.elapsed().as_secs_f64())
}

/// Reads the chains from the bones, with a person's corrections on top.
fn read_chains(run: &Run, input: &std::path::Path, scripts: &std::path::Path) -> CoreResult<()> {
    let mut args = vec![arg(input), arg(scripts)];
    let edits = run.snapshot().chain_edits;
    if !edits.is_empty() {
        let file = run.dir.join("chain_edits.json");
        crate::json::write_json(&file, &chains::by_first_bone(&edits))?;
        args.push(arg(&file));
    }
    run.script(ROUTE, "autoconfig.py", &args)
}

/// Reads the chains from the bones, then bends every chain.
pub(super) fn test(run: &Run) -> CoreResult<()> {
    let input = run.working_copy()?;
    let scripts = run.dir.join("scripts").join(ROUTE);
    let mut table = Value::Null;
    run.stage("Chains and weight map", || {
        read_chains(run, &input, &scripts)?;
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
    let mut seconds = 0.0;
    run.stage("Pose test before", || {
        seconds = posetest(
            run,
            "Pose test before",
            &input,
            "before",
            num(&table, "chains_to_test"),
        )?;
        Ok(format!("{} chains", num(&table, "chains_to_test")))
    })?;
    let mut before = sheet(run, "before")?;
    before.seconds = seconds;
    let found = defects::creature(&table, bent_pieces(run, "before")?);
    let rigid = found.iter().find(|d| d.title.starts_with("Rigid"));
    for row in &mut before.rows {
        if let Some(d) = rigid.filter(|_| row.note.as_deref().is_some_and(|n| n.contains("bend"))) {
            row.defects.push(d.number);
        }
    }
    let mut stats = steps::before_sums(&before.rows);
    stats.extend([
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
    ]);
    run.update(|c| {
        c.before = Some(before);
        c.defects = found;
        c.stats = stats;
        c.state = RigState::Tested;
        c.step = RigStep::Before;
    });
    Ok(())
}

fn table_one_shell(scripts: &std::path::Path) -> CoreResult<bool> {
    Ok(read_json(&scripts.join("rigconfig.json"))?["one_shell"] == true)
}

/// The fix (dry run first, as the skill asks), verify and the pose test after.
pub(super) fn fix(run: &Run) -> CoreResult<()> {
    let case = run.snapshot();
    let input = run.dir.join("in.glb");
    let draft = run.dir.join("draft.glb");
    let scripts = run.dir.join("scripts").join(ROUTE);
    // The round counts once it made a draft: while it runs, and if it fails or moves nothing, the case stays as it was.
    let round = case.round + 1;
    let last_round = round >= MAX_ROUNDS;
    // Installing the scripts put the example config back: read the chains again.
    read_chains(run, &input, &scripts)?;
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
    if num(&report, "changed") == 0 {
        let v = case.source.version.unwrap_or(1);
        let why = if table_one_shell(&scripts)? {
            "This is one continuous shell, so the per piece rules stay off and only stray weight can move, and none did."
        } else {
            "No weight on any vertex moved by more than 0.02."
        };
        run.nothing_changed(format!(
            "{why} No draft was made and v{v} stays. If the chains read wrong, correct them and run it again."
        ));
        return Ok(());
    }
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
    let mut seconds = 0.0;
    run.stage("Pose test after", || {
        seconds = posetest(
            run,
            "Pose test after",
            &draft,
            "after",
            num(&table, "chains_to_test"),
        )?;
        Ok(format!("{} chains", num(&table, "chains_to_test")))
    })?;
    let mut after = sheet(run, "after")?;
    after.seconds = seconds;
    let still_bent = bent_pieces(run, "after")?;
    // The same words judge both sheets, read before the after rows say how they moved.
    let flagged_after = after.rows.iter().filter(|r| r.note.is_some()).count();
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
    let worst = verdict::mark_after(&mut after, &before);
    let sums = steps::sums(&before.rows, &after.rows);
    let flagged_before = before.rows.iter().filter(|r| r.note.is_some()).count();
    let stats = vec![
        sums.stretch,
        sums.past,
        sums.gaps,
        stat(
            "Chains flagged",
            Some(count(flagged_before as u64)),
            count(flagged_after as u64),
            flagged_after > 0,
        ),
        stat(
            "New weights",
            None,
            format!("{} v", count(num(&report, "changed"))),
            false,
        ),
    ];
    let fix_steps = steps::creature(&report);
    let stop = steps::stop_of(&case.name, &checks, last_round, worst.as_ref());
    let draft_path = arg(&draft);
    run.update(|c| {
        c.round = round;
        c.after = Some(after);
        c.stats = stats;
        c.checks = checks;
        c.fix_steps = fix_steps;
        c.draft = Some(draft_path);
        defects::settle_creature(&mut c.defects, &report, still_bent);
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

/// Walks a copy of the draft on the spot and renders it to one strip, so a
/// person sees the rig move. The clip is a test: it never reaches the draft,
/// and a proof that fails leaves the draft as it is.
pub(super) fn walk_proof(run: &Run) -> CoreResult<()> {
    let draft = run.dir.join("draft.glb");
    let clip = run.dir.join("walk_proof.glb");
    let strip = run.dir.join("walk_proof.png");
    let outcome = run.stage("Walk clip proof", || {
        run.script(ROUTE, "walkclip.py", &[arg(&draft), arg(&clip)])?;
        run.script(ROUTE, "clipsheet.py", &[arg(&clip), arg(&strip)])?;
        Ok("In place, 30 fps, on a copy".to_string())
    });
    run.update(|c| {
        c.state = RigState::Fixed;
        c.step = RigStep::Land;
        match outcome {
            Ok(()) => c.walk_proof = Some(arg(&strip)),
            Err(error) => c.error = Some(format!("The walk clip proof did not finish. {error}")),
        }
    });
    Ok(())
}
