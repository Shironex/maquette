use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::sync::Mutex;

use super::detection::{BROKEN_HINT, CODEX_NOT_SIGNED_IN};
use super::lookup::{APP_BINARY, candidates, known_dirs, known_files, version_key};
use super::*;

struct FakeRunner {
    result: Result<ProbeOutput, ProbeError>,
    calls: Mutex<Vec<(PathBuf, Vec<String>, Duration)>>,
}

impl FakeRunner {
    fn ok(stdout: &str) -> Self {
        Self::with(Ok(ProbeOutput {
            status_ok: true,
            stdout: stdout.to_string(),
            stderr: String::new(),
        }))
    }

    fn with(result: Result<ProbeOutput, ProbeError>) -> Self {
        Self {
            result,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<(PathBuf, Vec<String>, Duration)> {
        self.calls.lock().expect("lock").clone()
    }
}

impl CommandRunner for FakeRunner {
    fn run(
        &self,
        program: &Path,
        args: &[&str],
        _path_env: &OsStr,
        timeout: Duration,
    ) -> Result<ProbeOutput, ProbeError> {
        self.calls.lock().expect("lock").push((
            program.to_path_buf(),
            args.iter().map(|a| a.to_string()).collect(),
            timeout,
        ));
        self.result.clone()
    }
}

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, b"#!/bin/sh\n").expect("write");
}

fn touch_exec(path: &Path) {
    touch(path);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod");
    }
}

fn unix_env(os: HostOs, path_dirs: Vec<PathBuf>) -> ProbeEnv {
    let mut env = ProbeEnv::empty(os);
    env.path_dirs = path_dirs;
    env
}

#[test]
fn tool_ids_and_labels() {
    assert_eq!(ToolId::ALL.len(), 6);
    assert_eq!(ToolId::Codex.label(), "Codex CLI");
    assert_eq!(ToolId::Toktx.label(), "KTX-Software toktx");
    assert!(ToolId::Blender.required());
    assert!(ToolId::ALL[1..].iter().all(|t| !t.required()));
    assert_eq!(
        serde_json::to_string(&ToolId::GltfTransform).expect("json"),
        "\"gltf-transform\""
    );
    assert_eq!(
        serde_json::to_string(&ToolSource::KnownLocation).expect("json"),
        "\"known-location\""
    );
}

#[test]
fn path_order_wins() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    touch_exec(&second.join("ffmpeg"));
    touch_exec(&first.join("ffmpeg"));
    let env = unix_env(HostOs::Mac, vec![first.clone(), second]);
    assert_eq!(find_on_path(&["ffmpeg"], &env), Some(first.join("ffmpeg")));
}

#[cfg(unix)]
#[test]
fn non_executable_is_skipped_on_unix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    touch(&first.join("node"));
    touch_exec(&second.join("node"));
    let env = unix_env(HostOs::Linux, vec![first, second.clone()]);
    assert_eq!(find_on_path(&["node"], &env), Some(second.join("node")));
}

#[test]
fn relative_path_entries_are_ignored() {
    let env = unix_env(HostOs::Mac, vec![PathBuf::from("."), PathBuf::from("")]);
    assert_eq!(find_on_path(&["sh"], &env), None);
}

#[test]
fn windows_pathext_lookup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin");
    let npm = dir.path().join("npm");
    touch(&bin.join("blender.exe"));
    touch(&npm.join("codex.CMD"));
    touch(&npm.join("codex"));
    let mut env = unix_env(HostOs::Windows, vec![bin.clone(), npm.clone()]);
    env.pathext = vec![".COM".into(), ".EXE".into(), ".BAT".into(), ".CMD".into()];

    assert_eq!(
        find_on_path(&["blender"], &env),
        Some(bin.join("blender.exe"))
    );
    assert_eq!(find_on_path(&["codex"], &env), Some(npm.join("codex.CMD")));
    assert_eq!(
        find_on_path(&["blender.exe"], &env),
        Some(bin.join("blender.exe"))
    );
    assert_eq!(find_on_path(&["ffmpeg"], &env), None);

    env.pathext = Vec::new();
    assert_eq!(find_on_path(&["codex"], &env), Some(npm.join("codex.CMD")));
}

#[test]
fn windows_blender_foundation_prefers_highest_version() {
    let dir = tempfile::tempdir().expect("tempdir");
    let pf = dir.path().join("Program Files");
    let foundation = pf.join("Blender Foundation");
    for version in ["Blender 3.6", "Blender 4.10", "Blender 4.3"] {
        touch(&foundation.join(version).join("blender.exe"));
    }
    let mut env = ProbeEnv::empty(HostOs::Windows);
    env.program_files = vec![pf];
    let locations = known_locations(ToolId::Blender, &env);
    assert_eq!(
        locations[0],
        foundation.join("Blender 4.10").join("blender.exe")
    );
    assert_eq!(
        locations[1],
        foundation.join("Blender 4.3").join("blender.exe")
    );
    assert_eq!(
        locations[2],
        foundation.join("Blender 3.6").join("blender.exe")
    );

    let runner = FakeRunner::ok("Blender 4.10.0");
    let report = detect(ToolId::Blender, &env, None, &runner);
    assert_eq!(report.status, ToolStatus::Found);
    assert_eq!(report.source, Some(ToolSource::KnownLocation));
    assert_eq!(
        report.path,
        Some(
            foundation
                .join("Blender 4.10")
                .join("blender.exe")
                .display()
                .to_string()
        )
    );
}

#[test]
fn windows_known_dirs_cover_npm_and_steam() {
    let mut env = ProbeEnv::empty(HostOs::Windows);
    env.app_data = Some(PathBuf::from("/AppData/Roaming"));
    env.program_files = vec![PathBuf::from("/PF86")];
    let codex = known_locations(ToolId::Codex, &env);
    assert!(codex.contains(&PathBuf::from("/AppData/Roaming/npm/codex.CMD")));
    let blender = known_locations(ToolId::Blender, &env);
    assert!(
        blender.contains(
            &PathBuf::from("/PF86")
                .join("Steam")
                .join("steamapps")
                .join("common")
                .join("Blender")
                .join("blender.exe")
        )
    );
}

#[test]
fn mac_versioned_apps_sorted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let apps = dir.path().join("Applications");
    for name in [
        "Blender.app",
        "Blender 4.2.app",
        "Blender 4.10.app",
        "Other.app",
    ] {
        fs::create_dir_all(apps.join(name)).expect("mkdir");
    }
    let mut env = ProbeEnv::empty(HostOs::Mac);
    env.applications_dirs = vec![apps.clone()];
    let files = known_files(ToolId::Blender, &env);
    assert_eq!(
        files,
        vec![
            apps.join("Blender.app").join(APP_BINARY),
            apps.join("Blender 4.10.app").join(APP_BINARY),
            apps.join("Blender 4.2.app").join(APP_BINARY),
        ]
    );
}

#[test]
fn nvm_dirs_newest_first() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().to_path_buf();
    for v in ["v18.20.0", "v22.3.0", "v20.11.1"] {
        fs::create_dir_all(home.join(".nvm/versions/node").join(v).join("bin")).expect("mkdir");
    }
    let mut env = ProbeEnv::empty(HostOs::Mac);
    env.home = Some(home.clone());
    let dirs = known_dirs(ToolId::Node, &env);
    let nvm: Vec<_> = dirs
        .iter()
        .filter(|d| d.starts_with(home.join(".nvm")))
        .cloned()
        .collect();
    assert_eq!(
        nvm,
        vec![
            home.join(".nvm/versions/node/v22.3.0/bin"),
            home.join(".nvm/versions/node/v20.11.1/bin"),
            home.join(".nvm/versions/node/v18.20.0/bin"),
        ]
    );
}

#[test]
fn mac_app_bundle_override_maps_to_binary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = dir.path().join("Blender 4.2.app");
    touch_exec(&app.join(APP_BINARY));
    let env = ProbeEnv::empty(HostOs::Mac);
    let runner = FakeRunner::ok("Blender 4.2.0 LTS");
    let report = detect(ToolId::Blender, &env, Some(&app), &runner);
    assert_eq!(report.status, ToolStatus::Found);
    assert_eq!(report.source, Some(ToolSource::Override));
    assert_eq!(
        report.path,
        Some(app.join(APP_BINARY).display().to_string())
    );
    assert_eq!(report.version.as_deref(), Some("4.2.0"));
    assert_eq!(report.detail.as_deref(), Some("runs headless"));
    assert_eq!(report.warning, None);
    let calls = runner.calls();
    assert_eq!(
        calls[0].1,
        vec!["--background", "--factory-startup", "--version"]
    );
    assert_eq!(calls[0].2, Duration::from_secs(20));
}

#[test]
fn missing_override_falls_back_to_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin");
    touch_exec(&bin.join("ffmpeg"));
    let env = unix_env(HostOs::Mac, vec![bin.clone()]);
    let runner = FakeRunner::ok("ffmpeg version 7.1 Copyright");
    let gone = dir.path().join("gone").join("ffmpeg");
    let report = detect(ToolId::Ffmpeg, &env, Some(&gone), &runner);
    assert_eq!(report.status, ToolStatus::Found);
    assert_eq!(report.source, Some(ToolSource::Path));
    assert_eq!(report.path, Some(bin.join("ffmpeg").display().to_string()));
    assert_eq!(report.version.as_deref(), Some("7.1"));
    assert!(report.warning.is_some());
    assert_eq!(runner.calls()[0].1, vec!["-version"]);
}

#[test]
fn gltf_transform_falls_back_to_npx() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin");
    touch_exec(&bin.join("npx"));
    let env = unix_env(HostOs::Linux, vec![bin.clone()]);
    let runner = FakeRunner::ok("");
    let report = detect(ToolId::GltfTransform, &env, None, &runner);
    assert_eq!(report.status, ToolStatus::Found);
    assert_eq!(report.source, Some(ToolSource::Npx));
    assert_eq!(report.path, Some(bin.join("npx").display().to_string()));
    assert_eq!(report.version, None);
    assert_eq!(
        report.detail.as_deref(),
        Some("via npx, @gltf-transform/cli")
    );
    assert!(runner.calls().is_empty());
}

#[test]
fn broken_on_timeout_and_error_exit() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin");
    touch_exec(&bin.join("blender"));
    let env = unix_env(HostOs::Linux, vec![bin]);

    let report = detect(
        ToolId::Blender,
        &env,
        None,
        &FakeRunner::with(Err(ProbeError::Timeout)),
    );
    assert_eq!(report.status, ToolStatus::Broken);
    assert_eq!(
        report.detail.as_deref(),
        Some("did not answer --version in 20 s")
    );
    assert_eq!(report.hint.as_deref(), Some(BROKEN_HINT));
    assert!(blocks_continue(&[report]));

    let report = detect(
        ToolId::Blender,
        &env,
        None,
        &FakeRunner::with(Ok(ProbeOutput {
            status_ok: false,
            stdout: String::new(),
            stderr: "boom".into(),
        })),
    );
    assert_eq!(report.status, ToolStatus::Broken);
    assert_eq!(report.detail.as_deref(), Some("exited with an error"));

    let report = detect(
        ToolId::Blender,
        &env,
        None,
        &FakeRunner::with(Err(ProbeError::Spawn("denied".into()))),
    );
    assert_eq!(report.status, ToolStatus::Broken);
}

#[test]
fn codex_signed_in_or_not() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin");
    touch_exec(&bin.join("codex"));
    let codex_home = dir.path().join("codex-home");
    fs::create_dir_all(&codex_home).expect("mkdir");
    let mut env = unix_env(HostOs::Mac, vec![bin]);
    env.codex_home = Some(codex_home.clone());
    let runner = FakeRunner::ok("codex-cli 0.46.0\n");

    let report = detect(ToolId::Codex, &env, None, &runner);
    assert_eq!(report.status, ToolStatus::Found);
    assert_eq!(report.version.as_deref(), Some("0.46.0"));
    assert_eq!(report.detail.as_deref(), Some(CODEX_NOT_SIGNED_IN));
    assert!(report.warning.is_some());

    fs::write(codex_home.join("auth.json"), b"{}").expect("write");
    let report = detect(ToolId::Codex, &env, None, &runner);
    assert_eq!(report.detail.as_deref(), Some("signed in"));
    assert_eq!(report.warning, None);
    assert_eq!(report.hint, None);
}

#[test]
fn missing_tools_have_os_hints() {
    let runner = FakeRunner::ok("");
    let mac = detect(ToolId::Ffmpeg, &ProbeEnv::empty(HostOs::Mac), None, &runner);
    assert_eq!(mac.status, ToolStatus::Missing);
    assert_eq!(mac.path, None);
    assert_eq!(mac.detail.as_deref(), Some("not found on PATH"));
    assert_eq!(
        mac.hint.as_deref(),
        Some("Install it with brew install ffmpeg.")
    );

    let win = detect(
        ToolId::Ffmpeg,
        &ProbeEnv::empty(HostOs::Windows),
        None,
        &runner,
    );
    assert_eq!(
        win.hint.as_deref(),
        Some("Install it with winget install Gyan.FFmpeg.")
    );

    let blender = detect(
        ToolId::Blender,
        &ProbeEnv::empty(HostOs::Windows),
        None,
        &runner,
    );
    assert_eq!(
        blender.hint.as_deref(),
        Some("Install Blender from blender.org, or locate blender.exe.")
    );
    assert!(runner.calls().is_empty());
}

#[test]
fn detect_all_keeps_order_and_only_blender_blocks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin");
    touch_exec(&bin.join("blender"));
    let env = unix_env(HostOs::Linux, vec![bin.clone()]);
    let runner = FakeRunner::ok("Blender 4.3.2");
    let reports = detect_all(&env, &BTreeMap::new(), &runner);
    let ids: Vec<_> = reports.iter().map(|r| r.id).collect();
    assert_eq!(ids, ToolId::ALL.to_vec());
    assert_eq!(reports[0].status, ToolStatus::Found);
    assert!(reports[1..].iter().all(|r| r.status == ToolStatus::Missing));
    assert!(!blocks_continue(&reports));

    let empty = ProbeEnv::empty(HostOs::Linux);
    let mut overrides = BTreeMap::new();
    overrides.insert(ToolId::Blender, bin.join("blender"));
    let reports = detect_all(&empty, &overrides, &runner);
    assert_eq!(reports[0].source, Some(ToolSource::Override));

    let reports = detect_all(&empty, &BTreeMap::new(), &runner);
    assert!(blocks_continue(&reports));
}

#[test]
fn child_path_puts_extra_first_and_adds_known_dirs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let extra = dir.path().join("extra");
    let on_path = dir.path().join("path");
    let brew = dir.path().join("brew");
    let absent = dir.path().join("absent");
    for d in [&extra, &on_path, &brew] {
        fs::create_dir_all(d).expect("mkdir");
    }
    let mut env = unix_env(HostOs::Mac, vec![on_path.clone()]);
    env.system_bin_dirs = vec![brew.clone(), absent.clone()];
    let joined = env.child_path(Some(&extra));
    let dirs: Vec<PathBuf> = std::env::split_paths(&joined).collect();
    assert_eq!(dirs, vec![extra, on_path, brew]);
}

#[test]
fn version_key_sorts_numerically() {
    assert_eq!(version_key("Blender 4.10"), vec![4, 10]);
    assert_eq!(version_key("Blender 4.2.app"), vec![4, 2]);
    assert_eq!(version_key("v22.1.0"), vec![22, 1, 0]);
    assert!(version_key("Blender") < version_key("Blender 3.6"));
}

struct FailsUnder(PathBuf);

impl CommandRunner for FailsUnder {
    fn run(
        &self,
        program: &Path,
        _args: &[&str],
        _path_env: &OsStr,
        _timeout: Duration,
    ) -> Result<ProbeOutput, ProbeError> {
        Ok(ProbeOutput {
            status_ok: !program.starts_with(&self.0),
            stdout: "codex-cli 0.159.3".to_string(),
            stderr: String::new(),
        })
    }
}

#[test]
fn broken_copy_falls_through_to_next_working_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let stale = dir.path().join("stale");
    let good = dir.path().join("good");
    touch_exec(&stale.join("codex"));
    touch_exec(&good.join("codex"));
    let env = unix_env(HostOs::Mac, vec![stale.clone(), good.clone()]);

    let report = detect(ToolId::Codex, &env, None, &FailsUnder(stale.clone()));
    assert_eq!(report.status, ToolStatus::Found);
    assert_eq!(report.path, Some(good.join("codex").display().to_string()));
    assert_eq!(report.version.as_deref(), Some("0.159.3"));

    let report = detect(
        ToolId::Codex,
        &env,
        None,
        &FailsUnder(dir.path().to_path_buf()),
    );
    assert_eq!(report.status, ToolStatus::Broken);
    assert_eq!(report.path, Some(stale.join("codex").display().to_string()));
}

#[cfg(unix)]
#[test]
fn symlinked_duplicates_are_probed_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let real = dir.path().join("real");
    let link = dir.path().join("link");
    touch_exec(&real.join("node"));
    fs::create_dir_all(&link).expect("mkdir");
    std::os::unix::fs::symlink(real.join("node"), link.join("node")).expect("symlink");
    let env = unix_env(HostOs::Linux, vec![link, real]);
    assert_eq!(candidates(ToolId::Node, &env).len(), 1);
}
