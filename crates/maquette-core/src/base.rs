//! The run log a pipeline run, a Lab job or a provider send writes, and the
//! programs a run spawns, resolved from the toolchain check. Both live in
//! the maquette-base crate, which registers their IPC types; this keeps the
//! `base::history` and `base::tools` paths.

pub use maquette_base::{history, tools};
