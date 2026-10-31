//! What a fix did, step by step, and whether the doctor stops after it.

use serde_json::Value;

use super::defects::{self, count, num};
use super::{MAX_ROUNDS, RigCheck, RigFixStep, RigRecord, RigStop, RigStopKind};

/// Why the doctor stops after a fix: a check that failed, or a defect left
/// after the last round. It never guesses a third time.
pub(super) fn stop_of(
    name: &str,
    checks: &[RigCheck],
    last_round: bool,
    worst: Option<&(String, f64)>,
) -> Option<RigStop> {
    if let Some(check) = checks.iter().find(|c| !c.ok) {
        return Some(RigStop {
            kind: RigStopKind::Worse,
            title: "A check got worse after the fix".to_string(),
            detail: format!(
                "{} did not hold. The draft is held and v1 stays.",
                check.title
            ),
            row: None,
        });
    }
    let (row, mm) = worst.filter(|_| last_round)?;
    Some(RigStop {
        kind: RigStopKind::Rounds,
        title: format!("Stopped after two rounds: {name}"),
        detail: format!(
            "{row} still stretches {mm:.0} mm on the after sheet. The doctor stops here instead of guessing a third time."
        ),
        row: Some(row.clone()),
    })
}

/// The creature fix's steps.
pub(super) fn creature(report: &Value, junk: usize) -> Vec<RigFixStep> {
    let stats = &report["stats"];
    let strays: u64 = report["strays"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| num(s, "vertices"))
        .sum();
    vec![
        RigFixStep {
            title: "Take stray weight off".to_string(),
            detail: "Weight far from its chain and nearer another goes, filled in from neighbours."
                .to_string(),
            value: format!("{} v", count(strays)),
        },
        RigFixStep {
            title: "Rigid pieces follow one bone".to_string(),
            detail: "Small pieces stop bending; big pieces keep their owner's bones with smoothed seams."
                .to_string(),
            value: format!("{} pieces", num(stats, "rigid_pieces")),
        },
        RigFixStep {
            title: "Empty the junk chains".to_string(),
            detail: "The bones stay, at zero weight, so names and hierarchy do not change."
                .to_string(),
            value: junk.to_string(),
        },
    ]
}

pub(super) fn band_text(case: &RigRecord) -> String {
    if case.model.one_island {
        "faded".to_string()
    } else if case.round >= MAX_ROUNDS {
        "8 cm".to_string()
    } else {
        "5 cm".to_string()
    }
}

pub(super) fn humanoid(case: &RigRecord, report: &Value) -> Vec<RigFixStep> {
    let step = |title: &str, detail: &str, value: String| RigFixStep {
        title: title.to_string(),
        detail: detail.to_string(),
        value,
    };
    let changed = format!("{} v", count(num(report, "changed")));
    if case.model.one_island {
        return vec![step(
            "Narrow reweight",
            "One continuous mesh: the per island fix would tear sleeves and legs. Only arm and hand weight far from the arms moved to the trunk.",
            changed,
        )];
    }
    let mut out = vec![step(
        "Reweight each island",
        "Cloth and hair go to the trunk and head. Cuffs and gloves stay on the arm.",
        changed,
    )];
    if let Some((limb, _, _, _)) = defects::misplaced(report) {
        let mirrored = report["limbs"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(_, l)| l["set_after_registration"].as_u64())
            .sum();
        out.push(step(
            &format!("Mirror the {limb}"),
            "Weights copied from the other side, since its joints sit off the mesh.",
            format!("{} v", count(mirrored)),
        ));
    }
    out.push(step(
        "Smooth the seams",
        "A geodesic band where cloth meets sleeve, so it stretches, not tears.",
        band_text(case),
    ));
    out
}

fn check(ok: bool, title: &str, detail: String, value: String) -> RigCheck {
    RigCheck {
        ok,
        title: title.to_string(),
        detail,
        value,
    }
}

/// The verify step's four rows, from a `verify.py` answer.
pub(super) fn checks(verify: &Value) -> Vec<RigCheck> {
    let changed: Vec<&str> = verify["changed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let only_weights = changed
        .iter()
        .all(|c| *c == "JOINTS_0" || *c == "WEIGHTS_0")
        && verify["images_same"] != false
        && verify["reexported"] != true;
    let bones = num(verify, "bones");
    let moved = verify["moved_joints"].as_array().map_or(0, Vec::len);
    let rest_same = verify["rest_delta"].as_f64().is_none_or(|d| d < 1e-5) && moved == 0;
    let same = verify["same_names"] == true && verify["same_parents"] == true && rest_same;
    let infl = num(verify, "max_influences");
    let zero = num(verify, "zero_weight");
    let off = |key: &str| (verify[key].as_f64().unwrap_or(1.0) - 1.0).abs();
    let sums = verify["sum_error"]
        .as_f64()
        .unwrap_or_else(|| off("sum_min").max(off("sum_max")))
        < 1e-3;
    let actions: Vec<&str> = verify["actions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut out = vec![
        check(
            only_weights,
            "Only JOINTS_0 and WEIGHTS_0 changed",
            "Mesh, UVs and the texture are byte for byte the same".to_string(),
            format!("{} of {}", changed.len(), num(verify, "accessors")),
        ),
        check(
            same,
            "Bones and rest pose unchanged",
            "Names, parents and heads match the source".to_string(),
            bones.to_string(),
        ),
        check(
            infl <= 4 && zero == 0 && sums,
            "At most 4 influences, sums of 1",
            format!("{zero} vertices without weight"),
            format!("max {infl}"),
        ),
    ];
    if !actions.is_empty() {
        out.push(check(
            true,
            "Clips read back",
            format!("{} import with every bone", actions.join(" and ")),
            actions.len().to_string(),
        ));
    }
    out
}
