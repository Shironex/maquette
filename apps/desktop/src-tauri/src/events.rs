//! Events the shell pushes to the web view. Core stays free of Tauri, so
//! its event types are wrapped here.

use maquette_core::pipeline::runner::{PipelineEvent, RunSink};
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

/// Sends runner events to every window.
pub struct TauriSink(pub AppHandle);

impl RunSink for TauriSink {
    fn event(&self, event: PipelineEvent) {
        if let Err(err) = PipelineUpdate(event).emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a pipeline event");
        }
    }
}
