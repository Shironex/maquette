//! Runs a project's `pipeline.json` through the real runner from a
//! terminal, the same way the app does, and prints each step's state and
//! time. Used for the end to end check.
//!
//! ```text
//! cargo run -p maquette-core --example run_pipeline -- <project dir> [--force] [--from <step>]
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use maquette_core::pipeline::history::LogStream;
use maquette_core::pipeline::runner::{PipelineEvent, PipelineRunner, RunSetup, RunSink};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::pipeline::{self, CompressDefaults};
use maquette_core::{MeshoptLevel, ProbeEnv, SystemRunner, TextureCodec, toolchain};

struct Print;

impl RunSink for Print {
    fn event(&self, event: PipelineEvent) {
        if let PipelineEvent::Log { entry } = event {
            let tag = match entry.stream {
                LogStream::System => "maquette",
                LogStream::Stdout => "out",
                LogStream::Stderr => "err",
            };
            println!(
                "  [{}] {tag}: {}",
                entry.step.as_deref().unwrap_or("run"),
                entry.text
            );
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut project_dir: Option<PathBuf> = None;
    let (mut force, mut from) = (false, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--force" => force = true,
            "--from" => from = args.next(),
            other => project_dir = Some(PathBuf::from(other)),
        }
    }
    let project_dir =
        project_dir.ok_or("usage: run_pipeline <project dir> [--force] [--from <step>]")?;
    let def = pipeline::load(&project_dir)?.ok_or("no pipeline.json in that folder")?;

    let env = ProbeEnv::from_system();
    let reports = toolchain::detect_all(&env, &BTreeMap::new(), &SystemRunner);
    for r in &reports {
        println!(
            "tool {:?}: {:?} {}",
            r.id,
            r.status,
            r.path.as_deref().unwrap_or("-")
        );
    }
    let tools = Toolbox::from_reports(&reports, &env);

    let setup = RunSetup {
        project: "e2e".to_string(),
        cache_dir: project_dir.join(".cache"),
        project_dir,
        def,
        tools,
        defaults: CompressDefaults {
            texture_size: 2048,
            codec: TextureCodec::Etc1s,
            meshopt: MeshoptLevel::High,
        },
        vars: BTreeMap::new(),
        from,
        force,
    };
    let started = Instant::now();
    let run = PipelineRunner::new(Arc::new(Print)).run_blocking(setup)?;
    println!(
        "\nrun {} {:?} in {:.2}s",
        run.number,
        run.state,
        started.elapsed().as_secs_f64()
    );
    for step in &run.steps {
        println!(
            "  {:<10} {:<9} {:>8.2}s {}",
            step.id,
            format!("{:?}", step.state),
            step.duration_ms.unwrap_or(0.0) / 1000.0,
            step.message.as_deref().unwrap_or("")
        );
    }
    if let Some(message) = run.message {
        println!("  {message}");
    }
    Ok(())
}
