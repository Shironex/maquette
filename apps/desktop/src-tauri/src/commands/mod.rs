//! The IPC surface. Every command returns `Result<_, ErrorPayload>` so the
//! generated TypeScript rejects with `{ code, message }` on failure.

pub mod agents;
pub mod app;
pub mod assets;
mod blender;
pub mod built;
pub mod credits;
pub mod dialogs;
pub mod downloads;
pub mod inbox;
pub mod inspect;
pub mod lab;
pub mod layout;
pub mod materials;
pub mod pipeline;
pub mod prepare;
pub mod project;
pub mod providers;
pub mod quit;
pub mod site;
pub mod thumbs;
pub mod toolchain;
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
