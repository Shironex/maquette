//! What the shell remembers between commands: the per machine app state and
//! the workspace that is open right now.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use maquette_core::inbox::{self, watch};
use maquette_core::project::repos;
use maquette_core::{AppState, CoreError, CoreResult, ToolReport, WorkspaceConfig, app_state};
use tauri::{AppHandle, Manager as _};
use tauri_specta::Event as _;

use crate::events::InboxChanged;

pub struct OpenWorkspace {
    pub root: PathBuf,
    pub config: WorkspaceConfig,
}

#[derive(Default)]
struct Inner {
    app: AppState,
    workspace: Option<OpenWorkspace>,
    /// The last tool check, reused by runs until a tool changes.
    tools: Option<Vec<ToolReport>>,
    watcher: Option<watch::InboxWatcher>,
}

pub struct Session {
    config_dir: PathBuf,
    home: Option<PathBuf>,
    inner: Mutex<Inner>,
    /// Set once the window exists; tests run without one.
    handle: OnceLock<AppHandle>,
    /// This computer's name, read once.
    host_name: OnceLock<String>,
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
                ..Inner::default()
            }),
            handle: OnceLock::new(),
            host_name: OnceLock::new(),
        }
    }

    /// The computer's name, as the workspace lock shows it.
    pub fn host_name(&self) -> String {
        self.host_name
            .get_or_init(maquette_core::workspace::lock::host_name)
            .clone()
    }

    /// This app's version, as the workspace lock records it.
    pub fn app_version(&self) -> String {
        self.handle.get().map_or_else(
            || env!("CARGO_PKG_VERSION").to_string(),
            |h| h.package_info().version.to_string(),
        )
    }

    /// Gives the session the app, so opening a workspace can watch its
    /// inbox and let the web view load its models.
    pub fn attach(&self, handle: AppHandle) {
        let _ = self.handle.set(handle);
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
        if let Ok((previous, _)) = self.workspace()
            && previous != root
        {
            crate::workspace_lock::release(self, &previous);
        }
        crate::workspace_lock::claim(self, &root);
        self.load_repos(&root, &config);
        let watcher = self.watch_inbox(&root, &config);
        {
            let mut inner = self.lock();
            inner.workspace = Some(OpenWorkspace {
                root: root.clone(),
                config: config.clone(),
            });
            // Replacing the watcher drops (stops) the previous workspace's one.
            inner.watcher = watcher;
        }
        // Send to 3D jobs of this workspace load, and running ones resume.
        if let Some(studio) = self
            .handle
            .get()
            .and_then(|h| h.try_state::<crate::commands::providers::Studio>())
        {
            studio.0.open_workspace(&root, &config);
        }
        if let Some(handle) = self.handle.get() {
            crate::commands::split::bakes_after_open(handle, &root, &config);
        }
        Ok(())
    }

    /// Hands this computer's repo paths for `root` to the core and moves an
    /// older `project.json`'s path into them. A failed move is logged: the
    /// old path still reads until the next open tries again.
    fn load_repos(&self, root: &Path, config: &WorkspaceConfig) {
        let key = repos::root_key(root);
        let mine = self.lock().app.site_repos.get(&key).cloned();
        repos::install(root, mine.unwrap_or_default());
        match repos::migrate(root, &config.layout) {
            Ok(moved) if !moved.is_empty() => {
                tracing::info!(projects = ?moved, "moved site repos into this computer's state");
            }
            Ok(_) => {}
            Err(err) => tracing::warn!(error = %err, "could not move the site repos"),
        }
        if let Err(err) = self.save_repos(root) {
            tracing::warn!(error = %err, "could not save the site repos");
        }
    }

    /// Saves this computer's repo paths for `root` after a change.
    pub fn save_repos(&self, root: &Path) -> CoreResult<()> {
        let key = repos::root_key(root);
        let entries = repos::entries(root);
        if self.lock().app.site_repos.get(&key) == Some(&entries) {
            return Ok(());
        }
        self.update_app_state(move |state| {
            if entries.is_empty() {
                state.site_repos.remove(&key);
            } else {
                state.site_repos.insert(key, entries);
            }
        })
        .map(|_| ())
    }

    /// Lets the web view read the workspace's files through the asset
    /// protocol and starts the inbox watcher. Without an app (tests) this
    /// does nothing.
    fn watch_inbox(&self, root: &Path, config: &WorkspaceConfig) -> Option<watch::InboxWatcher> {
        let handle = self.handle.get()?.clone();
        if let Err(err) = crate::asset_scope::open_workspace(&handle, root) {
            tracing::warn!(error = %err, "could not open the workspace to the asset protocol");
        }
        let dir = inbox::inbox_dir(root, config);
        let emitter = handle.clone();
        match watch::watch(&dir, watch::QUIET, move |paths| {
            let paths = paths.iter().map(|p| p.display().to_string()).collect();
            if let Err(err) = (InboxChanged { paths }).emit(&emitter) {
                tracing::warn!(error = %err, "could not emit an inbox event");
            }
        }) {
            Ok(watcher) => Some(watcher),
            Err(err) => {
                tracing::warn!(error = %err, "the inbox is not watched");
                None
            }
        }
    }

    pub fn tools(&self) -> Option<Vec<ToolReport>> {
        self.lock().tools.clone()
    }

    /// Remembers a tool check, or forgets it with `None` after a change.
    pub fn set_tools(&self, reports: Option<Vec<ToolReport>>) {
        self.lock().tools = reports;
    }

    /// True while the open workspace's inbox is watched.
    pub fn inbox_watched(&self) -> bool {
        self.lock().watcher.is_some()
    }

    pub fn set_config(&self, config: WorkspaceConfig) {
        if let Some(studio) = self
            .handle
            .get()
            .and_then(|h| h.try_state::<crate::commands::providers::Studio>())
        {
            studio.0.set_config(&config);
        }
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
    fn opening_a_workspace_moves_old_repo_paths_into_the_app_state() {
        use maquette_core::{FolderLayout, workspace};
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("ws");
        let repo = dir.path().join("site");
        std::fs::create_dir_all(&repo).expect("repo");
        let project = root.join(&FolderLayout::default().projects).join("studio");
        std::fs::create_dir_all(&project).expect("project");
        let old = format!(
            r#"{{"version":1,"id":"studio","name":"Studio","template":"room","siteRepo":{},"sourceFolder":null,"createdAt":"2026-01-01T00:00:00.000Z"}}"#,
            serde_json::to_string(&repo.display().to_string()).expect("json")
        );
        std::fs::write(project.join("project.json"), old).expect("old file");

        let cfg = dir.path().join("cfg");
        let session = Session::with_state(cfg.clone(), None, AppState::default());
        session
            .set_workspace(root.clone(), config.clone())
            .expect("open");
        let saved = app_state::load(&cfg);
        let mine = saved
            .site_repos
            .get(&repos::root_key(&root))
            .expect("repos");
        assert_eq!(mine.get("studio"), Some(&repo.display().to_string()));
        let text = std::fs::read_to_string(project.join("project.json")).expect("read");
        assert!(text.contains(r#""siteRepo": null"#), "{text}");

        // A later start reads it from the app state alone.
        repos::install(&root, Default::default());
        let again = Session::with_state(cfg.clone(), None, app_state::load(&cfg));
        again.set_workspace(root.clone(), config).expect("reopen");
        let manifest =
            maquette_core::project::load(&root, &FolderLayout::default(), "studio").expect("load");
        assert_eq!(manifest.site_repo, Some(repo.display().to_string()));
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
