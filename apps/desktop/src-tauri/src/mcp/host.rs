//! The services the MCP tools work with. The app hands them over from its
//! managed state; the integration test builds its own, without a window.

use maquette_core::ProjectManifest;
use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::thumbs::Thumbnails;
use tauri::{AppHandle, Manager as _};
use tauri_specta::Event as _;

use crate::commands::lab::Lab;
use crate::commands::pipeline::Pipelines;
use crate::commands::thumbs::Thumbs;
use crate::events::ProjectChanged;
use crate::session::Session;

pub trait Host: Send + Sync + 'static {
    fn session(&self) -> &Session;
    fn runner(&self) -> &PipelineRunner;
    fn concepts(&self) -> &Concepts;
    fn thumbs(&self) -> &Thumbnails;
    /// The tools a job runs with. May probe them, so it blocks.
    fn toolbox(&self) -> Toolbox;
    /// Tells the windows that a tool changed a project, so the Board updates.
    fn project_changed(&self, _project: &ProjectManifest) {}
}

/// The app's own services.
pub struct AppHost(pub AppHandle);

impl Host for AppHost {
    fn session(&self) -> &Session {
        self.0.state::<Session>().inner()
    }

    fn runner(&self) -> &PipelineRunner {
        &self.0.state::<Pipelines>().inner().0
    }

    fn concepts(&self) -> &Concepts {
        &self.0.state::<Lab>().inner().0
    }

    fn thumbs(&self) -> &Thumbnails {
        &self.0.state::<Thumbs>().inner().0
    }

    fn toolbox(&self) -> Toolbox {
        crate::commands::toolchain::toolbox_blocking(self.session())
    }

    fn project_changed(&self, project: &ProjectManifest) {
        let event = ProjectChanged {
            project: project.clone(),
        };
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a project event");
        }
    }
}
