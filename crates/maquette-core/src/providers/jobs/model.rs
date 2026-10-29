//! What a provider job is: its provider, state and phase, the record the
//! UI shows, the events it sends, and the remote chain saved to resume it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::project::ProjectManifest;
use crate::providers::handoff::HandoffCard;
use crate::providers::handoff::options::{StudioRig, StudioSettings};
use crate::providers::presets::{
    MeshyPreset, PresetVariant, ProviderId, TripoPreset, TripoRigSpec,
};
use crate::providers::split::{MeshySplit, TripoSplit};

/// How a model gets made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Tripo,
    Meshy,
    ByHand,
}

impl Provider {
    pub fn service(self) -> Option<ProviderId> {
        match self {
            Self::Tripo => Some(ProviderId::Tripo),
            Self::Meshy => Some(ProviderId::Meshy),
            Self::ByHand => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Tripo => "Tripo",
            Self::Meshy => "Meshy",
            Self::ByHand => "By hand",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum JobState {
    /// An agent asked; it waits for you before spending credits.
    Approval,
    Queued,
    Running,
    /// By hand: waiting for your model.
    Waiting,
    /// The GLB is in the inbox, linked to the asset.
    Inbox,
    Accepted,
    Failed,
    Cancelled,
}

impl JobState {
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Approval | Self::Queued | Self::Running | Self::Waiting
        )
    }
}

/// What the job is doing now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum JobPhase {
    Upload,
    Generate,
    RigCheck,
    Rig,
    Animate,
    /// A split cutting the model into parts.
    Split,
    /// Tripo completing the parts a split cut.
    Complete,
    Download,
}

/// What Cancel would do, so the UI can say it before you press it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum CancelMode {
    /// Nothing was sent, so nothing is spent.
    Free,
    /// Meshy refunds a task that has not started; a started one refuses.
    Provider,
    /// By hand: stop waiting.
    StopWaiting,
    /// Too late: Tripo has no cancel, or the job is over.
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProviderJob {
    /// `<asset id>-<n>`, unique across the workspace; also the handoff
    /// folder and the inbox file name.
    pub id: String,
    pub project: String,
    pub asset: String,
    pub asset_name: String,
    pub provider: Provider,
    /// The service whose settings it uses (for By hand, whose web app).
    pub service: ProviderId,
    pub variant: PresetVariant,
    /// Input images, relative to the project folder, front first.
    pub images: Vec<String>,
    /// The view each image goes as (`front`, `left`, ...), from the
    /// asset's tags. Older records have none: the images go in order.
    #[serde(default)]
    pub views: Vec<String>,
    pub tripo: Option<TripoPreset>,
    pub meshy: Option<MeshyPreset>,
    pub state: JobState,
    pub phase: Option<JobPhase>,
    /// 0 to 1.
    pub progress: f64,
    /// The provider's id of the current task.
    pub remote_id: Option<String>,
    /// Credits the provider reported, summed over the chain.
    pub credits: Option<f64>,
    pub created_at_ms: f64,
    pub started_at_ms: Option<f64>,
    pub finished_at_ms: Option<f64>,
    /// Why it failed or stopped, or a note on how it went.
    pub message: Option<String>,
    /// The file name in the inbox once the model is there.
    pub inbox_file: Option<String>,
    /// By hand: the handoff folder (absolute).
    pub handoff: Option<String>,
    pub card: Option<HandoffCard>,
    /// The asset version it became.
    pub version: Option<u32>,
    pub via_agent: bool,
    pub cancel: CancelMode,
    /// The batch it was sent with, if any; one handoff checklist each.
    pub batch: Option<String>,
    /// Meshy only: the text that guides the texture.
    #[serde(default)]
    pub prompt: Option<String>,
    /// The send this one redoes, from the Inbox.
    #[serde(default)]
    pub redo_of: Option<String>,
    /// What the price lists say it should cost, before the provider reports.
    #[serde(default)]
    pub estimate: Option<f64>,
    /// Set for a split into parts of an asset's model, instead of a model
    /// made from images.
    #[serde(default)]
    pub split: Option<SplitJob>,
    /// By hand on Tripo: the Studio tab and rig the card asks for. Older
    /// records have none.
    #[serde(default)]
    pub studio: Option<StudioSettings>,
}

impl ProviderJob {
    /// The settings in a few words, for an approval prompt: "Tripo H3.1,
    /// 2 images as multi-view, rig with Mixamo, 2 animations".
    pub fn summary(&self) -> String {
        if let Some(split) = &self.split {
            return format!(
                "{} of {}",
                crate::providers::split::summary(split.tripo.as_ref(), split.meshy.as_ref()),
                split.source_label()
            );
        }
        let images = match self.images.len() {
            1 => "1 image".to_string(),
            n => format!("{n} images"),
        };
        let mut parts = Vec::new();
        match (self.provider, &self.tripo, &self.meshy) {
            (Provider::Tripo, Some(t), _) => {
                parts.push(format!(
                    "Tripo {}",
                    crate::providers::catalog::tripo_label(&t.model)
                ));
                let multi = t.multi_view && self.images.len() >= 2;
                parts.push(if multi {
                    format!("{images} as multi-view")
                } else {
                    images
                });
                if t.face_limit > 0 {
                    parts.push(format!(
                        "{} faces",
                        crate::providers::presets::group(t.face_limit)
                    ));
                }
                if t.rig {
                    parts.push(match t.rig_spec {
                        TripoRigSpec::Mixamo => "rig with Mixamo".to_string(),
                        TripoRigSpec::Tripo => "rig".to_string(),
                    });
                    match t.animations.len().min(5) {
                        0 => {}
                        1 => parts.push("1 animation".to_string()),
                        n => parts.push(format!("{n} animations")),
                    }
                }
            }
            (Provider::Meshy, _, Some(m)) => {
                parts.push(format!(
                    "Meshy {}",
                    crate::providers::catalog::meshy_label(&m.ai_model)
                ));
                let multi = m.multi_view && self.images.len() >= 2 && !m.smart_topology();
                parts.push(if multi {
                    format!("{images} as multi-view")
                } else {
                    images
                });
                if m.should_remesh || m.smart_topology() {
                    parts.push(format!(
                        "{} polygons",
                        crate::providers::presets::group(m.target_polycount)
                    ));
                }
                if m.rig {
                    parts.push("rig".to_string());
                }
            }
            (Provider::ByHand, Some(_), _) if self.service == ProviderId::Tripo => {
                parts.push("By hand on Tripo Studio".to_string());
                if let Some(studio) = self.studio {
                    parts.push(studio.mesh.label().to_string());
                }
                parts.push(if self.images.len() >= 2 {
                    format!("{images} as multi-view")
                } else {
                    images
                });
                match self.studio.map(|s| s.rig) {
                    Some(StudioRig::None) | None => {}
                    Some(rig) => parts.push(format!("{} rig", rig.label())),
                }
            }
            (provider, _, _) => {
                parts.push(provider.label().to_string());
                parts.push(images);
            }
        }
        parts.join(", ")
    }
}

/// A split into parts: the model it cuts, its settings and what happens
/// to the texture once it lands.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SplitJob {
    /// The asset version it splits, or (from the Inbox) the inbox model.
    pub from_version: Option<u32>,
    pub from_item: Option<String>,
    pub tripo: Option<TripoSplit>,
    pub meshy: Option<MeshySplit>,
    /// The provider task that made the model, when the provider takes it
    /// by its task; `None` uploads the file (Tripo only).
    pub input_task: Option<String>,
    /// Texture images the model had before the split.
    pub textures_before: u32,
    /// Whether the split kept the texture, once it is in.
    pub texture_kept: Option<bool>,
    /// The texture bake back from the model it was split from.
    pub bake: Option<BakeBack>,
}

impl SplitJob {
    /// "v2", or "the Inbox model" for one not accepted yet.
    pub fn source_label(&self) -> String {
        match self.from_version {
            Some(v) => format!("v{v}"),
            None => "the Inbox model".to_string(),
        }
    }
}

/// Where a bake back is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum BakeState {
    /// Tripo dropped the texture: a bake back is offered, not run.
    Offered,
    /// Runs as soon as Blender is free (Meshy, always).
    Scheduled,
    Running,
    Done,
    Failed,
}

/// Baking the texture of the model a split came from onto its parts, in
/// headless Blender. Free; the untextured version stays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BakeBack {
    pub state: BakeState,
    /// The split's own version, the parts it bakes onto.
    pub onto: u32,
    /// The version the bake made.
    pub version: Option<u32>,
    pub message: Option<String>,
}

/// An inbox model that came from a send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InboxLink {
    pub job: String,
    pub project: String,
    pub asset: String,
    pub asset_name: String,
    pub provider: Provider,
    /// The send it redoes, when it came from a Redo.
    #[serde(default)]
    pub redo_of: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProviderEvent {
    Job {
        job: Box<ProviderJob>,
    },
    /// An asset moved or changed status.
    Project {
        project: ProjectManifest,
    },
}

/// The remote side of a job, saved so a restart picks it up.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct Chain {
    /// Set just before the create call. With no task id after a restart,
    /// the create may or may not have reached the provider. Cleared again
    /// when the create failed in a way that proves it made no task.
    pub(super) sending: bool,
    pub(super) model_task: Option<String>,
    /// Meshy: which endpoint the model task lives under.
    pub(super) meshy_kind: Option<String>,
    pub(super) rig_check_task: Option<String>,
    pub(super) rig_type: Option<String>,
    pub(super) riggable: Option<bool>,
    pub(super) rig_task: Option<String>,
    pub(super) retarget_task: Option<String>,
    /// Tripo: the part completion after a split.
    pub(super) complete_task: Option<String>,
    /// Credits per task id, so a resumed poll does not count twice.
    pub(super) credits: BTreeMap<String, f64>,
    /// Cancelled on the provider before its model task was charged, so the
    /// credits come back.
    pub(super) refunded: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Stored {
    pub(super) job: ProviderJob,
    #[serde(default)]
    pub(super) chain: Chain,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct JobsFile {
    pub(super) version: u32,
    pub(super) jobs: Vec<Stored>,
}
