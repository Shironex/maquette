fn main() {
    let attributes = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    tauri_build::try_build(attributes).expect("tauri-build");

    embed_app_manifest();
}

/// `tauri_build` embeds its manifest into the app binary only, so cargo test
/// binaries get none. `tauri-plugin-dialog` imports `TaskDialogIndirect`, which
/// only comctl32 v6 exports, so an unmanifested test binary would bind the v5
/// DLL and die at load on Windows before a single test runs. Linking
/// `app.manifest` here covers every target, and tauri's own copy is switched
/// off above so the two cannot collide.
fn embed_app_manifest() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let manifest = std::path::Path::new(&dir).join("app.manifest");

    println!("cargo::rerun-if-changed=app.manifest");
    println!("cargo::rustc-link-arg=/MANIFEST:EMBED");
    println!(
        "cargo::rustc-link-arg=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
