//! Whether a recipe can be baked at all: the blanks nothing fills. Every
//! kind of change bakes, and a file of a newer shape is refused when it is
//! read, so nothing else stops a bake here.

use crate::animations::recipes::RecipeDoc;

/// The blanks of a recipe nothing fills yet, by id.
pub fn open_blanks(doc: &RecipeDoc) -> Vec<String> {
    doc.recipe
        .blanks
        .iter()
        .filter(|b| !b.filled())
        .map(|b| b.id.clone())
        .collect()
}
