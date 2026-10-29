//! What every layer of the core shares beyond errors and files: the run
//! log a pipeline run, a Lab job or a provider send writes, and the
//! programs a run spawns, resolved from the toolchain check.

pub mod history;
pub mod tools;

/// Registers this module's IPC types (the step kinds and the run log) with the shell's TypeScript
/// export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<tools::StepKind>()
        .register::<history::RunState>()
        .register::<history::StepState>()
        .register::<history::StepRecord>()
        .register::<history::RunRecord>()
        .register::<history::LogLevel>()
        .register::<history::LogStream>()
        .register::<history::LogEntry>()
}
