//! The validation report written beside every `.vrm` a build writes, as
//! `exports/<asset>.vrm.report.json`: the source version and its hash,
//! when and how it was written, the readiness before and after with every
//! finding, what the file still lacks, the bone map with where each bone
//! came from, the meta, the writer and the round trip (the written bytes
//! read back and checked as a runtime would).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::avatar::normalize::{Elbows, Fingers};
use crate::avatar::reweight::{FINDING, ReweightReport};
use crate::inbox::vrm::{
    self, AvatarPermission, BoneNaming, BoneOverrides, CommercialUsage, CreditNotation,
    FindingLevel, MaterialMode, Modification, RoundTrip, VrmBone, VrmFinding, VrmMeta,
    VrmReadiness,
};

/// What wrote the file: the app and its version.
pub const WRITER: &str = concat!("Maquette ", env!("CARGO_PKG_VERSION"));

/// `exports/<asset>.vrm.report.json`, beside the `.vrm`.
pub fn report_file(vrm: &Path) -> PathBuf {
    let name = vrm
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    vrm.with_file_name(format!("{name}.report.json"))
}

/// The model version the file was written from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSource {
    pub version: u32,
    /// The version's model file, an absolute path.
    pub file: String,
    /// The SHA-256 of that file, as hex.
    pub sha256: String,
}

/// A readiness, short: the counts and every finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportReadiness {
    pub ready: bool,
    /// Bones mapped, of the 55.
    pub mapped: u32,
    /// Required bones mapped, of the 15.
    pub required: u32,
    /// Blockers first, then warnings, then notes.
    pub findings: Vec<VrmFinding>,
}

impl From<&VrmReadiness> for ReportReadiness {
    fn from(r: &VrmReadiness) -> Self {
        ReportReadiness {
            ready: r.ready,
            mapped: r.mapped,
            required: r.required,
            findings: r.findings.clone(),
        }
    }
}

/// Where a bone's joint came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoneSource {
    /// Matched by the naming most of the rig's joints use.
    Convention,
    /// Matched by another naming, or guessed from a descriptive name.
    Auto,
    /// Set by hand in the avatar spec.
    Override,
}

/// One humanoid bone in the written file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportBone {
    pub bone: String,
    pub required: bool,
    /// The joint (node name) it maps to; `None` for a bone left out.
    pub joint: Option<String>,
    pub source: Option<BoneSource>,
    /// The naming the joint matched by.
    pub naming: Option<BoneNaming>,
}

/// The meta, short: who made it and what others may do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaSummary {
    pub name: String,
    pub version: Option<String>,
    pub authors: Vec<String>,
    pub license_url: String,
    pub avatar_permission: AvatarPermission,
    pub commercial_usage: CommercialUsage,
    pub credit_notation: CreditNotation,
    pub allow_redistribution: bool,
    pub modification: Modification,
}

impl From<&VrmMeta> for MetaSummary {
    fn from(m: &VrmMeta) -> Self {
        MetaSummary {
            name: m.name.clone(),
            version: m.version.clone(),
            authors: m.authors.clone(),
            license_url: m.license_url.clone(),
            avatar_permission: m.avatar_permission,
            commercial_usage: m.commercial_usage,
            credit_notation: m.credit_notation,
            allow_redistribution: m.allow_redistribution,
            modification: m.modification,
        }
    }
}

/// The report beside a written `.vrm`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarReport {
    /// The `.vrm`, an absolute path.
    pub file: String,
    pub written_at: String,
    pub writer: String,
    pub source: ReportSource,
    pub material: MaterialMode,
    /// Written from a normalised rest pose (else from the model as it is).
    pub normalized: bool,
    /// The elbows mode of the normalise; `None` when nothing was normalised.
    pub elbows: Option<Elbows>,
    /// The fingers mode of the normalise; `None` when nothing was
    /// normalised, or for a report older than the option.
    #[serde(default)]
    pub fingers: Option<Fingers>,
    /// What the approved reweight the arms were lifted from changed: the
    /// vertices that gave arm weight to the trunk, from which arm bones and
    /// to which trunk bones, and those kept as sleeve. `None` when no
    /// reweight was applied.
    #[serde(default)]
    pub reweight: Option<ReweightReport>,
    /// The source version's readiness.
    pub before: ReportReadiness,
    /// The written file's readiness; `None` when it does not read back.
    pub after: Option<ReportReadiness>,
    /// What the file still lacks, blockers first.
    pub unresolved: Vec<VrmFinding>,
    pub bones: Vec<ReportBone>,
    pub meta: MetaSummary,
    pub round_trip: RoundTrip,
}

/// Where `bone`'s joint came from, against the rig's main naming.
fn bone_source(bone: &VrmBone, convention: Option<BoneNaming>) -> Option<BoneSource> {
    let naming = bone.naming?;
    Some(if naming == BoneNaming::Override {
        BoneSource::Override
    } else if Some(naming) == convention {
        BoneSource::Convention
    } else {
        BoneSource::Auto
    })
}

/// The bone map of `readiness`, each bone with its source.
pub fn bone_map(readiness: &VrmReadiness) -> Vec<ReportBone> {
    readiness
        .bones
        .iter()
        .map(|b| ReportBone {
            bone: b.bone.clone(),
            required: b.required,
            joint: b.joint.clone(),
            source: bone_source(b, readiness.naming.convention),
            naming: b.naming,
        })
        .collect()
}

/// The blocker a failed round trip adds to what the file still lacks.
pub fn round_trip_finding(round_trip: &RoundTrip) -> Option<VrmFinding> {
    (!round_trip.passed).then(|| VrmFinding {
        id: "round-trip".to_string(),
        level: FindingLevel::Blocker,
        title: "The written file does not read back as a VRM".to_string(),
        detail: Some(round_trip.problems.join(" ")),
    })
}

/// The written file's readiness; `None` when it does not read back (the
/// round trip says why). After an approved reweight, what still follows
/// the arms is what it left, and `unresolved` says so too.
pub fn written_readiness(
    bytes: &[u8],
    base: &Path,
    overrides: &BoneOverrides,
    reweighted: bool,
    unresolved: &mut Vec<VrmFinding>,
) -> Option<VrmReadiness> {
    if !reweighted {
        return vrm::readiness(bytes, base, overrides).ok();
    }
    let after = vrm::readiness_after_reweight(bytes, base, overrides).ok()?;
    freed_arms(unresolved, &after);
    Some(after)
}

/// Puts what an approved reweight left on the arms in place of the arm
/// weights finding the writer read without knowing of it: `after` is the
/// written file's readiness read as reweighted. Drops the finding when
/// nothing is left.
fn freed_arms(unresolved: &mut Vec<VrmFinding>, after: &VrmReadiness) {
    let left = after.findings.iter().find(|f| f.id == FINDING);
    match (unresolved.iter().position(|f| f.id == FINDING), left) {
        (Some(at), Some(left)) => unresolved[at] = left.clone(),
        (Some(at), None) => {
            unresolved.remove(at);
        }
        (None, _) => return,
    }
    // Stable, so each level keeps its order.
    unresolved.sort_by_key(|f| f.level);
}
