//! Built assets. `built.json`, adding a built asset and its state live in
//! the model crate; building one in Blender stays here.

pub use maquette_model::built::*;

mod blender;
mod build;
#[cfg(test)]
mod tests;

pub use blender::{LANE, work_dir};
pub use build::{BuildContext, BuildOutcome, BuildResult, Builder};
pub(crate) use build::{add_version as add_model_version, warnings as checked};

/// Registers the build's IPC types with the shell's TypeScript export; the
/// stored built assets' come from the model crate.
pub fn register(types: specta::Types) -> specta::Types {
    types.register::<BuildResult>().register::<BuildOutcome>()
}
