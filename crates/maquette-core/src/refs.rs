//! An asset's refs and the view each one shows. The views stored on the
//! asset live in the model crate; taking a Lab candidate into the refs and
//! reading which refs the Lab picked for an asset read the Lab's directions,
//! so they stay here.

pub use maquette_model::refs::*;

mod files;
mod suggested;

pub use files::{ORIGINALS_DIR, OldView, Replaced, UsedView, use_clean_copy, use_view_image};
pub use suggested::{ref_files, suggested};
