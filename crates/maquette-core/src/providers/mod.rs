//! Sending a picked concept or ref to a 3D service: Tripo and Meshy over
//! their REST APIs, or by hand through a handoff folder.

pub mod catalog;
pub mod http;
pub mod presets;

pub use catalog::{ModelOption, ProviderCatalog};
pub use presets::{
    MeshyPreset, MeshyPresets, MeshyTexture, PoseMode, PresetVariant, ProviderId, ProviderSettings,
    TripoGeometryQuality, TripoPreset, TripoPresets, TripoRigSpec, TripoTextureQuality,
    meshy_problem, tripo_problem,
};
