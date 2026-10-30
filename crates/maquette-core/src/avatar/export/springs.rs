//! Where the spring steps meet a build: which model they read, whether a
//! build waits for the chains' yes, and how the approved chains end up in
//! the written VRM with `VRMC_springBone`.

use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use super::rematch::{self, SpringCarried};
use super::{AvatarBuild, AvatarRequest, asset_dir, invalid, prepare, source};
use crate::avatar::normalize::{self, AVATAR_DIR, NormalizeState, needs_normalize};
use crate::avatar::springs::{
    self, ChainReport, Chained, FoundSprings, SpringWritten, SpringsInput, SpringsMode,
    SpringsSummary, chain, find,
};
use crate::avatar::{AvatarSpec, SpringSpec, SpringsChoice};
use crate::base::ports::ScriptRunner;
use crate::error::{CoreError, CoreResult};
use crate::inbox::vrm::{self, SpringBones, VrmReadiness, WriteSpec, Written};
use crate::pipeline::tools::Toolbox;

/// The model a version's VRM is written from, as the springs read it:
/// `glb` with the bone map of `readiness`.
pub(super) fn input(
    req: &AvatarRequest<'_>,
    asset: &str,
    version: u32,
    glb: &Path,
    readiness: &VrmReadiness,
) -> SpringsInput {
    let cache_root = req
        .workspace_root
        .join(&req.config.layout.cache)
        .join(AVATAR_DIR);
    let bones: BTreeMap<String, String> = readiness
        .bones
        .iter()
        .filter_map(|b| b.joint.clone().map(|j| (b.bone.clone(), j)))
        .collect();
    SpringsInput {
        project: req.project.to_string(),
        asset: asset.to_string(),
        version,
        dir: springs::work_dir(
            &asset_dir(req.workspace_root, req.config, req.project, asset)
                .join(format!("v{version}")),
        ),
        glb: glb.to_path_buf(),
        bones,
        cache_root,
    }
}

/// The springs' input for the version a request names, once it is ready:
/// the version's file when its rest pose is VRM's already, else its
/// approved normalised rest pose (`None` until a person approved it).
///
/// # Errors
///
/// Fails for an unknown project, asset or version, or a model that does
/// not read.
pub(super) fn ready_input(req: &AvatarRequest<'_>) -> CoreResult<Option<SpringsInput>> {
    let (src, _, readiness) = prepare(req)?;
    let version = src.version.unwrap_or_default();
    if !needs_normalize(&readiness) {
        let file = src.file.unwrap_or_default();
        return Ok(Some(input(req, &src.asset.id, version, &file, &readiness)));
    }
    let dir = asset_dir(req.workspace_root, req.config, req.project, &src.asset.id)
        .join(format!("v{version}"));
    Ok(normalize::load(&dir)?
        .filter(|n| n.state == NormalizeState::Approved)
        .map(|n| input(req, &src.asset.id, version, Path::new(&n.glb), &n.readiness)))
}

/// What the springs mean for one build.
pub(super) enum Step {
    /// Write the model as it is, without springs.
    Plain,
    /// Nobody picked yet while the last export carried springs: the
    /// candidates wait for a pick, and nothing is written.
    Unpicked(FoundSprings),
    /// The confirmed springs no longer match the parts found (`stale`, by
    /// id): the person picks again, and nothing is written.
    Stale {
        found: FoundSprings,
        stale: Vec<String>,
    },
    /// The chains wait for a person's yes; `carried` names the picks that
    /// were carried over to the parts the finder sees now.
    Waiting {
        chained: Chained,
        carried: Vec<SpringCarried>,
    },
    /// Write the approved chains in `dir`, with springs, swinging as
    /// `springs` say.
    Chained {
        dir: PathBuf,
        report: ChainReport,
        springs: Vec<SpringSpec>,
    },
}

/// Where a build's springs come from beyond the model: the project and
/// asset whose picks a carry over updates, and whether the last export
/// carried springs.
pub(super) struct Picks<'a> {
    pub(super) project_dir: &'a Path,
    pub(super) asset: &'a str,
    pub(super) had_springs: bool,
}

/// Runs what the springs need before a build writes. Without picks: the
/// finder once per model, so the candidates are there to confirm (a
/// failure there only leaves them out, unless the last export carried
/// springs and they would be dropped). With picks: the finder too, the
/// picks held to its candidates ([`rematch`]), and the chains.
///
/// # Errors
///
/// Fails where finding (with picks) or chaining fails, or a carried over
/// pick cannot be saved.
pub(super) fn step(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    mode: SpringsMode,
    spec: &AvatarSpec,
    input: &SpringsInput,
    picks: &Picks<'_>,
) -> CoreResult<Step> {
    if mode == SpringsMode::Off {
        return Ok(Step::Plain);
    }
    let springs = match spec.springs_choice() {
        SpringsChoice::None => return Ok(Step::Plain),
        SpringsChoice::Unpicked => {
            if !picks.had_springs {
                let _ = find::find(runner, tools, input);
                return Ok(Step::Plain);
            }
            let found = find::find(runner, tools, input)?;
            return Ok(if found.candidates.is_empty() {
                Step::Plain
            } else {
                Step::Unpicked(found)
            });
        }
        SpringsChoice::Picked(springs) => springs,
    };
    let found = find::find(runner, tools, input)?;
    let (springs, carried) = match rematch::hold(springs, &found) {
        rematch::Held::Stale(stale) => return Ok(Step::Stale { found, stale }),
        rematch::Held::Matched { springs, carried } => (springs, carried),
    };
    if !carried.is_empty() {
        carry_over(picks, &spec.springs, &springs)?;
    }
    let plan = chain::plan(input, &springs)?;
    let chained = chain::run(runner, tools, &plan)?;
    Ok(match chained.state {
        NormalizeState::Pending => Step::Waiting { chained, carried },
        NormalizeState::Denied => Step::Plain,
        NormalizeState::Approved => Step::Chained {
            dir: plan.dir,
            report: chained.report,
            springs,
        },
    })
}

/// Saves the carried over picks in place of `before`, unless someone
/// changed the asset's picks since the build read them.
fn carry_over(picks: &Picks<'_>, before: &[SpringSpec], after: &[SpringSpec]) -> CoreResult<()> {
    crate::avatar::update(picks.project_dir, |current| {
        let mut next = current.clone();
        if let Some(spec) = next.avatars.get_mut(picks.asset)
            && spec.springs == before
        {
            spec.springs = after.to_vec();
        }
        Ok(next)
    })
    .map(|_| ())
}

/// Writes the VRM: `plain` for a build without springs, else the approved
/// chains with `VRMC_springBone`, and what the file carries.
///
/// # Errors
///
/// Fails where the writer does, and refuses chains whose yes no longer
/// holds.
pub(super) fn write(
    step: Step,
    write_spec: &WriteSpec,
    plain: impl FnOnce() -> CoreResult<Written>,
) -> CoreResult<(Written, Option<SpringsSummary>)> {
    let Step::Chained {
        dir,
        report,
        springs,
    } = step
    else {
        return Ok((plain()?, None));
    };
    let bytes = chain::approved_glb(&dir)?;
    let mut written = vrm::write(&bytes, write_spec, &dir)?;
    let bones = chain::spring_bones(&report, &springs);
    written.bytes = vrm::add_springs(&written.bytes, &bones)?;
    Ok((written, Some(summary(&report, &bones, &springs))))
}

fn summary(report: &ChainReport, bones: &SpringBones, specs: &[SpringSpec]) -> SpringsSummary {
    let springs = report
        .chains
        .iter()
        .zip(&bones.springs)
        .map(|(chain, written)| SpringWritten {
            id: chain.id.clone(),
            kind: specs
                .iter()
                .find(|s| s.id == chain.id)
                .map(|s| s.kind)
                .or(chain.kind),
            root_bone: chain.root_bone.clone(),
            joints: chain.joints.clone(),
            length: chain.length,
            vertices: chain.vertices,
            settings: written.settings,
            collider_groups: written.collider_groups.clone(),
        })
        .collect();
    let mut groups: Vec<String> = bones.colliders.iter().map(|c| c.group.clone()).collect();
    groups.dedup();
    SpringsSummary {
        springs,
        colliders: u32::try_from(bones.colliders.len()).unwrap_or(u32::MAX),
        collider_groups: groups,
    }
}

/// Looks for the parts hanging off the model a version's VRM is written
/// from, or reads the last look. Blocks while Blender runs.
///
/// # Errors
///
/// Refuses a version whose new rest pose is not approved yet, and fails
/// where the finder does.
pub fn find_springs(
    runner: &dyn ScriptRunner,
    tools: &Toolbox,
    req: &AvatarRequest<'_>,
) -> CoreResult<FoundSprings> {
    let input = ready_input(req)?.ok_or_else(not_ready)?;
    find::find(runner, tools, &input)
}

fn not_ready() -> CoreError {
    invalid(
        "The springs read the rest pose the VRM is written from. Build once and approve the new rest pose first.",
    )
}

/// A person's yes or no to a version's new spring chains, from the app.
///
/// # Errors
///
/// Fails when nothing intact was chained for that version.
pub fn decide_springs(req: &AvatarRequest<'_>, approve: bool) -> CoreResult<Chained> {
    let src = source(req)?;
    let version = src
        .version
        .ok_or_else(|| invalid(format!("{} has no model yet.", src.asset.name)))?;
    let dir = chain::dir_in(&springs::work_dir(
        &asset_dir(req.workspace_root, req.config, req.project, &src.asset.id)
            .join(format!("v{version}")),
    ));
    if approve {
        chain::approve(&dir)
    } else {
        chain::deny(&dir)
    }
}

/// What the Avatar view shows of the springs.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpringsView {
    /// The model the springs read is ready: the version's rest pose is
    /// VRM's, or its new one is approved.
    pub ready: bool,
    /// The last look for dangling parts on that model.
    pub found: Option<FoundSprings>,
    /// The last chains made on it, with their sheets and approval state.
    pub chained: Option<Chained>,
}

/// The springs' part of the view; empty when nothing reads.
pub(super) fn view(req: &AvatarRequest<'_>) -> SpringsView {
    let Ok(Some(input)) = ready_input(req) else {
        return SpringsView::default();
    };
    SpringsView {
        ready: true,
        found: find::found(&input).ok().flatten(),
        chained: chain::load(&chain::dir(&input)).ok().flatten(),
    }
}

/// The build's answer when the springs stop it before anything is
/// written, else the step to write with.
pub(super) fn stopping(step: Step) -> ControlFlow<AvatarBuild, Step> {
    match step {
        Step::Waiting { chained, carried } => {
            ControlFlow::Break(AvatarBuild::SpringsWaiting { chained, carried })
        }
        Step::Unpicked(found) => ControlFlow::Break(AvatarBuild::SpringsUnpicked { found }),
        Step::Stale { found, stale } => {
            ControlFlow::Break(AvatarBuild::SpringsStale { found, stale })
        }
        step => ControlFlow::Continue(step),
    }
}
