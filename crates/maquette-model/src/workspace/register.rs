//! The workspace module's IPC types, kept apart so the module root stays
//! under the size cap.

use super::{
    AgentRules, AgentSettings, AvatarDefaults, Budgets, Compression, FolderLayout, LabSettings,
    MeshoptLevel, RootInspection, RootState, TextureCodec, TextureSize, Topology, WorkspaceConfig,
    sync,
};

/// Registers this module's IPC types (the workspace config and its root state) with the shell's
/// TypeScript export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<WorkspaceConfig>()
        .register::<FolderLayout>()
        .register::<Topology>()
        .register::<TextureSize>()
        .register::<Budgets>()
        .register::<Compression>()
        .register::<TextureCodec>()
        .register::<MeshoptLevel>()
        .register::<AgentSettings>()
        .register::<AgentRules>()
        .register::<RootState>()
        .register::<RootInspection>()
        .register::<sync::SyncConflict>()
        .register::<LabSettings>()
        .register::<AvatarDefaults>()
}
