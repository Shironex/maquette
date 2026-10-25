//! The Tauri shell: a thin composition root. Domain logic lives in
//! `maquette-core`; this crate owns the window, the command surface and the
//! TypeScript bindings it exports.

pub mod bindings;
pub mod commands;
pub mod events;
pub mod keychain;
pub mod mcp;
pub mod opener;
pub mod quit;
pub mod session;

use std::sync::Arc;

use tauri::Manager as _;

pub fn run() {
    let specta = bindings::builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(quit::on_window_event)
        .on_menu_event(quit::on_menu_event)
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            specta.mount_events(app);
            app.manage(quit::QuitGuard::default());
            quit::route_quit_through_close(app.handle())?;
            let config_dir = app.path().app_config_dir()?;
            let session = session::Session::load(config_dir);
            session.attach(app.handle().clone());
            app.manage(session);
            let emitter = app.handle().clone();
            let hub = Arc::new(mcp::hub::Hub::new(move || mcp::emit_status(&emitter)));
            let sink = Arc::new(events::TauriSink(app.handle().clone(), Arc::clone(&hub)));
            app.manage(commands::pipeline::Pipelines(
                maquette_core::pipeline::runner::PipelineRunner::new(sink),
            ));
            app.manage(commands::thumbs::Thumbs::new(app.handle().clone()));
            app.manage(commands::site::Preview::default());
            app.manage(commands::lab::Lab::new(app.handle().clone()));
            app.manage(commands::providers::Studio::new(app.handle().clone()));
            let host = Arc::new(mcp::host::AppHost(app.handle().clone()));
            app.manage(mcp::Agents::new(host, hub));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("the Tauri runtime failed to start")
        .run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(agents) = handle.try_state::<mcp::Agents>() {
                    agents.stop();
                }
                if let Some(preview) = handle.try_state::<commands::site::Preview>() {
                    preview.0.stop();
                }
                // Running sends stop polling; they resume on the next start.
                if let Some(studio) = handle.try_state::<commands::providers::Studio>() {
                    studio.0.close();
                }
            }
        });
}
