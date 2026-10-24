//! The seam the pipeline runner plugs into. Commands build a [`JobSpec`] and
//! hand it to a [`JobRunner`]; progress comes back as [`JobState`] values.
//! This version ships only [`UnavailableRunner`], which refuses every job, so
//! the shell and the UI can be wired against the real shapes before the
//! runner exists.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum JobKind {
    ConceptImages,
    TripoModel,
    BlenderScript,
    Compress,
    Ship,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum JobState {
    Queued,
    /// `progress` runs from 0 to 1.
    Running {
        progress: f32,
    },
    Done,
    Cached,
    Failed {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct JobSpec {
    pub id: String,
    pub kind: JobKind,
    /// Project id.
    pub project: String,
    pub label: String,
}

pub trait JobRunner: Send + Sync {
    fn submit(&self, spec: JobSpec) -> CoreResult<()>;
    fn cancel(&self, id: &str) -> CoreResult<()>;
}

pub const UNAVAILABLE_MESSAGE: &str = "The pipeline runner arrives in a later version.";

#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableRunner;

impl JobRunner for UnavailableRunner {
    fn submit(&self, _spec: JobSpec) -> CoreResult<()> {
        Err(CoreError::Unavailable(UNAVAILABLE_MESSAGE.to_string()))
    }

    fn cancel(&self, _id: &str) -> CoreResult<()> {
        Err(CoreError::Unavailable(UNAVAILABLE_MESSAGE.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_json_shape() {
        assert_eq!(
            serde_json::to_string(&JobState::Running { progress: 0.5 }).expect("json"),
            r#"{"state":"running","progress":0.5}"#
        );
        assert_eq!(
            serde_json::to_string(&JobState::Queued).expect("json"),
            r#"{"state":"queued"}"#
        );
        assert_eq!(
            serde_json::to_string(&JobKind::ConceptImages).expect("json"),
            "\"concept-images\""
        );
    }

    #[test]
    fn unavailable_runner_refuses() {
        let spec = JobSpec {
            id: "j1".into(),
            kind: JobKind::Compress,
            project: "moon-island".into(),
            label: "Compress".into(),
        };
        let err = UnavailableRunner.submit(spec).expect_err("refused");
        assert_eq!(err.code(), "unavailable");
        assert_eq!(err.to_string(), UNAVAILABLE_MESSAGE);
        assert!(UnavailableRunner.cancel("j1").is_err());
    }
}
