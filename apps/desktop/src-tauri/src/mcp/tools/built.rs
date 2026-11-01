//! Tools for built assets: models a Blender script inside the project makes.
//! Adding one copies its script and helpers into the project's `built/`
//! folder; building runs it headless (`blender -b`) and makes the asset's
//! next version, with no credits spent and no approval.

use maquette_core::built::{BuildOutcome, BuiltAsset, NewBuilt};
use maquette_core::inbox::display_name;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::{AssetKind, project};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::built::{self as commands, BuildEnv};
use crate::mcp::host::{Host, HostNotify};

use super::materials::find_asset;
use super::{McpTools, Out, ProjectArgs, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddBuiltArgs {
    /// The project id.
    pub project: String,
    /// The Python script that builds the model: an absolute path, or one relative to the
    /// project folder. It is copied into the project's built/ folder.
    pub script: String,
    /// Files the script imports or reads (shared helpers such as common.py, textures), the
    /// same way. A file already in built/ with the same content is shared.
    #[serde(default)]
    pub helpers: Vec<String>,
    /// The asset name. Defaults to the script's file name ("newtons_cradle.py" is
    /// "Newtons cradle").
    pub name: Option<String>,
    /// "prop" (default), "character" or "scene".
    pub kind: Option<String>,
    /// The object to export with its children, when the script's build() does not return
    /// its root (one script that builds several pieces can feed several assets this way).
    pub root: Option<String>,
    /// Overwrite files in built/ that have other content. Defaults to false, which refuses.
    #[serde(default)]
    pub replace: bool,
    /// Build it right away. Defaults to true.
    pub build: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RebuildArgs {
    /// The project id.
    pub project: String,
    /// The built asset's id as list_built_assets returns it, or its name. Leave out with all.
    pub asset: Option<String>,
    /// Build every built asset that is new or whose files changed.
    #[serde(default)]
    pub all: bool,
    /// Run Blender even when the files match the last good build. A build whose model
    /// matches the newest version byte for byte adds no version.
    #[serde(default)]
    pub force: bool,
}

fn parse_kind(text: Option<&str>) -> Result<AssetKind, String> {
    let Some(text) = text else {
        return Ok(AssetKind::Prop);
    };
    serde_json::from_value(json!(text.trim().to_lowercase()))
        .map_err(|_| format!("kind must be prop, character or scene, not {text:?}."))
}

/// One build as agents read it.
fn outcome(found: &BuildOutcome) -> Value {
    let record = found.record.as_ref();
    let mut out = json!({
        "asset": found.asset.id,
        "name": found.asset.name,
        "result": to_value(found.result),
        "stage": to_value(found.asset.stage),
        "version": record.and_then(|r| r.version),
        "file": found.version.as_ref().map(|v| v.file.clone()).or_else(|| {
            found.asset.latest().map(|v| v.file.clone())
        }),
        "objects": record.map(|r| r.objects.clone()),
        "materials": record.map(|r| r.materials.clone()),
        "triangles": record.map(|r| r.triangles),
        "warnings": record.map(|r| r.warnings.clone()),
        "error": record.and_then(|r| r.error.clone()),
    });
    if !found.log.is_empty() {
        out["log"] = json!(found.log);
    }
    out
}

fn listed(item: &BuiltAsset) -> Value {
    json!({
        "asset": item.asset,
        "name": item.name,
        "state": to_value(item.state),
        "script": item.source.script,
        "helpers": item.source.helpers,
        "root": item.source.root,
        "missing": item.missing,
        "last": item.source.last.as_ref().map(to_value),
    })
}

fn rebuild_one(
    host: &dyn Host,
    project_id: &str,
    asset: &str,
    force: bool,
    tools: &Toolbox,
) -> Result<Value, String> {
    let env = BuildEnv {
        session: host.session(),
        thumbs: host.thumbs(),
        tools,
        notify: &HostNotify(host),
    };
    commands::rebuild(&env, project_id, asset, force)
        .map(|o| outcome(&o))
        .map_err(|e| e.message)
}

/// What run_pipeline does before a full run: rebuilds the new and stale
/// built assets, refusing the run when one fails.
pub(super) fn rebuild_stale(
    host: &dyn Host,
    project_id: &str,
    tools: &Toolbox,
) -> Result<(), String> {
    let env = BuildEnv {
        session: host.session(),
        thumbs: host.thumbs(),
        tools,
        notify: &HostNotify(host),
    };
    commands::before_run(&env, project_id)
        .map(|_| ())
        .map_err(|e| e.message)
}

const NOTE: &str = "A built asset's script and helpers live in the project's built/ folder: edit them there, then rebuild. Each build runs headless in a scratch copy under the workspace cache, so a script that writes next to itself never writes into the project. The model lands as the asset's next version (no Inbox, no credits); the real size, fix-ups, layout and material tweaks apply as for any other model.";

#[tool_router(router = built_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Add a built asset: a model made by a Blender script inside the project. The script (and the helpers it imports, such as a shared common.py) is copied into the project's built/ folder, the asset goes on the Board at Blender and, unless build is false, it builds headless (blender -b) right away into its first version. Conventions: the script is imported as a module named after its file; when it has a build() function that is called, and the object it returns is exported with its children (else the root option, else the whole scene). Object names, parents, origins and materials stay as the script made them. Spends nothing and needs no approval. Returns the asset and the build's outcome (result built, unchanged, up-to-date or failed with the end of Blender's log)."
    )]
    async fn add_built_asset(&self, Parameters(args): Parameters<AddBuiltArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let kind = parse_kind(args.kind.as_deref())?;
                let name = args.name.clone().unwrap_or_else(|| display_name(&args.script));
                let request = NewBuilt {
                    name,
                    kind,
                    script: args.script.clone(),
                    helpers: args.helpers.clone(),
                    root: args.root.clone(),
                    replace: args.replace,
                };
                let added =
                    commands::add(host.session(), &args.project, &request)?;
                host.project_changed(&added.project);
                host.built_changed(&args.project);
                let build = if args.build.unwrap_or(true) {
                    let tools = host.toolbox();
                    Some(rebuild_one(host, &args.project, &added.asset.id, false, &tools)?)
                } else {
                    None
                };
                let all = commands::list(host.session(), &args.project)?;
                let source = all.iter().find(|b| b.asset == added.asset.id).map(listed);
                Ok(json!({
                    "asset": { "id": added.asset.id, "name": added.asset.name, "kind": to_value(added.asset.kind) },
                    "source": source,
                    "build": build,
                    "note": NOTE,
                }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Build a built asset again, headless (blender -b), or every built asset that is new or whose script, helpers or root changed (all: true). Files that match the last good build answer up-to-date without running Blender unless force is set. A good build becomes the asset's next version (or adds none when the model is byte for byte the newest one); a failing script keeps the old versions and answers failed with its error and the end of Blender's log. Spends nothing and needs no approval."
    )]
    async fn rebuild_built_asset(&self, Parameters(args): Parameters<RebuildArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let tools = host.toolbox();
                let targets: Vec<String> = match (&args.asset, args.all) {
                    (Some(wanted), _) => {
                        let (root, config) = workspace(host)?;
                        let manifest = project::load(&root, &config.layout, &args.project)?;
                        vec![find_asset(&manifest, wanted)?.id.clone()]
                    }
                    (None, true) => commands::list(host.session(), &args.project)?
                        .into_iter()
                        .filter(|b| {
                            args.force
                                || matches!(
                                    b.state,
                                    maquette_core::built::BuiltState::New
                                        | maquette_core::built::BuiltState::Stale
                                )
                        })
                        .map(|b| b.asset)
                        .collect(),
                    (None, false) => {
                        return Err("Name the asset to rebuild, or set all.".to_string().into());
                    }
                };
                let builds = targets
                    .iter()
                    .map(|id| rebuild_one(host, &args.project, id, args.force, &tools))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(json!({ "project": args.project, "builds": builds }))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "List a project's built assets: each one's script and helpers in built/, its root option, its state (new, current, stale when a file changed since the last build, failed, missing when a file is gone, building) and its last build (version, exported object and material names, triangles, warnings from the Inbox checks, error).",
        annotations(read_only_hint = true)
    )]
    async fn list_built_assets(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let items = commands::list(host.session(), &args.project)?;
                Ok(json!({
                    "project": args.project,
                    "assets": items.iter().map(listed).collect::<Vec<_>>(),
                    "note": NOTE,
                }))
            })
            .await;
        respond(found)
    }
}
