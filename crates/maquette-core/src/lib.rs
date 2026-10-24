//! Maquette domain logic: toolchain detection, the workspace config,
//! projects, the per machine app state and the jobs seam.

pub mod app_state;
pub mod error;
pub mod fs;
pub mod jobs;
mod json;
pub mod project;
pub mod toolchain;
pub mod workspace;

pub use app_state::AppState;
pub use error::{CoreError, CoreResult, ErrorPayload};
pub use jobs::{JobKind, JobRunner, JobSpec, JobState, UnavailableRunner};
pub use project::{
    BlendFile, ImportScan, NewProject, ProjectManifest, ProjectTemplate, RepoInspection,
};
pub use toolchain::{
    CommandRunner, HostOs, ProbeEnv, SystemRunner, ToolId, ToolReport, ToolSource, ToolStatus,
};
pub use workspace::{
    AgentRules, AgentSettings, Budgets, CharacterPreset, Compression, FolderLayout, MeshoptLevel,
    RigPreset, RootInspection, RootState, TextureCodec, TextureSize, Topology, TripoDefaults,
    WorkspaceConfig,
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
        .register::<JobKind>()
        .register::<JobState>()
        .register::<JobSpec>()
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
            "JobKind",
            "JobState",
            "JobSpec",
        ] {
            assert!(names.iter().any(|n| n == expected), "missing {expected}");
        }
    }
}
