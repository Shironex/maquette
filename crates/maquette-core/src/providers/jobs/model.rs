//! What a provider job is: its provider, state and phase, the record the
//! UI shows, the events it sends, and the remote chain saved to resume it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::project::ProjectManifest;
use crate::providers::handoff::HandoffCard;
use crate::providers::presets::{
    MeshyPreset, PresetVariant, ProviderId, TripoPreset, TripoRigSpec,
};

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
    /// `<asset id>-<n>`, also the handoff folder and the inbox file name.
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
}

impl ProviderJob {
    /// The settings in a few words, for an approval prompt: "Tripo H3.1,
    /// 2 images as multi-view, rig with Mixamo, 2 animations".
    pub fn summary(&self) -> String {
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
            (provider, _, _) => {
                parts.push(provider.label().to_string());
                parts.push(images);
            }
        }
        parts.join(", ")
    }
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
    /// the create may or may not have reached the provider.
    pub(super) sending: bool,
    pub(super) model_task: Option<String>,
    /// Meshy: which endpoint the model task lives under.
    pub(super) meshy_kind: Option<String>,
    pub(super) rig_check_task: Option<String>,
    pub(super) rig_type: Option<String>,
    pub(super) riggable: Option<bool>,
    pub(super) rig_task: Option<String>,
    pub(super) retarget_task: Option<String>,
    /// Credits per task id, so a resumed poll does not count twice.
    pub(super) credits: BTreeMap<String, f64>,
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
