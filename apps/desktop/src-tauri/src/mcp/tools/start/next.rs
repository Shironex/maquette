//! The `next` list of get_started: a small state machine over what the
//! computer and one project say, so it is tested without an app. The steps
//! come from the next steps table and name the project they are about.

use maquette_core::providers::{JobState, Provider, ProviderJob};
use maquette_core::{ProjectManifest, ProjectTarget, Stage};
use serde_json::{Value, json};

use super::super::groups::{ToolGroup, retain_on_steps};
use super::super::hints::table::{Can, Place, repo_step, start_of, step_with};
use super::super::hints::{person_step, step};
use super::open_workspace_step;

/// A project's sends to 3D that are not over yet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Sends {
    /// Tripo or Meshy sends a person has not approved yet.
    pub approval: usize,
    /// Tripo or Meshy sends queued or running.
    pub running: usize,
    /// By hand sends waiting for a person to bring the model back.
    pub by_hand: usize,
}

impl Sends {
    pub(super) fn of<'a>(jobs: impl IntoIterator<Item = &'a ProviderJob>) -> Self {
        Self::count(jobs.into_iter().map(|j| (j.state, j.provider)))
    }

    /// Counts sends by their state and provider.
    pub(super) fn count(sends: impl IntoIterator<Item = (JobState, Provider)>) -> Self {
        let mut out = Self::default();
        for (state, provider) in sends {
            match state {
                JobState::Approval => out.approval += 1,
                JobState::Queued | JobState::Running => out.running += 1,
                JobState::Waiting if provider == Provider::ByHand => out.by_hand += 1,
                _ => {}
            }
        }
        out
    }

    pub(super) fn any(self) -> bool {
        self.approval + self.running + self.by_hand > 0
    }
}

/// What a project says.
pub(super) struct ProjectFacts {
    pub id: String,
    pub target: ProjectTarget,
    pub has_repo: bool,
    pub assets: usize,
    /// Assets with refs and no model yet.
    pub at_refs: usize,
    /// Assets with at least one model.
    pub with_model: usize,
    pub sends: Sends,
}

impl ProjectFacts {
    pub(super) fn of(manifest: &ProjectManifest, jobs: &[ProviderJob]) -> Self {
        Self {
            id: manifest.id.clone(),
            target: manifest.target,
            has_repo: manifest.site_repo.is_some(),
            assets: manifest.assets.len(),
            at_refs: manifest
                .assets
                .iter()
                .filter(|a| a.stage == Stage::Refs && a.versions.is_empty())
                .count(),
            with_model: manifest
                .assets
                .iter()
                .filter(|a| !a.versions.is_empty())
                .count(),
            sends: Sends::of(jobs.iter().filter(|j| j.project == manifest.id)),
        }
    }

    fn place(&self) -> Place<'_> {
        Place {
            project: &self.id,
            target: self.target,
        }
    }
}

/// What the whole computer says.
pub(super) struct Facts {
    pub workspace_open: bool,
    pub blender: bool,
    pub codex: bool,
    pub any_key: bool,
    pub inbox: usize,
    /// The project `next` is about.
    pub project: Option<ProjectFacts>,
    /// The tool groups Settings > Agents turned off: their tools answer
    /// `group-off`, so `next` does not name them.
    pub off: Vec<&'static ToolGroup>,
}

impl Facts {
    pub(super) fn no_workspace() -> Self {
        Self {
            workspace_open: false,
            blender: false,
            codex: false,
            any_key: false,
            inbox: 0,
            project: None,
            off: Vec::new(),
        }
    }
}

/// Sends in flight come first: following them is the next step, and
/// starting the work again is what the instructions forbid.
fn follow_sends(p: &ProjectFacts, out: &mut Vec<Value>) {
    let Sends {
        approval,
        running,
        by_hand,
    } = p.sends;
    if approval + running > 0 {
        out.push(step_with(
            "get_provider_job",
            "Sends to 3D are in flight in this project: poll them until they reach the inbox. Do not send again.",
            json!({ "project": p.id }),
        ));
    }
    if by_hand > 0 {
        out.push(step_with(
            "list_handoffs",
            "By hand sends wait for a person to bring the model back. Do not send again.",
            json!({ "project": p.id }),
        ));
    }
}

fn with_assets(p: &ProjectFacts, facts: &Facts, out: &mut Vec<Value>) {
    if p.at_refs > 0 {
        out.push(step_with(
            "send_to_3d",
            if facts.any_key {
                "Send the refs to 3D: tripo or meshy wait for a person to approve the credits, by-hand is free."
            } else {
                "Send the refs with provider by-hand: free, a person makes the model on the provider's site. No API key is saved."
            },
            json!({ "project": p.id }),
        ));
    }
    if p.with_model > 0 {
        out.push(step_with(
            "run_pipeline",
            "Build the project with buildOnly true: no approval, nothing written to a repo.",
            json!({ "project": p.id, "buildOnly": true }),
        ));
    }
    if p.at_refs == 0 && p.with_model == 0 && !p.sends.any() {
        match p.target {
            ProjectTarget::Website => out.push(step_with(
                "get_concepts",
                "Pick concepts onto the Board with pick_concepts when they are ready.",
                json!({ "project": p.id }),
            )),
            ProjectTarget::Game => out.extend(start_of(p.place(), p.has_repo, can(facts))),
        }
    }
}

fn can(facts: &Facts) -> Can {
    Can {
        codex: facts.codex,
        blender: facts.blender,
    }
}

/// What to do now, the most blocking first, without the tools of a group
/// that is off.
pub(super) fn next_steps(facts: &Facts) -> Vec<Value> {
    let mut steps = all_steps(facts);
    retain_on_steps(&mut steps, &facts.off);
    steps
}

fn all_steps(facts: &Facts) -> Vec<Value> {
    if !facts.workspace_open {
        return vec![open_workspace_step()];
    }
    let mut out = Vec::new();
    if !facts.blender {
        out.push(person_step(
            "Install Blender from blender.org or locate it in Settings > Toolchain: every build runs it.",
        ));
    }
    if facts.inbox > 0 {
        out.push(step(
            "list_inbox",
            "Models wait in the inbox. Check them and accept the good ones.",
        ));
    }
    let Some(project) = &facts.project else {
        out.push(step(
            "create_project",
            "There is no project yet. Make a website or a game project.",
        ));
        return out;
    };
    follow_sends(project, &mut out);
    if project.assets == 0 && !project.sends.any() {
        out.extend(start_of(project.place(), project.has_repo, can(facts)));
    } else {
        with_assets(project, facts, &mut out);
    }
    if !project.has_repo {
        out.push(repo_step(project.place()));
    }
    // A step the table gave twice (a game's repo) stays once, first.
    let mut seen = Vec::new();
    out.retain(|s| match s["tool"].as_str() {
        Some(tool) if seen.contains(&tool.to_string()) => false,
        Some(tool) => {
            seen.push(tool.to_string());
            true
        }
        None => true,
    });
    out
}
