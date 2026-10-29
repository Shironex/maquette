//! What every layer of the core shares beyond errors and files: the run
//! log a pipeline run, a Lab job or a provider send writes, and the
//! programs a run spawns, resolved from the toolchain check.

pub mod history;
pub mod tools;
