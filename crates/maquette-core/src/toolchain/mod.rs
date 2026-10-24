//! Finds the external tools Maquette drives and reads their versions.
//!
//! Lookup order per tool: a caller supplied override, then PATH, then known
//! install locations. PATH lookup happens in Rust, never through `which` or
//! `where`, so it behaves the same when a macOS app starts from Finder with a
//! minimal PATH.

mod runner;
pub mod version;

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

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

fn push_unique(list: &mut Vec<PathBuf>, item: PathBuf) {
    if !list.contains(&item) {
        list.push(item);
    }
}

fn pathext(env: &ProbeEnv) -> Vec<String> {
    let list: Vec<String> = if env.pathext.is_empty() {
        [".COM", ".EXE", ".BAT", ".CMD"]
            .iter()
            .map(|e| e.to_string())
            .collect()
    } else {
        env.pathext.clone()
    };
    list.into_iter()
        .map(|e| e.trim().to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect()
}

/// Lowercase file names to try for `name` in a Windows folder.
fn windows_candidates(name: &str, env: &ProbeEnv) -> Vec<String> {
    let mut out = Vec::new();
    if Path::new(name).extension().is_some() {
        out.push(name.to_ascii_lowercase());
    }
    for ext in pathext(env) {
        out.push(format!("{name}{ext}").to_ascii_lowercase());
    }
    out
}

fn is_unix_executable(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn is_runnable(path: &Path, env: &ProbeEnv) -> bool {
    if env.os == HostOs::Windows {
        path.is_file()
    } else {
        is_unix_executable(path)
    }
}

fn find_in_dir(names: &[&str], dir: &Path, env: &ProbeEnv) -> Option<PathBuf> {
    // Relative PATH entries resolve against the working folder, which an
    // attacker could control, so they are skipped.
    if !dir.is_absolute() {
        return None;
    }
    if env.os == HostOs::Windows {
        let files: Vec<(String, PathBuf)> = fs::read_dir(dir)
            .ok()?
            .filter_map(Result::ok)
            .map(|e| {
                (
                    e.file_name().to_string_lossy().to_ascii_lowercase(),
                    e.path(),
                )
            })
            .collect();
        for name in names {
            for candidate in windows_candidates(name, env) {
                let hit = files
                    .iter()
                    .find(|(lower, path)| *lower == candidate && path.is_file());
                if let Some((_, path)) = hit {
                    return Some(path.clone());
                }
            }
        }
        None
    } else {
        names
            .iter()
            .map(|name| dir.join(name))
            .find(|path| is_unix_executable(path))
    }
}

fn find_in_dirs(names: &[&str], dirs: &[PathBuf], env: &ProbeEnv) -> Option<PathBuf> {
    dirs.iter().find_map(|dir| find_in_dir(names, dir, env))
}

/// First executable named any of `names` in `env.path_dirs`, in PATH order.
pub fn find_on_path(names: &[&str], env: &ProbeEnv) -> Option<PathBuf> {
    find_in_dirs(names, &env.path_dirs, env)
}

/// Numbers in the first `1.2.3` style run of `name`, for sorting folders
/// such as `Blender 4.10` or `v22.1.0`.
fn version_key(name: &str) -> Vec<u32> {
    let Some(start) = name.find(|c: char| c.is_ascii_digit()) else {
        return Vec::new();
    };
    name[start..]
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()
        .unwrap_or_default()
        .split('.')
        .map_while(|part| part.parse().ok())
        .collect()
}

/// Children of `dir` whose names pass `keep`, newest version first.
fn versioned_children(dir: &Path, keep: impl Fn(&str) -> bool) -> Vec<(Vec<u32>, PathBuf)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(Vec<u32>, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            keep(&name).then(|| (version_key(&name), e.path()))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    found
}

fn home_dir(env: &ProbeEnv, rel: &str) -> Option<PathBuf> {
    env.home.as_ref().map(|h| h.join(rel))
}

fn unix_node_dirs(env: &ProbeEnv) -> Vec<PathBuf> {
    let Some(home) = &env.home else {
        return Vec::new();
    };
    let mut dirs = vec![home.join(".npm-global/bin"), home.join(".volta/bin")];
    if env.os == HostOs::Mac {
        dirs.push(home.join("Library/pnpm"));
    }
    dirs.push(home.join(".local/share/pnpm"));
    dirs.push(home.join(".bun/bin"));
    for (_, version_dir) in versioned_children(&home.join(".nvm/versions/node"), |_| true) {
        dirs.push(version_dir.join("bin"));
    }
    dirs.push(home.join(".fnm/aliases/default/bin"));
    dirs
}

fn windows_node_dirs(env: &ProbeEnv) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.extend(env.app_data.as_ref().map(|d| d.join("npm")));
    dirs.extend(env.local_app_data.as_ref().map(|d| d.join("pnpm")));
    dirs.extend(env.program_files.iter().map(|d| d.join("nodejs")));
    dirs.extend(
        env.local_app_data
            .as_ref()
            .map(|d| d.join("Volta").join("bin")),
    );
    dirs
}

fn windows_shim_dirs(env: &ProbeEnv) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.extend(home_dir(env, "scoop").map(|d| d.join("shims")));
    dirs.extend(
        env.program_data
            .as_ref()
            .map(|d| d.join("chocolatey").join("bin")),
    );
    dirs.extend(
        env.local_app_data
            .as_ref()
            .map(|d| d.join("Microsoft").join("WinGet").join("Links")),
    );
    dirs
}

/// Bin folders searched for `tool` after PATH, in priority order.
fn known_dirs(tool: ToolId, env: &ProbeEnv) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for dir in &env.system_bin_dirs {
        push_unique(&mut dirs, dir.clone());
    }
    let extra = match env.os {
        HostOs::Windows => {
            let mut extra = Vec::new();
            match tool {
                ToolId::Codex | ToolId::GltfTransform | ToolId::Node => {
                    extra.extend(windows_node_dirs(env));
                }
                ToolId::Ffmpeg => {
                    if let Some(drive) = &env.system_drive {
                        extra.push(PathBuf::from(format!("{drive}\\ffmpeg\\bin")));
                    }
                    extra.extend(
                        env.program_files
                            .iter()
                            .map(|d| d.join("ffmpeg").join("bin")),
                    );
                }
                ToolId::Toktx => {
                    extra.extend(
                        env.program_files
                            .iter()
                            .map(|d| d.join("KTX-Software").join("bin")),
                    );
                }
                ToolId::Blender => {}
            }
            extra.extend(windows_shim_dirs(env));
            extra
        }
        _ if tool.uses_node() => unix_node_dirs(env),
        _ => Vec::new(),
    };
    for dir in extra {
        push_unique(&mut dirs, dir);
    }
    dirs
}

/// Exact install paths (not bin folders) checked before the bin folders.
fn known_files(tool: ToolId, env: &ProbeEnv) -> Vec<PathBuf> {
    if tool != ToolId::Blender {
        return Vec::new();
    }
    match env.os {
        HostOs::Mac => {
            let mut files: Vec<PathBuf> = env
                .applications_dirs
                .iter()
                .map(|d| d.join("Blender.app").join(APP_BINARY))
                .collect();
            let mut versioned = Vec::new();
            for dir in &env.applications_dirs {
                versioned.extend(versioned_children(dir, |name| {
                    name.starts_with("Blender") && name.ends_with(".app") && name != "Blender.app"
                }));
            }
            versioned.sort_by(|a, b| b.0.cmp(&a.0));
            files.extend(versioned.into_iter().map(|(_, p)| p.join(APP_BINARY)));
            files
        }
        HostOs::Windows => {
            let mut roots: Vec<PathBuf> = env.program_files.clone();
            roots.extend(env.local_app_data.as_ref().map(|d| d.join("Programs")));
            let mut versioned = Vec::new();
            for root in &roots {
                versioned.extend(versioned_children(
                    &root.join("Blender Foundation"),
                    |name| name.starts_with("Blender"),
                ));
            }
            versioned.sort_by(|a, b| b.0.cmp(&a.0));
            let mut files: Vec<PathBuf> = versioned
                .into_iter()
                .map(|(_, p)| p.join("blender.exe"))
                .collect();
            files.extend(env.program_files.iter().map(|d| {
                d.join("Steam")
                    .join("steamapps")
                    .join("common")
                    .join("Blender")
                    .join("blender.exe")
            }));
            files
        }
        HostOs::Linux => Vec::new(),
    }
}

const APP_BINARY: &str = "Contents/MacOS/Blender";

/// Candidate files for `tool` outside PATH, in priority order. On Windows
/// every PATHEXT spelling of the binary in each bin folder is listed.
pub fn known_locations(tool: ToolId, env: &ProbeEnv) -> Vec<PathBuf> {
    let mut out = known_files(tool, env);
    let name = tool.binary_name();
    for dir in known_dirs(tool, env) {
        if env.os == HostOs::Windows {
            for ext in pathext(env) {
                out.push(dir.join(format!("{name}{}", ext.to_ascii_uppercase())));
            }
        } else {
            out.push(dir.join(name));
        }
    }
    out
}

fn resolve_override(tool: ToolId, path: &Path, env: &ProbeEnv) -> Option<PathBuf> {
    let is_bundle = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"));
    if env.os == HostOs::Mac && tool == ToolId::Blender && is_bundle {
        let inner = path.join(APP_BINARY);
        return is_runnable(&inner, env).then_some(inner);
    }
    is_runnable(path, env).then(|| path.to_path_buf())
}

/// Most copies of one tool probed before giving up. Bounds the wait when
/// several stale installs shadow each other.
const MAX_PROBES: usize = 4;

/// Every copy of `tool` outside an override, PATH hits first, without
/// duplicates (symlinks to the same file count once).
fn candidates(tool: ToolId, env: &ProbeEnv) -> Vec<(PathBuf, ToolSource)> {
    let names = [tool.binary_name()];
    let mut found: Vec<(PathBuf, ToolSource)> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut add = |path: PathBuf, source: ToolSource| {
        let key = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !seen.contains(&key) {
            seen.push(key);
            found.push((path, source));
        }
    };
    for dir in &env.path_dirs {
        if let Some(path) = find_in_dir(&names, dir, env) {
            add(path, ToolSource::Path);
        }
    }
    for path in known_files(tool, env) {
        if is_runnable(&path, env) {
            add(path, ToolSource::KnownLocation);
        }
    }
    for dir in known_dirs(tool, env) {
        if let Some(path) = find_in_dir(&names, &dir, env) {
            add(path, ToolSource::KnownLocation);
        }
    }
    found
}

fn find_npx(env: &ProbeEnv) -> Option<PathBuf> {
    let names = ["npx"];
    find_on_path(&names, env).or_else(|| find_in_dirs(&names, &known_dirs(ToolId::Node, env), env))
}

fn missing_hint(tool: ToolId, os: HostOs) -> &'static str {
    let windows = os == HostOs::Windows;
    match tool {
        ToolId::Blender if windows => "Install Blender from blender.org, or locate blender.exe.",
        ToolId::Blender if os == HostOs::Mac => {
            "Install Blender from blender.org, or locate Blender.app."
        }
        ToolId::Blender => "Install Blender from blender.org, or locate the blender binary.",
        ToolId::Codex => "Install it with npm i -g @openai/codex, then run codex login.",
        ToolId::Ffmpeg if windows => "Install it with winget install Gyan.FFmpeg.",
        ToolId::Ffmpeg if os == HostOs::Mac => "Install it with brew install ffmpeg.",
        ToolId::Ffmpeg => "Install ffmpeg with your package manager.",
        ToolId::Toktx => {
            "Install KTX-Software, then Re-check. Until then Compress keeps textures as PNG."
        }
        ToolId::GltfTransform => {
            "Install it with npm i -g @gltf-transform/cli, or install Node so I can use npx."
        }
        ToolId::Node if windows => {
            "Install Node 22 from nodejs.org or with winget install OpenJS.NodeJS.LTS."
        }
        ToolId::Node if os == HostOs::Mac => {
            "Install Node 22 from nodejs.org or with brew install node."
        }
        ToolId::Node => "Install Node 22 from nodejs.org or with your package manager.",
    }
}

const BROKEN_HINT: &str = "Locate a working copy, then Re-check.";
const STALE_OVERRIDE: &str =
    "The saved location no longer works, so Maquette looked for another copy.";
const CODEX_NOT_SIGNED_IN: &str = "not signed in, run codex login";

/// Finds `tool`, runs its version probe and reports what happened.
pub fn detect(
    tool: ToolId,
    env: &ProbeEnv,
    override_path: Option<&Path>,
    runner: &dyn CommandRunner,
) -> ToolReport {
    let mut warning = None;
    let mut resolved = None;
    if let Some(path) = override_path {
        match resolve_override(tool, path, env) {
            Some(found) => resolved = Some((found, ToolSource::Override)),
            None => warning = Some(STALE_OVERRIDE.to_string()),
        }
    }
    let mut report = match resolved {
        Some((path, source)) => probe(tool, env, path, source, runner),
        None => probe_candidates(tool, env, runner),
    };
    if report.warning.is_none() {
        report.warning = warning;
    }
    report
}

/// Probes copies in order and keeps the first that works, so a broken
/// global install does not hide a working one later on PATH. When none
/// works, the first failure is reported.
fn probe_candidates(tool: ToolId, env: &ProbeEnv, runner: &dyn CommandRunner) -> ToolReport {
    let mut first_broken = None;
    for (path, source) in candidates(tool, env).into_iter().take(MAX_PROBES) {
        let report = probe(tool, env, path, source, runner);
        if report.status == ToolStatus::Found {
            return report;
        }
        first_broken.get_or_insert(report);
    }
    if let Some(report) = first_broken {
        return report;
    }
    match (tool, find_npx(env)) {
        (ToolId::GltfTransform, Some(npx)) => {
            let mut report = ToolReport::new(tool, ToolStatus::Found);
            report.source = Some(ToolSource::Npx);
            report.path = Some(npx.display().to_string());
            report.detail = Some("via npx, @gltf-transform/cli".to_string());
            report
        }
        _ => {
            let mut report = ToolReport::new(tool, ToolStatus::Missing);
            report.detail = Some("not found on PATH".to_string());
            report.hint = Some(missing_hint(tool, env.os).to_string());
            report
        }
    }
}

fn probe(
    tool: ToolId,
    env: &ProbeEnv,
    path: PathBuf,
    source: ToolSource,
    runner: &dyn CommandRunner,
) -> ToolReport {
    let args = tool.version_args();
    let timeout = tool.timeout();
    let path_env = env.child_path(path.parent());
    let outcome = runner.run(&path, args, &path_env, timeout);

    let mut report = ToolReport::new(tool, ToolStatus::Found);
    report.source = Some(source);
    report.path = Some(path.display().to_string());

    let version_flag = args.last().copied().unwrap_or("--version");
    let broken = |mut report: ToolReport, detail: String| {
        report.status = ToolStatus::Broken;
        report.detail = Some(detail);
        report.hint = Some(BROKEN_HINT.to_string());
        report
    };
    match outcome {
        Err(ProbeError::Timeout) => broken(
            report,
            format!("did not answer {version_flag} in {} s", timeout.as_secs()),
        ),
        Err(ProbeError::Spawn(_)) => broken(report, "could not be started".to_string()),
        Ok(out) if !out.status_ok => broken(report, "exited with an error".to_string()),
        Ok(out) => {
            report.version = version::parse(tool, &out.stdout, &out.stderr);
            match tool {
                ToolId::Blender => report.detail = Some("runs headless".to_string()),
                ToolId::Codex => {
                    let signed_in = env.codex_auth_file().is_some_and(|f| f.is_file());
                    if signed_in {
                        report.detail = Some("signed in".to_string());
                    } else {
                        report.detail = Some(CODEX_NOT_SIGNED_IN.to_string());
                        report.warning = Some("Codex CLI is not signed in.".to_string());
                        report.hint =
                            Some("Run codex login in a terminal, then Re-check.".to_string());
                    }
                }
                _ => {}
            }
            report
        }
    }
}

/// Probes every tool in parallel and returns reports in [`ToolId::ALL`] order.
pub fn detect_all(
    env: &ProbeEnv,
    overrides: &BTreeMap<ToolId, PathBuf>,
    runner: &dyn CommandRunner,
) -> Vec<ToolReport> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = ToolId::ALL
            .iter()
            .map(|&tool| {
                let override_path = overrides.get(&tool).map(PathBuf::as_path);
                (
                    tool,
                    scope.spawn(move || detect(tool, env, override_path, runner)),
                )
            })
            .collect();
        handles
            .into_iter()
            .map(|(tool, handle)| {
                handle.join().unwrap_or_else(|_| {
                    let mut report = ToolReport::new(tool, ToolStatus::Broken);
                    report.detail = Some("the check failed".to_string());
                    report.hint = Some(BROKEN_HINT.to_string());
                    report
                })
            })
            .collect()
    })
}

/// True when a required tool is not usable, so setup cannot go on.
pub fn blocks_continue(reports: &[ToolReport]) -> bool {
    reports
        .iter()
        .any(|r| r.required && r.status != ToolStatus::Found)
}

#[cfg(test)]
mod tests;
