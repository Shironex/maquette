//! Maquette domain logic: toolchain detection, the workspace config,
//! projects and their assets, the pipeline runner, model thumbnails and the
//! per machine app state.

pub mod app_state;
pub mod assets;
pub mod concepts;
pub mod error;
pub mod fs;
pub mod inbox;
mod json;
pub mod pipeline;
pub mod project;
pub mod thumbs;
pub mod toolchain;
pub mod workspace;

pub use app_state::AppState;
pub use assets::{Asset, AssetKind, AssetStatus, AssetVersion, ModelStats, NewAsset, Stage};
pub use error::{CoreError, CoreResult, ErrorPayload};
pub use project::{
    BlendFile, ImportScan, NewProject, ProjectManifest, ProjectTemplate, RepoInspection,
};
pub use toolchain::{
    CommandRunner, HostOs, ProbeEnv, SystemRunner, ToolId, ToolReport, ToolSource, ToolStatus,
};
pub use workspace::{
    AgentRules, AgentSettings, Budgets, CharacterPreset, Compression, FolderLayout, LabSettings,
    MeshoptLevel, RigPreset, RootInspection, RootState, TextureCodec, TextureSize, Topology,
    TripoDefaults, WorkspaceConfig,
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
        .register::<TripoDefaults>()
        .register::<Topology>()
        .register::<TextureSize>()
        .register::<CharacterPreset>()
        .register::<RigPreset>()
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
        .register::<NewProject>()
        .register::<RepoInspection>()
        .register::<BlendFile>()
        .register::<ImportScan>()
        .register::<AppState>()
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
        .register::<concepts::ConceptJobState>()
        .register::<concepts::ConceptJob>()
        .register::<concepts::ConceptEvent>()
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
            "TripoDefaults",
            "Topology",
            "TextureSize",
            "CharacterPreset",
            "RigPreset",
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
        ] {
            assert!(names.iter().any(|n| n == expected), "missing {expected}");
        }
    }
}
