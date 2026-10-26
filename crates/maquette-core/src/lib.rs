//! Maquette domain logic: toolchain detection, the workspace config,
//! projects and their assets, the pipeline runner, model thumbnails and the
//! per machine app state.

pub mod app_state;
pub mod assets;
pub mod built;
pub mod concepts;
pub mod error;
pub mod fs;
pub mod inbox;
pub mod inspect;
mod json;
pub mod layout;
pub mod materials;
pub mod motion;
pub mod pipeline;
pub mod prepare;
pub mod project;
pub mod providers;
pub mod site;
#[cfg(test)]
mod test_blender;
pub mod thumbs;
pub mod toolchain;
pub mod workspace;

pub use app_state::AppState;
pub use assets::{Asset, AssetKind, AssetStatus, AssetVersion, ModelStats, NewAsset, Stage};
pub use error::{CoreError, CoreResult, ErrorPayload};
pub use project::{
    BlendFile, ImportScan, NewProject, ProjectActivity, ProjectManifest, ProjectSummary,
    ProjectTemplate, RepoInspection,
};
pub use toolchain::{
    CommandRunner, HostOs, ProbeEnv, SystemRunner, ToolId, ToolReport, ToolSource, ToolStatus,
};
pub use workspace::{
    AgentRules, AgentSettings, Budgets, Compression, FolderLayout, LabSettings, MeshoptLevel,
    RootInspection, RootState, TextureCodec, TextureSize, Topology, WorkspaceConfig,
};

/// Every IPC facing type, so the shell can export them before any command
/// uses them.
pub fn types() -> specta::Types {
    specta::Types::default()
        .register::<ErrorPayload>()
        .register::<ToolId>()
        .register::<ToolStatus>()
        .register::<ToolSource>()
        .register::<ToolReport>()
        .register::<HostOs>()
        .register::<WorkspaceConfig>()
        .register::<FolderLayout>()
        .register::<Topology>()
        .register::<TextureSize>()
        .register::<providers::ProviderSettings>()
        .register::<providers::TripoPresets>()
        .register::<providers::TripoPreset>()
        .register::<providers::TripoTextureQuality>()
        .register::<providers::TripoGeometryQuality>()
        .register::<providers::TripoRigSpec>()
        .register::<providers::MeshyPresets>()
        .register::<providers::MeshyPreset>()
        .register::<providers::MeshyTexture>()
        .register::<providers::PoseMode>()
        .register::<providers::ProviderId>()
        .register::<providers::PresetVariant>()
        .register::<providers::ModelOption>()
        .register::<providers::ProviderCatalog>()
        .register::<providers::Provider>()
        .register::<providers::JobState>()
        .register::<providers::JobPhase>()
        .register::<providers::CancelMode>()
        .register::<providers::ProviderJob>()
        .register::<providers::SendRequest>()
        .register::<providers::InboxLink>()
        .register::<providers::ProviderEvent>()
        .register::<providers::CardRow>()
        .register::<providers::HandoffCard>()
        .register::<providers::KeyStatus>()
        .register::<providers::KeyTest>()
        .register::<providers::BatchItem>()
        .register::<providers::BatchRequest>()
        .register::<providers::BatchFailure>()
        .register::<providers::BatchSent>()
        .register::<providers::DownloadMatch>()
        .register::<providers::Balance>()
        .register::<providers::Spend>()
        .register::<providers::SpendOutcome>()
        .register::<providers::Ledger>()
        .register::<providers::ServiceTotals>()
        .register::<providers::DownloadsScan>()
        .register::<Budgets>()
        .register::<Compression>()
        .register::<TextureCodec>()
        .register::<MeshoptLevel>()
        .register::<AgentSettings>()
        .register::<AgentRules>()
        .register::<RootState>()
        .register::<RootInspection>()
        .register::<ProjectTemplate>()
        .register::<ProjectManifest>()
        .register::<ProjectSummary>()
        .register::<ProjectActivity>()
        .register::<NewProject>()
        .register::<RepoInspection>()
        .register::<BlendFile>()
        .register::<ImportScan>()
        .register::<AppState>()
        .register::<app_state::DownloadsWatch>()
        .register::<Asset>()
        .register::<AssetKind>()
        .register::<AssetStatus>()
        .register::<AssetVersion>()
        .register::<ModelStats>()
        .register::<NewAsset>()
        .register::<Stage>()
        .register::<pipeline::StepKind>()
        .register::<pipeline::CompressSpec>()
        .register::<pipeline::StepDef>()
        .register::<pipeline::PipelineDef>()
        .register::<pipeline::presets::CompressPreset>()
        .register::<pipeline::graph::IssueLevel>()
        .register::<pipeline::graph::PipelineIssue>()
        .register::<pipeline::history::RunState>()
        .register::<pipeline::history::StepState>()
        .register::<pipeline::history::StepRecord>()
        .register::<pipeline::history::RunRecord>()
        .register::<pipeline::history::LogLevel>()
        .register::<pipeline::history::LogStream>()
        .register::<pipeline::history::LogEntry>()
        .register::<pipeline::runner::PipelineEvent>()
        .register::<inbox::analyse::RigKind>()
        .register::<inbox::analyse::TextureInfo>()
        .register::<inbox::analyse::PbrMaps>()
        .register::<inbox::analyse::JointCount>()
        .register::<inbox::analyse::FingerWeights>()
        .register::<inbox::analyse::Islands>()
        .register::<inbox::analyse::GlbReport>()
        .register::<inbox::checks::CheckStatus>()
        .register::<inbox::checks::Check>()
        .register::<inbox::InboxState>()
        .register::<inbox::InboxItem>()
        .register::<inbox::AcceptRequest>()
        .register::<inbox::AcceptResult>()
        .register::<thumbs::ThumbState>()
        .register::<workspace::LabSettings>()
        .register::<concepts::Aspect>()
        .register::<concepts::ConceptImage>()
        .register::<concepts::Direction>()
        .register::<concepts::NewConcepts>()
        .register::<concepts::SentPicks>()
        .register::<concepts::MovedDirection>()
        .register::<concepts::NewObjectRefs>()
        .register::<concepts::ObjectAsset>()
        .register::<concepts::SentObjects>()
        .register::<pipeline::starter::StarterState>()
        .register::<pipeline::starter::StarterStatus>()
        .register::<concepts::ConceptJobState>()
        .register::<concepts::ConceptJob>()
        .register::<concepts::ConceptEvent>()
        .register::<inspect::InspectView>()
        .register::<layout::SceneLayout>()
        .register::<layout::LayoutView>()
        .register::<layout::ObjectStates>()
        .register::<motion::MotionView>()
        .register::<materials::SceneMaterials>()
        .register::<prepare::ScenePrep>()
        .register::<prepare::preview::FixupReport>()
        .register::<prepare::Suggestion>()
        .register::<prepare::PrepFrame>()
        .register::<built::BuiltSource>()
        .register::<built::BuildRecord>()
        .register::<built::BuiltState>()
        .register::<built::BuiltAsset>()
        .register::<built::NewBuilt>()
        .register::<built::BuildResult>()
        .register::<built::BuildOutcome>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn types_registers_every_ipc_type() {
        let types = super::types();
        let names: Vec<String> = types
            .into_unsorted_iter()
            .map(|t| t.name.to_string())
            .collect();
        for expected in [
            "ErrorPayload",
            "ToolId",
            "ToolStatus",
            "ToolSource",
            "ToolReport",
            "HostOs",
            "WorkspaceConfig",
            "FolderLayout",
            "Topology",
            "TextureSize",
            "ProviderSettings",
            "TripoPresets",
            "TripoPreset",
            "TripoTextureQuality",
            "TripoGeometryQuality",
            "TripoRigSpec",
            "MeshyPresets",
            "MeshyPreset",
            "MeshyTexture",
            "PoseMode",
            "ProviderId",
            "PresetVariant",
            "ModelOption",
            "ProviderCatalog",
            "Provider",
            "JobState",
            "JobPhase",
            "CancelMode",
            "ProviderJob",
            "SendRequest",
            "InboxLink",
            "ProviderEvent",
            "CardRow",
            "HandoffCard",
            "BatchRequest",
            "BatchSent",
            "DownloadsScan",
            "KeyStatus",
            "KeyTest",
            "Budgets",
            "Compression",
            "TextureCodec",
            "MeshoptLevel",
            "AgentSettings",
            "AgentRules",
            "RootState",
            "RootInspection",
            "ProjectTemplate",
            "ProjectManifest",
            "NewProject",
            "RepoInspection",
            "BlendFile",
            "ImportScan",
            "AppState",
            "Asset",
            "AssetKind",
            "AssetStatus",
            "AssetVersion",
            "ModelStats",
            "NewAsset",
            "Stage",
            "StepKind",
            "CompressSpec",
            "StepDef",
            "PipelineDef",
            "CompressPreset",
            "IssueLevel",
            "PipelineIssue",
            "RunState",
            "StepState",
            "StepRecord",
            "RunRecord",
            "LogLevel",
            "LogStream",
            "LogEntry",
            "PipelineEvent",
            "RigKind",
            "TextureInfo",
            "PbrMaps",
            "JointCount",
            "FingerWeights",
            "Islands",
            "GlbReport",
            "CheckStatus",
            "Check",
            "InboxState",
            "InboxItem",
            "AcceptRequest",
            "AcceptResult",
            "ThumbState",
            "LabSettings",
            "Aspect",
            "ConceptImage",
            "Direction",
            "NewConcepts",
            "SentPicks",
            "ConceptJobState",
            "ConceptJob",
            "ConceptEvent",
            "InspectView",
            "InspectModel",
            "CameraStopDef",
            "SceneLayout",
            "Placement",
            "LayoutStop",
            "Desk",
            "LayoutView",
            "ObjectStates",
            "ObjectState",
            "PartPose",
            "StateTransition",
            "StateTrigger",
            "TriggerKind",
            "Easing",
            "ClipSettings",
            "MotionView",
            "MotionModel",
            "MotionBudget",
            "Clip",
            "Rig",
            "BoneMap",
            "BoneMatch",
            "LayoutModel",
            "FirstView",
            "Measured",
            "SceneMaterials",
            "ModelMaterials",
            "MaterialTweak",
            "RoughnessTweak",
            "FactorMode",
            "MapCodecs",
            "CodecSet",
            "BuiltSource",
            "BuildRecord",
            "BuiltState",
            "BuiltAsset",
            "NewBuilt",
            "BuildResult",
            "BuildOutcome",
        ] {
            assert!(names.iter().any(|n| n == expected), "missing {expected}");
        }
    }
}
