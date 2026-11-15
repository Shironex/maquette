//! The two tools an agent starts from: get_started reads where things stand
//! and says what to do next, create_project makes a project to work in.

use maquette_core::app_state::RepoSetBy;
use maquette_core::game::inspect_game_repo;
use maquette_core::pipeline::history::now_ms;
use maquette_core::settings::game::{DEFAULT_MODELS_DIR, GameFormat, GameTemplateId, NewGame};
use maquette_core::{
    ProjectManifest, ProjectTarget, ProjectTemplate, Stage, ToolId, ToolReport, ToolStatus, inbox,
    project,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::hints::person_step;
use super::hints::table::{Can, Place, start_of};
use super::image_api::status_value;
use super::{McpTools, Out, ToolError, respond, to_value};
use crate::commands::image_api;
use crate::commands::project::{checked_repo, create_in_workspace, summaries};
use crate::mcp::host::Host;
use crate::mcp::hub::Hub;

mod compact;
mod keys;
mod next;
#[cfg(test)]
mod tests;
mod waiting;

use keys::{Keys, read_keys};
use next::{Facts, ProjectFacts, next_steps};

/// Where the project ships, as list_projects says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TargetArg {
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
    /// website or game, the target list_projects answers with. `kind` is
    /// still read as another name for it.
    #[serde(alias = "kind")]
    pub target: TargetArg,
    /// A website template id such as floating-diorama (the default), room or character-showcase,
    /// or a game template id such as prop-set (set_game_settings lists the ids) (none by default).
    pub template: Option<String>,
    /// The site repo, or the game repo for a game: a git repo or worktree on this computer,
    /// absolute or starting with ~, outside the workspace. It can be set later
    /// (set_ship_settings siteRepo, set_game_settings repo).
    pub site_repo: Option<String>,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetStartedArgs {
    /// true adds every tool's version and hint, each project's template, repo,
    /// stages and activity, the image backend's settings and every kind of
    /// gate a person passes. Left out, the answer is short. The approvals
    /// waiting now are in waiting either way.
    pub detail: Option<bool>,
    /// The project id next is about. Left out, the app's open project, else
    /// the first one listed. nextFor names it.
    pub project: Option<String>,
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
            Err("game-import is for games. Pass target game.".to_string())
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
    let id = project::folder_slug(wanted);
    // A name with no Latin letters slugs to the fallback, and takes a suffix as
    // New project in the app does, so only its name can clash.
    let fell_back = id == "project" && !wanted.eq_ignore_ascii_case("project");
    existing
        .into_iter()
        .find(|(other_id, other)| {
            (!fell_back && *other_id == id) || other.trim().eq_ignore_ascii_case(wanted)
        })
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
    let (template, target, game) = match args.target {
        TargetArg::Website => (
            website_template(args.template.as_deref())?,
            ProjectTarget::Website,
            None,
        ),
        TargetArg::Game => {
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
    let place = Place {
        project: &manifest.id,
        target: manifest.target,
    };
    start_of(place, manifest.site_repo.is_some(), Can::BOTH)
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

/// A found Blender runs builds, warning or not: the warning says a saved
/// path is gone and another copy took over, and the report carries it.
fn blender_works(reports: &[ToolReport]) -> bool {
    reports
        .iter()
        .any(|r| r.id == ToolId::Blender && r.status == ToolStatus::Found)
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

/// What only a person can get past, as far as this computer says now: the
/// approvals that always wait for a person, then [`blocking_gates`].
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
                "Approve Ship in a game project, in a run with repo and into a site repo or site folders an agent set (Ask before shipping is off for other website runs)"
            },
            "The approval prompt in the Maquette app",
        ),
        gate(
            "Approve the first dev server start of a repo an agent set, and again when its path or command changes",
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
    out.extend(blocking_gates(reports, keys));
    out
}

/// The gates that stand in the way on this computer right now: a missing
/// key, Blender or Codex.
fn blocking_gates(reports: &[ToolReport], keys: &Keys) -> Vec<Value> {
    let gate = |what: &str, place: &str| json!({ "gate": what, "where": place });
    let mut out = Vec::new();
    if keys.tripo == Some(false) && keys.meshy == Some(false) {
        out.push(gate(
            "Save a Tripo or Meshy API key, or use provider by-hand for free",
            "Settings > 3D providers",
        ));
    }
    if !blender_works(reports) {
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

fn started(
    host: &dyn Host,
    hub: &Hub,
    detail: bool,
    wanted: Option<&str>,
) -> Result<Value, ToolError> {
    let reports = host.tool_reports();
    let tools: Vec<Value> = reports.iter().map(tool_value).collect();
    let jobs = host.studio().list(None);
    let waiting = waiting::waiting(&hub.approvals(), &jobs, now_ms());
    let Ok((root, config)) = host.session().workspace() else {
        let next = next_steps(&Facts::no_workspace());
        let full = json!({
            "workspace": Value::Null,
            "toolchain": tools,
            "projects": [],
            "waiting": waiting,
            "nextFor": Value::Null,
            "next": next,
        });
        return Ok(if detail { full } else { compact::compact(full) });
    };
    let keys = read_keys(host);
    let image = image_api::status_with(
        host.session(),
        image_api::OpenAiKeyStatus {
            saved: keys.openai == Some(true),
            tail: None,
        },
        host.concepts(),
    )?;
    let mut image_value = status_value(&image);
    image_value["keySaved"] = json!(keys.openai);
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
        facts.push(ProjectFacts::of(&manifest, &jobs));
    }
    let current = match wanted {
        Some(id) => Some(facts.iter().position(|p| p.id == id).ok_or_else(|| {
            ToolError::from(format!(
                "There is no project {id:?}. list_projects names every project."
            ))
        })?),
        None => config
            .current_project
            .as_deref()
            .and_then(|id| facts.iter().position(|p| p.id == id))
            .or_else(|| (!facts.is_empty()).then_some(0)),
    };
    let next_for = current.map(|i| facts[i].id.clone());
    let next = next_steps(&Facts {
        workspace_open: true,
        blender: blender_works(&reports),
        codex: tool_works(&reports, ToolId::Codex),
        any_key: keys.any_saved(),
        inbox: inbox_count,
        project: current.and_then(|i| facts.into_iter().nth(i)),
        off: super::groups::off_in(&config),
    });
    let full = json!({
        "workspace": host.session().display(&root),
        "toolchain": tools,
        "keys": { "tripo": keys.tripo, "meshy": keys.meshy, "openai": keys.openai, "unreadable": keys.unreadable() },
        "imageBackend": image_value,
        "askBeforeShipping": ask,
        "inboxModels": inbox_count,
        "projects": projects,
        "openProject": config.current_project,
        "personGates": if detail { gates(&reports, &keys, ask) } else { blocking_gates(&reports, &keys) },
        "waiting": waiting,
        "nextFor": next_for,
        "next": next,
    });
    Ok(if detail { full } else { compact::compact(full) })
}

#[tool_router(router = start_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Start here: read where things stand on this computer, short by default. The workspace, how many tools of the toolchain work and the ones that are missing or broken with how to fix them, whether a Tripo, Meshy and OpenAI key is saved (true or false, never a value), the image backends, Ask before shipping, the inbox count, every project with its target, stage (empty, or the earliest stage an asset is at) and asset count, the open project, the gates in the way now that only a person can pass, waiting: every approval waiting for a person now (kind, project, age and poll, the tool and arguments that follow it) and each project's sends to 3D in flight, and next: what to do now in the project nextFor names (project picks it, else the open project), each step a tool with its args or a thing a person has to do. Follow what waits; never start it again. detail true adds every tool with its version, each project's template, repo, stages and activity, the image backend's settings and every kind of approval a person gives. Works with no workspace open. Reads only; spends nothing.",
        annotations(read_only_hint = true)
    )]
    async fn get_started(&self, Parameters(args): Parameters<GetStartedArgs>) -> Out {
        let detail = args.detail.unwrap_or(false);
        respond(
            self.blocking(move |host, hub| {
                started(host, hub, detail, args.project.as_deref())
                    .map(|v| super::groups::with_off_line(v, host))
            })
            .await,
        )
    }

    #[tool(
        description = "Make a project, as New project in the app does: name, target (website or game, as list_projects says it; kind is read as another name for it), optional template and optional site repo (the game repo for a game). A name another project has is refused and nothing is overwritten. No tool removes a project: a person removes one to the workspace trash in the app. The app's open project stays as it is, unless none was open. Spends nothing and needs no approval. Returns the project and next."
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
                let manifest = create_in_workspace(host.session(), new, RepoSetBy::Agent)?;
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
