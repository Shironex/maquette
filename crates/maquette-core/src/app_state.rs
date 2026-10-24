//! Per machine app state (last workspace, tool overrides). Lives in the app
//! data folder, not in the workspace, because install paths differ per
//! machine.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::{from_json_with_defaults, write_json};
use crate::toolchain::ToolId;

pub const APP_STATE_FILE: &str = "state.json";
pub const APP_STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub version: u32,
    pub last_workspace: Option<String>,
    /// Keyed by tool id, for example `{"blender": "/Applications/Blender.app"}`.
    pub tool_overrides: BTreeMap<ToolId, String>,
    /// The bearer token agents send to the built in MCP server. Made once on
    /// first use and kept here, per machine, so it never lands in a
    /// workspace or a repo.
    #[serde(default)]
    pub mcp_token: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            version: APP_STATE_VERSION,
            last_workspace: None,
            tool_overrides: BTreeMap::new(),
            mcp_token: None,
        }
    }
}

/// Never fails: a missing file gives the default, and an unreadable or
/// corrupt one is moved aside to `state.json.corrupt-<millis>` first.
pub fn load(dir: &Path) -> AppState {
    let path = dir.join(APP_STATE_FILE);
    let parsed = match fs::read_to_string(&path) {
        Ok(text) => from_json_with_defaults::<AppState>(&text).map_err(|e| e.to_string()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => return AppState::default(),
        Err(err) => Err(err.to_string()),
    };
    match parsed {
        Ok(mut state) if state.version <= APP_STATE_VERSION => {
            state.version = APP_STATE_VERSION;
            state
        }
        Ok(state) => {
            tracing::warn!(
                found = state.version,
                "app state comes from a newer version, starting fresh"
            );
            AppState::default()
        }
        Err(reason) => {
            let millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let aside = dir.join(format!("{APP_STATE_FILE}.corrupt-{millis}"));
            match fs::rename(&path, &aside) {
                Ok(()) => tracing::warn!(
                    %reason,
                    moved_to = %aside.display(),
                    "app state was unreadable, moved it aside"
                ),
                Err(err) => tracing::warn!(
                    %reason,
                    error = %err,
                    "app state was unreadable and could not be moved aside"
                ),
            }
            AppState::default()
        }
    }
}

pub fn save(dir: &Path, state: &AppState) -> CoreResult<()> {
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    write_json(&dir.join(APP_STATE_FILE), state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_is_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(load(dir.path()), AppState::default());
    }

    #[test]
    fn round_trip_with_overrides() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("app").join("data");
        let mut state = AppState {
            last_workspace: Some("/Users/me/Maquette".to_string()),
            ..AppState::default()
        };
        state
            .tool_overrides
            .insert(ToolId::Blender, "/Applications/Blender.app".to_string());
        state
            .tool_overrides
            .insert(ToolId::GltfTransform, "/opt/gt".to_string());
        save(&nested, &state).expect("save");
        let text = fs::read_to_string(nested.join(APP_STATE_FILE)).expect("read");
        assert!(text.contains("\"toolOverrides\""));
        assert!(text.contains("\"blender\": \"/Applications/Blender.app\""));
        assert!(text.contains("\"gltf-transform\": \"/opt/gt\""));
        assert_eq!(load(&nested), state);
    }

    #[test]
    fn corrupt_file_is_quarantined() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join(APP_STATE_FILE), "{ not json").expect("write");
        assert_eq!(load(dir.path()), AppState::default());
        assert!(!dir.path().join(APP_STATE_FILE).exists());
        let aside: Vec<String> = fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("state.json.corrupt-"))
            .collect();
        assert_eq!(aside.len(), 1);
    }

    #[test]
    fn unknown_tool_key_is_corrupt() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join(APP_STATE_FILE),
            r#"{"toolOverrides": {"maya": "/x"}}"#,
        )
        .expect("write");
        assert_eq!(load(dir.path()), AppState::default());
    }
}
