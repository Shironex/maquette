//! Finds the external tools Maquette drives and reads their versions.
//!
//! Lookup order per tool: a caller supplied override, then PATH, then known
//! install locations. PATH lookup happens in Rust, never through `which` or
//! `where`, so it behaves the same when a macOS app starts from Finder with a
//! minimal PATH.

mod detection;
mod lookup;
mod runner;
pub mod version;

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use lookup::{known_dirs, push_unique};

pub use detection::{blocks_continue, detect, detect_all};
pub use lookup::{find_on_path, known_locations};
pub use runner::{CommandRunner, ProbeError, ProbeOutput, SystemRunner};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum ToolId {
    Blender,
    Codex,
    Ffmpeg,
    Toktx,
    GltfTransform,
    Node,
}

impl ToolId {
    pub const ALL: [ToolId; 6] = [
        ToolId::Blender,
        ToolId::Codex,
        ToolId::Ffmpeg,
        ToolId::Toktx,
        ToolId::GltfTransform,
        ToolId::Node,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ToolId::Blender => "Blender",
            ToolId::Codex => "Codex CLI",
            ToolId::Ffmpeg => "ffmpeg",
            ToolId::Toktx => "KTX-Software toktx",
            ToolId::GltfTransform => "gltf-transform",
            ToolId::Node => "Node",
        }
    }

    pub fn required(self) -> bool {
        matches!(self, ToolId::Blender)
    }

    fn binary_name(self) -> &'static str {
        match self {
            ToolId::Blender => "blender",
            ToolId::Codex => "codex",
            ToolId::Ffmpeg => "ffmpeg",
            ToolId::Toktx => "toktx",
            ToolId::GltfTransform => "gltf-transform",
            ToolId::Node => "node",
        }
    }

    fn version_args(self) -> &'static [&'static str] {
        match self {
            ToolId::Blender => &["--background", "--factory-startup", "--version"],
            ToolId::Ffmpeg => &["-version"],
            _ => &["--version"],
        }
    }

    fn timeout(self) -> Duration {
        match self {
            ToolId::Blender => Duration::from_secs(20),
            _ => Duration::from_secs(8),
        }
    }

    fn uses_node(self) -> bool {
        matches!(self, ToolId::Codex | ToolId::GltfTransform | ToolId::Node)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ToolStatus {
    Found,
    Missing,
    /// A binary exists but `--version` failed or timed out.
    Broken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ToolSource {
    Override,
    Path,
    KnownLocation,
    Npx,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ToolReport {
    pub id: ToolId,
    pub label: String,
    pub required: bool,
    pub status: ToolStatus,
    pub source: Option<ToolSource>,
    pub path: Option<String>,
    pub version: Option<String>,
    pub detail: Option<String>,
    pub hint: Option<String>,
    /// Set when the tool works but needs attention, for example Codex CLI
    /// that is not signed in, or an override that no longer exists.
    pub warning: Option<String>,
}

impl ToolReport {
    fn new(id: ToolId, status: ToolStatus) -> Self {
        Self {
            id,
            label: id.label().to_string(),
            required: id.required(),
            status,
            source: None,
            path: None,
            version: None,
            detail: None,
            hint: None,
            warning: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum HostOs {
    Mac,
    Windows,
    Linux,
}

impl HostOs {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            HostOs::Mac
        } else if cfg!(windows) {
            HostOs::Windows
        } else {
            HostOs::Linux
        }
    }
}

/// Everything detection reads from the machine. A test seam: tests build one
/// by hand around temp folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeEnv {
    pub os: HostOs,
    pub path_dirs: Vec<PathBuf>,
    /// Windows executable extensions, for example `.EXE`. Empty means the
    /// Windows default `.COM;.EXE;.BAT;.CMD`.
    pub pathext: Vec<String>,
    pub home: Option<PathBuf>,
    /// `ProgramFiles`, `ProgramW6432` and `ProgramFiles(x86)` on Windows.
    pub program_files: Vec<PathBuf>,
    pub local_app_data: Option<PathBuf>,
    pub app_data: Option<PathBuf>,
    pub program_data: Option<PathBuf>,
    /// `SystemDrive`, for example `C:`.
    pub system_drive: Option<String>,
    pub codex_home: Option<PathBuf>,
    /// Folders that hold `.app` bundles on macOS.
    pub applications_dirs: Vec<PathBuf>,
    /// Package manager bin folders, for example Homebrew's.
    pub system_bin_dirs: Vec<PathBuf>,
}

impl ProbeEnv {
    /// An environment with nothing in it, for tests.
    pub fn empty(os: HostOs) -> Self {
        Self {
            os,
            path_dirs: Vec::new(),
            pathext: Vec::new(),
            home: None,
            program_files: Vec::new(),
            local_app_data: None,
            app_data: None,
            program_data: None,
            system_drive: None,
            codex_home: None,
            applications_dirs: Vec::new(),
            system_bin_dirs: Vec::new(),
        }
    }

    pub fn from_system() -> Self {
        let os = HostOs::current();
        let var_path = |name: &str| {
            env::var_os(name)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let home = if os == HostOs::Windows {
            var_path("USERPROFILE").or_else(|| var_path("HOME"))
        } else {
            var_path("HOME").or_else(|| var_path("USERPROFILE"))
        };
        let path_dirs = env::var_os("PATH")
            .map(|p| env::split_paths(&p).collect())
            .unwrap_or_default();
        let pathext = env::var("PATHEXT")
            .map(|v| {
                v.split(';')
                    .filter(|e| !e.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let mut program_files = Vec::new();
        for name in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
            if let Some(dir) = var_path(name) {
                push_unique(&mut program_files, dir);
            }
        }

        let (applications_dirs, system_bin_dirs) = match os {
            HostOs::Mac => {
                let mut apps = vec![PathBuf::from("/Applications")];
                if let Some(home) = &home {
                    apps.push(home.join("Applications"));
                }
                (
                    apps,
                    vec![
                        PathBuf::from("/opt/homebrew/bin"),
                        PathBuf::from("/usr/local/bin"),
                    ],
                )
            }
            HostOs::Linux => (
                Vec::new(),
                vec![
                    PathBuf::from("/usr/local/bin"),
                    PathBuf::from("/usr/bin"),
                    PathBuf::from("/snap/bin"),
                    PathBuf::from("/home/linuxbrew/.linuxbrew/bin"),
                ],
            ),
            HostOs::Windows => (Vec::new(), Vec::new()),
        };

        Self {
            os,
            path_dirs,
            pathext,
            home,
            program_files,
            local_app_data: var_path("LOCALAPPDATA"),
            app_data: var_path("APPDATA"),
            program_data: var_path("ProgramData"),
            system_drive: env::var("SystemDrive").ok().filter(|v| !v.is_empty()),
            codex_home: var_path("CODEX_HOME"),
            applications_dirs,
            system_bin_dirs,
        }
    }

    /// PATH for child processes: `extra_first`, then PATH, then every known
    /// bin folder that exists. Node shebang scripts such as codex and
    /// gltf-transform need node on it even when the app has a minimal PATH.
    pub fn child_path(&self, extra_first: Option<&Path>) -> OsString {
        let mut dirs = Vec::new();
        if let Some(extra) = extra_first {
            push_unique(&mut dirs, extra.to_path_buf());
        }
        for dir in &self.path_dirs {
            push_unique(&mut dirs, dir.clone());
        }
        for tool in ToolId::ALL {
            for dir in known_dirs(tool, self) {
                if dir.is_dir() {
                    push_unique(&mut dirs, dir);
                }
            }
        }
        let joinable: Vec<PathBuf> = dirs
            .into_iter()
            .filter(|d| env::join_paths([d]).is_ok())
            .collect();
        env::join_paths(joinable).unwrap_or_default()
    }

    fn codex_auth_file(&self) -> Option<PathBuf> {
        self.codex_home
            .clone()
            .or_else(|| self.home.as_ref().map(|h| h.join(".codex")))
            .map(|dir| dir.join("auth.json"))
    }
}

#[cfg(test)]
mod tests;
