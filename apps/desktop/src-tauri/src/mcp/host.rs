//! The services the MCP tools work with. The app hands them over from its
//! managed state; the integration test builds its own, without a window.

use std::sync::Arc;

use maquette_core::ProjectManifest;
use maquette_core::concepts::backend::ImageSettings;
use maquette_core::concepts::codex_usage::CodexUsage;
use maquette_core::concepts::{ConceptEvent, Concepts, Direction, StyleBoard};
use maquette_core::packs::Packs;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::{Arrivals, ProviderJobs};
use maquette_core::renders::Renders;
use maquette_core::rig::RigDoctors;
use maquette_core::site::preview::SitePreview;
use maquette_core::thumbs::Thumbnails;
use tauri::{AppHandle, Manager as _};
use tauri_specta::Event as _;

use crate::commands::downloads::Watch;
use crate::commands::lab::Lab;
use crate::commands::packs::PackJobs;
use crate::commands::pipeline::Pipelines;
use crate::commands::providers::Studio;
use crate::commands::renders::RenderJobs;
use crate::commands::rig::RigJobs;
use crate::commands::thumbs::Thumbs;
use crate::events::{
    AvatarChanged, BuiltChanged, ConceptUpdate, LayoutChanged, LibraryChanged, MaterialsChanged,
    PipelineChanged, PrepareChanged, ProjectChanged,
};
use crate::session::Session;

pub trait Host: Send + Sync + 'static {
    fn session(&self) -> &Session;
    fn runner(&self) -> &PipelineRunner;
    fn concepts(&self) -> &Concepts;
    fn thumbs(&self) -> &Thumbnails;
    /// Send to 3D: the provider jobs.
    fn studio(&self) -> &ProviderJobs;
    /// The tools a job runs with. May probe them, so it blocks.
    fn toolbox(&self) -> Toolbox;
    /// The site's dev server the Page view and Preview on site share, when
    /// the host runs one.
    fn site_preview(&self) -> Option<Arc<SitePreview>> {
        None
    }
    /// The By hand Downloads watcher, when the app runs one.
    fn arrivals(&self) -> Option<&Arrivals> {
        None
    }

    /// The export packs service, where the app runs one.
    fn packs(&self) -> Option<&Packs> {
        None
    }

    /// The rig doctor service, where the app runs one.
    fn rig(&self) -> Option<&RigDoctors> {
        None
    }

    /// The render styles service, where the app runs one.
    fn renders(&self) -> Option<&Renders> {
        None
    }

    /// Tells the windows that a tool changed a project, so the Board updates.
    fn project_changed(&self, _project: &ProjectManifest) {}
    /// Tells the windows that a tool changed a project's pipeline file, so
    /// the Pipeline view reloads it.
    fn pipeline_changed(&self, _project: &str) {}
    /// Tells the windows that a tool changed a project's scene layout, so
    /// the Layout editor reloads it.
    fn layout_changed(&self, _project: &str) {}
    /// Tells the windows that a tool changed a project's material tweaks,
    /// so the Inspector reloads them.
    fn materials_changed(&self, _project: &str) {}
    /// Tells the windows that a tool changed a project's real sizes or
    /// fix-ups, so the Board, the Inbox and the Inspector reload them.
    fn prepare_changed(&self, _project: &str) {}
    /// Tells the windows that a tool changed or built an asset's avatar,
    /// so the Avatar view reloads it.
    fn avatar_changed(&self, _project: &str, _asset: &str) {}
    /// Tells the windows that a project's built assets changed, so the
    /// Board shows how they stand.
    fn built_changed(&self, _project: &str) {}
    /// Tells the windows that a tool starred or picked concepts of a
    /// direction, so the Lab shows its marks and refs.
    fn concepts_changed(&self, _project: &str, _direction: &Direction) {}
    /// Tells the windows that a tool changed a project's style board, so
    /// the Lab shows it.
    fn style_changed(&self, _project: &str, _style: &StyleBoard) {}
    /// Tells the windows that a tool changed the library, so the Library
    /// view and the update marks reload.
    fn library_changed(&self) {}
    /// Keeps a Codex usage read and tells the windows, so the status bar
    /// shows what an agent read.
    fn codex_usage_seen(&self, _usage: &CodexUsage) {}

    /// An agent changed the Lab's image backends.
    fn image_settings_changed(&self, _settings: &ImageSettings) {}
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

    fn studio(&self) -> &ProviderJobs {
        &self.0.state::<Studio>().inner().0
    }

    fn toolbox(&self) -> Toolbox {
        crate::commands::toolchain::toolbox_blocking(self.session())
    }

    fn arrivals(&self) -> Option<&Arrivals> {
        self.0.try_state::<Watch>().map(|w| &w.inner().0)
    }

    fn packs(&self) -> Option<&Packs> {
        self.0.try_state::<PackJobs>().map(|p| &p.inner().0)
    }

    fn rig(&self) -> Option<&RigDoctors> {
        self.0.try_state::<RigJobs>().map(|r| &r.inner().0)
    }

    fn renders(&self) -> Option<&Renders> {
        self.0.try_state::<RenderJobs>().map(|r| &r.inner().0)
    }

    fn site_preview(&self) -> Option<Arc<SitePreview>> {
        self.0
            .try_state::<crate::commands::site::Preview>()
            .map(|p| Arc::clone(&p.inner().0))
    }

    fn project_changed(&self, project: &ProjectManifest) {
        let event = ProjectChanged {
            project: project.clone(),
        };
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a project event");
        }
    }

    fn pipeline_changed(&self, project: &str) {
        let event = PipelineChanged {
            project: project.to_string(),
        };
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a pipeline event");
        }
    }

    fn layout_changed(&self, project: &str) {
        let event = LayoutChanged {
            project: project.to_string(),
        };
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a layout event");
        }
    }

    fn materials_changed(&self, project: &str) {
        let event = MaterialsChanged {
            project: project.to_string(),
        };
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a materials event");
        }
    }

    fn avatar_changed(&self, project: &str, asset: &str) {
        let event = AvatarChanged {
            project: project.to_string(),
            asset: asset.to_string(),
        };
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit an avatar event");
        }
    }

    fn prepare_changed(&self, project: &str) {
        let event = PrepareChanged {
            project: project.to_string(),
        };
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a prepare event");
        }
    }

    fn built_changed(&self, project: &str) {
        let event = BuiltChanged {
            project: project.to_string(),
        };
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a built event");
        }
    }

    fn codex_usage_seen(&self, usage: &CodexUsage) {
        crate::commands::codex_usage::publish(&self.0, usage);
    }

    fn image_settings_changed(&self, settings: &ImageSettings) {
        if let Err(err) = crate::events::ImageSettingsChanged(*settings).emit(&self.0) {
            tracing::warn!(error = %err, "could not emit the image settings");
        }
    }

    fn library_changed(&self) {
        if let Err(err) = LibraryChanged.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a library event");
        }
    }

    fn concepts_changed(&self, project: &str, direction: &Direction) {
        let event = ConceptUpdate(ConceptEvent::Direction {
            project: project.to_string(),
            direction: direction.clone(),
        });
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a concept event");
        }
    }

    fn style_changed(&self, project: &str, style: &StyleBoard) {
        let event = ConceptUpdate(ConceptEvent::Style {
            project: project.to_string(),
            style: style.clone(),
        });
        if let Err(err) = event.emit(&self.0) {
            tracing::warn!(error = %err, "could not emit a concept event");
        }
    }
}

/// A build tells an agent's host, which tells the windows.
pub(crate) struct HostNotify<'a>(pub &'a dyn Host);

impl crate::commands::built::Notify for HostNotify<'_> {
    fn project(&self, project: &ProjectManifest) {
        self.0.project_changed(project);
    }

    fn built(&self, project: &str) {
        self.0.built_changed(project);
    }
}
