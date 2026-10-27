fn main() {
    let attributes = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    tauri_build::try_build(attributes).expect("tauri-build");

    embed_app_manifest();
    stamp_build_info();
}

/// The commit and date a release build shows in About. A dev build shows
/// "dev" instead, so nothing here runs for it and a new commit never
/// rebuilds the shell. CI may pass both values in the environment.
fn stamp_build_info() {
    println!("cargo::rerun-if-env-changed=MAQUETTE_COMMIT");
    println!("cargo::rerun-if-env-changed=MAQUETTE_BUILD_DATE");
    if std::env::var("PROFILE").as_deref() != Ok("release") {
        return;
    }
    let commit = std::env::var("MAQUETTE_COMMIT")
        .ok()
        .filter(|c| !c.trim().is_empty())
        .or_else(git_commit)
        .unwrap_or_else(|| "unknown".to_string());
    let date = std::env::var("MAQUETTE_BUILD_DATE")
        .ok()
        .filter(|d| !d.trim().is_empty())
        .unwrap_or_else(today);
    println!("cargo::rustc-env=MAQUETTE_COMMIT={commit}");
    println!("cargo::rustc-env=MAQUETTE_BUILD_DATE={date}");
    watch_git_head();
}

fn git(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn git_commit() -> Option<String> {
    git(&["rev-parse", "--short=10", "HEAD"])
}

/// Rebuilds a release when HEAD moves, so About never names an old commit.
fn watch_git_head() {
    for path in ["HEAD", "logs/HEAD"] {
        if let Some(file) = git(&["rev-parse", "--git-path", path]) {
            println!("cargo::rerun-if-changed={file}");
        }
    }
}

/// Today in UTC as `YYYY-MM-DD`, from the days since 1970 (the civil
/// calendar conversion by Howard Hinnant), so no date crate is needed.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = i64::try_from(secs / 86_400).unwrap_or(0) + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// `tauri_build` embeds its manifest into the app binary only, so cargo test
/// binaries get none. `tauri-plugin-dialog` imports `TaskDialogIndirect`, which
/// only comctl32 v6 exports, so an unmanifested test binary would bind the v5
/// DLL and die at load on Windows before a single test runs. Linking
/// `app.manifest` here covers every target, and tauri's own copy is switched
/// off above so the two cannot collide.
fn embed_app_manifest() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let manifest = std::path::Path::new(&dir).join("app.manifest");

    println!("cargo::rerun-if-changed=app.manifest");
    println!("cargo::rustc-link-arg=/MANIFEST:EMBED");
    println!(
        "cargo::rustc-link-arg=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
