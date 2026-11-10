//! The two starters every project gets for built assets: the helper file
//! their scripts import and an example that uses each helper. They land in
//! `built/`, where those scripts live.
//!
//! Nothing about a build knows them. A script imports the helper file
//! because its asset lists it among its helpers, which stages the file with
//! the script and hashes it into the build key. So updating the starter
//! shows every asset that lists it as stale, and no other asset sees it.

use super::StarterScript;

/// The helpers a built asset's script imports, once the asset lists the file.
pub const BUILT_HELPERS: StarterScript = StarterScript {
    path: crate::built::HELPERS_FILE,
    text: include_str!("../../../starter/maquette_helpers.py"),
};
/// A built asset script that uses every helper once.
pub const BUILT_EXAMPLE: StarterScript = StarterScript {
    path: crate::built::EXAMPLE_FILE,
    text: include_str!("../../../starter/maquette_example.py"),
};
/// Both, in the order they follow a template's own scripts.
pub(super) static ALL: [StarterScript; 2] = [BUILT_HELPERS, BUILT_EXAMPLE];

#[cfg(test)]
mod blender_tests;
#[cfg(test)]
mod tests;
