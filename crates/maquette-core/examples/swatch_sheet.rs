//! Renders the page light's swatch sheet with the real Blender (headless):
//! the reference picture of the preview rig, in the docs as
//! `apps/site/public/docs-media/page-light-swatches.png`. Every argument
//! after the picture goes to `swatch_sheet.py` as it is.
//!
//! ```text
//! cargo run -p maquette-core --example swatch_sheet -- <out.png> \
//!     [--swatches "lavender=#c9b8e8,cream=#f3e9d2"] [--columns 5] [--width 1600] \
//!     [--report sheet.json] [--plain] [--glb tiles.glb]
//! ```

use std::collections::BTreeMap;
use std::process::Command;

use maquette_core::pipeline::tools::Toolbox;
use maquette_core::thumbs::{RIG_FILE, RIG_SCRIPT, SWATCH_FILE, SWATCH_SCRIPT};
use maquette_core::{ProbeEnv, SystemRunner, ToolId, toolchain};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(out) = args.next() else {
        return Err(
            "usage: swatch_sheet <out.png> [--swatches ...] [--report file] [--plain] [--glb file]"
                .into(),
        );
    };
    let out = std::path::absolute(out)?;
    // The scripts sit together, as every render has them.
    let work = std::env::temp_dir().join(format!("maquette-swatch-sheet-{}", std::process::id()));
    std::fs::create_dir_all(&work)?;
    for (name, text) in [(SWATCH_FILE, SWATCH_SCRIPT), (RIG_FILE, RIG_SCRIPT)] {
        std::fs::write(work.join(name), text)?;
    }
    let env = ProbeEnv::from_system();
    let reports = toolchain::detect_all(&env, &BTreeMap::new(), &SystemRunner);
    let tools = Toolbox::from_reports(&reports, &env);
    let blender = tools
        .tools
        .get(&ToolId::Blender)
        .ok_or("Blender was not found: install it, or put it on the PATH")?;
    let status = Command::new(&blender.program)
        .args(&blender.prefix)
        .args([
            "-b",
            "--factory-startup",
            "--python-exit-code",
            "1",
            "--python",
        ])
        .arg(work.join(SWATCH_FILE))
        .arg("--")
        .arg("--out")
        .arg(&out)
        .args(args)
        .status()?;
    if !status.success() {
        return Err(format!("Blender stopped with {status}").into());
    }
    println!("wrote {}", out.display());
    Ok(())
}
