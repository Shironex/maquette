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
// The Page view's settings live with the preview server they start.
pub use maquette_base::{error, fs, toolchain};
pub use maquette_preview::page;

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

/// One `register` per crate and per core module, in this order: each adds
/// the IPC types it owns, so a new type touches only its own module.
const REGISTRIES: [fn(specta::Types) -> specta::Types; 22] = [
    maquette_base::register,
    maquette_glb::register,
    app_state::register,
    settings::register,
    workspace::register,
    project::register,
    assets::register,
    layout::register,
    timeline::register,
    motion::register,
    inbox::register,
    pipeline::register,
    prepare::register,
    built::register,
    thumbs::register,
    materials::register,
    inspect::register,
    game::register,
    providers::register,
    concepts::register,
    library::register,
    maquette_preview::register,
];

/// Every IPC facing type, so the shell can export them before any command
/// uses them.
pub fn types() -> specta::Types {
    REGISTRIES
        .into_iter()
        .fold(specta::Types::default(), |types, register| register(types))
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
