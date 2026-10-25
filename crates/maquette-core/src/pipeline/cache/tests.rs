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
