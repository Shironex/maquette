//! The IPC surface. Every command returns `Result<_, ErrorPayload>` so the
//! generated TypeScript rejects with `{ code, message }` on failure.

pub mod about;
pub mod agents;
pub mod animations;
pub mod app;
pub mod assets;
pub mod avatar;
mod blender;
pub mod built;
pub mod codex_usage;
pub mod credits;
pub mod dialogs;
pub mod downloads;
pub mod game;
pub mod image_api;
pub mod inbox;
pub mod inspect;
pub mod lab;
pub mod layout;
pub mod library;
pub mod materials;
pub mod motion;
pub mod page;
pub mod parts;
pub mod paths;
pub mod pipeline;
pub mod prepare;
pub mod project;
pub mod providers;
pub mod quit;
pub mod refs;
pub mod ship;
pub mod site;
pub mod split;
pub mod thumbs;
pub mod timeline;
pub mod toolchain;
pub mod trash;
pub mod updater;
pub mod workspace;

use maquette_core::{CoreError, ErrorPayload};

pub type CommandResult<T> = Result<T, ErrorPayload>;

pub(crate) fn invalid(message: &str) -> ErrorPayload {
    CoreError::Invalid(message.to_string()).payload()
}

/// For `spawn_blocking` joins: the task only fails if it panicked.
pub(crate) fn join_failed(err: impl std::fmt::Display) -> ErrorPayload {
    CoreError::Unavailable(format!("A background task stopped unexpectedly. {err}")).payload()
}
