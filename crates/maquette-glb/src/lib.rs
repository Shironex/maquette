//! Reading GLB and glTF files without Blender: the analysis the inbox
//! checks run on (counts, textures, the rig and two geometry heuristics),
//! texture sizes from image headers, the matrix math they share, and small
//! generated fixture models with known flaws. It is pure logic on bytes and
//! uses only the maquette-base crate.

pub mod analyse;
pub mod fixture;
pub mod images;
pub mod math;

/// Registers this crate's IPC types (the GLB report the inbox shows) with the shell's TypeScript
/// export; `maquette_core::types` folds it in with the core's.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<analyse::RigKind>()
        .register::<analyse::TextureInfo>()
        .register::<analyse::PbrMaps>()
        .register::<analyse::JointCount>()
        .register::<analyse::FingerWeights>()
        .register::<analyse::Islands>()
        .register::<analyse::GlbReport>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn register_names_the_glb_report_types() {
        let types = super::register(specta::Types::default());
        let names: Vec<String> = types
            .into_unsorted_iter()
            .map(|t| t.name.to_string())
            .collect();
        for expected in ["RigKind", "TextureInfo", "PbrMaps", "GlbReport", "Islands"] {
            assert!(names.iter().any(|n| n == expected), "missing {expected}");
        }
    }
}
