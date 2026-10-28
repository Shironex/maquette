use std::fs;
use std::path::PathBuf;

use super::*;
use crate::assets::{AssetKind, Stage};
use crate::project::{self, NewProject, ProjectTemplate};
use crate::workspace::{self, WorkspaceConfig};

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    config: WorkspaceConfig,
    project: String,
    project_dir: PathBuf,
    /// Where the "outside" scripts live, like a folder of hand made pieces.
    src: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Study room".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    let src = dir.path().join("pieces");
    fs::create_dir_all(&src).expect("src");
    fs::write(
        src.join("laptop.py"),
        "import common\ndef build():\n    pass\n",
    )
    .expect("laptop");
    fs::write(src.join("desk.py"), "import common\n").expect("desk");
    fs::write(src.join("common.py"), "PALETTE = 1\n").expect("common");
    Fixture {
        _dir: dir,
        root,
        config,
        project: manifest.id,
        project_dir,
        src,
    }
}

fn new_built(f: &Fixture, name: &str, script: &str) -> NewBuilt {
    NewBuilt {
        name: name.into(),
        kind: AssetKind::Prop,
        script: f.src.join(script).display().to_string(),
        helpers: vec![f.src.join("common.py").display().to_string()],
        root: None,
        replace: false,
    }
}

fn add_one(f: &Fixture, name: &str, script: &str) -> Asset {
    add(
        &f.root,
        &f.config.layout,
        &f.project,
        &new_built(f, name, script),
    )
    .expect("add")
    .0
}

fn source(f: &Fixture, id: &str) -> BuiltSource {
    load(&f.project_dir).expect("load").assets[id].clone()
}

#[test]
fn adding_copies_the_files_and_puts_the_asset_at_blender() {
    let f = fixture();
    let asset = add_one(&f, "Laptop", "laptop.py");
    assert_eq!(asset.id, "laptop");
    assert_eq!(asset.stage, Stage::Blender);
    assert!(asset.versions.is_empty());
    let found = source(&f, "laptop");
    assert_eq!(found.script, "built/laptop.py");
    assert_eq!(found.helpers, vec!["built/common.py".to_string()]);
    assert!(found.last.is_none());
    assert_eq!(
        fs::read_to_string(f.project_dir.join("built/common.py")).expect("copy"),
        "PALETTE = 1\n"
    );
    let manifest = project::load(&f.root, &f.config.layout, &f.project).expect("manifest");
    let listed = list(&f.project_dir, &manifest, &[]).expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].state, BuiltState::New);
}

#[test]
fn a_shared_helper_is_kept_once_and_a_different_one_is_refused() {
    let f = fixture();
    add_one(&f, "Laptop", "laptop.py");
    add_one(&f, "Desk", "desk.py");
    assert_eq!(
        source(&f, "desk").helpers,
        vec!["built/common.py".to_string()]
    );

    fs::write(f.src.join("common.py"), "PALETTE = 2\n").expect("edit");
    fs::write(f.src.join("rug.py"), "import common\n").expect("rug");
    let refused = add(
        &f.root,
        &f.config.layout,
        &f.project,
        &new_built(&f, "Rug", "rug.py"),
    )
    .expect_err("clash");
    assert!(
        refused.to_string().contains("built/common.py is already"),
        "{refused}"
    );
    // Nothing was written: no rug script, no rug asset.
    assert!(!f.project_dir.join("built/rug.py").exists());
    let manifest = project::load(&f.root, &f.config.layout, &f.project).expect("manifest");
    assert!(manifest.assets.iter().all(|a| a.id != "rug"));

    let mut replacing = new_built(&f, "Rug", "rug.py");
    replacing.replace = true;
    add(&f.root, &f.config.layout, &f.project, &replacing).expect("replace");
    assert_eq!(
        fs::read_to_string(f.project_dir.join("built/common.py")).expect("copy"),
        "PALETTE = 2\n"
    );
}

#[test]
fn adding_refuses_bad_scripts_and_taken_names() {
    let f = fixture();
    add_one(&f, "Laptop", "laptop.py");
    let again = add(
        &f.root,
        &f.config.layout,
        &f.project,
        &new_built(&f, "laptop", "desk.py"),
    );
    assert!(
        again
            .expect_err("taken")
            .to_string()
            .contains("already an asset")
    );

    fs::write(f.src.join("notes.txt"), "x").expect("txt");
    let text = add(
        &f.root,
        &f.config.layout,
        &f.project,
        &new_built(&f, "Notes", "notes.txt"),
    );
    assert!(
        text.expect_err("not py")
            .to_string()
            .contains("not a Python script")
    );

    let gone = add(
        &f.root,
        &f.config.layout,
        &f.project,
        &new_built(&f, "Gone", "gone.py"),
    );
    assert!(
        gone.expect_err("missing")
            .to_string()
            .contains("no script file")
    );

    let other = f.src.join("other");
    fs::create_dir_all(&other).expect("other");
    fs::write(other.join("common.py"), "B = 2\n").expect("second common");
    let mut twice = new_built(&f, "Desk", "desk.py");
    twice
        .helpers
        .push(other.join("common.py").display().to_string());
    let clash = add(&f.root, &f.config.layout, &f.project, &twice);
    assert!(
        clash
            .expect_err("two names")
            .to_string()
            .contains("Two files are called common.py")
    );
}

#[test]
fn the_key_follows_the_files_and_not_the_helper_order() {
    let f = fixture();
    add_one(&f, "Laptop", "laptop.py");
    let mut src = source(&f, "laptop");
    let first = key(&f.project_dir, &src).expect("key");
    fs::write(f.project_dir.join("built/extra.json"), "{}").expect("extra");
    src.helpers.push("built/extra.json".into());
    let with_extra = key(&f.project_dir, &src).expect("key");
    assert_ne!(first, with_extra);
    src.helpers.reverse();
    assert_eq!(key(&f.project_dir, &src).expect("key"), with_extra);
    // CRLF line endings key the same as LF.
    fs::write(f.project_dir.join("built/extra.json"), "{\r\n}").expect("crlf");
    let crlf = key(&f.project_dir, &src).expect("key");
    fs::write(f.project_dir.join("built/extra.json"), "{\n}").expect("lf");
    assert_eq!(key(&f.project_dir, &src).expect("key"), crlf);
    src.root = Some("laptop".into());
    assert_ne!(key(&f.project_dir, &src).expect("key"), crlf);
    fs::remove_file(f.project_dir.join("built/extra.json")).expect("rm");
    assert_eq!(
        key(&f.project_dir, &src).expect_err("missing"),
        vec!["built/extra.json".to_string()]
    );
}

#[test]
fn a_newer_built_file_is_refused() {
    let f = fixture();
    fs::write(
        f.project_dir.join(BUILT_FILE),
        r#"{"version": 9, "assets": {}}"#,
    )
    .expect("w");
    assert_eq!(
        load(&f.project_dir).expect_err("newer").code(),
        "unsupported-version"
    );
    let refused = add(
        &f.root,
        &f.config.layout,
        &f.project,
        &new_built(&f, "Laptop", "laptop.py"),
    );
    assert!(refused.is_err());
    assert!(!f.project_dir.join("built/laptop.py").exists());
}

#[cfg(unix)]
mod building {
    use super::*;
    use crate::assets::AssetStatus;
    use crate::built::build::BuildContext;
    use crate::pipeline::tools::{ResolvedTool, Toolbox};
    use crate::toolchain::{HostOs, ToolId};
    use std::ffi::OsString;

    /// A Blender stand in: it copies one of two models to `--output` (the
    /// second when the staged script says "two"), and fails on "boom".
    fn fake_blender(f: &Fixture) -> Toolbox {
        let dir = f.root.parent().expect("parent").join("fake");
        fs::create_dir_all(&dir).expect("fake dir");
        let one = dir.join("one.glb");
        let two = dir.join("two.glb");
        fs::write(&one, crate::inbox::fixture::clean_prop()).expect("one");
        fs::write(&two, crate::inbox::fixture::messy_prop()).expect("two");
        let script = format!(
            "#!/bin/sh\n\
             while [ $# -gt 0 ]; do case \"$1\" in --script) s=\"$2\"; shift;; --output) o=\"$2\"; shift;; --report) r=\"$2\"; shift;; esac; shift; done\n\
             if grep -q boom \"$s\"; then echo 'Traceback'; echo '[built] Error: NameError: boom'; exit 1; fi\n\
             if grep -q two \"$s\"; then cp '{two}' \"$o\"; else cp '{one}' \"$o\"; fi\n\
             echo '{{\"objects\": [\"laptop\", \"lid\"], \"materials\": [\"screen\"], \"triangles\": 12}}' > \"$r\"\n",
            one = one.display(),
            two = two.display(),
        );
        let program = dir.join("blender");
        fs::write(&program, script).expect("script");
        let mut perms = fs::metadata(&program).expect("meta").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        fs::set_permissions(&program, perms).expect("chmod");
        let mut tools = Toolbox {
            path_env: OsString::from("/bin:/usr/bin"),
            os: Some(HostOs::current()),
            ..Toolbox::default()
        };
        tools.tools.insert(
            ToolId::Blender,
            ResolvedTool {
                program,
                prefix: Vec::new(),
                version: "5.2.2".into(),
            },
        );
        tools
    }

    fn build(f: &Fixture, tools: &Toolbox, force: bool) -> BuildOutcome {
        let ctx = BuildContext {
            workspace_root: &f.root,
            config: &f.config,
            project: &f.project,
            tools,
        };
        Builder::default()
            .build(&ctx, "laptop", force, &|_| {})
            .expect("build")
    }

    fn state(f: &Fixture) -> BuiltState {
        let manifest = project::load(&f.root, &f.config.layout, &f.project).expect("manifest");
        list(&f.project_dir, &manifest, &[]).expect("list")[0].state
    }

    #[test]
    fn a_build_makes_a_version_and_skips_what_did_not_change() {
        let f = fixture();
        add_one(&f, "Laptop", "laptop.py");
        let tools = fake_blender(&f);

        let first = build(&f, &tools, false);
        assert_eq!(first.result, BuildResult::Built);
        let version = first.version.expect("version");
        assert_eq!(version.version, 1);
        assert_eq!(version.file, "models/laptop_v1.glb");
        assert!(f.project_dir.join("models/laptop_v1.glb").is_file());
        assert!(version.stats.is_some());
        assert_eq!(first.asset.status, AssetStatus::Done);
        let record = first.record.expect("record");
        assert_eq!(
            record.objects,
            vec!["laptop".to_string(), "lid".to_string()]
        );
        assert_eq!(record.version, Some(1));
        assert_eq!(state(&f), BuiltState::Current);
        // The script ran in the cache, never in the project.
        assert!(!f.project_dir.join("built/built.glb").exists());

        assert_eq!(build(&f, &tools, false).result, BuildResult::UpToDate);
        let again = build(&f, &tools, true);
        assert_eq!(again.result, BuildResult::Unchanged);
        assert_eq!(again.asset.versions.len(), 1);

        fs::write(f.project_dir.join("built/laptop.py"), "# two\n").expect("edit");
        assert_eq!(state(&f), BuiltState::Stale);
        let manifest = project::load(&f.root, &f.config.layout, &f.project).expect("manifest");
        assert_eq!(
            to_rebuild(&f.project_dir, &manifest).expect("stale").len(),
            1
        );
        let second = build(&f, &tools, false);
        assert_eq!(second.result, BuildResult::Built);
        assert_eq!(second.version.expect("v2").file, "models/laptop_v2.glb");
        assert_eq!(state(&f), BuiltState::Current);
    }

    #[test]
    fn a_failing_script_is_recorded_with_its_error() {
        let f = fixture();
        add_one(&f, "Laptop", "laptop.py");
        fs::write(f.project_dir.join("built/laptop.py"), "boom\n").expect("edit");
        let tools = fake_blender(&f);
        let failed = build(&f, &tools, false);
        assert_eq!(failed.result, BuildResult::Failed);
        assert_eq!(failed.asset.status, AssetStatus::Failed);
        let error = failed.record.and_then(|r| r.error).expect("error");
        assert_eq!(error, "NameError: boom");
        assert!(failed.log.iter().any(|l| l.contains("Traceback")));
        assert_eq!(state(&f), BuiltState::Failed);
        let manifest = project::load(&f.root, &f.config.layout, &f.project).expect("manifest");
        assert!(
            to_rebuild(&f.project_dir, &manifest)
                .expect("list")
                .is_empty()
        );
    }

    #[test]
    fn without_blender_nothing_runs() {
        let f = fixture();
        add_one(&f, "Laptop", "laptop.py");
        let ctx = BuildContext {
            workspace_root: &f.root,
            config: &f.config,
            project: &f.project,
            tools: &Toolbox::default(),
        };
        let err = Builder::default()
            .build(&ctx, "laptop", false, &|_| {})
            .expect_err("no blender");
        assert_eq!(err.code(), "unavailable");
        assert_eq!(state(&f), BuiltState::New);
    }

    #[test]
    fn a_plain_asset_does_not_build() {
        let f = fixture();
        project::modify(&f.root, &f.config.layout, &f.project, |m| {
            crate::assets::add(
                m,
                crate::assets::NewAsset {
                    name: "Laptop".into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Concept,
                },
            )
            .map(|_| ())
        })
        .expect("plain");
        let tools = fake_blender(&f);
        let ctx = BuildContext {
            workspace_root: &f.root,
            config: &f.config,
            project: &f.project,
            tools: &tools,
        };
        let err = Builder::default()
            .build(&ctx, "laptop", false, &|_| {})
            .expect_err("not built");
        assert!(err.to_string().contains("not a built asset"));
    }

    #[test]
    fn busy_lists_nothing_once_a_build_is_done() {
        let f = fixture();
        add_one(&f, "Laptop", "laptop.py");
        let tools = fake_blender(&f);
        let builder = Builder::default();
        let ctx = BuildContext {
            workspace_root: &f.root,
            config: &f.config,
            project: &f.project,
            tools: &tools,
        };
        let seen = std::sync::Mutex::new(Vec::new());
        builder
            .build(&ctx, "laptop", false, &|m: &ProjectManifest| {
                let status = m.assets[0].status;
                seen.lock().expect("seen").push(status);
            })
            .expect("build");
        assert_eq!(
            *seen.lock().expect("seen"),
            vec![AssetStatus::Running, AssetStatus::Done]
        );
        assert!(builder.busy(&f.project).is_empty());
    }
}
