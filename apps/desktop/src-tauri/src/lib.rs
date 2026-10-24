//! The Tauri shell: a thin composition root. Domain logic lives in
//! `maquette-core`; this crate owns the window, the command surface and the
//! TypeScript bindings it exports.

pub mod bindings;
pub mod commands;
pub mod events;
pub mod session;

use tauri::Manager as _;

pub fn run() {
    let specta = bindings::builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            specta.mount_events(app);
            let config_dir = app.path().app_config_dir()?;
            let session = session::Session::load(config_dir);
            session.attach(app.handle().clone());
            app.manage(session);
            let sink = std::sync::Arc::new(events::TauriSink(app.handle().clone()));
            app.manage(commands::pipeline::Pipelines(
                maquette_core::pipeline::runner::PipelineRunner::new(sink),
            ));
            app.manage(commands::thumbs::Thumbs::new(app.handle().clone()));
            app.manage(commands::lab::Lab::new(app.handle().clone()));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("the Tauri runtime failed to start");
}
