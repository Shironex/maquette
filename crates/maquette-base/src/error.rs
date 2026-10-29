//! The error type every fallible core function returns.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Could not read or write {}. {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{} is not valid JSON. {source}", path.display())]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "This file comes from a newer Maquette (format {found}). This version reads up to format {supported}, so update Maquette to open it."
    )]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("This folder is not empty. Pick an empty folder or an existing Maquette workspace.")]
    NotEmpty(PathBuf),
    #[error("This folder is not a Maquette workspace. It has no maquette.json.")]
    NotAWorkspace(PathBuf),
    #[error("{0}")]
    Invalid(String),
    #[error("Could not find {0}.")]
    NotFound(String),
    #[error("{0}")]
    Unavailable(String),
}

impl CoreError {
    pub fn io(path: impl AsRef<Path>, source: io::Error) -> Self {
        Self::Io {
            path: path.as_ref().to_path_buf(),
            source,
        }
    }

    pub fn json(path: impl AsRef<Path>, source: serde_json::Error) -> Self {
        Self::Json {
            path: path.as_ref().to_path_buf(),
            source,
        }
    }

    /// Stable code the UI can switch on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::Json { .. } => "json",
            Self::UnsupportedVersion { .. } => "unsupported-version",
            Self::NotEmpty(_) => "not-empty",
            Self::NotAWorkspace(_) => "not-a-workspace",
            Self::Invalid(_) => "invalid",
            Self::NotFound(_) => "not-found",
            Self::Unavailable(_) => "unavailable",
        }
    }

    pub fn payload(&self) -> ErrorPayload {
        ErrorPayload {
            code: self.code().to_string(),
            message: self.to_string(),
        }
    }
}

/// Serializable form of [`CoreError`] for IPC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

impl From<CoreError> for ErrorPayload {
    fn from(err: CoreError) -> Self {
        err.payload()
    }
}

pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_messages() {
        let err = CoreError::NotEmpty(PathBuf::from("/x"));
        assert_eq!(err.code(), "not-empty");
        assert_eq!(
            err.to_string(),
            "This folder is not empty. Pick an empty folder or an existing Maquette workspace."
        );
        let payload = CoreError::NotAWorkspace(PathBuf::from("/x")).payload();
        assert_eq!(payload.code, "not-a-workspace");
        assert_eq!(
            CoreError::UnsupportedVersion {
                found: 2,
                supported: 1
            }
            .code(),
            "unsupported-version"
        );
    }
}
