//! The skin, node tree and limb chain findings.

use super::{finding, list, plural};
use crate::vrm::skin::{MAX_INFLUENCES, PerMesh, SkinFacts, WEIGHT_SUM_SLACK};
use crate::vrm::{FindingLevel, VrmFinding};

/// "Body (812), Hair (40) and 2 more meshes".
fn meshes(counts: &PerMesh) -> String {
    let mut named: Vec<String> = counts
        .0
        .iter()
        .take(3)
        .map(|(mesh, n)| format!("{mesh} ({n})"))
        .collect();
    let more = counts.0.len().saturating_sub(3);
    if more > 0 {
        named.push(plural(more, "more mesh", "more meshes"));
    }
    list(&named)
}

fn vertex_finding(
    counts: &PerMesh,
    id: &str,
    level: FindingLevel,
    what: &str,
    then: &str,
) -> Option<VrmFinding> {
    let total = counts.total() as usize;
    (total > 0).then(|| {
        finding(
            id,
            level,
            format!("{} {what}", plural(total, "vertex", "vertices")),
            Some(format!("In {}. {then}", meshes(counts))),
        )
    })
}

/// The skin, node tree and limb chain findings.
pub(in crate::vrm) fn skin_findings(skin: &SkinFacts) -> Vec<VrmFinding> {
    let mut out = Vec::new();
    if !skin.cycle.is_empty() {
        out.push(finding(
            "node-cycle",
            FindingLevel::Blocker,
            "The node tree loops".to_string(),
            Some(format!(
                "{} sit in a loop, each under the next. A runtime cannot place them. Fix the parenting and export again.",
                list(&skin.cycle)
            )),
        ));
    }
    for (name, joints) in &skin.twice {
        out.push(finding(
            "skin-joints-twice",
            FindingLevel::Blocker,
            format!(
                "{name} lists {} twice",
                plural(joints.len(), "joint", "joints")
            ),
            Some(format!(
                "{}. A skin names each joint once. Export the skin again.",
                list(joints)
            )),
        ));
    }
    out.extend(vertex_finding(
        &skin.past_joints,
        "skin-joint-index",
        FindingLevel::Blocker,
        "use a joint the skin does not have",
        "Their joint index runs past the skin's joint list. Export the skin again.",
    ));
    out.extend(vertex_finding(
        &skin.unweighted,
        "skin-unweighted",
        FindingLevel::Blocker,
        "carry no weight",
        "Every weight is zero, so the body leaves them behind as it moves. Weight them to a bone or delete them.",
    ));
    out.extend(vertex_finding(
        &skin.off_sum,
        "skin-weight-sum",
        FindingLevel::Blocker,
        "have weights that do not add up to 1",
        &format!(
            "Off by more than {WEIGHT_SUM_SLACK}, more than a normaliser's rounding. Normalise all vertex groups in Blender."
        ),
    ));
    out.extend(vertex_finding(
        &skin.over_four,
        "skin-influences",
        FindingLevel::Warning,
        &format!("have more than {MAX_INFLUENCES} influences"),
        &format!(
            "VRM runtimes keep {MAX_INFLUENCES} joints per vertex, so these bend a little differently. Limit the total to {MAX_INFLUENCES} in Blender and normalise."
        ),
    ));
    if !skin.short.is_empty() {
        let bones: Vec<String> = skin
            .short
            .iter()
            .map(|(bone, length)| format!("{bone} ({:.1} cm)", length * 100.0))
            .collect();
        out.push(finding(
            "short-bones",
            FindingLevel::Warning,
            format!(
                "{} too short to bend by",
                plural(skin.short.len(), "limb bone is", "limb bones are")
            ),
            Some(format!(
                "{}, each measured to the next joint down the chain. A bone with no length gives a runtime no direction to bend or aim it.",
                list(&bones)
            )),
        ));
    }
    if !skin.sides.is_empty() {
        out.push(finding(
            "side-topology",
            FindingLevel::Warning,
            "Left and right limbs differ".to_string(),
            Some(format!(
                "{} Mirrored poses and animations expect the same bones on both sides.",
                skin.sides.join(" ")
            )),
        ));
    }
    if !skin.unread.is_empty() {
        out.push(finding(
            "skin-unread",
            FindingLevel::Note,
            format!(
                "Skin weights of {} not checked",
                plural(skin.unread.len(), "mesh", "meshes")
            ),
            Some(format!(
                "{}: compressed or packed, so the weights cannot be read here.",
                list(&skin.unread)
            )),
        ));
    }
    out
}
