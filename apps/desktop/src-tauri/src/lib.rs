//! The Tauri shell: a thin composition root. Domain logic lives in
//! `maquette-core`; this crate owns the window, the command surface and the
//! TypeScript bindings it exports.

mod asset_scope;
pub mod bindings;
pub mod commands;
pub mod events;
pub mod keychain;
pub mod mcp;
pub mod opener;
pub mod quit;
pub mod session;
pub mod updater;
pub mod workspace_lock;

use std::sync::Arc;

use tauri::Manager as _;

pub fn run() {
    let specta = bindings::builder();
    let context = tauri::generate_context!();
    let feed = updater::FeedConfig::from_plugin_config(context.config().plugins.0.get("updater"));
    let mode = updater::mode_for(&feed);
    tracing::info!(?mode, "updater");

    let mut builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    // The updater plugin is only registered when it is on: with no key it
    // could only fail later, and an app start must never fail over updates.
    if mode.is_on() {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }
    builder
        .on_window_event(quit::on_window_event)
        .on_menu_event(quit::on_menu_event)
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            specta.mount_events(app);
            app.manage(quit::QuitGuard::default());
            quit::route_quit_through_close(app.handle())?;
            let config_dir = app.path().app_config_dir()?;
            let session = session::Session::load(config_dir.clone());
            session.attach(app.handle().clone());
            app.manage(session);
            let emitter = app.handle().clone();
            let hub = Arc::new(mcp::hub::Hub::new(move || mcp::emit_status(&emitter)));
            let sink = Arc::new(events::TauriSink(app.handle().clone(), Arc::clone(&hub)));
            app.manage(commands::pipeline::Pipelines(
                maquette_core::pipeline::runner::PipelineRunner::new(sink),
            ));
            app.manage(commands::thumbs::Thumbs::new(app.handle().clone()));
            app.manage(commands::site::Preview::new(app.handle().clone()));
            app.manage(commands::codex_usage::CodexUsageWatch::default());
            app.manage(commands::lab::Lab::new(app.handle().clone(), &config_dir));
            app.manage(commands::packs::PackJobs::new(app.handle().clone()));
            app.manage(commands::rig::RigJobs::new(
                app.handle().clone(),
                Arc::clone(&hub),
            ));
            app.manage(commands::renders::RenderJobs::new(app.handle().clone()));
            let studio = commands::providers::Studio::new(app.handle().clone());
            commands::image_api::connect(&app.state::<commands::lab::Lab>().0, studio.0.secrets());
            let watch =
                commands::downloads::Watch::new(app.handle().clone(), studio.0.clone(), config_dir);
            app.manage(studio);
            app.manage(watch);
            commands::downloads::apply(app.handle());
            let host = Arc::new(mcp::host::AppHost(app.handle().clone()));
            app.manage(mcp::Agents::new(host, hub));
            workspace_lock::start_heartbeat(app.handle().clone());
            let updates = updater::Updates::new(mode, feed.endpoint.clone());
            updater::schedule(app.handle(), &updates);
            app.manage(updates);
            Ok(())
        })
        .build(context)
        .expect("the Tauri runtime failed to start")
        .run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                shutdown(handle);
            }
        });
}

/// Stops what must not outlive the app: the MCP server, the preview server,
/// the Downloads watcher and provider polling, and lets go of the workspace.
/// Runs on exit and before an update restarts the app.
pub(crate) fn shutdown(handle: &tauri::AppHandle) {
    if let Some(agents) = handle.try_state::<mcp::Agents>() {
        agents.stop();
    }
    if let Some(preview) = handle.try_state::<commands::site::Preview>() {
        preview.0.stop();
    }
    if let Some(watch) = handle.try_state::<commands::downloads::Watch>() {
        watch.0.stop();
    }
    // Running sends stop polling; they resume on the next start.
    if let Some(studio) = handle.try_state::<commands::providers::Studio>() {
        studio.0.close();
    }
    workspace_lock::release_open(handle);
}
