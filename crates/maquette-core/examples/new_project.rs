//! Creates a workspace (when the folder has none) and a project in it the
//! way the app does: `project::create`, then `pipeline::ensure`, which
//! writes the template's pipeline and starter scripts. Prints the project
//! folder, ready for `run_pipeline`.
//!
//! ```text
//! cargo run -p maquette-core --example new_project -- <workspace dir> <name> [--site <repo>] [--template floating-diorama|room|character-showcase]
//! ```

use std::path::PathBuf;

use maquette_core::{NewProject, ProjectTemplate, WorkspaceConfig, pipeline, project, workspace};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut positional = Vec::new();
    let (mut site, mut template) = (None, ProjectTemplate::FloatingDiorama);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--site" => site = args.next(),
            "--template" => {
                let name = args.next().ok_or("--template needs a value")?;
                template = serde_json::from_value(serde_json::Value::String(name))?;
            }
            _ => positional.push(arg),
        }
    }
    let [root, name] = <[String; 2]>::try_from(positional).map_err(
        |_| "usage: new_project <workspace dir> <name> [--site <repo>] [--template <t>]",
    )?;
    let root = PathBuf::from(root);
    let config = match workspace::open(&root) {
        Ok(config) => config,
        Err(_) => workspace::create(&root, &WorkspaceConfig::default())?,
    };
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name,
            template,
            site_repo: site,
            source_folder: None,
        },
    )?;
    let dir = project::dir(&root, &config.layout, &manifest.id)?;
    let def = pipeline::ensure(&dir, &manifest)?;
    println!("{}", dir.display());
    for step in &def.steps {
        println!("  {:<8} {}", step.id, step.script.as_deref().unwrap_or("-"));
    }
    Ok(())
}
