//! What New project finds in a game repo: a CMake project, the model folder
//! and the format its files are in, the library that reads them and
//! whether the engine can decode KTX2 or meshopt. It only reads.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::GameFormat;
use crate::error::{CoreError, CoreResult};
use crate::project::{has_ext, sorted_children};
use crate::toolchain::CommandRunner;

/// Where engines keep their models, in the order they are tried.
pub const MODEL_DIRS: [&str; 5] = [
    "assets/models",
    "assets/meshes",
    "models",
    "res/models",
    "data/models",
];

/// How deep the model folder is searched.
const MAX_DEPTH: usize = 4;

/// Model loading libraries, by a word that names them in the build files
/// or the vendored folders, and the format each reads.
const LOADERS: &[(&str, &str)] = &[
    ("cgltf", "cgltf"),
    ("tinygltf", "tinygltf"),
    ("fastgltf", "fastgltf"),
    ("tinyobjloader", "tinyobjloader"),
    ("tiny_obj_loader", "tinyobjloader"),
    ("assimp", "Assimp"),
];
const VENDOR_DIRS: [&str; 5] = ["external", "extern", "third_party", "vendor", "libs"];
const KTX2_WORDS: [&str; 3] = ["ktx", "basisu", "basis_universal"];
const MESHOPT_WORDS: [&str; 1] = ["meshoptimizer"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameRepoInspection {
    pub path: String,
    pub exists: bool,
    pub is_git: bool,
    /// The checked out branch, read from `.git/HEAD`.
    pub branch: Option<String>,
    /// `CMakeLists.txt` at the root.
    pub cmake: bool,
    /// The executables it builds (`add_executable`), in file order.
    pub targets: Vec<String>,
    /// The first of [`MODEL_DIRS`] that exists, relative to the repo.
    pub models_dir: Option<String>,
    /// The format most of its files are in; none without model files.
    pub format: Option<GameFormat>,
    pub glb_files: u32,
    pub obj_files: u32,
    /// The folders under the model folder that hold models, sorted.
    pub folders: Vec<String>,
    /// The library the engine reads models with, when one is named.
    pub loader: Option<String>,
    pub reads_ktx2: bool,
    pub reads_meshopt: bool,
}

/// The branch `.git/HEAD` names. `.git` is a file in a worktree, pointing
/// at the real git folder.
pub(crate) fn branch(repo: &Path) -> Option<String> {
    let git = repo.join(".git");
    let dir = if git.is_file() {
        let text = fs::read_to_string(&git).ok()?;
        let target = text.trim().strip_prefix("gitdir:")?.trim().to_string();
        let path = PathBuf::from(&target);
        if path.is_absolute() {
            path
        } else {
            repo.join(path)
        }
    } else {
        git
    };
    let head = fs::read_to_string(dir.join("HEAD")).ok()?;
    head.trim()
        .strip_prefix("ref: refs/heads/")
        .map(str::to_string)
}

/// How long `git status` may take before Ship's diff stops waiting for it.
const GIT_STATUS_TIMEOUT: Duration = Duration::from_secs(5);

/// How many changed or new files `git status` lists in `repo`. It runs with
/// `--no-optional-locks`, so git writes nothing there, not even its index.
/// None for a folder that is not a git repo, or when git cannot tell.
pub fn uncommitted(
    repo: &Path,
    git: &Path,
    runner: &dyn CommandRunner,
    path_env: &OsStr,
) -> Option<u32> {
    if !repo.join(".git").exists() {
        return None;
    }
    let repo_arg = repo.to_string_lossy();
    let args = [
        "-C",
        repo_arg.as_ref(),
        "--no-optional-locks",
        "status",
        "--porcelain",
    ];
    let out = runner
        .run(git, &args, path_env, GIT_STATUS_TIMEOUT)
        .ok()
        .filter(|o| o.status_ok)?;
    let lines = out.stdout.lines().filter(|l| !l.trim().is_empty()).count();
    Some(u32::try_from(lines).unwrap_or(u32::MAX))
}

/// The names after `add_executable(` in a CMake file.
fn executables(cmake: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in cmake.lines() {
        let line = line.trim_start();
        if line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix("add_executable") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('(') else {
            continue;
        };
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
            .collect();
        if !name.is_empty() && !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// Every model file under `dir`, relative to it with `/`, sorted.
pub(crate) fn model_files(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, depth: usize, out: &mut Vec<String>) {
        for (name, path) in sorted_children(dir) {
            if name.starts_with('.') {
                continue;
            }
            let rel = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            if path.is_dir() && depth < MAX_DEPTH {
                walk(&path, &rel, depth + 1, out);
            } else if path.is_file() && has_ext(&path, &["glb", "obj"]) {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", 0, &mut out);
    out
}

/// The folders under `dir` that hold model files (empty for its top level).
pub(crate) fn model_folders(dir: &Path) -> Vec<String> {
    let mut folders: Vec<String> = model_files(dir)
        .iter()
        .map(|f| super::folder_of(f))
        .collect();
    folders.sort();
    folders.dedup();
    folders
}

/// The lowercased text the loader and decoder words are looked for in: the
/// root CMake file, the files in `cmake/` and the vendored folder names.
fn build_text(repo: &Path, cmake: &str) -> String {
    let mut text = cmake.to_lowercase();
    for (name, path) in sorted_children(&repo.join("cmake")) {
        if path.is_file()
            && (name.ends_with(".cmake") || name == "CMakeLists.txt")
            && let Ok(more) = fs::read_to_string(&path)
        {
            text.push('\n');
            text.push_str(&more.to_lowercase());
        }
    }
    for vendor in VENDOR_DIRS {
        for (name, _) in sorted_children(&repo.join(vendor)) {
            text.push('\n');
            text.push_str(&name.to_lowercase());
        }
    }
    text
}

/// Checks a game repo path given by an agent: an absolute path to a folder
/// that is a git repo (a `.git` folder) or a git worktree (a `.git` file
/// naming its repo). Returns it as given.
///
/// # Errors
///
/// `Invalid`, saying why, for anything else.
pub fn check_repo(path: &str) -> CoreResult<PathBuf> {
    let at = Path::new(path.trim());
    let invalid = |why: &str| Err(CoreError::Invalid(format!("{} {why}", at.display())));
    if !at.is_absolute() {
        return invalid("is not an absolute path.");
    }
    if !at.is_dir() {
        return invalid("is not a folder on this computer.");
    }
    let git = at.join(".git");
    let worktree = git.is_file()
        && fs::read_to_string(&git).is_ok_and(|t| t.trim_start().starts_with("gitdir:"));
    if !git.is_dir() && !worktree {
        return invalid("is not a git repo or worktree (it has no .git).");
    }
    Ok(at.to_path_buf())
}

/// Looks at a folder picked as a game repo.
pub fn inspect_game_repo(path: &Path) -> GameRepoInspection {
    let mut out = GameRepoInspection {
        path: path.display().to_string(),
        exists: path.is_dir(),
        is_git: path.join(".git").exists(),
        ..GameRepoInspection::default()
    };
    if !out.exists {
        return out;
    }
    out.branch = branch(path);
    let cmake = fs::read_to_string(path.join("CMakeLists.txt")).ok();
    out.cmake = cmake.is_some();
    let cmake = cmake.unwrap_or_default();
    out.targets = executables(&cmake);
    out.models_dir = MODEL_DIRS
        .iter()
        .find(|d| path.join(d).is_dir())
        .map(|d| (*d).to_string());
    if let Some(dir) = &out.models_dir {
        let files = model_files(&path.join(dir));
        let count = |f: GameFormat| {
            let n = files
                .iter()
                .filter(|x| GameFormat::of(x) == Some(f))
                .count();
            u32::try_from(n).unwrap_or(u32::MAX)
        };
        out.glb_files = count(GameFormat::Glb);
        out.obj_files = count(GameFormat::Obj);
        out.format = match (out.glb_files, out.obj_files) {
            (0, 0) => None,
            (glb, obj) if obj > glb => Some(GameFormat::Obj),
            _ => Some(GameFormat::Glb),
        };
        out.folders = model_folders(&path.join(dir));
    }
    let text = build_text(path, &cmake);
    out.loader = LOADERS
        .iter()
        .find(|(word, _)| text.contains(word))
        .map(|(_, name)| (*name).to_string());
    out.reads_ktx2 = KTX2_WORDS.iter().any(|w| text.contains(w));
    out.reads_meshopt = MESHOPT_WORDS.iter().any(|w| text.contains(w));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executables_are_read_in_order_and_once() {
        let cmake = "project(Game)\n# add_executable(old)\nadd_executable(frost)\n\
                     add_executable( frost_tests\n  src/a.cpp)\nadd_executable(frost)\n";
        assert_eq!(executables(cmake), ["frost", "frost_tests"]);
    }

    #[test]
    fn a_repo_path_must_be_an_absolute_git_repo_or_worktree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = dir.path().join("game");
        fs::create_dir_all(repo.join(".git")).expect("git");
        let worktree = dir.path().join("game-wt");
        fs::create_dir_all(&worktree).expect("worktree");
        fs::write(
            worktree.join(".git"),
            "gitdir: /x/game/.git/worktrees/game-wt\n",
        )
        .expect("file");
        let plain = dir.path().join("plain");
        fs::create_dir_all(&plain).expect("plain");
        let text = |p: &Path| p.display().to_string();
        assert_eq!(check_repo(&text(&repo)).expect("repo"), repo);
        assert_eq!(check_repo(&text(&worktree)).expect("worktree"), worktree);
        for bad in [
            text(&plain),
            text(&dir.path().join("none")),
            "game".to_string(),
        ] {
            assert!(check_repo(&bad).is_err(), "{bad}");
        }
    }
}
