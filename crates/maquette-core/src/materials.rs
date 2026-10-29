//! Material tweaks. `materials.json` and its patches live in the model
//! crate; applying the tweaks to a model file and marking small maps stay
//! here with the pipeline presets they read.

pub use maquette_model::materials::*;

mod apply;
mod glb;
mod split;
#[cfg(test)]
mod tests;

pub use apply::{Applied, apply_file, apply_to_json, map_groups, material_names};
pub use split::{mark_small_maps, unmark_small_maps};
