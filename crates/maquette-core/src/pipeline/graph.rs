//! Pipeline validation and ordering: unique ids, known dependencies, no
//! cycles, Blender always headless, and the run plan for "Run from here".

use std::collections::{BTreeSet, HashMap};
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use super::{PipelineDef, StepDef, StepKind};
use crate::error::{CoreError, CoreResult};
use crate::workspace::is_single_segment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum IssueLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PipelineIssue {
    pub level: IssueLevel,
    /// The step the issue is about, if any.
    pub step: Option<String>,
    pub message: String,
}

impl PipelineIssue {
    pub fn is_error(&self) -> bool {
        self.level == IssueLevel::Error
    }

    fn error(step: Option<&str>, message: String) -> Self {
        Self {
            level: IssueLevel::Error,
            step: step.map(str::to_string),
            message,
        }
    }

    fn warning(step: Option<&str>, message: String) -> Self {
        Self {
            level: IssueLevel::Warning,
            step: step.map(str::to_string),
            message,
        }
    }
}

/// Placeholders the runner fills in args.
pub const PLACEHOLDERS: [&str; 6] = [
    "script",
    "root",
    "project",
    "workspace",
    "siteRepo",
    "shipIndex",
];

/// Flags after which Blender runs Python or loads a file. `-b` has to come
/// before all of them, or Blender may open its window first.
const BLENDER_LATE_FLAGS: [&str; 5] = ["--python", "-P", "--python-expr", "--python-text", "--"];

/// True when Blender's args start it headless: `-b` or `--background`
/// before any script, file or `--`.
pub fn blender_headless(args: &[String]) -> bool {
    for arg in args {
        if arg == "-b" || arg == "--background" {
            return true;
        }
        let late = BLENDER_LATE_FLAGS.contains(&arg.as_str())
            || Path::new(arg)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("blend"))
            || arg.ends_with(".blend}");
        if late {
            return false;
        }
    }
    false
}

pub const BLENDER_HEADLESS_MESSAGE: &str = "Blender steps always run headless. Put -b first in the args, before the blend file and --python.";

/// `{name}` tokens in `arg` (letters only), in order.
pub fn placeholders(arg: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = arg;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close)
                if close > 0 && after[..close].chars().all(|c| c.is_ascii_alphanumeric()) =>
            {
                out.push(&after[..close]);
                rest = &after[close + 1..];
            }
            _ => rest = after,
        }
    }
    out
}

fn is_safe_relative(path: &str) -> bool {
    let p = Path::new(path);
    !path.trim().is_empty()
        && !p.is_absolute()
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn check_step(step: &StepDef, issues: &mut Vec<PipelineIssue>) {
    let id = Some(step.id.as_str());
    let label = if step.name.trim().is_empty() {
        step.id.as_str()
    } else {
        step.name.as_str()
    };
    if step.name.trim().is_empty() {
        issues.push(PipelineIssue::error(
            id,
            format!("Step {} needs a name.", step.id),
        ));
    }
    match step.kind {
        StepKind::Blender if !blender_headless(&step.args) => {
            issues.push(PipelineIssue::error(
                id,
                format!("{label}: {BLENDER_HEADLESS_MESSAGE}"),
            ));
        }
        StepKind::Shell if step.command.as_deref().is_none_or(|c| c.trim().is_empty()) => {
            issues.push(PipelineIssue::error(
                id,
                format!(
                    "{label} is a shell step without a command. Set command to sh, python3 or node."
                ),
            ));
        }
        _ => {}
    }
    match (&step.compress, step.kind) {
        (Some(_), kind) if kind != StepKind::GltfTransform => issues.push(PipelineIssue::error(
            id,
            format!("{label}: only gltf-transform steps can use the compression presets."),
        )),
        (Some(spec), _) => {
            for (what, value) in [("from", &spec.from), ("to", &spec.to)] {
                if !is_safe_relative(value) {
                    issues.push(PipelineIssue::error(
                        id,
                        format!(
                            "{label}: compress {what} must be a folder inside the pipeline root."
                        ),
                    ));
                }
            }
        }
        (None, StepKind::GltfTransform | StepKind::Toktx | StepKind::Ffmpeg)
            if step.args.is_empty() =>
        {
            issues.push(PipelineIssue::error(id, format!("{label} has no args.")));
        }
        (None, _) => {}
    }
    for arg in &step.args {
        for name in placeholders(arg) {
            if !PLACEHOLDERS.contains(&name) {
                issues.push(PipelineIssue::error(
                    id,
                    format!(
                        "{label}: {{{name}}} is not a placeholder. Use one of {}.",
                        PLACEHOLDERS.map(|p| format!("{{{p}}}")).join(", ")
                    ),
                ));
            }
        }
    }
    if step.args.iter().any(|a| a.contains("{script}")) && step.script.is_none() {
        issues.push(PipelineIssue::error(
            id,
            format!("{label} uses {{script}} but has no script."),
        ));
    }
    let paths = step
        .script
        .iter()
        .map(|p| ("script", p))
        .chain(step.inputs.iter().map(|p| ("input", p)))
        .chain(step.outputs.iter().map(|p| ("output", p)));
    for (what, path) in paths {
        if !is_safe_relative(path) {
            issues.push(PipelineIssue::error(
                id,
                format!(
                    "{label}: the {what} {path} must be relative to the pipeline root, without .."
                ),
            ));
        }
    }
}

/// Every problem with the definition. Errors block saving and running;
/// warnings are shown next to the step.
pub fn validate(def: &PipelineDef) -> Vec<PipelineIssue> {
    let mut issues = Vec::new();
    if def.steps.is_empty() {
        issues.push(PipelineIssue::warning(
            None,
            "The pipeline has no steps yet.".to_string(),
        ));
    }
    let mut seen = BTreeSet::new();
    for step in &def.steps {
        if !is_single_segment(&step.id) || step.id.contains(' ') {
            issues.push(PipelineIssue::error(
                Some(&step.id),
                format!(
                    "{:?} is not a step id. Use letters, digits and dashes.",
                    step.id
                ),
            ));
        }
        if !seen.insert(step.id.as_str()) {
            issues.push(PipelineIssue::error(
                Some(&step.id),
                format!("Two steps share the id {}.", step.id),
            ));
        }
        check_step(step, &mut issues);
    }
    for step in &def.steps {
        for dep in &step.depends_on {
            if dep == &step.id {
                issues.push(PipelineIssue::error(
                    Some(&step.id),
                    format!("{} depends on itself.", step.name),
                ));
            } else if !seen.contains(dep.as_str()) {
                issues.push(PipelineIssue::error(
                    Some(&step.id),
                    format!("{} depends on {dep}, which is not a step.", step.name),
                ));
            }
        }
    }
    if let Err(cycle) = topo_order(def) {
        issues.push(PipelineIssue::error(
            cycle.first().map(String::as_str),
            format!(
                "These steps depend on each other in a loop: {}.",
                cycle.join(", ")
            ),
        ));
    }
    for preset in &def.presets {
        if let Err(message) = preset.check() {
            issues.push(PipelineIssue::error(None, message));
        }
    }
    issues
}

/// Step indices in dependency order, ties kept in definition order. Unknown
/// dependencies are ignored here (validation reports them). On a cycle,
/// returns the ids of the steps stuck in it.
pub fn topo_order(def: &PipelineDef) -> Result<Vec<usize>, Vec<String>> {
    let index: HashMap<&str, usize> = def
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();
    let mut remaining: Vec<usize> = def
        .steps
        .iter()
        .map(|s| {
            s.depends_on
                .iter()
                .filter(|d| *d != &s.id && index.contains_key(d.as_str()))
                .collect::<BTreeSet<_>>()
                .len()
        })
        .collect();
    let mut done = vec![false; def.steps.len()];
    let mut order = Vec::with_capacity(def.steps.len());
    loop {
        let Some(next) = (0..def.steps.len()).find(|&i| !done[i] && remaining[i] == 0) else {
            break;
        };
        done[next] = true;
        order.push(next);
        let id = def.steps[next].id.as_str();
        for (i, step) in def.steps.iter().enumerate() {
            if !done[i] && step.depends_on.iter().any(|d| d == id) {
                remaining[i] -= 1;
            }
        }
    }
    if order.len() == def.steps.len() {
        Ok(order)
    } else {
        Err(def
            .steps
            .iter()
            .enumerate()
            .filter(|(i, _)| !done[*i])
            .map(|(_, s)| s.id.clone())
            .collect())
    }
}

/// What the runner does with each step of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Planned {
    /// Run unless the cache says the inputs did not change.
    Run,
    /// Run even when cached.
    Force,
    /// Not part of this run ("Run from here" leaves earlier steps alone).
    Skip,
}

/// The run plan in dependency order. Without `from`, every step runs. With
/// `from`, that step is forced and everything that depends on it (directly
/// or not) runs; other steps are skipped. `force` forces every planned step.
pub fn plan(
    def: &PipelineDef,
    from: Option<&str>,
    force: bool,
) -> CoreResult<Vec<(usize, Planned)>> {
    if let Some(issue) = validate(def).into_iter().find(PipelineIssue::is_error) {
        return Err(CoreError::Invalid(issue.message));
    }
    let order = topo_order(def).map_err(|cycle| {
        CoreError::Invalid(format!("These steps form a loop: {}.", cycle.join(", ")))
    })?;
    let run = if force { Planned::Force } else { Planned::Run };
    let Some(from) = from else {
        return Ok(order.into_iter().map(|i| (i, run)).collect());
    };
    if def.step(from).is_none() {
        return Err(CoreError::NotFound(format!("the step {from}")));
    }
    let mut included: BTreeSet<&str> = BTreeSet::from([from]);
    for &i in &order {
        let step = &def.steps[i];
        if step
            .depends_on
            .iter()
            .any(|d| included.contains(d.as_str()))
        {
            included.insert(step.id.as_str());
        }
    }
    Ok(order
        .into_iter()
        .map(|i| {
            let id = def.steps[i].id.as_str();
            let planned = if id == from {
                Planned::Force
            } else if included.contains(id) {
                run
            } else {
                Planned::Skip
            };
            (i, planned)
        })
        .collect())
}

#[cfg(test)]
mod tests;
