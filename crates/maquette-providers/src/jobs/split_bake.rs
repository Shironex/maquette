//! What happens to a split's texture once it is in: whether the provider
//! kept it (looked at on arrival), and the free bake back in Blender from
//! the model it was split from onto the split's own version, which makes
//! the next version. A Meshy split always bakes back; a Tripo split that
//! lost its texture offers it. The untextured version stays, so keeping it
//! undoes the bake.

use super::split_files::{source_of, split_dir, version_file};
use super::{BakeBack, BakeState, ProviderEvent, ProviderJob, ProviderJobs, Stored, Workspace};
use super::{Provider, update};
use crate::error::{CoreError, CoreResult};
use crate::split::SplitAfter;
use maquette_base::ports::TextureBake;
use maquette_base::tools::Toolbox;
use maquette_glb::analyse;
use maquette_model::assets::versions::{self, Target};

/// Texture images in an Inbox model.
pub(super) fn textures_in(workspace: &Workspace, item: &str) -> u32 {
    let path = maquette_model::sends::inbox_dir(&workspace.root, &workspace.config).join(item);
    analyse::analyse(&path)
        .map(|r| u32::try_from(r.textures.len()).unwrap_or(u32::MAX))
        .unwrap_or(0)
}

/// Records whether the split kept the texture, and says what follows.
pub(super) fn arrived(stored: &mut Stored, textures: u32) {
    let provider = stored.job.provider;
    let Some(split) = stored.job.split.as_mut() else {
        return;
    };
    let lost = split.textures_before > 0 && textures == 0;
    split.texture_kept = Some(!lost);
    let from = split.source_label();
    let after = split.after;
    stored.job.message = Some(match (provider, lost) {
        (Provider::Meshy, true) if after == SplitAfter::Bake => format!(
            "In the Inbox. Meshy drops the texture: accept it and the texture bakes back from {from} in Blender, free."
        ),
        (Provider::Meshy, true) => format!(
            "In the Inbox. Meshy drops the texture: accept it, then re-texture it or bake it back from {from}, or keep it grey."
        ),
        (_, true) => format!(
            "In the Inbox. Tripo did not keep the texture: accept it, then bake it back from {from} for free."
        ),
        _ => "In the Inbox, ready for its checks".to_string(),
    });
}

/// Once the split is accepted as `version`: a Meshy split that lost its
/// texture schedules the bake back unless another choice was made; the
/// others, and a Tripo one, offer it.
pub(super) fn on_accept(stored: &mut Stored, version: u32) {
    let provider = stored.job.provider;
    let Some(split) = stored.job.split.as_mut() else {
        return;
    };
    if split.texture_kept != Some(false) || split.bake.is_some() {
        return;
    }
    let state = if provider == Provider::Meshy && split.after == SplitAfter::Bake {
        BakeState::Scheduled
    } else {
        BakeState::Offered
    };
    split.bake = Some(BakeBack {
        state,
        onto: version,
        version: None,
        message: None,
    });
}

/// A bake that was running when the app closed runs again.
pub(super) fn resume(stored: &mut Stored) -> bool {
    let Some(bake) = stored.job.split.as_mut().and_then(|s| s.bake.as_mut()) else {
        return false;
    };
    if bake.state != BakeState::Running {
        return false;
    }
    bake.state = BakeState::Scheduled;
    true
}

fn set_bake(jobs: &ProviderJobs, id: &str, change: impl FnOnce(&mut BakeBack)) -> CoreResult<()> {
    update(&jobs.shared, id, true, |stored| {
        let bake = stored
            .job
            .split
            .as_mut()
            .and_then(|s| s.bake.as_mut())
            .ok_or_else(|| {
                CoreError::Invalid("This split has nothing to bake back.".to_string())
            })?;
        change(bake);
        Ok(())
    })
}

impl ProviderJobs {
    /// The splits whose bake back waits to run, oldest first.
    pub fn scheduled_bakes(&self) -> Vec<String> {
        self.list(None)
            .into_iter()
            .filter(|j| {
                j.split
                    .as_ref()
                    .and_then(|s| s.bake.as_ref())
                    .is_some_and(|b| b.state == BakeState::Scheduled)
            })
            .map(|j| j.id)
            .collect()
    }

    /// The split of an asset whose bake back can run now (offered, or
    /// failed): the one onto `version`, or the newest without it.
    ///
    /// # Errors
    ///
    /// Fails when no split of the asset has a bake back waiting.
    pub fn waiting_bake(
        &self,
        project: &str,
        asset: &str,
        version: Option<u32>,
    ) -> CoreResult<String> {
        self.list(None)
            .into_iter()
            .filter(|j| j.project == project && j.asset == asset)
            .filter_map(|j| {
                let bake = j.split.as_ref()?.bake.as_ref()?;
                let waiting = matches!(bake.state, BakeState::Offered | BakeState::Failed);
                (waiting && version.is_none_or(|v| v == bake.onto))
                    .then(|| (bake.onto, j.id.clone()))
            })
            .max_by_key(|(onto, _)| *onto)
            .map(|(_, id)| id)
            .ok_or_else(|| {
                let onto = version.map(|v| format!(" onto v{v}")).unwrap_or_default();
                CoreError::NotFound(format!("a texture bake back offered for {asset}{onto}"))
            })
    }

    /// Runs every scheduled bake back, one after another. Blocks.
    pub fn run_scheduled_bakes(&self, tools: &Toolbox) -> Vec<ProviderJob> {
        self.scheduled_bakes()
            .iter()
            .filter_map(|id| self.bake_back(id, tools).ok())
            .collect()
    }

    /// Bakes the texture of the model a split came from onto the split's
    /// version, headless, and adds the result as the asset's next version.
    /// An offered or failed bake runs when asked. Blocks until Blender is
    /// done; the job says how it went either way.
    ///
    /// # Errors
    ///
    /// Fails for a job that is not a split with a bake back, one already
    /// running or done, or without a workspace.
    pub fn bake_back(&self, id: &str, tools: &Toolbox) -> CoreResult<ProviderJob> {
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        let job = self
            .get(id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
        let bake = job
            .split
            .as_ref()
            .and_then(|s| s.bake.clone())
            .ok_or_else(|| {
                CoreError::Invalid("This split has nothing to bake back.".to_string())
            })?;
        match bake.state {
            BakeState::Running => {
                return Err(CoreError::Invalid(
                    "The texture is baking back already.".to_string(),
                ));
            }
            BakeState::Done => {
                return Err(CoreError::Invalid(format!(
                    "The texture is already back, in v{}.",
                    bake.version.unwrap_or(bake.onto)
                )));
            }
            _ => {}
        }
        let from = job
            .split
            .as_ref()
            .map(super::SplitJob::source_label)
            .unwrap_or_default();
        set_bake(self, id, |b| {
            b.state = BakeState::Running;
            b.message = Some(format!("Baking the texture back from {from} in Blender"));
        })?;
        let made = bake_onto(&*self.shared.ports.bake, &workspace, &job, bake.onto, tools);
        match made {
            Ok(landed) => {
                self.shared.emit(ProviderEvent::Project {
                    project: landed.project,
                });
                let version = landed.version.version;
                set_bake(self, id, |b| {
                    b.state = BakeState::Done;
                    b.version = Some(version);
                    b.message = Some(format!(
                        "Texture baked back from {from} as v{version}. v{} keeps the untextured parts.",
                        b.onto
                    ));
                })?;
            }
            Err(err) => {
                set_bake(self, id, |b| {
                    b.state = BakeState::Failed;
                    b.message = Some(format!("The bake back did not finish. {err}"));
                })?;
            }
        }
        self.get(id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))
    }

    /// Offers a bake back again after it failed, or runs an offered one:
    /// marks it scheduled for [`ProviderJobs::run_scheduled_bakes`].
    ///
    /// # Errors
    ///
    /// Fails for a job with no bake back, or one running or done.
    pub fn schedule_bake(&self, id: &str) -> CoreResult<ProviderJob> {
        let mut refused = None;
        set_bake(self, id, |b| match b.state {
            BakeState::Offered | BakeState::Failed => b.state = BakeState::Scheduled,
            BakeState::Scheduled => {}
            BakeState::Running | BakeState::Done => {
                refused = Some("The texture is already back, or baking.".to_string());
            }
        })?;
        if let Some(message) = refused {
            return Err(CoreError::Invalid(message));
        }
        self.get(id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))
    }
}

fn bake_onto(
    baker: &dyn TextureBake,
    workspace: &Workspace,
    job: &ProviderJob,
    onto: u32,
    tools: &Toolbox,
) -> CoreResult<versions::Landed> {
    let source = source_of(workspace, job).ok_or_else(|| {
        CoreError::NotFound("the model this split came from, to bake from".to_string())
    })?;
    let target = version_file(workspace, &job.project, &job.asset, onto)
        .ok_or_else(|| CoreError::NotFound(format!("v{onto} of {}", job.asset_name)))?;
    let work = split_dir(workspace, &job.id).join("bake");
    let baked = baker.bake(&work, &source, &target, tools)?;
    versions::add(
        &workspace.root,
        &workspace.config.layout,
        &job.project,
        Target::Asset(&job.asset),
        &baked,
    )
}
