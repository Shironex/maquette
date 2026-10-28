//! Sending a split into parts: which model it cuts (an asset version, or
//! from the Inbox the model a send brought), whether the provider takes it
//! by the task that made it, and queueing it like any paid send.
//!
//! The model is copied into `<workspace cache>/splits/<job id>/source.glb`
//! when the split goes out: Tripo uploads that copy when it needs a file,
//! and the texture bakes back from it after a Meshy split, whatever
//! happens to the Inbox model or the version meanwhile.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::ids::next_id;
use super::{
    CancelMode, Chain, Entry, JobState, Provider, ProviderEvent, ProviderJob, ProviderJobs, Shared,
    SplitJob, Stored, Workspace, api_key, persist, pump, snapshot, touch_asset,
};
use crate::assets::{self, Asset, AssetKind, AssetStatus};
use crate::error::{CoreError, CoreResult};
use crate::inbox::{self, analyse};
use crate::pipeline::history::now_ms;
use crate::project;
use crate::providers::http::StopFlag;
use crate::providers::presets::{PresetVariant, ProviderId};
use crate::providers::split::{self as rules, MeshySplit, TripoSplit};

/// The copy of the model a split cuts, inside its job's folder.
pub(super) const SOURCE_FILE: &str = "source.glb";

/// What a split into parts asks for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SplitRequest {
    pub project: String,
    pub asset: String,
    /// The version to split; `None` takes the current one.
    #[serde(default)]
    pub version: Option<u32>,
    /// An Inbox model of the asset instead of a version.
    #[serde(default)]
    pub item: Option<String>,
    /// Tripo or Meshy.
    pub provider: Provider,
    /// This split's settings; `None` takes the defaults.
    #[serde(default)]
    pub tripo: Option<TripoSplit>,
    #[serde(default)]
    pub meshy: Option<MeshySplit>,
}

/// The model a split would cut, and what each provider can do with it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SplitSource {
    pub project: String,
    pub asset: String,
    pub asset_name: String,
    pub version: Option<u32>,
    pub item: Option<String>,
    /// The provider whose send made it, if one did.
    pub made_by: Option<Provider>,
    /// The Tripo task Tripo splits it by; `None` uploads the file.
    pub tripo_task: Option<String>,
    /// Why Meshy cannot split it, when it cannot.
    pub meshy_problem: Option<String>,
    pub triangles: u32,
    pub materials: u32,
    /// Texture images: what a split may lose.
    pub textures: u32,
    /// Mesh nodes; one means it moves only as a whole.
    pub objects: u32,
    /// A split of this asset already on its way.
    pub busy: Option<String>,
}

/// The model found for a request, and the send that made it.
struct Found {
    asset: Asset,
    file: PathBuf,
    version: Option<u32>,
    item: Option<String>,
    made: Option<Stored>,
}

/// The split's own folder in the workspace cache.
pub(super) fn split_dir(workspace: &Workspace, id: &str) -> PathBuf {
    workspace
        .root
        .join(&workspace.config.layout.cache)
        .join("splits")
        .join(id)
}

fn find_model(
    shared: &Shared,
    workspace: &Workspace,
    project_id: &str,
    asset_id: &str,
    version: Option<u32>,
    item: Option<&str>,
) -> CoreResult<Found> {
    let manifest = project::load(&workspace.root, &workspace.config.layout, project_id)?;
    let asset = assets::find(&manifest, asset_id)?.clone();
    let jobs = shared.jobs();
    if let Some(item) = item {
        let found = inbox::item(&workspace.root, &workspace.config, item)?;
        let made = jobs
            .iter()
            .find(|e| e.stored.job.inbox_file.as_deref() == Some(item))
            .map(|e| e.stored.clone());
        if made
            .as_ref()
            .is_some_and(|m| m.job.project != project_id || m.job.asset != asset.id)
        {
            return Err(CoreError::Invalid(format!(
                "{item} was sent for another asset, not {}.",
                asset.name
            )));
        }
        return Ok(Found {
            asset,
            file: PathBuf::from(found.file),
            version: None,
            item: Some(item.to_string()),
            made,
        });
    }
    let chosen = match version {
        Some(v) => asset.versions.iter().find(|x| x.version == v),
        None => asset.versions.last(),
    }
    .ok_or_else(|| {
        CoreError::Invalid(match version {
            Some(v) => format!("{} has no v{v}.", asset.name),
            None => format!(
                "{} has no model yet. Accept one from the Inbox first.",
                asset.name
            ),
        })
    })?;
    let dir = workspace.project_dir(project_id)?;
    let number = chosen.version;
    let made = jobs
        .iter()
        .map(|e| &e.stored)
        .find(|s| {
            s.job.project == project_id
                && s.job.asset == asset.id
                && s.job.version == Some(number)
                && s.job.state == JobState::Accepted
        })
        .cloned();
    Ok(Found {
        file: dir.join(&chosen.file),
        asset,
        version: Some(number),
        item: None,
        made,
    })
}

/// The Tripo task a model can be split by: a generation Tripo made, not a
/// rig or an earlier split (those go up as a file).
fn tripo_task(made: Option<&Stored>) -> Option<String> {
    let made = made.filter(|m| m.job.provider == Provider::Tripo && m.job.split.is_none())?;
    if made.chain.rig_task.is_some() {
        return None;
    }
    made.chain.model_task.clone()
}

/// The Meshy task Auto Split takes, or why there is none.
fn meshy_task(made: Option<&Stored>, label: &str) -> Result<String, String> {
    let Some(made) = made.filter(|m| m.job.provider == Provider::Meshy) else {
        return Err(format!(
            "Meshy splits only models it made, and {label} did not come from Meshy."
        ));
    };
    if made.job.split.is_some() {
        return Err("This model is already a Meshy split.".to_string());
    }
    if made.chain.rig_task.is_some() {
        return Err(
            "This is Meshy's rigged model. Auto Split takes the model before the rig.".to_string(),
        );
    }
    if let Some(problem) = made
        .job
        .meshy
        .as_ref()
        .and_then(|m| rules::meshy_model_problem(&m.ai_model))
    {
        return Err(problem);
    }
    made.chain
        .model_task
        .clone()
        .ok_or_else(|| "Meshy has no task for this model.".to_string())
}

fn label(found: &Found) -> String {
    match found.version {
        Some(v) => format!("v{v}"),
        None => "this Inbox model".to_string(),
    }
}

/// The split of the asset already on its way, if any.
fn busy(shared: &Shared, project: &str, asset: &str) -> Option<String> {
    shared
        .jobs()
        .iter()
        .map(|e| &e.stored.job)
        .find(|j| {
            j.project == project
                && j.asset == asset
                && j.split.is_some()
                && matches!(
                    j.state,
                    JobState::Approval | JobState::Queued | JobState::Running
                )
        })
        .map(|j| j.id.clone())
}

impl ProviderJobs {
    /// What the split sheet shows for a model: its numbers and what each
    /// provider can do with it. Sends nothing.
    pub fn split_source(
        &self,
        project: &str,
        asset: &str,
        version: Option<u32>,
        item: Option<&str>,
    ) -> CoreResult<SplitSource> {
        let workspace = self.workspace()?;
        let found = find_model(&self.shared, &workspace, project, asset, version, item)?;
        let report = analyse::analyse(&found.file)?;
        Ok(SplitSource {
            project: project.to_string(),
            asset: found.asset.id.clone(),
            asset_name: found.asset.name.clone(),
            version: found.version,
            item: found.item.clone(),
            made_by: found.made.as_ref().map(|m| m.job.provider),
            tripo_task: tripo_task(found.made.as_ref()),
            meshy_problem: meshy_task(found.made.as_ref(), &label(&found)).err(),
            triangles: report.triangles,
            materials: report.materials,
            textures: u32::try_from(report.textures.len()).unwrap_or(u32::MAX),
            objects: report.objects,
            busy: busy(&self.shared, project, &found.asset.id),
        })
    }

    /// Checks a split and resolves its settings, without starting it.
    pub fn prepare_split(&self, request: &SplitRequest) -> CoreResult<ProviderJob> {
        let workspace = self.workspace()?;
        prepare(&self.shared, &workspace, request).map(|(job, _)| job)
    }

    /// Starts a split. With `approval` it waits in Approval until
    /// [`ProviderJobs::approve`], as a paid send does.
    pub fn send_split(
        &self,
        request: &SplitRequest,
        approval: bool,
        via_agent: bool,
    ) -> CoreResult<ProviderJob> {
        let _one = self.shared.sending();
        let workspace = self.workspace()?;
        let (mut job, file) = prepare(&self.shared, &workspace, request)?;
        job.via_agent = via_agent;
        let dir = split_dir(&workspace, &job.id);
        fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
        let copy = dir.join(SOURCE_FILE);
        fs::copy(&file, &copy).map_err(|e| CoreError::io(&copy, e))?;
        if approval {
            job.state = JobState::Approval;
            job.message = Some("Waiting for your approval before spending credits".to_string());
        }
        let stored = Stored {
            job: job.clone(),
            chain: Chain::default(),
        };
        self.shared.jobs().push(Entry {
            stored: stored.clone(),
            stop: Arc::new(StopFlag::default()),
            thread: false,
            cancelled: false,
        });
        persist(&self.shared, &job.project);
        touch_asset(
            &self.shared,
            &job.project,
            &job.asset,
            None,
            AssetStatus::Queued,
        );
        self.shared.emit(ProviderEvent::Job {
            job: Box::new(snapshot(&stored)),
        });
        pump(&self.shared);
        Ok(self.get(&job.id).unwrap_or(job))
    }

    fn workspace(&self) -> CoreResult<Workspace> {
        self.shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))
    }
}

fn prepare(
    shared: &Shared,
    workspace: &Workspace,
    request: &SplitRequest,
) -> CoreResult<(ProviderJob, PathBuf)> {
    let found = find_model(
        shared,
        workspace,
        &request.project,
        &request.asset,
        request.version,
        request.item.as_deref(),
    )?;
    let report = analyse::analyse(&found.file)?;
    let (tripo, meshy, input_task) = match request.provider {
        Provider::Tripo => {
            let settings = request.tripo.clone().unwrap_or_default();
            (Some(settings), None, tripo_task(found.made.as_ref()))
        }
        Provider::Meshy => {
            let settings = request.meshy.clone().unwrap_or_default();
            if let Some(problem) = rules::meshy_split_problem(&settings) {
                return Err(CoreError::Invalid(problem));
            }
            let task =
                meshy_task(found.made.as_ref(), &label(&found)).map_err(CoreError::Invalid)?;
            (None, Some(settings), Some(task))
        }
        Provider::ByHand => {
            return Err(CoreError::Invalid(
                "Split with Tripo or Meshy here; the free split in Blender is a fix-up."
                    .to_string(),
            ));
        }
    };
    let service = match request.provider {
        Provider::Meshy => ProviderId::Meshy,
        _ => ProviderId::Tripo,
    };
    api_key(shared, service)?;
    if let Some(id) = busy(shared, &request.project, &found.asset.id) {
        return Err(CoreError::Invalid(format!(
            "{} is already being split ({id}).",
            found.asset.name
        )));
    }
    let project_dir = workspace.project_dir(&request.project)?;
    let id = next_id(&shared.jobs(), &project_dir, &found.asset.id);
    let split = SplitJob {
        from_version: found.version,
        from_item: found.item.clone(),
        tripo,
        meshy,
        input_task,
        textures_before: u32::try_from(report.textures.len()).unwrap_or(u32::MAX),
        texture_kept: None,
        bake: None,
    };
    let estimate = rules::credits(split.tripo.as_ref(), split.meshy.as_ref());
    let job = ProviderJob {
        id,
        project: request.project.clone(),
        asset: found.asset.id.clone(),
        asset_name: found.asset.name.clone(),
        provider: request.provider,
        service,
        variant: if found.asset.kind == AssetKind::Character {
            PresetVariant::Character
        } else {
            PresetVariant::Prop
        },
        images: Vec::new(),
        views: Vec::new(),
        tripo: None,
        meshy: None,
        state: JobState::Queued,
        phase: None,
        progress: 0.0,
        remote_id: None,
        credits: None,
        created_at_ms: now_ms(),
        started_at_ms: None,
        finished_at_ms: None,
        message: None,
        inbox_file: None,
        handoff: None,
        card: None,
        version: None,
        via_agent: false,
        cancel: CancelMode::Free,
        batch: None,
        prompt: None,
        redo_of: None,
        estimate,
        split: Some(split),
    };
    Ok((job, found.file))
}

/// The model a split cuts, as it was when it went out; the version it
/// came from when an older job has no copy.
pub(super) fn source_of(workspace: &Workspace, job: &ProviderJob) -> Option<PathBuf> {
    let copy = split_dir(workspace, &job.id).join(SOURCE_FILE);
    if copy.is_file() {
        return Some(copy);
    }
    let version = job.split.as_ref()?.from_version?;
    version_file(workspace, &job.project, &job.asset, version)
}

/// The file of one version of an asset.
pub(super) fn version_file(
    workspace: &Workspace,
    project_id: &str,
    asset: &str,
    version: u32,
) -> Option<PathBuf> {
    let manifest = project::load(&workspace.root, &workspace.config.layout, project_id).ok()?;
    let found = assets::find(&manifest, asset).ok()?;
    let file = &found.versions.iter().find(|v| v.version == version)?.file;
    Some(workspace.project_dir(project_id).ok()?.join(file))
}
