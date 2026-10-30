//! Tools that read, change and build a character asset's VRM avatar. A
//! build that changes the rest pose waits for a person to approve the
//! before and after sheets in the app; no tool approves it.

use std::collections::BTreeMap;

use maquette_core::avatar::export::{self, AvatarBuild, AvatarPatch};
use maquette_core::avatar::normalize::{Elbows, Normalized};
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

/// A normalised rest pose, short: the sheets to judge and its numbers.
fn pose_summary(found: &Normalized) -> Value {
    json!({
        "sourceVersion": found.source_version,
        "state": to_short_value(found.state),
        "sheets": to_short_value(&found.sheets),
        "turnedDeg": found.report.turned_deg,
        "elbows": to_short_value(found.report.elbows),
        "lifted": to_short_value(&found.report.lifted),
        "after": to_short_value(&found.report.after),
        "findingsAfter": to_short_value(&found.readiness.findings),
    })
}

/// The answer to a build, and the approval it leaves for a person.
fn build_answer(hub: &Hub, name: &str, built: &AvatarBuild) -> Value {
    match built {
        AvatarBuild::Waiting { normalized } => {
            let approval = hub.request_rest_pose(normalized, name);
            json!({
                "status": WAITING,
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
        AvatarBuild::Written { export, normalized } => json!({
            "status": "written",
            "path": export.path,
            "bytes": export.bytes,
            "sourceVersion": export.source_version,
            "material": to_short_value(export.material),
            "unresolved": to_short_value(&export.unresolved),
            "pose": normalized.as_ref().map(pose_summary),
        }),
    }
}

#[tool_router(router = avatar_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get a character asset's VRM avatar: its VRM readiness (the 55 humanoid bones with the joint each maps to, the 15 required, the rest pose measures and the findings, blockers first), the spec (the VRM meta, bone map overrides and material mode; from the workspace's avatar defaults until first saved), whether the rest pose needs normalising, the last normalised rest pose with its before and after sheets and its state (pending, approved or denied), the last export (path, size, what it still lacks) and whether an approval waits.",
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
                };
                let view = commands::view(host.session(), &target).map_err(|e| e.message)?;
                let mut value = to_short_value(&view);
                value["waitingApproval"] =
                    json!(hub.rest_pose_for(&args.project, &asset).is_some());
                Ok(value)
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Change a character asset's VRM avatar spec, as the Avatar view's form does: meta fields by their VRM 1.0 names (only the ones you give change; null puts one back to its default; the licence URL stays VRM's own), bone map overrides (a VRM bone name to a joint, or null to leave it unmapped; clearBones drops overrides) and the material mode (mtoon-soft or pbr). reset starts again from the workspace's avatar defaults. A name and at least one author are needed before a build writes. Returns the avatar like get_avatar; the open app updates live.",
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
                let patch = AvatarPatch {
                    reset: args.reset,
                    meta: args.meta.unwrap_or_default(),
                    bones: args.bones,
                    clear_bones: args.clear_bones,
                    material,
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
        description = "Build a character asset's VRM 1.0 avatar into the project's exports/<asset>.vrm (a derived export, never a new version). When the rest pose is not VRM's T-pose yet (facing +Z, arms level, on the ground, centred), it is first normalised headless in Blender on a copy (elbows straighten by default, or keep), which makes before and after pose sheets. A changed rest pose needs a person's yes: the call then answers \"waiting for approval\" with the sheet paths, and a person approves or denies it in the app's Avatar view. You cannot approve it: call get_avatar or build_avatar again later (the normalised result is cached, Blender does not run again), and once approved it writes the file and returns its path, size and what it still lacks. \"denied\" means the person turned the pose down. dryRun returns the plan and does nothing. Needs a meta name and an author (set_avatar).",
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
