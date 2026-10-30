//! Avatars. `avatar.json` and its specs live in the model crate; normalising
//! a rigged model's rest pose for VRM in Blender ([`normalize`]) and
//! building the `.vrm` export ([`export`]) stay here.

pub use maquette_model::avatar::*;

pub mod export;
pub mod normalize;
