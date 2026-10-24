//! Renders studio thumbnails through the same thumbnail service the app
//! uses (headless Blender on the runner's thumbnail lane) and prints where
//! each PNG landed.
//!
//! ```text
//! cargo run -p maquette-core --example render_thumbnail -- <thumbs dir> <model>...
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use maquette_core::pipeline::tools::Toolbox;
use maquette_core::thumbs::{Requested, Thumbnails};
use maquette_core::{ProbeEnv, SystemRunner, toolchain};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1).map(PathBuf::from);
    let dir = args
        .next()
        .ok_or("usage: render_thumbnail <thumbs dir> <model>...")?;
    let models: Vec<PathBuf> = args.collect();
    let env = ProbeEnv::from_system();
    let reports = toolchain::detect_all(&env, &BTreeMap::new(), &SystemRunner);
    let tools = Toolbox::from_reports(&reports, &env);
    let started = Instant::now();
    let service = Thumbnails::new(|done| match (&done.png, &done.error) {
        (Some(png), _) => println!("rendered {} -> {}", done.file.display(), png.display()),
        (None, error) => println!(
            "failed {}: {}",
            done.file.display(),
            error.as_deref().unwrap_or("unknown")
        ),
    });
    for model in &models {
        match service.request(&dir, model, None, &tools)? {
            Requested::Ready(png) => println!("cached {} -> {}", model.display(), png.display()),
            Requested::Rendering => println!("queued {}", model.display()),
            Requested::Fallback(why) => println!("fallback {}: {why}", model.display()),
        }
    }
    service.wait();
    println!("done in {:.1}s", started.elapsed().as_secs_f64());
    Ok(())
}
