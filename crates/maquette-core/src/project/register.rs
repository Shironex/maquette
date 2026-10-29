//! The project module's IPC types, kept apart so the module root stays
//! under the size cap.

use super::{
    BlendFile, ImportScan, NewProject, ProjectActivity, ProjectManifest, ProjectSummary,
    ProjectTemplate, RepoInspection, SiteFolders, SiteRepoState,
};

/// Registers this module's IPC types (the project manifest, its summary and the import scan) with
/// the shell's TypeScript export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<ProjectTemplate>()
        .register::<ProjectManifest>()
        .register::<ProjectSummary>()
        .register::<SiteRepoState>()
        .register::<ProjectActivity>()
        .register::<NewProject>()
        .register::<RepoInspection>()
        .register::<SiteFolders>()
        .register::<BlendFile>()
        .register::<ImportScan>()
}
