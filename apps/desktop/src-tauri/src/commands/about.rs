//! What Settings > About shows: the build, the machine and the updater, and
//! the links it opens. Nothing here is sent anywhere; the web view builds the
//! bug report text and the user decides what to paste.

use std::path::Path;
use std::time::Duration;

use maquette_core::{CommandRunner as _, HostOs, SystemRunner, ToolId};
use serde::Serialize;
use tauri::{AppHandle, State};

use super::CommandResult;
use crate::opener;
use crate::session::Session;
use crate::updater::{UpdaterStatus, Updates};

/// The commit a release build was made from, or "dev".
pub fn commit() -> &'static str {
    option_env!("MAQUETTE_COMMIT").unwrap_or("dev")
}

/// The day a release build was made, or "dev".
pub fn build_date() -> &'static str {
    option_env!("MAQUETTE_BUILD_DATE").unwrap_or("dev")
}

/// The build, the machine and the updater, for Settings > About.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    pub name: String,
    pub version: String,
    /// The short commit hash, or "dev" for a development build.
    pub commit: String,
    /// `YYYY-MM-DD`, or "dev" for a development build.
    pub build_date: String,
    pub os: HostOs,
    /// For example "15.2" on macOS or "10.0.26100" on Windows, when known.
    pub os_version: Option<String>,
    /// The CPU architecture, for example "aarch64" or "x86_64".
    pub arch: String,
    pub tauri_version: String,
    /// The Blender version from the last toolchain check, if one ran.
    pub blender: Option<String>,
    pub updater: UpdaterStatus,
}

/// `Microsoft Windows [Version 10.0.26100.2314]` to `10.0.26100.2314`.
pub(crate) fn parse_windows_ver(text: &str) -> Option<String> {
    let start = text.find("Version ")? + "Version ".len();
    let rest = &text[start..];
    let end = rest.find(']').unwrap_or(rest.len());
    let version = rest[..end].trim();
    (!version.is_empty()).then(|| version.to_string())
}

/// Asks the OS for its version, with a short timeout. None when it cannot
/// tell; About still shows the OS name.
fn os_version() -> Option<String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let timeout = Duration::from_secs(3);
    let (program, args, windows): (&str, &[&str], bool) = if cfg!(target_os = "macos") {
        ("/usr/bin/sw_vers", &["-productVersion"], false)
    } else if cfg!(windows) {
        ("cmd", &["/C", "ver"], true)
    } else {
        return None;
    };
    let out = SystemRunner
        .run(Path::new(program), args, &path, timeout)
        .ok()?;
    if !out.status_ok {
        return None;
    }
    if windows {
        parse_windows_ver(&out.stdout)
    } else {
        let version = out.stdout.trim();
        (!version.is_empty()).then(|| version.to_string())
    }
}

#[tauri::command]
#[specta::specta]
pub async fn app_about(
    app: AppHandle,
    session: State<'_, Session>,
    updates: State<'_, Updates>,
) -> CommandResult<AboutInfo> {
    let os_version = tauri::async_runtime::spawn_blocking(os_version)
        .await
        .map_err(super::join_failed)?;
    let blender = session.tools().and_then(|reports| {
        reports
            .into_iter()
            .find(|r| r.id == ToolId::Blender)
            .and_then(|r| r.version)
    });
    let info = app.package_info();
    Ok(AboutInfo {
        name: info.name.clone(),
        version: info.version.to_string(),
        commit: commit().to_string(),
        build_date: build_date().to_string(),
        os: HostOs::current(),
        os_version,
        arch: std::env::consts::ARCH.to_string(),
        tauri_version: tauri::VERSION.to_string(),
        blender,
        updater: updates.status(),
    })
}

/// Opens one of the project's own pages (the repo, its releases and issue
/// forms, the docs site) in the browser. Anything else is refused.
#[tauri::command(async)]
#[specta::specta]
pub fn app_open_link(url: String) -> CommandResult<()> {
    opener::open_project_link(&url).map_err(|err| {
        tracing::warn!(error = %err, "could not open a project link");
        super::invalid(&format!("Could not open that link. {err}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_windows_version_out_of_ver() {
        assert_eq!(
            parse_windows_ver("\r\nMicrosoft Windows [Version 10.0.26100.2314]\r\n").as_deref(),
            Some("10.0.26100.2314")
        );
        assert_eq!(parse_windows_ver("something else"), None);
        assert_eq!(parse_windows_ver("[Version ]"), None);
    }

    #[test]
    fn a_debug_build_names_itself_dev() {
        if cfg!(debug_assertions) && option_env!("MAQUETTE_COMMIT").is_none() {
            assert_eq!(commit(), "dev");
            assert_eq!(build_date(), "dev");
        }
    }
}
