//! Where a tool can be: PATH search done in Rust (with PATHEXT on Windows),
//! the known install locations per OS and the ordered candidate list.

use std::fs;
use std::path::{Path, PathBuf};

use super::{HostOs, ProbeEnv, ToolId, ToolSource};

pub(super) fn push_unique(list: &mut Vec<PathBuf>, item: PathBuf) {
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
pub(super) fn version_key(name: &str) -> Vec<u32> {
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
pub(super) fn known_dirs(tool: ToolId, env: &ProbeEnv) -> Vec<PathBuf> {
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
pub(super) fn known_files(tool: ToolId, env: &ProbeEnv) -> Vec<PathBuf> {
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

pub(super) const APP_BINARY: &str = "Contents/MacOS/Blender";

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

pub(super) fn resolve_override(tool: ToolId, path: &Path, env: &ProbeEnv) -> Option<PathBuf> {
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
pub(super) const MAX_PROBES: usize = 4;

/// Every copy of `tool` outside an override, PATH hits first, without
/// duplicates (symlinks to the same file count once).
pub(super) fn candidates(tool: ToolId, env: &ProbeEnv) -> Vec<(PathBuf, ToolSource)> {
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

pub(super) fn find_npx(env: &ProbeEnv) -> Option<PathBuf> {
    let names = ["npx"];
    find_on_path(&names, env).or_else(|| find_in_dirs(&names, &known_dirs(ToolId::Node, env), env))
}
