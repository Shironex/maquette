//! The run log a pipeline run, a Lab job or a provider send writes, the
//! programs a run spawns, resolved from the toolchain check, and the ports
//! for Blender work the pipeline implements. They live in the maquette-base
//! crate, which registers their IPC types; this keeps the `base::history`,
//! `base::tools` and `base::ports` paths.

pub use maquette_base::{history, ports, tools};
