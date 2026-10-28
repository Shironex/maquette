//! Generates concept images with the real Codex CLI, the same way the Lab
//! does, and prints each job's state, time and log. Used for the smoke test
//! of the command line and the file pickup. Every image spends Codex usage.
//!
//! ```text
//! cargo run -p maquette-core --example generate_concept -- <project dir> \
//!     --direction "Stone lantern" --prompt "..." [--ref refs/a.png]... [--count 1]
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use maquette_core::concepts::jobs::Start;
use maquette_core::concepts::{self, Aspect, ConceptEvent, Concepts, NewConcepts};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::{ProbeEnv, SystemRunner, toolchain};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut project_dir: Option<PathBuf> = None;
    let mut direction = "Smoke test".to_string();
    let mut prompt = String::new();
    let mut refs = Vec::new();
    let mut count = 1;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--direction" => direction = args.next().unwrap_or_default(),
            "--prompt" => prompt = args.next().unwrap_or_default(),
            "--ref" => refs.extend(args.next()),
            "--count" => count = args.next().and_then(|n| n.parse().ok()).unwrap_or(1),
            other => project_dir = Some(PathBuf::from(other)),
        }
    }
    let project_dir = project_dir.ok_or("usage: generate_concept <project dir> --prompt ...")?;
    let env = ProbeEnv::from_system();
    let reports = toolchain::detect_all(&env, &BTreeMap::new(), &SystemRunner);
    let tools = Toolbox::from_reports(&reports, &env);
    let codex_home = env
        .codex_home
        .clone()
        .or_else(|| env.home.as_ref().map(|h| h.join(".codex")));
    let request = concepts::clean_request(
        &project_dir,
        &NewConcepts {
            project: "smoke".into(),
            direction,
            prompt,
            refs,
            aspect: Aspect::Wide,
            count,
            use_style: true,
        },
    )?;
    let started = Instant::now();
    let service = Concepts::new(codex_home, 4, move |event| match event {
        ConceptEvent::Job { job } if job.state.is_finished() => println!(
            "[{:>6.1}s] {} {:?}: {}",
            started.elapsed().as_secs_f64(),
            job.file,
            job.state,
            job.message.as_deref().unwrap_or("")
        ),
        ConceptEvent::Job { job } => println!(
            "[{:>6.1}s] {} {:?} {}",
            started.elapsed().as_secs_f64(),
            job.file,
            job.state,
            job.log_tail.last().map(String::as_str).unwrap_or("")
        ),
        ConceptEvent::Direction { direction, .. } => println!(
            "[{:>6.1}s] direction {} now has {} images",
            started.elapsed().as_secs_f64(),
            direction.id,
            direction.images.len()
        ),
        ConceptEvent::Style { style, .. } => println!("style board at revision {}", style.revision),
    });
    let cache = project_dir.join(".concept-runs");
    let (direction, jobs) = service.start(Start {
        project_dir: &project_dir,
        cache_dir: &cache,
        tools: &tools,
        request,
        timeout: Duration::from_secs(600),
        api: None,
    })?;
    println!("queued {} jobs for {}", jobs.len(), direction.id);
    service.wait();
    for job in service.jobs(None) {
        println!(
            "{} {:?} {}x{} in {:.1}s\n  log tail:\n    {}",
            job.file,
            job.state,
            job.width.unwrap_or(0),
            job.height.unwrap_or(0),
            (job.finished_at_ms.unwrap_or(0.0) - job.started_at_ms.unwrap_or(0.0)) / 1000.0,
            job.log_tail.join("\n    ")
        );
    }
    Ok(())
}
