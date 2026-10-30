//! Tools that read, change and build a character asset's VRM avatar. A
//! build that frees cloth and hair from the arms, changes the rest pose or
//! adds spring chains waits for a person to approve the before and after
//! sheets of each step in the app; no tool approves them.

mod springs;

use std::collections::BTreeMap;

use maquette_core::avatar::export::{self, AvatarBuild, AvatarPatch};
use maquette_core::avatar::normalize::{Elbows, Fingers, Normalized};
use maquette_core::avatar::reweight::{ReweightMode, Reweighted};
use maquette_core::avatar::springs::SpringsMode;
use maquette_core::project;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::commands::avatar::{self as commands, Target};
use crate::mcp::host::Host;
use crate::mcp::hub::Hub;

use super::materials::find_asset;
use super::{McpTools, Out, WAITING, reason, respond, to_short_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetAvatarArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// The version to read; the newest when left out.
    pub version: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetAvatarArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// Start again from the workspace's avatar defaults, before the rest applies.
    #[serde(default)]
    pub reset: bool,
    /// VRM 1.0 meta fields by name: name, version, authors (a list), copyrightInformation,
    /// contactInformation, references, thirdPartyLicenses, avatarPermission (onlyAuthor,
    /// onlySeparatelyLicensedPerson, everyone), commercialUsage (personalNonProfit,
    /// personalProfit, corporation), creditNotation (required, unnecessary), modification
    /// (prohibited, allowModification, allowModificationRedistribution), allowRedistribution,
    /// allowExcessivelyViolentUsage, allowExcessivelySexualUsage, allowPoliticalOrReligiousUsage,
    /// allowAntisocialOrHateUsage and otherLicenseUrl. Only the fields you give change; null puts
    /// one back to its VRM default.
    pub meta: Option<Map<String, Value>>,
    /// Bone map overrides: a VRM humanoid bone name (leftUpperArm) to the joint it maps to, or
    /// null to leave that bone unmapped.
    #[serde(default)]
    pub bones: BTreeMap<String, Option<String>>,
    /// Bones whose override is dropped, so they map by name again.
    #[serde(default)]
    pub clear_bones: Vec<String>,
    /// "mtoon-soft" (toon shading from the base color, no outline) or "pbr" (the model's
    /// materials as they are).
    pub material: Option<String>,
    /// With mtoon-soft: "soften" (the default) writes a copy of the base color texture with
    /// only strong highlights baked into dark material toned down toward the colour around
    /// them, so they do not read as white blotches under toon shading; "keep" writes the
    /// texture as it is. The model's own texture never changes.
    pub highlights: Option<String>,
    /// When cloth and hair are freed from the arms: "hide" (the default) takes the faces fused
    /// between an arm and the body (a sleeve sewn to the hip), which stretch whatever the
    /// weights when the arm lifts, out of the reweighted model so they are not drawn; "keep"
    /// leaves them. A build reweights again after a change.
    pub bridges: Option<String>,
    /// The springs to confirm, replacing the ones confirmed before: name a candidate from
    /// get_avatar's springs.found by id, and tune joints, preset or the swing settings if you
    /// like. "none" confirms that the VRM carries no springs. An empty list only clears the
    /// picks: when the last export had springs, a build then waits for a pick instead of
    /// writing without them.
    pub springs: Option<springs::SpringsArg>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildAvatarArgs {
    /// The project id.
    pub project: String,
    /// The asset id as list_assets returns it, or the asset's name.
    pub asset: String,
    /// The version to build from; the newest when left out.
    pub version: Option<u32>,
    /// Return the plan (what it reads, whether it normalises, what is cached, where it
    /// writes) and do nothing.
    #[serde(default)]
    pub dry_run: bool,
    /// "straighten" (the default) levels the lower arms too; "keep" leaves bent elbows bent.
    pub elbows: Option<String>,
    /// "auto" (the default) first frees cloth and hair weighted to the arms (a cape, a coat's
    /// skirt, long hair) when the readiness finds them and the arms are lifted, as a step with
    /// its own approval; "off" lifts the arms without it.
    pub reweight: Option<String>,
    /// "straighten" (the default) lays the fingers straight along the hand and the thumb 45
    /// degrees forward in the normalise, as far as the hand's mesh holds; "keep" leaves them.
    pub fingers: Option<String>,
    /// "auto" (the default) adds a chain of joints for each confirmed spring, as a step with
    /// its own approval, and writes them as VRM spring bones; with none confirmed it looks for
    /// the parts that could swing. "off" writes no springs.
    pub springs: Option<String>,
}

/// The asset id an agent names, by id or by name.
fn asset_id(host: &dyn Host, project_id: &str, wanted: &str) -> Result<String, String> {
    let (root, config) = workspace(host)?;
    let manifest = project::load(&root, &config.layout, project_id).map_err(reason)?;
    Ok(find_asset(&manifest, wanted)?.id.clone())
}

fn parse_elbows(word: Option<&str>) -> Result<Elbows, String> {
    word.map_or(Ok(Elbows::default()), |w| {
        serde_json::from_value(json!(w.to_lowercase()))
            .map_err(|_| format!("elbows must be straighten or keep, not {w:?}."))
    })
}

fn parse_fingers(word: Option<&str>) -> Result<Fingers, String> {
    word.map_or(Ok(Fingers::default()), |w| {
        serde_json::from_value(json!(w.to_lowercase()))
            .map_err(|_| format!("fingers must be straighten or keep, not {w:?}."))
    })
}

fn parse_reweight(word: Option<&str>) -> Result<ReweightMode, String> {
    word.map_or(Ok(ReweightMode::default()), |w| {
        serde_json::from_value(json!(w.to_lowercase()))
            .map_err(|_| format!("reweight must be auto or off, not {w:?}."))
    })
}

/// A reweight, short: the sheets to judge and its counts.
fn reweight_summary(found: &Reweighted) -> Value {
    let report = &found.report;
    json!({
        "sourceVersion": found.source_version,
        "state": to_short_value(found.state),
        "sheets": to_short_value(&found.sheets),
        "moved": report.moved,
        "keptAsSleeve": report.kept_as_sleeve,
        "band": report.band,
        "bridges": to_short_value(&report.bridges),
        "bridgeFaces": to_short_value(found.bridges),
        "hiddenFaces": report.hidden_faces,
        "fromBones": to_short_value(&report.from_bones),
        "toBones": to_short_value(&report.to_bones),
    })
}

/// A normalised rest pose, short: the sheets to judge and its numbers.
fn pose_summary(found: &Normalized) -> Value {
    json!({
        "sourceVersion": found.source_version,
        "state": to_short_value(found.state),
        "sheets": to_short_value(&found.sheets),
        "turnedDeg": found.report.turned_deg,
        "elbows": to_short_value(found.report.elbows),
        "fingers": to_short_value(found.report.fingers),
        "straightened": to_short_value(&found.report.straightened),
        "lifted": to_short_value(&found.report.lifted),
        "after": to_short_value(&found.report.after),
        "findingsAfter": to_short_value(&found.readiness.findings),
        "reweight": found.reweight.as_ref().map(|r| json!({
            "moved": r.moved,
            "keptAsSleeve": r.kept_as_sleeve,
        })),
    })
}

/// The answer to a build, and the approval it leaves for a person.
fn build_answer(hub: &Hub, name: &str, built: &AvatarBuild) -> Value {
    match built {
        AvatarBuild::ReweightWaiting { reweighted } => {
            let approval = hub.request_reweight(reweighted, name);
            json!({
                "status": WAITING,
                "step": "reweight",
                "approval": approval,
                "message": "Cloth and hair weighted to the arms were moved to the trunk on a copy, so lifting the arms does not drag them along. The new weights wait for a person to look at the before and after sheets and approve them in Maquette's Avatar view; the rest pose comes after. Call get_avatar or build_avatar again later; nothing is written until then.",
                "reweight": reweight_summary(reweighted),
            })
        }
        AvatarBuild::Waiting { normalized } => {
            let approval = hub.request_rest_pose(normalized, name);
            json!({
                "status": WAITING,
                "step": "rest pose",
                "approval": approval,
                "message": "The new rest pose waits for a person to look at the before and after sheets and approve it in Maquette's Avatar view. Call get_avatar or build_avatar again later; nothing is written until then.",
                "pose": pose_summary(normalized),
            })
        }
        AvatarBuild::Denied { normalized } => json!({
            "status": "denied",
            "message": "A person turned this rest pose down in Maquette. Ask them what to change (for example elbows keep), then build again.",
            "pose": pose_summary(normalized),
        }),
        AvatarBuild::SpringsWaiting { chained, carried } => {
            let approval = hub.request_springs(chained, name);
            let mut answer = json!({
                "status": WAITING,
                "step": "springs",
                "approval": approval,
                "message": "A chain of joints was added along each confirmed spring and the part weighted to it, on a copy. The chains wait for a person to look at the parts and the sway sheet and approve them in Maquette's Avatar view. Call get_avatar or build_avatar again later; nothing is written until then.",
                "springs": springs::chain_summary(chained),
            });
            if !carried.is_empty() {
                answer["carried"] = to_short_value(carried);
                answer["message"] = json!(format!(
                    "The model changed under the confirmed springs, so the finder names their parts differently now; each was carried over to the part on its line (carried: from, to, rootBone) and saved, keeping its kind, joints and preset. {}",
                    answer["message"].as_str().unwrap_or_default()
                ));
            }
            answer
        }
        AvatarBuild::SpringsUnpicked { found } => json!({
            "status": "candidates waiting for a pick",
            "message": "No springs are confirmed, and the last export carried some: the parts that could swing wait for a pick so the file does not quietly lose them. Confirm the ones to keep (set_avatar springs, by candidate id) or confirm none (set_avatar springs \"none\"), then build again. Nothing was written.",
            "springs": springs::candidates_summary(found),
        }),
        AvatarBuild::SpringsStale { found, stale } => json!({
            "status": "springs no longer match",
            "message": format!(
                "The confirmed springs no longer match; pick again. No part the finder sees on this model matches {} clearly enough to carry it over. Pick from the candidates (set_avatar springs) or confirm none, then build again. Nothing was written.",
                stale.join(", ")
            ),
            "stale": stale,
            "springs": springs::candidates_summary(found),
        }),
        AvatarBuild::Written { export, normalized } => json!({
            "status": "written",
            "path": export.path,
            "bytes": export.bytes,
            "sourceVersion": export.source_version,
            "material": to_short_value(export.material),
            "highlights": to_short_value(&export.highlights),
            "unresolved": to_short_value(&export.unresolved),
            "report": export.report,
            "roundTrip": to_short_value(&export.round_trip),
            "springs": export.springs,
            "pose": normalized.as_ref().map(pose_summary),
        }),
    }
}

#[tool_router(router = avatar_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get a character asset's VRM avatar: its VRM readiness (the 55 humanoid bones with the joint each maps to and the naming it matched by, the 15 required, the naming report saying which convention the joints matched, any ambiguous matches and whether the map is confident, the skin's joints to pick from with set_avatar bones, the rest pose measures and the findings, blockers first), the spec (the VRM meta, bone map overrides and material mode; from the workspace's avatar defaults until first saved), whether the rest pose needs normalising, whether cloth or hair weighted to the arms needs freeing first (needsReweight) and the last reweight with its sheets, counts and state, the last normalised rest pose with its before and after sheets and its state (pending, approved or denied), the springs (springs.ready once the model the VRM is written from is: its rest pose is VRM's or its new one is approved; springs.found, the parts that could swing that a build looked for, each with an id such as hair-1, a kind (hair, cloth, accessory), the bone it hangs from, its centre line, length, vertex count, how sure the finder is and a colour on springs.found.sheet, the small pieces close to the body (a charm on a belt or jacket, a short chain, an earring) after them as charm-1, charm-2 with small true and a short chain of 2 to 4 joints; springs.chained, the last chains with their sheets and state; the confirmed ones are spec.springs), the last export (path, size, what it still lacks) and whether an approval waits.",
        annotations(read_only_hint = true)
    )]
    async fn get_avatar(&self, Parameters(args): Parameters<GetAvatarArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                let asset = asset_id(host, &args.project, &args.asset)?;
                let target = Target {
                    project: &args.project,
                    asset: &asset,
                    version: args.version,
                    elbows: Elbows::default(),
                    reweight: ReweightMode::default(),
                    fingers: Fingers::default(),
                    springs: SpringsMode::default(),
                };
                let view = commands::view(host.session(), &target).map_err(|e| e.message)?;
                let mut value = to_short_value(&view);
                value["waitingApproval"] = json!(
                    hub.rest_pose_for(&args.project, &asset).is_some()
                        || hub.reweight_for(&args.project, &asset).is_some()
                        || hub.springs_for(&args.project, &asset).is_some()
                );
                Ok(value)
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Change a character asset's VRM avatar spec, as the Avatar view's form does: meta fields by their VRM 1.0 names (only the ones you give change; null puts one back to its default; the licence URL stays VRM's own), bone map overrides (a VRM bone name to a joint, or null to leave it unmapped; clearBones drops overrides), the material mode (mtoon-soft or pbr), what MToon does with highlights baked into the base color texture (soften, the default, or keep), whether the reweight hides the bridge faces fused between an arm and the body (bridges hide, the default, or keep) and the confirmed springs (by a candidate id from get_avatar's springs.found, with joints, a preset (soft, heavy, stiff) or stiffness, gravityPower, dragForce and hitRadius to tune; nothing is confirmed on its own, so confirm only parts a person or you judged from the candidates sheet; springs \"none\" confirms no springs, while an empty list only clears the picks). reset starts again from the workspace's avatar defaults. A name and at least one author are needed before a build writes. Returns the avatar like get_avatar; the open app updates live.",
        annotations(idempotent_hint = true)
    )]
    async fn set_avatar(&self, Parameters(args): Parameters<SetAvatarArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let asset = asset_id(host, &args.project, &args.asset)?;
                let target = Target {
                    project: &args.project,
                    asset: &asset,
                    version: None,
                    elbows: Elbows::default(),
                    reweight: ReweightMode::default(),
                    fingers: Fingers::default(),
                    springs: SpringsMode::default(),
                };
                let current = commands::view(host.session(), &target).map_err(|e| e.message)?;
                let (_, config) = workspace(host)?;
                let material = args
                    .material
                    .as_deref()
                    .map(|m| {
                        serde_json::from_value(json!(m.to_lowercase()))
                            .map_err(|_| format!("material must be mtoon-soft or pbr, not {m:?}."))
                    })
                    .transpose()?;
                let highlights = args
                    .highlights
                    .as_deref()
                    .map(|h| {
                        serde_json::from_value(json!(h.to_lowercase()))
                            .map_err(|_| format!("highlights must be soften or keep, not {h:?}."))
                    })
                    .transpose()?;
                let bridges = args
                    .bridges
                    .as_deref()
                    .map(|b| {
                        serde_json::from_value(json!(b.to_lowercase()))
                            .map_err(|_| format!("bridges must be hide or keep, not {b:?}."))
                    })
                    .transpose()?;
                let patch = AvatarPatch {
                    reset: args.reset,
                    meta: args.meta.unwrap_or_default(),
                    bones: args.bones,
                    clear_bones: args.clear_bones,
                    material,
                    highlights,
                    bridges,
                    springs: args
                        .springs
                        .as_ref()
                        .map(|arg| springs::change(&current.springs, arg))
                        .transpose()?,
                };
                let defaults = config.avatar.spec(&current.name);
                let next = export::apply_patch(&current.spec, &defaults, &patch).map_err(reason)?;
                let saved = commands::save(host.session(), &target, next).map_err(|e| e.message)?;
                host.avatar_changed(&args.project, &asset);
                Ok(to_short_value(&saved))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Build a character asset's VRM 1.0 avatar into the project's exports/<asset>.vrm (a derived export, never a new version). When the rest pose is not VRM's T-pose yet (facing +Z, arms level, on the ground, centred), it is first normalised headless in Blender on a copy (elbows straighten by default, or keep; fingers straighten by default, as far as the hand's mesh holds, or keep), which makes before and after pose sheets with a row of the hands up close. When the readiness finds cloth or hair weighted to the arms (the arm-weights finding), reweight auto (the default) frees them before that: Blender moves their weight from the arm bones to the trunk on a copy, keeping the arms, sleeves and gloves, and makes before and after sheets; the call answers \"waiting for approval\" with step reweight, and the rest pose comes after that yes (or after a no, from the model as it is). The faces fused between an arm and the body that still stretch when it lifts (bridges, with their side, faces and where) are taken out of the reweighted model by default (set_avatar bridges keep keeps them; reweight.hiddenFaces says how many). reweight off skips it. A changed rest pose needs a person's yes: the call then answers \"waiting for approval\" with step rest pose and the sheet paths, and a person approves or denies it in the app's Avatar view. You cannot approve it: call get_avatar or build_avatar again later (the normalised result is cached, Blender does not run again), and once approved it writes the file and returns its path, size, what it still lacks, the validation report beside it (exports/<asset>.vrm.report.json: source version and hash, readiness before and after, the bone map with each bone's source, the meta, what the reweight changed when one was applied, and with MToon how much of each base color texture is baked highlights and whether a softened copy was written) and the round trip (the written file read back and checked; a failure is a blocker in what it still lacks). \"denied\" means the person turned the pose down. Springs auto (the default): with springs confirmed (set_avatar springs), once the rest pose is approved Blender adds a chain of joints along each one, weights the part to it and derives colliders from the body, on a copy, and makes a sheet of the parts and a sway test; the call answers \"waiting for approval\" with step springs, and after that yes the file carries VRMC_springBone (each spring with its preset or tuned settings, the colliders and their groups) and the report records them; after a no it is written without springs. With none confirmed, a build looks for the parts that could swing (cached) so get_avatar lists them; when the last export carried springs it answers \"candidates waiting for a pick\" and writes nothing until set_avatar confirms some or \"none\". A confirmed spring the finder now names differently (after a new reweight or rest pose) is carried over to the part on its line, keeping its kind, joints and preset, and listed in carried; when none matches clearly the call answers \"springs no longer match\" with stale and writes nothing: pick again. springs off writes no springs. dryRun returns the plan and does nothing. Needs a meta name and an author (set_avatar).",
        annotations(idempotent_hint = true)
    )]
    async fn build_avatar(&self, Parameters(args): Parameters<BuildAvatarArgs>) -> Out {
        let found = self
            .blocking(move |host, hub| {
                let asset = asset_id(host, &args.project, &args.asset)?;
                let target = Target {
                    project: &args.project,
                    asset: &asset,
                    version: args.version,
                    elbows: parse_elbows(args.elbows.as_deref())?,
                    reweight: parse_reweight(args.reweight.as_deref())?,
                    fingers: parse_fingers(args.fingers.as_deref())?,
                    springs: springs::parse_springs(args.springs.as_deref())?,
                };
                if args.dry_run {
                    let plan = commands::plan(host.session(), &target).map_err(|e| e.message)?;
                    let mut value = to_short_value(&plan);
                    value["status"] = json!("dry run");
                    return Ok(value);
                }
                let tools = host.toolbox();
                let built = commands::build(host.session(), host.runner(), &tools, &target)
                    .map_err(|e| e.message)?;
                let name = commands::view(host.session(), &target)
                    .map(|v| v.name)
                    .unwrap_or_else(|_| asset.clone());
                host.avatar_changed(&args.project, &asset);
                Ok(build_answer(hub, &name, &built))
            })
            .await;
        respond(found)
    }
}

#[cfg(test)]
mod tests;
