//! The step cache. A step's key hashes everything that decides its output:
//! the contents of its declared inputs and script, its resolved args, the
//! tool and its version, and for the compress step its presets. When the
//! key matches the last successful run and the outputs it left are still
//! there, untouched, the step reports "cached" and is skipped.
//!
//! Records live in `<workspace cache>/pipeline/<project>/<step>.json`, and
//! a memo of file hashes (keyed by path, size and modified time) in
//! `hashes.json` next to them, so a 300 MB blend is read once, not every run.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::StepDef;
use super::tools::ResolvedTool;
use crate::error::{CoreError, CoreResult};
use crate::json::write_json;

const KEY_VERSION: &str = "maquette step cache v1";
const MEMO_FILE: &str = "hashes.json";

/// `<cache>/pipeline/<project>`.
pub fn project_cache_dir(workspace_cache: &Path, project: &str) -> PathBuf {
    workspace_cache.join("pipeline").join(project)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputStamp {
    /// Relative to the pipeline root, `/` separated.
    pub path: String,
    pub len: u64,
    pub modified_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheRecord {
    pub key: String,
    pub outputs: Vec<OutputStamp>,
    pub duration_ms: u64,
}

fn modified_ns(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Files matching `patterns` under `root`, sorted and without duplicates.
/// Each pattern's match count comes back too, so "matched nothing" is
/// part of the key instead of silently hashing to the same value.
pub fn expand(root: &Path, patterns: &[String]) -> Vec<(String, Vec<PathBuf>)> {
    let base = glob::Pattern::escape(&root.to_string_lossy());
    let options = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    patterns
        .iter()
        .map(|pattern| {
            let full = format!("{base}/{}", pattern.replace('\\', "/"));
            let mut files: Vec<PathBuf> = glob::glob_with(&full, options)
                .map(|paths| {
                    paths
                        .filter_map(Result::ok)
                        .filter(|p| p.is_file())
                        .collect()
                })
                .unwrap_or_default();
            files.sort();
            files.dedup();
            (pattern.clone(), files)
        })
        .collect()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct MemoEntry {
    len: u64,
    modified_ns: u64,
    sha256: String,
}

/// File hashing with a memo on disk.
#[derive(Debug, Default)]
pub struct FileHasher {
    memo_file: Option<PathBuf>,
    memo: BTreeMap<String, MemoEntry>,
    dirty: bool,
}

impl FileHasher {
    /// A hasher that remembers hashes in `<dir>/hashes.json`.
    pub fn with_memo(dir: &Path) -> Self {
        let memo_file = dir.join(MEMO_FILE);
        let memo = fs::read_to_string(&memo_file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            memo_file: Some(memo_file),
            memo,
            dirty: false,
        }
    }

    pub fn hash(&mut self, path: &Path) -> io::Result<String> {
        let meta = fs::metadata(path)?;
        let key = path.to_string_lossy().into_owned();
        let (len, modified) = (meta.len(), modified_ns(&meta));
        if let Some(hit) = self.memo.get(&key)
            && hit.len == len
            && hit.modified_ns == modified
            && modified != 0
        {
            return Ok(hit.sha256.clone());
        }
        let sha256 = hash_file(path)?;
        self.memo.insert(
            key,
            MemoEntry {
                len,
                modified_ns: modified,
                sha256: sha256.clone(),
            },
        );
        self.dirty = true;
        Ok(sha256)
    }

    /// Writes the memo if it changed. Failing to write it only costs time.
    pub fn flush(&mut self) {
        if !self.dirty {
            return;
        }
        if let Some(file) = &self.memo_file {
            if let Some(dir) = file.parent() {
                let _ = fs::create_dir_all(dir);
            }
            // Drop entries for files that are gone, so the memo cannot grow forever.
            self.memo.retain(|path, _| Path::new(path).exists());
            if let Err(err) = write_json(file, &self.memo) {
                tracing::warn!(error = %err, "could not save the hash memo");
            }
        }
        self.dirty = false;
    }
}

pub fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Everything that goes into one step's key.
pub struct KeyParts<'a> {
    pub root: &'a Path,
    pub step: &'a StepDef,
    pub tool: &'a ResolvedTool,
    /// The args after placeholders are filled, prefix included.
    pub args: &'a [String],
    /// Anything else that changes the output, for example the presets.
    pub extra: Option<String>,
}

/// The step's cache key: a sha256 over the parts, one line each.
pub fn key(parts: &KeyParts<'_>, hasher: &mut FileHasher) -> CoreResult<String> {
    let mut lines = vec![
        KEY_VERSION.to_string(),
        format!("kind {}", parts.step.kind.label()),
        format!(
            "tool {} {}",
            parts
                .tool
                .program
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            parts.tool.version
        ),
        format!("command {}", parts.step.command.as_deref().unwrap_or("")),
    ];
    lines.extend(parts.args.iter().map(|a| format!("arg {a}")));
    if let Some(script) = &parts.step.script {
        let path = parts.root.join(script);
        let digest = hasher.hash(&path).map_err(|e| CoreError::io(&path, e))?;
        lines.push(format!("script {script} {digest}"));
    }
    for (pattern, files) in expand(parts.root, &parts.step.inputs) {
        if files.is_empty() {
            lines.push(format!("input {pattern} none"));
        }
        for file in files {
            let digest = hasher.hash(&file).map_err(|e| CoreError::io(&file, e))?;
            lines.push(format!("input {} {digest}", rel(parts.root, &file)));
        }
    }
    if let Some(extra) = &parts.extra {
        lines.push(format!("extra {extra}"));
    }
    let mut hasher = Sha256::new();
    for line in lines {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Size and modified time of every file the output globs match now.
pub fn stamp_outputs(root: &Path, outputs: &[String]) -> Vec<OutputStamp> {
    expand(root, outputs)
        .into_iter()
        .flat_map(|(_, files)| files)
        .filter_map(|file| {
            let meta = fs::metadata(&file).ok()?;
            Some(OutputStamp {
                path: rel(root, &file),
                len: meta.len(),
                modified_ns: modified_ns(&meta),
            })
        })
        .collect()
}

/// True when the step can be skipped: same key, every output glob still
/// matches, and the files it wrote last time are unchanged. A step that
/// declares no outputs is never fresh.
pub fn is_fresh(record: &CacheRecord, key: &str, root: &Path, outputs: &[String]) -> bool {
    if record.key != key || outputs.is_empty() || record.outputs.is_empty() {
        return false;
    }
    if expand(root, outputs)
        .iter()
        .any(|(_, files)| files.is_empty())
    {
        return false;
    }
    record.outputs.iter().all(|stamp| {
        fs::metadata(root.join(&stamp.path))
            .is_ok_and(|meta| meta.len() == stamp.len && modified_ns(&meta) == stamp.modified_ns)
    })
}

fn record_path(dir: &Path, step: &str) -> PathBuf {
    dir.join(format!("{step}.json"))
}

pub fn load_record(dir: &Path, step: &str) -> Option<CacheRecord> {
    let text = fs::read_to_string(record_path(dir, step)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_record(dir: &Path, step: &str, record: &CacheRecord) -> CoreResult<()> {
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    write_json(&record_path(dir, step), record)
}

pub fn forget(dir: &Path, step: &str) {
    let _ = fs::remove_file(record_path(dir, step));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::StepKind;
    use std::thread::sleep;
    use std::time::Duration;

    fn step() -> StepDef {
        StepDef {
            id: "export".into(),
            name: "Export".into(),
            kind: StepKind::Blender,
            command: None,
            script: Some("tools/export.py".into()),
            args: vec!["-b".into()],
            inputs: vec!["models/*.blend".into(), "missing/*.png".into()],
            outputs: vec!["web/raw/*.glb".into()],
            depends_on: Vec::new(),
            compress: None,
        }
    }

    fn tool(version: &str) -> ResolvedTool {
        ResolvedTool {
            program: PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender"),
            prefix: Vec::new(),
            version: version.into(),
        }
    }

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, bytes).expect("write");
    }

    fn key_of(
        root: &Path,
        step: &StepDef,
        tool: &ResolvedTool,
        args: &[String],
        extra: Option<&str>,
    ) -> String {
        let mut hasher = FileHasher::default();
        key(
            &KeyParts {
                root,
                step,
                tool,
                args,
                extra: extra.map(str::to_string),
            },
            &mut hasher,
        )
        .expect("key")
    }

    #[test]
    fn the_key_follows_contents_args_script_and_tool_version() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        write(root, "tools/export.py", b"print('v1')");
        write(root, "models/island_v7.blend", b"blend one");
        let s = step();
        let args = vec!["-b".to_string(), "models/island_v7.blend".to_string()];
        let base = key_of(root, &s, &tool("4.3.2"), &args, None);
        assert_eq!(
            base,
            key_of(root, &s, &tool("4.3.2"), &args, None),
            "stable"
        );
        assert_eq!(base.len(), 64);

        // Touching a file without changing it keeps the key.
        sleep(Duration::from_millis(5));
        write(root, "models/island_v7.blend", b"blend one");
        assert_eq!(base, key_of(root, &s, &tool("4.3.2"), &args, None));

        write(root, "models/island_v7.blend", b"blend two");
        let changed_input = key_of(root, &s, &tool("4.3.2"), &args, None);
        assert_ne!(base, changed_input);

        write(root, "tools/export.py", b"print('v2')");
        let changed_script = key_of(root, &s, &tool("4.3.2"), &args, None);
        assert_ne!(changed_input, changed_script);

        assert_ne!(
            changed_script,
            key_of(root, &s, &tool("4.4.0"), &args, None)
        );
        let other_args = vec!["-b".to_string(), "--out".to_string()];
        assert_ne!(
            changed_script,
            key_of(root, &s, &tool("4.3.2"), &other_args, None)
        );
        assert_ne!(
            changed_script,
            key_of(root, &s, &tool("4.3.2"), &args, Some("presets"))
        );

        write(root, "models/extra_v1.blend", b"new input");
        assert_ne!(
            changed_script,
            key_of(root, &s, &tool("4.3.2"), &args, None)
        );
    }

    #[test]
    fn missing_script_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut hasher = FileHasher::default();
        let s = step();
        let err = key(
            &KeyParts {
                root: dir.path(),
                step: &s,
                tool: &tool("1"),
                args: &[],
                extra: None,
            },
            &mut hasher,
        )
        .expect_err("no script");
        assert_eq!(err.code(), "io");
    }

    #[test]
    fn fresh_only_with_the_same_key_and_untouched_outputs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let outputs = vec!["web/raw/*.glb".to_string()];
        write(root, "web/raw/island.glb", b"glb");
        let record = CacheRecord {
            key: "k1".into(),
            outputs: stamp_outputs(root, &outputs),
            duration_ms: 10,
        };
        assert_eq!(record.outputs.len(), 1);
        assert_eq!(record.outputs[0].path, "web/raw/island.glb");
        assert!(is_fresh(&record, "k1", root, &outputs));
        assert!(!is_fresh(&record, "k2", root, &outputs));
        assert!(!is_fresh(&record, "k1", root, &[]));

        sleep(Duration::from_millis(5));
        write(root, "web/raw/island.glb", b"glb changed");
        assert!(!is_fresh(&record, "k1", root, &outputs));

        fs::remove_file(root.join("web/raw/island.glb")).expect("rm");
        assert!(!is_fresh(&record, "k1", root, &outputs));
    }

    #[test]
    fn records_and_memo_persist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = project_cache_dir(dir.path(), "moon-island");
        assert_eq!(load_record(&cache, "build"), None);
        let record = CacheRecord {
            key: "abc".into(),
            outputs: Vec::new(),
            duration_ms: 1200,
        };
        save_record(&cache, "build", &record).expect("save");
        assert_eq!(load_record(&cache, "build"), Some(record));
        forget(&cache, "build");
        assert_eq!(load_record(&cache, "build"), None);

        let file = dir.path().join("big.blend");
        fs::write(&file, b"payload").expect("write");
        let mut hasher = FileHasher::with_memo(&cache);
        let first = hasher.hash(&file).expect("hash");
        hasher.flush();
        assert!(cache.join(MEMO_FILE).is_file());
        let mut again = FileHasher::with_memo(&cache);
        assert_eq!(again.hash(&file).expect("memo"), first);
        assert!(!again.dirty, "served from the memo");
        assert_eq!(first, hash_file(&file).expect("direct"));
    }

    #[test]
    fn globs_escape_the_root_and_report_empty_patterns() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("odd [name]");
        write(&root, "a/one.png", b"1");
        write(&root, "a/two.png", b"2");
        let found = expand(&root, &["a/*.png".to_string(), "b/*.png".to_string()]);
        assert_eq!(found[0].1.len(), 2);
        assert!(found[1].1.is_empty());
    }
}
