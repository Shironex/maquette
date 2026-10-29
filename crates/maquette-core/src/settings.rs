//! The settings types stored in `maquette.json` and `project.json` that the
//! modules below the providers, the Lab and the pipeline read: the provider
//! presets and model catalog, the Lab's image settings, a game project's
//! settings and a site's `maquette.site.json`.

pub mod catalog;
pub mod game;
pub mod images;
pub mod presets;
// A site's config is read by the preview crate too, so it lives in the base
// crate; this keeps the old path.
pub use maquette_base::site_config as site;
