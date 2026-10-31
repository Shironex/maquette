//! Turns the scripts' numbers into what the rig doctor shows: defects named
//! by place, pose rows, fix steps and checks. A person still looks at the
//! sheets; these say where to look.

use serde_json::Value;

use super::{DefectKind, RigDefect, RigRow};

/// 3912 reads "3,912".
pub(super) fn count(n: u64) -> String {
    let text = n.to_string();
    let mut out = String::new();
    for (i, c) in text.chars().enumerate() {
        if i > 0 && (text.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub(super) fn num(value: &Value, key: &str) -> u64 {
    value[key].as_u64().unwrap_or(0)
}

fn bare(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn bone_tags(probe: &Value, pick: impl Fn(&str) -> bool) -> Vec<String> {
    let mut found: Vec<(String, u64)> = probe["bones"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(name, _)| pick(bare(name)))
        .map(|(name, n)| (bare(name).to_string(), n.as_u64().unwrap_or(0)))
        .collect();
    found.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    found
        .into_iter()
        .take(3)
        .map(|(n, c)| format!("{n} {} v", count(c)))
        .collect()
}

/// The chip with the number the tiles show: each vertex once.
fn total(n: u64) -> String {
    format!("{} v in all", count(n))
}

/// Why the bone chips add up to more than the total.
const SHARED: &str =
    "A vertex can count under several bones, so the bones add up to more than the total.";

/// A misplaced limb from a fixweights report: its words, how far off and how
/// many joints.
pub(super) fn misplaced(report: &Value) -> Option<(String, f64, u32, Vec<String>)> {
    let limbs = report["limbs"].as_object()?;
    let (key, limb) = limbs.iter().find(|(_, l)| l["misplaced"] == true)?;
    let joints = limb["registered_joints"].as_object()?;
    let shifts: Vec<f64> = limb["icp_shift_cm"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_f64)
        .collect();
    let off = shifts.iter().copied().fold(0.0, f64::max);
    let words = key
        .replace("Left", "left ")
        .replace("Right", "right ")
        .to_lowercase();
    let words = if words.trim().contains(' ') {
        words.trim().to_string()
    } else {
        format!("{} arm", words.trim())
    };
    Some((
        words,
        off,
        u32::try_from(joints.len()).unwrap_or(0),
        joints.keys().cloned().collect(),
    ))
}

/// The humanoid defects, from the probe and the fix plan's report.
pub(super) fn humanoid(probe: &Value, plan: Option<&Value>, one_island: bool) -> Vec<RigDefect> {
    let mut out = Vec::new();
    let far = |key: &str| num(probe, &format!("Left_{key}")) + num(probe, &format!("Right_{key}"));
    let mut push = |title: String, place: String, tags, kind, detail: String| {
        let number = u32::try_from(out.len()).unwrap_or(0) + 1;
        out.push(RigDefect {
            number,
            title,
            place,
            tags,
            kind,
            detail,
            outcome: None,
            fixed: false,
        });
    };
    let arm = far("arm_far");
    if arm > 0 {
        let hair = num(&probe["places"]["arm"], "hair") >= num(&probe["places"]["arm"], "cloth");
        let what = if hair { "Hair" } else { "Cape or cloth" };
        let mut tags = bone_tags(probe, |n| n.ends_with("Arm"));
        tags.push(total(arm));
        push(
            format!("{what} weighted to the arm bones"),
            if hair { "hair" } else { "cloth, near the arms" }.to_string(),
            tags,
            DefectKind::WeightsOnly,
            format!(
                "In raise and forward it swings out with the arms. Fix: reweight it to the trunk with a {} seam at the sleeve. {SHARED}",
                if one_island { "faded" } else { "5 cm" }
            ),
        );
    }
    let hand = far("hand_joint");
    if hand > 0 {
        let hair = num(&probe["places"]["hand"], "hair") >= num(&probe["places"]["hand"], "cloth");
        let what = if hair { "Hair strands" } else { "Cloth" };
        push(
            format!("{what} on finger bones"),
            if hair {
                "hair"
            } else {
                "cloth, away from the hands"
            }
            .to_string(),
            {
                let mut tags = bone_tags(probe, |n| n.contains("Hand"));
                tags.push(total(hand));
                tags
            },
            DefectKind::WeightsOnly,
            format!(
                "In hand they follow the fingers. Fix: hand them to the head, neck and trunk. {SHARED}"
            ),
        );
    }
    if let Some((limb, off, joints, _)) = plan.and_then(misplaced) {
        push(
            format!("{} chain off its mesh", capitalise(&limb)),
            limb.clone(),
            vec![format!("{off:.0} cm off"), format!("{joints} joints")],
            DefectKind::NeedsYes,
            format!(
                "Weights get mirrored from the other side. Moving the {joints} joints onto the mesh changes the rest pose, so it waits for your yes."
            ),
        );
    }
    if one_island {
        push(
            "One continuous mesh".to_string(),
            "the whole body".to_string(),
            vec!["1 island".to_string()],
            DefectKind::Note,
            "The per island fix would tear sleeves and legs here, so it does not run. A narrow reweight moves only the arm and hand weight far from the arms.".to_string(),
        );
    }
    out
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

fn pose_label(row: &str) -> String {
    row.strip_prefix("clip:")
        .map_or_else(|| row.to_string(), |r| r.replace(':', " "))
}

/// The rows of a `rendertest.py --rows` answer for the file `label`.
pub(super) fn rows(data: &Value, label: &str, xray: Option<&Value>) -> Vec<RigRow> {
    let cell = |data: &Value, row: &str, view: &str| {
        data["cells"][format!("{label}/{row}/{view}")]
            .as_str()
            .map(str::to_string)
    };
    let other = |row: &str, view: &str| {
        data["cells"][format!("before/{row}/{view}")]
            .as_str()
            .filter(|_| label != "before")
            .map(str::to_string)
    };
    data["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|row| {
            let m = &data["metrics"][format!("{label}/{row}")];
            RigRow {
                id: row.to_string(),
                label: pose_label(row),
                beyond_body_mm: m["beyond_body_mm"].as_f64(),
                stretch_mm: m["stretch_mm"].as_f64().unwrap_or(0.0),
                seam_gap_mm: m["seam_gap_mm"].as_f64().unwrap_or(0.0),
                front: cell(data, row, "front"),
                side: cell(data, row, "side"),
                back: cell(data, row, "back"),
                xray: xray.and_then(|x| cell(x, row, "front")),
                // A sheet of two files has the before under the draft's own camera.
                before_front: other(row, "front"),
                before_side: other(row, "side"),
                ..RigRow::default()
            }
        })
        .collect()
}

pub(super) fn names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

/// The creature defects, from the chain table and the before sheet.
pub(super) fn creature(table: &Value, bent: usize) -> Vec<RigDefect> {
    let mut out = Vec::new();
    let mut push = |title: String, place: String, tags: Vec<String>, kind, detail: String| {
        let number = u32::try_from(out.len()).unwrap_or(0) + 1;
        out.push(RigDefect {
            number,
            title,
            place,
            tags,
            kind,
            detail,
            outcome: None,
            fixed: false,
        });
    };
    let stray = table["stray"].as_array().cloned().unwrap_or_default();
    if let Some(worst) = stray
        .iter()
        .filter(|s| s["on"] != "torso")
        .max_by_key(|s| num(s, "vertices"))
    {
        let on = worst["on"].as_str().unwrap_or("a chain");
        let owner = worst["owner"].as_str().unwrap_or("a chain");
        push(
            format!("The {on} pulls {owner}"),
            on.to_string(),
            vec![format!("{} v", count(num(worst, "vertices")))],
            DefectKind::WeightsOnly,
            format!(
                "Vertices of the {on} carry weight of {owner}, far from it. Fix: take that weight off and fill in from neighbours."
            ),
        );
    }
    if bent > 0 {
        push(
            "Rigid pieces split between bones".to_string(),
            "spikes and plates".to_string(),
            vec![
                format!("{bent} of {} pieces", num(table, "rigid_pieces")),
                "one bone each".to_string(),
            ],
            DefectKind::WeightsOnly,
            "They bend when a chain moves. Fix: each small piece follows exactly one bone."
                .to_string(),
        );
    }
    let junk = names(&table["junk"]);
    if !junk.is_empty() {
        push(
            "Junk duplicate chains".to_string(),
            "on other chains".to_string(),
            junk.clone(),
            DefectKind::WeightsOnly,
            "Extra bones on top of a real chain carry weight. Fix: empty them. The bones stay, so names and hierarchy do not change.".to_string(),
        );
    }
    if num(table, "swapped") > 0 {
        push(
            "Side names do not match the sides".to_string(),
            "legs".to_string(),
            vec![format!("{} legs", num(table, "swapped"))],
            DefectKind::Note,
            "The doctor reads sides by position. Bone names stay as they are, so a clip map must say which is which.".to_string(),
        );
    }
    out
}

/// Says of each creature defect what the fix really did: a defect is fixed
/// only when the report or the after sheet shows it gone.
pub(super) fn settle_creature(defects: &mut [RigDefect], report: &Value, still_bent: usize) {
    let strays: u64 = report["strays"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| num(s, "vertices"))
        .sum();
    let junk_left = report["junk_left"].as_array().map_or(0, Vec::len);
    for d in defects
        .iter_mut()
        .filter(|d| d.kind == DefectKind::WeightsOnly)
    {
        let open = if d.title.starts_with("Rigid") {
            (still_bent > 0).then(|| format!("{still_bent} pieces still bend"))
        } else if d.title.starts_with("Junk") {
            (junk_left > 0).then(|| format!("{junk_left} junk bones still carry weight"))
        } else {
            (strays == 0).then(|| "no stray weight was moved".to_string())
        };
        d.fixed = open.is_none();
        d.outcome = Some(open.unwrap_or_else(|| "fixed".to_string()));
    }
}
