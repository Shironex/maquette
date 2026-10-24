//! The Tauri shell: a thin composition root. Domain logic lives in
//! `maquette-core`; this crate owns the window, the command surface and the
//! TypeScript bindings it exports.

pub mod bindings;
pub mod commands;
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
            app.manage(session::Session::load(config_dir));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("the Tauri runtime failed to start");
}
