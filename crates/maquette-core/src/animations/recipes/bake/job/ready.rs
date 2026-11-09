//! Whether a recipe can be baked at all: what of version 2 the bake does
//! not make yet, and the blanks nothing fills.

use crate::animations::recipes::RecipeDoc;

/// How a refusal of a recipe whose version 2 features a bake cannot make
/// yet starts, so an agent's answer can carry its own code.
pub const NOT_BAKEABLE_YET: &str = "This Maquette cannot bake";

/// Why `doc` cannot be baked yet: it uses pins, pose keys or grips, which
/// the bake does not make yet. None for a recipe a bake takes (joined clips,
/// trims, mirror and time edits bake).
pub fn not_bakeable(doc: &RecipeDoc) -> Option<String> {
    let unsupported = crate::recipes::bake_unsupported(&doc.recipe);
    (!unsupported.is_empty()).then(|| {
        format!(
            "{NOT_BAKEABLE_YET} {} yet: it uses {}. Take those out to bake it now; a later Maquette bakes them.",
            doc.recipe.name,
            unsupported.join(", ")
        )
    })
}

/// The blanks of a recipe nothing fills yet, by id.
pub fn open_blanks(doc: &RecipeDoc) -> Vec<String> {
    doc.recipe
        .blanks
        .iter()
        .filter(|b| !b.filled())
        .map(|b| b.id.clone())
        .collect()
}
