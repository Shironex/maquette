//! The settings types stored in `maquette.json` and `project.json` that the
//! modules below the providers, the Lab and the pipeline read: the provider
//! presets and model catalog, the Lab's image settings, a game project's
//! settings and a site's `maquette.site.json`.

pub mod catalog;
pub mod game;
pub mod images;
pub mod presets;
// A site's config is read by the preview crate too, so it lives in the base
// crate; this keeps the old path.
pub use maquette_base::site_config as site;

/// Registers this module's IPC types (the provider presets, the model catalog and a game project's
/// settings) with the shell's TypeScript export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<presets::ProviderSettings>()
        .register::<presets::TripoPresets>()
        .register::<presets::TripoPreset>()
        .register::<presets::TripoTextureQuality>()
        .register::<presets::TripoGeometryQuality>()
        .register::<presets::TripoRigSpec>()
        .register::<presets::MeshyPresets>()
        .register::<presets::MeshyPreset>()
        .register::<presets::MeshyTexture>()
        .register::<presets::PoseMode>()
        .register::<presets::ProviderId>()
        .register::<presets::PresetVariant>()
        .register::<catalog::ModelOption>()
        .register::<catalog::ProviderCatalog>()
        .register::<game::ProjectTarget>()
        .register::<game::GameFormat>()
        .register::<game::GameTextures>()
        .register::<game::GameExport>()
        .register::<game::FolderBudget>()
        .register::<game::GameModel>()
        .register::<game::GameSettings>()
        .register::<game::NewGame>()
}
