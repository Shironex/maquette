//! Helpers for tests that run the starter scripts in a real, headless
//! Blender. They are skipped where no Blender is installed: set
//! `MAQUETTE_TEST_BLENDER` to its binary, or keep it in the usual place.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The Blender binary to test with, or `None` (the test then skips).
pub(crate) fn blender() -> Option<PathBuf> {
    let found = std::env::var_os("MAQUETTE_TEST_BLENDER")
        .map(PathBuf::from)
        .or_else(|| {
            [
                "/Applications/Blender.app/Contents/MacOS/Blender",
                "/usr/bin/blender",
                "C:\\Program Files\\Blender Foundation\\Blender\\blender.exe",
            ]
            .iter()
            .map(PathBuf::from)
            .find(|p| p.is_file())
        });
    if found.is_none() {
        eprintln!("no Blender found, skipped (set MAQUETTE_TEST_BLENDER to run it)");
    }
    found
}

/// Runs Blender headless with `args` after `-b --factory-startup` and
/// returns what it printed, failing the test when it exits with an error.
pub(crate) fn run(blender: &Path, args: &[&str]) -> String {
    let out = Command::new(blender)
        .args(["-b", "--factory-startup", "--python-exit-code", "1"])
        .args(args)
        .output()
        .expect("start blender");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "blender failed:\n{text}");
    text
}

/// The JSON chunk of a GLB file.
pub(crate) fn glb_json(path: &Path) -> serde_json::Value {
    let bytes = std::fs::read(path).expect("read glb");
    let len = u32::from_le_bytes(bytes[12..16].try_into().expect("chunk length")) as usize;
    serde_json::from_slice(&bytes[20..20 + len]).expect("glb json")
}

/// A Toolbox whose Blender is `blender`, for code that runs a pipeline step.
pub(crate) fn toolbox(blender: &Path) -> crate::pipeline::tools::Toolbox {
    let mut tools = crate::pipeline::tools::Toolbox {
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        os: Some(crate::toolchain::HostOs::current()),
        ..Default::default()
    };
    tools.tools.insert(
        crate::toolchain::ToolId::Blender,
        crate::pipeline::tools::ResolvedTool {
            program: blender.to_path_buf(),
            prefix: Vec::new(),
            version: "test".into(),
        },
    );
    tools
}

/// One of the small Frostveil props of `parts/fixtures/parts_models.py`
/// (`lantern`, `lantern-tripo`, `gate-door`, ...), written into `dir`.
pub(crate) fn parts_model(blender: &Path, dir: &Path, which: &str) -> PathBuf {
    let script = dir.join("parts_models.py");
    if !script.is_file() {
        std::fs::write(&script, include_str!("parts/fixtures/parts_models.py")).expect("fixture");
    }
    let model = dir.join(format!("{which}.glb"));
    run(
        blender,
        &[
            "--python",
            &script.display().to_string(),
            "--",
            which,
            &model.display().to_string(),
        ],
    );
    model
}
