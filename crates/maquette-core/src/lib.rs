//! Maquette domain logic: toolchain detection, the workspace config,
//! projects and their assets, the pipeline runner, model thumbnails and the
//! per machine app state.

pub mod animations;
pub mod app_state;
pub mod assets;
pub mod base;
pub mod built;
pub mod concepts;
pub mod game;
pub mod inbox;
pub mod inspect;
pub mod layout;
pub mod library;
pub mod materials;
pub mod motion;
pub mod page;
pub mod parts;
pub mod pipeline;
pub mod prepare;
pub mod project;
pub mod providers;
pub mod refs;
pub mod settings;
pub mod ship;
pub mod site;
#[cfg(test)]
mod test_blender;
pub mod thumbs;
pub mod timeline;
pub mod workspace;

// The bottom layer lives in its own crate; these keep the old paths.
pub(crate) use maquette_base::json;
pub use maquette_base::{error, fs, toolchain};

pub use app_state::AppState;
pub use assets::{
    Asset, AssetKind, AssetOrigin, AssetStatus, AssetVersion, ModelStats, NewAsset, Stage,
};
pub use error::{CoreError, CoreResult, ErrorPayload};
pub use game::{GameFormat, GameSettings, ProjectTarget};
pub use project::{
    BlendFile, ImportScan, NewProject, ProjectActivity, ProjectManifest, ProjectSummary,
    ProjectTemplate, RepoInspection, SiteFolders,
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
        .register::<workspace::sync::SyncConflict>()
        .register::<ProjectTemplate>()
        .register::<ProjectManifest>()
        .register::<ProjectSummary>()
        .register::<project::SiteRepoState>()
        .register::<ProjectActivity>()
        .register::<NewProject>()
        .register::<RepoInspection>()
        .register::<SiteFolders>()
        .register::<ProjectTarget>()
        .register::<GameFormat>()
        .register::<game::GameTextures>()
        .register::<game::GameExport>()
        .register::<game::FolderBudget>()
        .register::<game::GameModel>()
        .register::<GameSettings>()
        .register::<game::GameSettingsChange>()
        .register::<game::NewGame>()
        .register::<game::GameRepoInspection>()
        .register::<game::BudgetCheck>()
        .register::<game::ScannedModel>()
        .register::<game::GameScan>()
        .register::<game::ImportResult>()
        .register::<game::import::SkippedFile>()
        .register::<game::ShipChange>()
        .register::<game::ShipFile>()
        .register::<game::ship::ShipCheck>()
        .register::<game::ShipPlan>()
        .register::<BlendFile>()
        .register::<ImportScan>()
        .register::<AppState>()
        .register::<app_state::DownloadsWatch>()
        .register::<Asset>()
        .register::<AssetKind>()
        .register::<AssetStatus>()
        .register::<AssetVersion>()
        .register::<AssetOrigin>()
        .register::<library::LibrarySource>()
        .register::<library::VersionFrom>()
        .register::<library::LibraryVersion>()
        .register::<library::LibraryEntry>()
        .register::<library::LibraryUse>()
        .register::<library::LibraryItem>()
        .register::<library::LibraryView>()
        .register::<library::AddToLibrary>()
        .register::<library::LibraryAdded>()
        .register::<library::LibraryUsed>()
        .register::<library::SettingChange>()
        .register::<library::UpdatePlan>()
        .register::<library::LibraryUpdated>()
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
        .register::<layout::Timeline>()
        .register::<timeline::TimelineView>()
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
        .register::<page::PageSource>()
        .register::<page::PageSettings>()
        .register::<page::PageView>()
        .register::<site::preview::CommandSource>()
        .register::<site::preview::DevCommand>()
        .register::<site::preview::ServerState>()
        .register::<site::preview::ServerOwner>()
        .register::<site::preview::ServerStatus>()
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
            "AssetOrigin",
            "LibrarySource",
            "LibraryEntry",
            "LibraryView",
            "UpdatePlan",
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
            "Timeline",
            "ExplodeTrack",
            "ExplodeKey",
            "TimelineView",
            "TimelineModel",
            "ScenePath",
            "PathCurve",
            "PathTrack",
            "PathKey",
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
