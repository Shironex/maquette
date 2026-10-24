//! The seam for the pipeline runner. The command and its types are real so
//! the UI can call it today; the runner behind it refuses every job.

use maquette_core::{JobRunner as _, JobSpec, UnavailableRunner};

use super::CommandResult;

#[tauri::command]
#[specta::specta]
pub fn pipeline_submit(spec: JobSpec) -> CommandResult<()> {
    Ok(UnavailableRunner.submit(spec)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::JobKind;

    #[test]
    fn the_runner_seam_refuses_jobs_for_now() {
        let spec = JobSpec {
            id: "job-1".to_string(),
            kind: JobKind::Compress,
            project: "moon-island".to_string(),
            label: "Compress".to_string(),
        };
        let err = pipeline_submit(spec).expect_err("no runner yet");
        assert_eq!(err.code, "unavailable");
    }
}
