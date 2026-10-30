//! Avatars. `avatar.json` and its specs live in the model crate; freeing
//! cloth and hair from the arms in Blender ([`reweight`]), normalising a
//! rigged model's rest pose for VRM there ([`normalize`]), finding and
//! chaining the parts that swing ([`springs`]), softening the highlights
//! baked into an MToon export's texture ([`highlights`]) and building the
//! `.vrm` export ([`export`]) stay here.

pub use maquette_model::avatar::*;

pub mod export;
mod gate;
pub mod highlights;
pub mod normalize;
pub mod reweight;
pub mod springs;
