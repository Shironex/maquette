//! The settings types stored in `maquette.json` and `project.json` that the
//! modules below the providers, the Lab and the pipeline read: the provider
//! presets and model catalog, the Lab's image settings, a game project's
//! settings and a site's `maquette.site.json`.

pub mod catalog;
pub mod game;
pub mod images;
pub mod presets;
pub mod site;
