//! Maquette's model layer: the shared vocabulary stored in `maquette.json`,
//! `project.json` and the files next to it. The workspace config and its
//! lock, projects and their assets (with removing and restoring them), the
//! scene layout, an asset's refs, the timeline, the settings types the
//! layers above read, what the inbox and the provider sends share, and the
//! stored part of `prepare.json`, `built.json`, `materials.json`,
//! `avatar.json` and `ship.json`. Nothing here talks to a network service
//! or runs Blender; it uses only the maquette-base and maquette-glb crates.

pub mod assets;
pub mod avatar;
pub mod built;
pub mod layout;
pub mod materials;
pub mod prepare;
pub mod project;
pub mod refs;
pub mod sends;
pub mod settings;
pub mod ship;
pub mod timeline;
pub mod workspace;

// The bottom layer, at the paths the model code uses.
pub use maquette_base::{error, fs, json};

/// One `register` per model module, in this order: each adds the IPC types
/// it owns.
const REGISTRIES: [fn(specta::Types) -> specta::Types; 11] = [
    settings::register,
    sends::register,
    workspace::register,
    project::register,
    assets::register,
    layout::register,
    timeline::register,
    prepare::register,
    built::register,
    materials::register,
    avatar::register,
];

/// Registers this crate's IPC types with the shell's TypeScript export;
/// `maquette_core::types` folds it in with the core's.
pub fn register(types: specta::Types) -> specta::Types {
    REGISTRIES
        .into_iter()
        .fold(types, |types, register| register(types))
}

#[cfg(test)]
mod tests {
    #[test]
    fn register_names_the_stored_types() {
        let types = super::register(specta::Types::default());
        let names: Vec<String> = types
            .into_unsorted_iter()
            .map(|t| t.name.to_string())
            .collect();
        for expected in [
            "WorkspaceConfig",
            "ProjectManifest",
            "Asset",
            "SceneLayout",
            "ScenePrep",
            "BuiltSource",
            "SceneMaterials",
            "SceneAvatars",
            "AvatarDefaults",
            "ProviderSettings",
        ] {
            assert!(names.iter().any(|n| n == expected), "missing {expected}");
        }
    }
}
