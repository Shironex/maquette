//! The two tools an agent starts from: get_started reads where things stand
//! and says what to do next, create_project makes a project to work in.

use maquette_core::game::inspect_game_repo;
use maquette_core::providers::{ProviderId, secrets};
use maquette_core::settings::game::{DEFAULT_MODELS_DIR, GameFormat, GameTemplateId, NewGame};
use maquette_core::{
    ProjectManifest, ProjectTarget, ProjectTemplate, Stage, ToolId, ToolReport, ToolStatus, inbox,
    project,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::hints::{person_step, step};
use super::image_api::status_value;
use super::{McpTools, Out, ToolError, respond, to_value};
use crate::commands::image_api;
use crate::commands::project::{checked_repo, create_in_workspace, summaries};
use crate::mcp::host::Host;

mod next;
#[cfg(test)]
mod tests;

use next::{Facts, ProjectFacts, next_steps};

/// The kind of project to make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum KindArg {
    /// Ships a 3D scene or models to a site repo.
    Website,
    /// Ships models into a game repo.
    Game,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectArgs {
    /// The project's name. A name another project has, or that makes the same id, is refused.
    pub name: String,
    /// website or game.
    pub kind: KindArg,
    /// A website template id such as floating-diorama (the default), room or character-showcase,
    /// or a game template id as get_game lists them, such as prop-set (none by default).
    pub template: Option<String>,
    /// The site repo, or the game repo for a game: a git repo or worktree on this computer,
    /// absolute or starting with ~, outside the workspace. It can be set later
    /// (set_ship_settings siteRepo, set_game_settings repo).
    pub site_repo: Option<String>,
}

const WEBSITE_DEFAULT: ProjectTemplate = ProjectTemplate::FloatingDiorama;

fn word<T: serde::de::DeserializeOwned>(value: &str) -> Option<T> {
    serde_json::from_value(json!(value.trim().to_lowercase())).ok()
}

/// The template of a new website: any the app's New project knows, but the
/// two that do not start from a template.
fn website_template(template: Option<&str>) -> Result<ProjectTemplate, String> {
    let Some(template) = template else {
        return Ok(WEBSITE_DEFAULT);
    };
    match word::<ProjectTemplate>(template) {
        Some(ProjectTemplate::Imported) => Err(
            "The imported template starts from a folder of Blender files: a person picks it in New project in the app. Use a template such as floating-diorama here.".to_string(),
        ),
        Some(ProjectTemplate::GameImport) => {
            Err("game-import is for games. Pass kind game.".to_string())
        }
        Some(found) => Ok(found),
        None => Err(format!(
            "{template:?} is not a website template. Use one such as floating-diorama, room or character-showcase."
        )),
    }
}

fn game_template(template: Option<&str>) -> Result<Option<GameTemplateId>, String> {
    template
        .map(|t| {
            word(t).ok_or_else(|| {
                let known: Vec<Value> = maquette_core::game::game_templates()
                    .iter()
                    .map(|g| to_value(g.id))
                    .collect();
                format!("{t:?} is not a game template. Use one of {known:?}.")
            })
        })
        .transpose()
}

/// A project whose name or id the new name would take.
fn taken<'a>(name: &str, existing: impl IntoIterator<Item = (&'a str, &'a str)>) -> Option<String> {
    let wanted = name.trim();
    let id = project::slugify(wanted);
    existing
        .into_iter()
        .find(|(other_id, other)| *other_id == id || other.trim().eq_ignore_ascii_case(wanted))
        .map(|(other_id, other)| format!("{other:?} is already a project (id {other_id})."))
}

fn new_project(
    host: &dyn Host,
    args: &CreateProjectArgs,
) -> Result<maquette_core::NewProject, ToolError> {
    let repo = args
        .site_repo
        .as_deref()
        .map(|r| checked_repo(host.session(), r))
        .transpose()?;
    let site_repo = repo.as_ref().map(|r| r.display().to_string());
    let (template, target, game) = match args.kind {
        KindArg::Website => (
            website_template(args.template.as_deref())?,
            ProjectTarget::Website,
            None,
        ),
        KindArg::Game => {
            let found = repo.as_deref().map(inspect_game_repo).unwrap_or_default();
            (
                ProjectTemplate::GameImport,
                ProjectTarget::Game,
                Some(NewGame {
                    format: found.format.unwrap_or(GameFormat::Glb),
                    models_dir: found
                        .models_dir
                        .unwrap_or_else(|| DEFAULT_MODELS_DIR.to_string()),
                    template: game_template(args.template.as_deref())?,
                }),
            )
        }
    };
    Ok(maquette_core::NewProject {
        name: args.name.clone(),
        template,
        target,
        site_repo,
        source_folder: None,
        game,
    })
}

/// What to do in a project just made.
fn after_create(manifest: &ProjectManifest) -> Vec<Value> {
    let mut steps = vec![
        step(
            "start_concepts",
            "Make concept images (Codex usage, or a paid backend a person approves).",
        ),
        step(
            "add_built_asset",
            "Or make a model with a Blender script: no credits, no approval.",
        ),
    ];
    if manifest.site_repo.is_none() {
        steps.push(match manifest.target {
            ProjectTarget::Website => step(
                "set_ship_settings",
                "Set siteRepo before shipping. Until then run with buildOnly true.",
            ),
            ProjectTarget::Game => step(
                "set_game_settings",
                "Set repo before shipping. Until then run with buildOnly true.",
            ),
        });
    }
    steps
}

fn tool_value(report: &ToolReport) -> Value {
    json!({
        "id": to_value(report.id),
        "label": report.label,
        "required": report.required,
        "status": to_value(report.status),
        "version": report.version,
        "hint": report.hint,
        "warning": report.warning,
    })
}

fn tool_works(reports: &[ToolReport], id: ToolId) -> bool {
    reports
        .iter()
        .any(|r| r.id == id && r.status == ToolStatus::Found && r.warning.is_none())
}

/// The earliest stage any asset is at, "empty" without assets.
fn project_stage(manifest: &ProjectManifest) -> Value {
    Stage::ALL
        .iter()
        .find(|s| manifest.assets.iter().any(|a| a.stage == **s))
        .map_or(json!("empty"), to_value)
}

fn stage_counts(manifest: &ProjectManifest) -> Value {
    let mut counts = serde_json::Map::new();
    for stage in Stage::ALL {
        let n = manifest.assets.iter().filter(|a| a.stage == stage).count();
        if n > 0
            && let Some(word) = to_value(stage).as_str()
        {
            counts.insert(word.to_string(), json!(n));
        }
    }
    Value::Object(counts)
}

/// What only a person can get past, as far as this computer says now.
fn gates(reports: &[ToolReport], keys: &Keys, ask_before_shipping: bool) -> Vec<Value> {
    let gate = |what: &str, place: &str| json!({ "gate": what, "where": place });
    let mut out = vec![
        gate(
            "Approve credits for a send or split on Tripo or Meshy",
            "The approval prompt in the Maquette app",
        ),
        gate(
            "Approve an openai-images round an agent asks for",
            "The approval prompt in the Maquette app",
        ),
        gate(
            if ask_before_shipping {
                "Approve Ship for runs an agent starts (Ask before shipping is on), and always in a game"
            } else {
                "Approve Ship in a game project (Ask before shipping is off for websites)"
            },
            "The approval prompt in the Maquette app",
        ),
        gate(
            "Approve an avatar's new weights, rest pose and spring chains",
            "The Avatar view in the Maquette app",
        ),
        gate(
            "Change the monthly OpenAI image cap",
            "Settings > Image generation",
        ),
    ];
    if !keys.tripo && !keys.meshy {
        out.push(gate(
            "Save a Tripo or Meshy API key, or use provider by-hand for free",
            "Settings > 3D providers",
        ));
    }
    if !tool_works(reports, ToolId::Blender) {
        out.push(gate(
            "Install or locate Blender",
            "blender.org, then Settings > Toolchain",
        ));
    }
    if !tool_works(reports, ToolId::Codex) {
        out.push(gate(
            "Install Codex CLI and run codex login, or bring your own images as refs",
            "A terminal, then Settings > Toolchain",
        ));
    }
    out
}

struct Keys {
    tripo: bool,
    meshy: bool,
    openai: bool,
}

fn saved(host: &dyn Host, service: ProviderId) -> bool {
    secrets::status(host.studio().secrets().as_ref(), service).is_ok_and(|s| s.saved)
}

fn started(host: &dyn Host) -> Result<Value, ToolError> {
    let reports = host.tool_reports();
    let tools: Vec<Value> = reports.iter().map(tool_value).collect();
    let Ok((root, config)) = host.session().workspace() else {
        let next = next_steps(&Facts::no_workspace());
        return Ok(json!({
            "workspace": Value::Null,
            "toolchain": tools,
            "projects": [],
            "next": next,
        }));
    };
    let image = image_api::status(
        host.session(),
        host.studio().secrets().as_ref(),
        host.concepts(),
    )?;
    let keys = Keys {
        tripo: saved(host, ProviderId::Tripo),
        meshy: saved(host, ProviderId::Meshy),
        openai: image.key.saved,
    };
    let inbox_count = inbox::list(&root, &config).map_or(0, |items| items.len());
    let rows = summaries(
        host.session(),
        host.runner(),
        host.concepts(),
        host.studio(),
    )?;
    let ask = config.agents.rules.ask_before_shipping;
    let mut facts = Vec::new();
    let mut projects = Vec::new();
    for row in &rows {
        let manifest = project::load(&root, &config.layout, &row.id)?;
        projects.push(json!({
            "id": row.id,
            "name": row.name,
            "target": to_value(row.target),
            "template": to_value(row.template),
            "siteRepo": row.site_repo,
            "siteRepoMissing": row.site_repo_state.is_missing(),
            "assets": row.asset_count,
            "stage": project_stage(&manifest),
            "stages": stage_counts(&manifest),
            "activity": to_value(&row.activity),
        }));
        facts.push(ProjectFacts::of(&manifest));
    }
    let current = config
        .current_project
        .as_deref()
        .and_then(|id| facts.iter().position(|p| p.id == id))
        .unwrap_or(0);
    let next = next_steps(&Facts {
        workspace_open: true,
        blender: tool_works(&reports, ToolId::Blender),
        codex: tool_works(&reports, ToolId::Codex),
        any_key: keys.tripo || keys.meshy,
        inbox: inbox_count,
        project: facts.into_iter().nth(current),
    });
    Ok(json!({
        "workspace": host.session().display(&root),
        "toolchain": tools,
        "keys": { "tripo": keys.tripo, "meshy": keys.meshy, "openai": keys.openai },
        "imageBackend": status_value(&image),
        "askBeforeShipping": ask,
        "inboxModels": inbox_count,
        "projects": projects,
        "personGates": gates(&reports, &keys, ask),
        "next": next,
    }))
}

#[tool_router(router = start_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Start here: read where things stand on this computer. The workspace, each tool of the toolchain (found, missing or broken, with its version and how to fix it), whether a Tripo, Meshy and OpenAI key is saved (true or false, never a value), the image backend, Ask before shipping, the inbox count, every project with its target and stage (empty, or the earliest stage an asset is at), the gates only a person can pass, and next: what to do now, each step a tool or a thing a person has to do. Works with no workspace open. Reads only; spends nothing.",
        annotations(read_only_hint = true)
    )]
    async fn get_started(&self) -> Out {
        respond(self.blocking(|host, _| started(host)).await)
    }

    #[tool(
        description = "Make a project, as New project in the app does: name, kind (website or game), optional template and optional site repo (the game repo for a game). A name another project has is refused, nothing is overwritten, and there is no tool that deletes a project. The app's open project stays as it is, unless none was open. Spends nothing and needs no approval. Returns the project and next."
    )]
    async fn create_project(&self, Parameters(args): Parameters<CreateProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = host.session().workspace()?;
                let rows = project::summaries(&root, &config.layout)?;
                let existing = rows.iter().map(|r| (r.id.as_str(), r.name.as_str()));
                if let Some(message) = taken(&args.name, existing) {
                    return Err(ToolError {
                        message,
                        code: "taken".to_string(),
                    });
                }
                let new = new_project(host, &args)?;
                let manifest = create_in_workspace(host.session(), new)?;
                host.project_created(&manifest);
                Ok(json!({
                    "project": {
                        "id": manifest.id,
                        "name": manifest.name,
                        "target": to_value(manifest.target),
                        "template": to_value(manifest.template),
                        "siteRepo": manifest.site_repo,
                    },
                    "next": after_create(&manifest),
                }))
            })
            .await;
        respond(found)
    }
}

/// Steps for a person, re-exported for the other answers.
pub(super) fn open_workspace_step() -> Value {
    person_step("Open or create a workspace in the Maquette app. No tool can.")
}
