//! What the shell remembers between commands: the per machine app state and
//! the workspace that is open right now.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use maquette_core::{AppState, CoreError, CoreResult, WorkspaceConfig, app_state};

pub struct OpenWorkspace {
    pub root: PathBuf,
    pub config: WorkspaceConfig,
}

#[derive(Default)]
struct Inner {
    app: AppState,
    workspace: Option<OpenWorkspace>,
}

pub struct Session {
    config_dir: PathBuf,
    home: Option<PathBuf>,
    inner: Mutex<Inner>,
}

impl Session {
    pub fn load(config_dir: PathBuf) -> Self {
        let app = app_state::load(&config_dir);
        Self::with_state(config_dir, std::env::home_dir(), app)
    }

    pub fn with_state(config_dir: PathBuf, home: Option<PathBuf>, app: AppState) -> Self {
        Self {
            config_dir,
            home,
            inner: Mutex::new(Inner {
                app,
                workspace: None,
            }),
        }
    }

    pub fn home(&self) -> Option<&Path> {
        self.home.as_deref()
    }

    pub fn expand(&self, input: &str) -> PathBuf {
        maquette_core::fs::expand_home(input.trim(), self.home())
    }

    pub fn display(&self, path: &Path) -> String {
        maquette_core::fs::collapse_home(path, self.home())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // A panic while holding the lock leaves plain data behind, never a
        // half written file, so the state is still safe to use.
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn app_state(&self) -> AppState {
        self.lock().app.clone()
    }

    pub fn update_app_state(&self, change: impl FnOnce(&mut AppState)) -> CoreResult<AppState> {
        let mut inner = self.lock();
        let mut next = inner.app.clone();
        change(&mut next);
        app_state::save(&self.config_dir, &next)?;
        inner.app = next.clone();
        Ok(next)
    }

    pub fn set_workspace(&self, root: PathBuf, config: WorkspaceConfig) -> CoreResult<()> {
        let pointer = root.display().to_string();
        self.update_app_state(|state| state.last_workspace = Some(pointer))?;
        self.lock().workspace = Some(OpenWorkspace { root, config });
        Ok(())
    }

    pub fn set_config(&self, config: WorkspaceConfig) {
        if let Some(open) = self.lock().workspace.as_mut() {
            open.config = config;
        }
    }

    /// The open workspace, or the error every workspace command returns without one.
    pub fn workspace(&self) -> CoreResult<(PathBuf, WorkspaceConfig)> {
        self.lock()
            .workspace
            .as_ref()
            .map(|open| (open.root.clone(), open.config.clone()))
            .ok_or_else(|| CoreError::Invalid("No workspace is open yet.".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_workspace_remembers_the_pointer_on_disk() {
        let dir = tempfile::tempdir().expect("temp dir");
        let session = Session::with_state(dir.path().to_path_buf(), None, AppState::default());
        let root = dir.path().join("Maquette");
        session
            .set_workspace(root.clone(), WorkspaceConfig::default())
            .expect("set workspace");

        let reloaded = app_state::load(dir.path());
        assert_eq!(reloaded.last_workspace, Some(root.display().to_string()));
        assert_eq!(session.workspace().expect("open").0, root);
    }

    #[test]
    fn workspace_commands_fail_cleanly_before_one_is_open() {
        let dir = tempfile::tempdir().expect("temp dir");
        let session = Session::with_state(dir.path().to_path_buf(), None, AppState::default());
        assert_eq!(
            session.workspace().expect_err("none open").code(),
            "invalid"
        );
    }
}
