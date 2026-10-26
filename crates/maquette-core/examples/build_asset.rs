//! Adds a built asset to a project and builds it with the real Blender
//! (headless), the way the app does, then prints the outcome. Creates the
//! workspace and the project when they are not there yet. With
//! `--rebuild`, the asset is only built again.
//!
//! ```text
//! cargo run -p maquette-core --example build_asset -- <workspace dir> <project name> \
//!     <script.py> [--helper common.py]... [--name Laptop] [--kind prop] [--root laptop] [--rebuild]
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;

use maquette_core::built::{self, BuildContext, Builder, NewBuilt};
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::project::{self, NewProject};
use maquette_core::{
    AssetKind, ProbeEnv, ProjectTemplate, SystemRunner, WorkspaceConfig, toolchain, workspace,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut positional = Vec::new();
    let mut helpers = Vec::new();
    let mut name = None;
    let mut kind = AssetKind::Prop;
    let mut root = None;
    let mut rebuild = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--helper" => helpers.extend(args.next()),
            "--name" => name = args.next(),
            "--root" => root = args.next(),
            "--rebuild" => rebuild = true,
            "--kind" => {
                kind = serde_json::from_value(serde_json::json!(args.next().unwrap_or_default()))?;
            }
            other => positional.push(other.to_string()),
        }
    }
    let [ws, project_name, script] = positional.as_slice() else {
        return Err("usage: build_asset <workspace dir> <project name> <script.py> ...".into());
    };
    let ws = PathBuf::from(ws);
    let config = match workspace::open(&ws) {
        Ok(config) => config,
        Err(_) => workspace::create(&ws, &WorkspaceConfig::default())?,
    };
    let existing = project::list(&ws, &config.layout)?
        .into_iter()
        .find(|p| p.name == *project_name);
    let manifest = match existing {
        Some(m) => m,
        None => project::create(
            &ws,
            &config.layout,
            NewProject {
                name: project_name.clone(),
                template: ProjectTemplate::FloatingDiorama,
                target: Default::default(),
                game: None,
                site_repo: None,
                source_folder: None,
            },
        )?,
    };
    let script_path = PathBuf::from(script);
    let name = name.unwrap_or_else(|| {
        script_path
            .file_stem()
            .map(|s| s.to_string_lossy().replace('_', " "))
            .unwrap_or_default()
    });
    let asset = if rebuild {
        maquette_core::assets::find_by_name(&manifest, &name)
            .ok_or("no asset by that name to rebuild")?
            .clone()
    } else {
        let new = NewBuilt {
            name,
            kind,
            script: script.clone(),
            helpers,
            root,
            replace: false,
        };
        built::add(&ws, &config.layout, &manifest.id, &new)?.0
    };
    println!("asset {} ({})", asset.name, asset.id);
    let env = ProbeEnv::from_system();
    let reports = toolchain::detect_all(&env, &BTreeMap::new(), &SystemRunner);
    let tools = Toolbox::from_reports(&reports, &env);
    let ctx = BuildContext {
        workspace_root: &ws,
        config: &config,
        project: &manifest.id,
        tools: &tools,
    };
    let outcome = Builder::default().build(&ctx, &asset.id, false, &|m| {
        if let Some(a) = m.assets.iter().find(|a| a.id == asset.id) {
            println!("  status {:?}", a.status);
        }
    })?;
    println!("{}", serde_json::to_string_pretty(&outcome.record)?);
    println!(
        "result {:?}, version {:?}",
        outcome.result,
        outcome.version.map(|v| v.file)
    );
    for line in outcome.log {
        println!("  | {line}");
    }
    Ok(())
}
