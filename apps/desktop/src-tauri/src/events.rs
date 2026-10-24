//! Events the shell pushes to the web view. Core stays free of Tauri, so
//! its event types are wrapped here.

use maquette_core::ProjectManifest;
use maquette_core::concepts::ConceptEvent;
use maquette_core::pipeline::runner::{PipelineEvent, RunSink};
use maquette_core::providers::ProviderEvent;
use maquette_core::thumbs::ThumbState;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_specta::Event;

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
/// `project`, a Board asset now shows it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailReady {
    pub file: String,
    pub thumbnail: Option<String>,
    pub state: ThumbState,
    pub project: Option<ProjectManifest>,
}

/// A concept job changed state or printed a line, or a direction got a new
/// image.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ConceptUpdate(pub ConceptEvent);

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

/// An agent saved a project's compression presets through the MCP server,
/// so the Pipeline view reloads them.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct PipelineChanged {
    pub project: String,
}

/// The MCP server started or stopped, took a call, or a run started by an
/// agent waits for a shipping approval.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct McpUpdate(pub crate::mcp::McpStatus);

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
