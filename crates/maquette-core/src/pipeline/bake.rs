//! Bake settings: the lightmap size, the Cycles samples and the denoiser of
//! a project's Bake light step. They live where the step reads them, as
//! `--size`, `--samples` and `--denoise` / `--no-denoise` in its args in
//! `pipeline.json`, so the args stay the one place a person reads or edits
//! them and the cache reruns the bake when they change. Only the starter
//! step (`tools/bake_light.py`) is managed here; a pipeline without it has
//! no bake settings.
//!
//! Older starters know `--size` and `--samples` but cannot denoise, so the
//! denoiser is only switched on or off when the project's script is the
//! current starter or its args already carry the flag (a script of the
//! person's own that takes it).

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::starter::{self, StarterState};
use super::{PipelineDef, StepDef};
use crate::error::{CoreError, CoreResult};
use crate::prepare;
use crate::project::ProjectTemplate;

/// The script whose step these settings belong to.
pub const SCRIPT: &str = "tools/bake_light.py";
/// The lightmap edges a person can pick, in pixels.
pub const SIZES: [u32; 5] = [256, 512, 1024, 2048, 4096];
pub const MIN_SAMPLES: u32 = 1;
pub const MAX_SAMPLES: u32 = 4096;

/// What a new project bakes with: sharp and clean enough for close camera
/// stops in a small scene.
pub const DEFAULT: BakeSettings = BakeSettings {
    size: 1024,
    samples: 32,
    denoise: true,
};

/// What an older starter does with a flag left out of its args.
const OLDER_STARTER: BakeSettings = BakeSettings {
    size: 512,
    samples: 16,
    denoise: false,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BakeSettings {
    /// The lightmap edge in pixels, one of [`SIZES`].
    pub size: u32,
    /// Cycles samples per lightmap.
    pub samples: u32,
    /// Runs OpenImageDenoise over each lightmap.
    pub denoise: bool,
}

/// A project's bake settings as the app and agents read them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BakeView {
    pub settings: BakeSettings,
    /// Whether the step's script takes the denoise flag.
    pub can_denoise: bool,
    /// How the project's `tools/bake_light.py` compares with the shipped one.
    pub script: StarterState,
    /// The assets set to skip the bake, by id.
    pub skipped: Vec<String>,
}

/// A change: the fields left out keep what the step has.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BakePatch {
    pub size: Option<u32>,
    pub samples: Option<u32>,
    pub denoise: Option<bool>,
}

fn is_bake(step: &StepDef) -> bool {
    step.script.as_deref() == Some(SCRIPT)
}

/// The Bake light step, when the pipeline runs the starter.
pub fn step(def: &PipelineDef) -> Option<&StepDef> {
    def.steps.iter().find(|s| is_bake(s))
}

fn value_after(args: &[String], flag: &str) -> Option<u32> {
    let at = args.iter().position(|a| a == flag)?;
    args.get(at + 1)?.parse().ok()
}

fn has_denoise_flag(args: &[String]) -> bool {
    args.iter().any(|a| a == "--denoise" || a == "--no-denoise")
}

/// The settings the args ask for, filling the flags they leave out with
/// what the script does without them.
pub fn read_args(args: &[String], current_script: bool) -> BakeSettings {
    let fallback = if current_script {
        DEFAULT
    } else {
        OLDER_STARTER
    };
    let denoise = if args.iter().any(|a| a == "--no-denoise") {
        false
    } else if args.iter().any(|a| a == "--denoise") {
        true
    } else {
        fallback.denoise
    };
    BakeSettings {
        size: value_after(args, "--size").unwrap_or(fallback.size),
        samples: value_after(args, "--samples").unwrap_or(fallback.samples),
        denoise,
    }
}

/// Checks the numbers.
///
/// # Errors
///
/// Returns the problem as a sentence.
pub fn validate(settings: &BakeSettings) -> CoreResult<()> {
    if !SIZES.contains(&settings.size) {
        return Err(CoreError::Invalid(format!(
            "A lightmap is 256, 512, 1024, 2048 or 4096 px, not {}.",
            settings.size
        )));
    }
    if !(MIN_SAMPLES..=MAX_SAMPLES).contains(&settings.samples) {
        return Err(CoreError::Invalid(format!(
            "The bake takes {MIN_SAMPLES} to {MAX_SAMPLES} samples, not {}.",
            settings.samples
        )));
    }
    Ok(())
}

fn set_value(args: &mut Vec<String>, flag: &str, value: u32) {
    match args.iter().position(|a| a == flag) {
        Some(at) if at + 1 < args.len() => args[at + 1] = value.to_string(),
        Some(_) => args.push(value.to_string()),
        None => args.extend([flag.to_string(), value.to_string()]),
    }
}

/// Writes `settings` into the step's args. The denoise flag is only
/// written when the script takes it; asking an older script to denoise is
/// refused.
///
/// # Errors
///
/// Fails when [`validate`] refuses the numbers, or for denoise on a script
/// that cannot.
pub fn write_args(
    args: &mut Vec<String>,
    settings: &BakeSettings,
    can_denoise: bool,
) -> CoreResult<()> {
    validate(settings)?;
    if settings.denoise && !can_denoise {
        return Err(CoreError::Invalid(format!(
            "{SCRIPT} is an older or edited copy that cannot denoise. Update it from the Pipeline view's starter banner first, or leave denoise off."
        )));
    }
    set_value(args, "--size", settings.size);
    set_value(args, "--samples", settings.samples);
    if can_denoise {
        args.retain(|a| a != "--denoise" && a != "--no-denoise");
        args.push(
            if settings.denoise {
                "--denoise"
            } else {
                "--no-denoise"
            }
            .to_string(),
        );
    }
    Ok(())
}

fn script_state(project_dir: &Path, template: ProjectTemplate) -> StarterState {
    starter::status(project_dir, template)
        .into_iter()
        .find(|s| s.path == SCRIPT)
        .map_or(StarterState::Edited, |s| s.state)
}

/// The project's bake settings, or `None` when its pipeline has no starter
/// Bake light step. A `prepare.json` that cannot be read lists no skipped
/// assets.
pub fn view(project_dir: &Path, template: ProjectTemplate, def: &PipelineDef) -> Option<BakeView> {
    let step = step(def)?;
    let script = script_state(project_dir, template);
    let current = script == StarterState::Current;
    let skipped = prepare::load(project_dir)
        .map(|prep| {
            prep.models
                .iter()
                .filter(|(_, entry)| entry.skips_bake())
                .map(|(id, _)| id.clone())
                .collect()
        })
        .unwrap_or_default();
    Some(BakeView {
        settings: read_args(&step.args, current),
        can_denoise: current || has_denoise_flag(&step.args),
        script,
        skipped,
    })
}

/// Applies `patch` to the Bake light step of `def` and returns the
/// settings it ends up with. The caller saves the pipeline.
///
/// # Errors
///
/// Fails when the pipeline has no starter Bake light step, or as
/// [`write_args`].
pub fn apply(
    project_dir: &Path,
    template: ProjectTemplate,
    def: &mut PipelineDef,
    patch: &BakePatch,
) -> CoreResult<BakeSettings> {
    let found = view(project_dir, template, def).ok_or_else(|| {
        CoreError::Invalid(format!(
            "This pipeline has no Bake light step that runs {SCRIPT}, so it has no bake settings."
        ))
    })?;
    let next = BakeSettings {
        size: patch.size.unwrap_or(found.settings.size),
        samples: patch.samples.unwrap_or(found.settings.samples),
        denoise: patch
            .denoise
            .unwrap_or(found.settings.denoise && found.can_denoise),
    };
    let step = def
        .steps
        .iter_mut()
        .find(|s| is_bake(s))
        .ok_or_else(|| CoreError::Invalid("The Bake light step is gone.".to_string()))?;
    write_args(&mut step.args, &next, found.can_denoise)?;
    Ok(next)
}

#[cfg(test)]
mod tests;
