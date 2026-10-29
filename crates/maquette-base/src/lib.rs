//! The bottom layer every other Maquette crate builds on: the error type
//! every fallible function returns, file and JSON helpers, the clock files
//! write timestamps with, the lookup of the external tools Maquette drives,
//! the run log a pipeline run, a Lab job or a provider send writes, the
//! programs a run spawns (the Toolbox) and a site's `maquette.site.json`. It
//! uses no other Maquette crate.

pub mod clock;
pub mod error;
pub mod fs;
pub mod history;
pub mod json;
pub mod site_config;
pub mod toolchain;
pub mod tools;

/// Registers this crate's IPC types (the error payload, the toolchain report, the step kinds and
/// the run log) with the shell's TypeScript export; `maquette_core::types` folds it in with the
/// core's.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<error::ErrorPayload>()
        .register::<toolchain::ToolId>()
        .register::<toolchain::ToolStatus>()
        .register::<toolchain::ToolSource>()
        .register::<toolchain::ToolReport>()
        .register::<toolchain::HostOs>()
        .register::<tools::StepKind>()
        .register::<history::RunState>()
        .register::<history::StepState>()
        .register::<history::StepRecord>()
        .register::<history::RunRecord>()
        .register::<history::LogLevel>()
        .register::<history::LogStream>()
        .register::<history::LogEntry>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn register_names_the_error_toolchain_and_run_log_types() {
        let types = super::register(specta::Types::default());
        let names: Vec<String> = types
            .into_unsorted_iter()
            .map(|t| t.name.to_string())
            .collect();
        for expected in [
            "ErrorPayload",
            "ToolId",
            "ToolReport",
            "HostOs",
            "StepKind",
            "RunRecord",
            "LogEntry",
        ] {
            assert!(names.iter().any(|n| n == expected), "missing {expected}");
        }
    }
}
