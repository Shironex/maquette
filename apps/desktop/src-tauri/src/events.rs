//! Events the shell pushes to the web view. Core stays free of Tauri, so
//! its event types are wrapped here.

use maquette_core::ProjectManifest;
use maquette_core::concepts::ConceptEvent;
use maquette_core::concepts::backend::ImageSettings;
use maquette_core::concepts::codex_usage::CodexUsage;
use maquette_core::pipeline::runner::{PipelineEvent, RunSink};
use maquette_core::providers::ProviderEvent;
use maquette_core::thumbs::ThumbState;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_specta::Event;

/// The window was asked to close while the web view has unapplied work. The
/// close is held back until the web view answers with `quitNow`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct QuitRequested;

/// A run changed state or printed a line.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct PipelineUpdate(pub PipelineEvent);

/// Models in the inbox folder were added, changed or removed.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct InboxChanged {
    pub paths: Vec<String>,
}

/// A model's thumbnail finished, or will not come from Blender (`state` is
/// fallback, so the web view stores a snapshot of its own preview). With
/// `project`, a Board asset now shows it. `bytes` and `modified_ms` are the
/// file's as the render was asked for, so the web view can drop a late event
/// for an old file that shared the name.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailReady {
    pub file: String,
    pub bytes: Option<f64>,
    pub modified_ms: Option<f64>,
    pub thumbnail: Option<String>,
    pub state: ThumbState,
    pub project: Option<ProjectManifest>,
}

/// A concept job changed state or printed a line, or a direction got a new
/// image.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ConceptUpdate(pub ConceptEvent);

/// A Codex usage read finished: the status bar and the Lab show it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct CodexUsageUpdate(pub CodexUsage);

/// An agent changed the Lab's image backends (`set_image_backend`):
/// Settings takes them into its form.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ImageSettingsChanged(pub ImageSettings);

/// A Send to 3D job changed state or progress.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ProviderUpdate(pub ProviderEvent);

/// An agent changed a project through the MCP server, for example by
/// accepting an inbox model, so the Board shows the new state.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChanged {
    pub project: ProjectManifest,
}

/// The library changed: a tool added a model to it or used or updated one
/// from it, so the Library view and the Board's update marks reload.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct LibraryChanged;

/// A project's built assets changed: one was added, started or finished
/// building, so the Board reloads how they stand.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct BuiltChanged {
    pub project: String,
}

/// An agent saved a project's compression presets through the MCP server,
/// so the Pipeline view reloads them.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct PipelineChanged {
    pub project: String,
}

/// An agent changed a project's scene layout through the MCP server, so
/// the Layout editor reloads it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct LayoutChanged {
    pub project: String,
}

/// An agent changed a project's material tweaks through the MCP server, so
/// the Inspector reloads them.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct MaterialsChanged {
    pub project: String,
}

/// An agent changed or built an asset's avatar through the MCP server, so
/// the Avatar view reloads it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct AvatarChanged {
    pub project: String,
    pub asset: String,
}

/// An agent changed a project's real sizes or fix-ups through the MCP
/// server, so the Board, the Inbox and the Inspector reload them.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct PrepareChanged {
    pub project: String,
}

/// The Downloads watcher took a look at a new model file: copied it into
/// the inbox, holds it for a look, or asks which asset it is.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DownloadsArrival(pub maquette_core::providers::Arrival);

/// The site's dev server changed state or printed, for the Page view.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct PageServerChanged(pub maquette_core::site::preview::ServerStatus);

/// The MCP server started or stopped, took a call, or a run started by an
/// agent waits for a shipping approval.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct McpUpdate(pub crate::mcp::McpStatus);

/// A step of an app update: a check, its answer, download progress, or an
/// update ready to install.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct UpdaterUpdate(pub crate::updater::UpdaterEvent);

/// Sends runner events to every window.
/// The MCP hub sees every event too, so agents can follow runs.
pub struct TauriSink(pub AppHandle, pub std::sync::Arc<crate::mcp::hub::Hub>);

impl RunSink for TauriSink {
    fn event(&self, event: PipelineEvent) {
        self.1.observe(&event);
        if let Err(err) = PipelineUpdate(event).emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a pipeline event");
        }
    }
}
