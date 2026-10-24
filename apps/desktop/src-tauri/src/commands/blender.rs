//! The one place Maquette opens Blender's window: the Open in Blender
//! action. Pipeline steps never come through here; they always run `-b`.

use std::path::Path;
use std::process::{Command, Stdio};

use maquette_core::inbox::open_in_blender_args;
use maquette_core::{ProbeEnv, SystemRunner, ToolId, ToolStatus, toolchain};

use super::{CommandResult, invalid};
use crate::session::Session;

/// Finds Blender from the last tool check (running one if needed) and opens
/// `file` in a new Blender window. It does not wait for Blender to close.
pub(crate) async fn open_gui(session: &Session, file: &Path) -> CommandResult<()> {
    if !file.is_file() {
        return Err(invalid("That file is gone. Refresh and try again."));
    }
    let reports = match session.tools() {
        Some(reports) => reports,
        None => {
            let overrides = super::toolchain::overrides(session, &session.app_state());
            let reports = tauri::async_runtime::spawn_blocking(move || {
                toolchain::detect_all(&ProbeEnv::from_system(), &overrides, &SystemRunner)
            })
            .await
            .map_err(super::join_failed)?;
            session.set_tools(Some(reports.clone()));
            reports
        }
    };
    let blender = reports
        .iter()
        .find(|r| r.id == ToolId::Blender && r.status == ToolStatus::Found)
        .and_then(|r| r.path.clone())
        .ok_or_else(|| invalid("Blender is not set up. Point Maquette at it in Settings."))?;
    let mut child = Command::new(&blender)
        .args(open_in_blender_args(file))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| invalid(&format!("Could not start Blender. {e}")))?;
    // Reap it when the user closes it, so it never lingers as a zombie.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
