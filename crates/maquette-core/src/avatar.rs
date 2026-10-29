//! Avatars. `avatar.json` and its specs live in the model crate; normalising
//! a rigged model's rest pose for VRM in Blender stays here ([`normalize`]).

pub use maquette_model::avatar::*;

pub mod normalize;
