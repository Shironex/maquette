//! Projects: one folder per scene under the workspace's projects folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::workspace::{FolderLayout, is_single_segment};

pub const PROJECT_FILE: &str = "project.json";
pub const PROJECT_VERSION: u32 = 1;
pub const NAME_MAX_CHARS: usize = 80;
const SLUG_MAX_CHARS: usize = 60;
const PROJECT_DIRS: [&str; 6] = ["concepts", "refs", "models", "blender", "web", "tools"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectTemplate {
    FloatingDiorama,
    Room,
    CharacterShowcase,
    Imported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub version: u32,
    /// The folder name. Always matches the folder on load, even if the user
    /// renamed it by hand.
    pub id: String,
    pub name: String,
    pub template: ProjectTemplate,
    pub site_repo: Option<String>,
    pub source_folder: Option<String>,
    /// RFC 3339 UTC with milliseconds, for example `2026-10-24T09:30:00.123Z`.
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewProject {
    pub name: String,
    pub template: ProjectTemplate,
    pub site_repo: Option<String>,
    pub source_folder: Option<String>,
}

/// `"Moon Island"` becomes `"moon-island"`. Common accented Latin letters are
/// folded to ASCII, anything else is dropped. Never empty.
pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for c in name.chars() {
        let folded = fold_char(c);
        if folded.is_empty() {
            if c.is_whitespace() || c.is_ascii_punctuation() {
                pending_dash = true;
            }
            continue;
        }
        if pending_dash && !slug.is_empty() {
            slug.push('-');
        }
        pending_dash = false;
        slug.push_str(&folded);
    }
    let mut slug: String = slug.chars().take(SLUG_MAX_CHARS).collect();
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "project".to_string()
    } else {
        slug
    }
}

fn fold_char(c: char) -> String {
    if c.is_ascii_alphanumeric() {
        return c.to_ascii_lowercase().to_string();
    }
    let folded = match c.to_lowercase().next().unwrap_or(c) {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ą' | 'ā' | 'ă' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'č' | 'ĉ' => "c",
        'ď' | 'đ' | 'ð' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ę' | 'ē' | 'ě' => "e",
        'ğ' | 'ĝ' => "g",
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'ı' => "i",
        'ł' | 'ľ' | 'ĺ' => "l",
        'ñ' | 'ń' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => "o",
        'œ' => "oe",
        'ř' | 'ŕ' => "r",
        'ś' | 'š' | 'ş' | 'ŝ' => "s",
        'ß' => "ss",
        'ť' | 'ţ' => "t",
        'þ' => "th",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => "u",
        'ý' | 'ÿ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        _ => "",
    };
    folded.to_string()
}

fn projects_dir(workspace_root: &Path, layout: &FolderLayout) -> PathBuf {
    workspace_root.join(&layout.projects)
}

fn clean_name(name: &str) -> CoreResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("Give the project a name.".to_string()));
    }
    if name.chars().count() > NAME_MAX_CHARS {
        return Err(CoreError::Invalid(format!(
            "Keep the project name to {NAME_MAX_CHARS} characters or fewer."
        )));
    }
    Ok(name.to_string())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn check_id(id: &str) -> CoreResult<()> {
    if is_single_segment(id) {
        Ok(())
    } else {
        Err(CoreError::Invalid(format!("{id:?} is not a project id.")))
    }
}

pub fn create(
    workspace_root: &Path,
    layout: &FolderLayout,
    new: NewProject,
) -> CoreResult<ProjectManifest> {
    let name = clean_name(&new.name)?;
    let base = slugify(&name);
    let parent = projects_dir(workspace_root, layout);
    fs::create_dir_all(&parent).map_err(|e| CoreError::io(&parent, e))?;

    // create_dir (not create_dir_all) claims the slug atomically, so two
    // creates with the same name cannot share a folder.
    let mut suffix = 1u32;
    let (id, dir) = loop {
        let id = if suffix == 1 {
            base.clone()
        } else {
            format!("{base}-{suffix}")
        };
        let dir = parent.join(&id);
        match fs::create_dir(&dir) {
            Ok(()) => break (id, dir),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => suffix += 1,
            Err(err) => return Err(CoreError::io(&dir, err)),
        }
    };
    for sub in PROJECT_DIRS {
        let path = dir.join(sub);
        fs::create_dir_all(&path).map_err(|e| CoreError::io(&path, e))?;
    }

    let manifest = ProjectManifest {
        version: PROJECT_VERSION,
        id,
        name,
        template: new.template,
        site_repo: clean_optional(new.site_repo),
        source_folder: clean_optional(new.source_folder),
        created_at: rfc3339_utc(SystemTime::now()),
    };
    write_json(&dir.join(PROJECT_FILE), &manifest)?;
    Ok(manifest)
}

pub fn load(workspace_root: &Path, layout: &FolderLayout, id: &str) -> CoreResult<ProjectManifest> {
    check_id(id)?;
    let path = projects_dir(workspace_root, layout)
        .join(id)
        .join(PROJECT_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!("the project {id}")));
        }
        Err(err) => return Err(CoreError::io(&path, err)),
    };
    let mut manifest: ProjectManifest =
        serde_json::from_str(&text).map_err(|e| CoreError::json(&path, e))?;
    if manifest.version > PROJECT_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: manifest.version,
            supported: PROJECT_VERSION,
        });
    }
    manifest.version = PROJECT_VERSION;
    manifest.id = id.to_string();
    Ok(manifest)
}

/// Every readable project, oldest first. Folders without a readable
/// manifest are skipped with a warning.
pub fn list(workspace_root: &Path, layout: &FolderLayout) -> CoreResult<Vec<ProjectManifest>> {
    let parent = projects_dir(workspace_root, layout);
    let entries = match fs::read_dir(&parent) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(CoreError::io(&parent, err)),
    };
    let mut projects = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.join(PROJECT_FILE).is_file() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        match load(workspace_root, layout, &id) {
            Ok(manifest) => projects.push(manifest),
            Err(err) => tracing::warn!(project = %id, error = %err, "skipping unreadable project"),
        }
    }
    projects.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(projects)
}

pub fn update(
    workspace_root: &Path,
    layout: &FolderLayout,
    id: &str,
    name: String,
    site_repo: Option<String>,
) -> CoreResult<ProjectManifest> {
    let mut manifest = load(workspace_root, layout, id)?;
    manifest.name = clean_name(&name)?;
    manifest.site_repo = clean_optional(site_repo);
    let path = projects_dir(workspace_root, layout)
        .join(id)
        .join(PROJECT_FILE);
    write_json(&path, &manifest)?;
    Ok(manifest)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoInspection {
    pub path: String,
    pub exists: bool,
    pub is_git: bool,
    pub has_public_dir: bool,
}

pub fn inspect_site_repo(path: &Path) -> RepoInspection {
    RepoInspection {
        path: path.display().to_string(),
        exists: path.is_dir(),
        // `.git` is a file in worktrees and submodules.
        is_git: path.join(".git").exists(),
        has_public_dir: path.join("public").is_dir(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BlendFile {
    /// Path relative to the scanned folder, with `/` separators.
    pub file: String,
    pub stem: String,
    pub version: Option<u32>,
}

impl BlendFile {
    /// `moon_island_v7.blend` gives stem `moon_island` and version 7. The
    /// separator before the `v` may be `_`, `-` or a space.
    pub fn parse(file: &str) -> Self {
        let name = file.rsplit('/').next().unwrap_or(file);
        let base = match name.rsplit_once('.') {
            Some((base, _)) => base,
            None => name,
        };
        let (stem, version) = split_version(base);
        Self {
            file: file.to_string(),
            stem: stem.to_string(),
            version,
        }
    }
}

fn split_version(base: &str) -> (&str, Option<u32>) {
    let digits = base.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return (base, None);
    }
    let head = &base[..base.len() - digits];
    let Some(before_v) = head.strip_suffix(['v', 'V']) else {
        return (base, None);
    };
    let Some(stem) = before_v.strip_suffix(['_', '-', ' ']) else {
        return (base, None);
    };
    let stem = stem.trim_end();
    match base[base.len() - digits..].parse() {
        Ok(version) if !stem.is_empty() => (stem, Some(version)),
        _ => (base, None),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportScan {
    pub path: String,
    pub blend_files: Vec<BlendFile>,
    /// File names of `.py`, `.mjs`, `.js` and `.sh` scripts directly inside
    /// `<folder>/tools/`.
    pub tool_scripts: Vec<String>,
}

fn has_ext(path: &Path, exts: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

fn sorted_children(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Looks for `.blend` files (top level and one folder down, `.blend1`
/// backups ignored) and tool scripts in an existing project folder.
pub fn scan_import(folder: &Path) -> CoreResult<ImportScan> {
    if !folder.is_dir() {
        return Err(CoreError::NotFound(format!(
            "the folder {}",
            folder.display()
        )));
    }
    let mut blend_files = Vec::new();
    for (name, path) in sorted_children(folder) {
        if path.is_file() && has_ext(&path, &["blend"]) {
            blend_files.push(BlendFile::parse(&name));
        } else if path.is_dir() && !name.starts_with('.') {
            for (child, child_path) in sorted_children(&path) {
                if child_path.is_file() && has_ext(&child_path, &["blend"]) {
                    blend_files.push(BlendFile::parse(&format!("{name}/{child}")));
                }
            }
        }
    }
    let tool_scripts = sorted_children(&folder.join("tools"))
        .into_iter()
        .filter(|(_, path)| path.is_file() && has_ext(path, &["py", "mjs", "js", "sh"]))
        .map(|(name, _)| name)
        .collect();
    Ok(ImportScan {
        path: folder.display().to_string(),
        blend_files,
        tool_scripts,
    })
}

fn rfc3339_utc(time: SystemTime) -> String {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = since.as_secs();
    let millis = since.subsec_millis();
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        rem / 3600,
        (rem / 60) % 60,
        rem % 60
    )
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's
/// algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn new_project(name: &str) -> NewProject {
        NewProject {
            name: name.to_string(),
            template: ProjectTemplate::FloatingDiorama,
            site_repo: None,
            source_folder: None,
        }
    }

    #[test]
    fn slugify_cases() {
        assert_eq!(slugify("Moon Island"), "moon-island");
        assert_eq!(slugify("  Moon   Island!! "), "moon-island");
        assert_eq!(slugify("Zażółć gęślą jaźń"), "zazolc-gesla-jazn");
        assert_eq!(slugify("Café Straße"), "cafe-strasse");
        assert_eq!(slugify("moon_island_v2"), "moon-island-v2");
        assert_eq!(slugify("月の島"), "project");
        assert_eq!(slugify(""), "project");
        assert_eq!(slugify("---"), "project");
        assert_eq!(slugify(&"a ".repeat(100)).chars().count(), 59);
    }

    #[test]
    fn rfc3339_formatting() {
        assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
        let t = UNIX_EPOCH + Duration::from_millis(1_709_210_096_789);
        assert_eq!(rfc3339_utc(t), "2024-02-29T12:34:56.789Z");
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn create_makes_folders_and_manifest() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = FolderLayout::default();
        let mut new = new_project("  Moon Island ");
        new.site_repo = Some("  ".to_string());
        new.source_folder = Some("/tmp/src".to_string());
        let manifest = create(dir.path(), &layout, new).expect("create");
        assert_eq!(manifest.id, "moon-island");
        assert_eq!(manifest.name, "Moon Island");
        assert_eq!(manifest.site_repo, None);
        assert_eq!(manifest.source_folder.as_deref(), Some("/tmp/src"));
        assert!(manifest.created_at.ends_with('Z'));
        let root = dir.path().join("projects").join("moon-island");
        for sub in PROJECT_DIRS {
            assert!(root.join(sub).is_dir(), "{sub}");
        }
        let json = fs::read_to_string(root.join(PROJECT_FILE)).expect("read");
        assert!(json.contains("\"createdAt\""));
        assert!(json.contains("\"template\": \"floating-diorama\""));
        assert_eq!(
            load(dir.path(), &layout, "moon-island").expect("load"),
            manifest
        );
    }

    #[test]
    fn create_rejects_bad_names_and_suffixes_collisions() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = FolderLayout::default();
        assert_eq!(
            create(dir.path(), &layout, new_project("   "))
                .expect_err("empty")
                .code(),
            "invalid"
        );
        assert!(create(dir.path(), &layout, new_project(&"x".repeat(81))).is_err());
        assert!(create(dir.path(), &layout, new_project(&"x".repeat(80))).is_ok());

        let a = create(dir.path(), &layout, new_project("Moon Island")).expect("a");
        let b = create(dir.path(), &layout, new_project("moon island")).expect("b");
        let c = create(dir.path(), &layout, new_project("Moon-Island")).expect("c");
        assert_eq!(
            [a.id.as_str(), b.id.as_str(), c.id.as_str()],
            ["moon-island", "moon-island-2", "moon-island-3"]
        );
    }

    #[test]
    fn update_and_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = FolderLayout::default();
        assert!(list(dir.path(), &layout).expect("empty list").is_empty());
        let first = create(dir.path(), &layout, new_project("First")).expect("first");
        let second = create(dir.path(), &layout, new_project("Second")).expect("second");
        fs::create_dir_all(dir.path().join("projects").join("stray")).expect("mkdir");

        let updated = update(
            dir.path(),
            &layout,
            &first.id,
            " Renamed ".to_string(),
            Some("/sites/moon".to_string()),
        )
        .expect("update");
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.id, "first");
        assert_eq!(load(dir.path(), &layout, "first").expect("load"), updated);

        let ids: Vec<String> = list(dir.path(), &layout)
            .expect("list")
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, vec![first.id, second.id]);

        assert_eq!(
            load(dir.path(), &layout, "nope")
                .expect_err("missing")
                .code(),
            "not-found"
        );
        assert_eq!(
            load(dir.path(), &layout, "../x")
                .expect_err("traversal")
                .code(),
            "invalid"
        );
        assert!(update(dir.path(), &layout, "second", String::new(), None).is_err());
    }

    #[test]
    fn site_repo_inspection() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = dir.path().join("site");
        let missing = inspect_site_repo(&repo);
        assert!(!missing.exists && !missing.is_git && !missing.has_public_dir);

        fs::create_dir_all(repo.join(".git")).expect("mkdir");
        fs::create_dir_all(repo.join("public")).expect("mkdir");
        let found = inspect_site_repo(&repo);
        assert!(found.exists && found.is_git && found.has_public_dir);

        let worktree = dir.path().join("wt");
        fs::create_dir_all(&worktree).expect("mkdir");
        fs::write(worktree.join(".git"), b"gitdir: /x").expect("write");
        assert!(inspect_site_repo(&worktree).is_git);
    }

    #[test]
    fn blend_file_names() {
        let cases = [
            ("moon_island_v7.blend", "moon_island", Some(7)),
            ("moon_island-v7.blend", "moon_island", Some(7)),
            ("moon island v7.blend", "moon island", Some(7)),
            ("moon_island_V12.blend", "moon_island", Some(12)),
            ("moon_island.blend", "moon_island", None),
            ("moon7.blend", "moon7", None),
            ("v7.blend", "v7", None),
            ("sub/room_v2.blend", "room", Some(2)),
        ];
        for (file, stem, version) in cases {
            let parsed = BlendFile::parse(file);
            assert_eq!(parsed.stem, stem, "{file}");
            assert_eq!(parsed.version, version, "{file}");
            assert_eq!(parsed.file, file);
        }
    }

    #[test]
    fn scan_import_finds_blends_and_scripts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for file in [
            "moon_v2.blend",
            "moon_v2.blend1",
            "notes.txt",
            "scenes/room_v1.blend",
            "scenes/deep/too_deep.blend",
            ".hidden/secret.blend",
            "tools/bake.py",
            "tools/ship.mjs",
            "tools/run.sh",
            "tools/readme.md",
            "tools/nested/x.py",
        ] {
            let path = root.join(file);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            fs::write(&path, b"x").expect("write");
        }
        let scan = scan_import(root).expect("scan");
        let files: Vec<&str> = scan.blend_files.iter().map(|b| b.file.as_str()).collect();
        assert_eq!(files, vec!["moon_v2.blend", "scenes/room_v1.blend"]);
        assert_eq!(scan.blend_files[0].version, Some(2));
        assert_eq!(scan.tool_scripts, vec!["bake.py", "run.sh", "ship.mjs"]);

        assert_eq!(
            scan_import(&root.join("missing"))
                .expect_err("missing")
                .code(),
            "not-found"
        );
    }
}
