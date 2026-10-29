//! Real size and fix-ups. `prepare.json` and its checks live in the model
//! crate; the fix-up script and the preview that runs it in Blender
//! ([`preview`]) stay here.

pub use maquette_model::prepare::*;

#[cfg(test)]
mod pad_tests;
pub mod preview;
#[cfg(test)]
mod rotate_tests;
#[cfg(test)]
mod split_tests;

/// The fix-up script, a starter in `tools/` that Build scene imports.
pub const SCRIPT_PATH: &str = "tools/fixups.py";
pub const SCRIPT: &str = include_str!("../starter/fixups.py");

/// Registers the fix-up report's IPC type with the shell's TypeScript
/// export; the stored prep's come from the model crate.
pub fn register(types: specta::Types) -> specta::Types {
    types.register::<preview::FixupReport>()
}
