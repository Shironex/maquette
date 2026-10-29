//! The scene layout. `layout.json` and its checks live in the model crate;
//! the Inspector's read model of it ([`view`]) measures prepared models and
//! reads the built files, so it stays here.

pub use maquette_model::layout::*;

mod view;

pub use view::{
    FirstView, FirstViewModel, LayoutModel, LayoutView, Measured, first_view, first_view_of,
    models, placement_of, view, view_of,
};

pub(crate) use view::measured_file;

/// Registers the layout view's IPC types with the shell's TypeScript export;
/// the stored layout's come from the model crate.
pub fn register(types: specta::Types) -> specta::Types {
    types.register::<LayoutView>()
}
