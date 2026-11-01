//! The `next` list of get_started: a small state machine over what the
//! computer and the current project say, so it is tested without an app.

use maquette_core::{ProjectManifest, ProjectTarget, Stage};
use serde_json::Value;

use super::super::hints::{person_step, step};
use super::open_workspace_step;

/// What the current project says.
pub(super) struct ProjectFacts {
    pub id: String,
    pub target: ProjectTarget,
    pub has_repo: bool,
    pub assets: usize,
    /// Assets with refs and no model yet.
    pub at_refs: usize,
    /// Assets with at least one model.
    pub with_model: usize,
}

impl ProjectFacts {
    pub(super) fn of(manifest: &ProjectManifest) -> Self {
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
    pub project: Option<ProjectFacts>,
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
        }
    }
}

fn empty_project(facts: &Facts, out: &mut Vec<Value>) {
    if facts.codex {
        out.push(step(
            "start_concepts",
            "Make concept images on Codex usage (the maquette-lab-round skill walks it).",
        ));
    } else {
        out.push(person_step(
            "Sign in to Codex (codex login) for concept images, or bring your own images as refs.",
        ));
    }
    if facts.blender {
        out.push(step(
            "add_built_asset",
            "Or make a model with a Blender script: no credits, no approval.",
        ));
    }
}

fn with_assets(p: &ProjectFacts, facts: &Facts, out: &mut Vec<Value>) {
    if p.at_refs > 0 {
        out.push(step(
            "send_to_3d",
            if facts.any_key {
                "Send the refs to 3D: tripo or meshy wait for a person to approve the credits, by-hand is free."
            } else {
                "Send the refs with provider by-hand: free, a person makes the model on the provider's site. No API key is saved."
            },
        ));
    }
    if p.with_model > 0 {
        out.push(step(
            "run_pipeline",
            "Build the project with buildOnly true: no approval, nothing written to a repo.",
        ));
    }
    if p.at_refs == 0 && p.with_model == 0 {
        out.push(step(
            "get_concepts",
            "Pick concepts onto the Board with pick_concepts when they are ready.",
        ));
    }
}

/// What to do now, the most blocking first.
pub(super) fn next_steps(facts: &Facts) -> Vec<Value> {
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
    if project.assets == 0 {
        empty_project(facts, &mut out);
    } else {
        with_assets(project, facts, &mut out);
    }
    if !project.has_repo {
        out.push(match project.target {
            ProjectTarget::Website => step(
                "set_ship_settings",
                "Set siteRepo: without a repo a run leaves Ship out and answers shipped false.",
            ),
            ProjectTarget::Game => step(
                "set_game_settings",
                "Set repo: without a repo a run leaves Ship out and answers shipped false.",
            ),
        });
    }
    out
}
