//! One step: resolving its tool, filling its args, the cache check, the
//! compress step's per model calls and recording what it wrote.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::execution::{Execution, StepOutcome};
use super::spawn::{Invocation, SpawnError, human_duration, program_name};
use crate::materials::map_groups;
use crate::pipeline::cache::{self, CacheRecord, FileHasher, KeyParts};
use crate::pipeline::graph::{BLENDER_HEADLESS_MESSAGE, blender_headless};
use crate::pipeline::history::LogLevel;
use crate::pipeline::presets::{compress_passes, resolve};
use crate::pipeline::progress::Tracker;
use crate::pipeline::tools::ResolvedTool;
use crate::pipeline::{CompressSpec, StepDef, StepKind};
use crate::toolchain::ToolId;

/// One model the compress step handles: its file, its stem, and the stem it ships under.
pub(super) struct CompressModel {
    pub(super) input: PathBuf,
    pub(super) stem: String,
    pub(super) shipped: String,
}

/// A step's resolved tool and calls, and its cache key.
struct Prepared {
    tool: ResolvedTool,
    invocations: Vec<Invocation>,
    key: String,
}

impl Execution {
    fn root(&self) -> PathBuf {
        self.setup.def.root_dir(&self.setup.project_dir)
    }

    fn fill(&self, arg: &str, step: &StepDef, root: &Path) -> String {
        let mut out = arg.to_string();
        if let Some(script) = &step.script {
            out = out.replace("{script}", script);
        }
        out = out.replace("{root}", &root.display().to_string());
        out = out.replace("{project}", &self.setup.project_dir.display().to_string());
        for (name, value) in &self.setup.vars {
            out = out.replace(&format!("{{{name}}}"), value);
        }
        out
    }

    /// Resolves the tool and the calls of `step` and its cache key.
    fn prepare(&self, step: &StepDef, hasher: &mut FileHasher) -> Result<Prepared, StepOutcome> {
        let root = self.root();
        let tool = self
            .setup
            .tools
            .resolve(step.kind, step.command.as_deref(), &root)
            .map_err(|err| StepOutcome::Failed(err.to_string(), None))?;
        if let Some(script) = &step.script
            && !root.join(script).is_file()
        {
            return Err(StepOutcome::Failed(
                format!("The script {script} is missing."),
                None,
            ));
        }
        let invocations = self
            .invocations(step, &root)
            .map_err(|message| StepOutcome::Failed(message, None))?;
        if step.kind == StepKind::Blender
            && invocations.iter().any(|(_, args)| !blender_headless(args))
        {
            // The definition was validated, but the filled args are checked
            // again right before spawning: Blender never starts with a window.
            return Err(StepOutcome::Failed(
                BLENDER_HEADLESS_MESSAGE.to_string(),
                None,
            ));
        }
        let all_args: Vec<String> = invocations
            .iter()
            .flat_map(|(_, args)| tool.prefix.iter().chain(args.iter()).cloned())
            .collect();
        let extra = self.cache_extra(step);
        let key = cache::key(
            &KeyParts {
                root: &root,
                step,
                tool: &tool,
                args: &all_args,
                extra,
            },
            hasher,
        )
        .map_err(|err| StepOutcome::Failed(err.to_string(), None))?;
        Ok(Prepared {
            tool,
            invocations,
            key,
        })
    }

    /// The record of the last finished run of a step without outputs, such
    /// as Ship. Only used to skip a gate when nothing changed since.
    fn last_id(step: &StepDef) -> String {
        format!("{}.last", step.id)
    }

    /// True when a gated step without outputs would do exactly what its
    /// last finished run did, so asking again is pointless.
    pub(super) fn unchanged_since_last(&self, step: &StepDef, hasher: &mut FileHasher) -> bool {
        if !step.outputs.is_empty() || self.setup.force {
            return false;
        }
        let Ok(prepared) = self.prepare(step, hasher) else {
            return false;
        };
        cache::load_record(&self.setup.cache_dir, &Self::last_id(step))
            .is_some_and(|record| record.key == prepared.key)
    }

    pub(super) fn run_step(
        &mut self,
        step: &StepDef,
        force: bool,
        hasher: &mut FileHasher,
    ) -> StepOutcome {
        let root = self.root();
        let Prepared {
            tool,
            invocations,
            key,
        } = match self.prepare(step, hasher) {
            Ok(prepared) => prepared,
            Err(outcome) => return outcome,
        };
        if !force
            && let Some(record) = cache::load_record(&self.setup.cache_dir, &step.id)
            && cache::is_fresh(&record, &key, &root, &self.outputs_of(step))
        {
            return StepOutcome::Cached;
        }
        let last: Vec<PathBuf> = cache::load_record(&self.setup.cache_dir, &step.id)
            .map(|record| record.outputs.iter().map(|o| root.join(&o.path)).collect())
            .unwrap_or_default();
        cache::forget(&self.setup.cache_dir, &step.id);
        let started = Instant::now();
        if let Some(spec) = &step.compress {
            let dir = root.join(&spec.to);
            if let Err(err) = fs::create_dir_all(&dir) {
                return StepOutcome::Failed(format!("Could not create {}. {err}", spec.to), None);
            }
            self.drop_stale_outputs(spec, &root, &last);
            if let Err(message) = self.write_tweaked(step, spec, &root) {
                return StepOutcome::Failed(message, None);
            }
            if !self.setup.tools.has(ToolId::Toktx) {
                self.system(
                    Some(&step.id),
                    LogLevel::Warn,
                    "toktx is missing, so textures stay PNG or JPEG. Install KTX-Software for KTX2.",
                );
            }
        }
        let mut tracker = Tracker::new(invocations.len());
        for (index, (label, args)) in invocations.iter().enumerate() {
            if let Some(label) = label {
                self.system(Some(&step.id), LogLevel::Info, label.clone());
            }
            tracker.start_call(index);
            self.report_progress(&tracker);
            let mut full = tool.prefix.clone();
            full.extend(args.iter().cloned());
            match self.spawn(step, &tool, &full, &root, &mut tracker) {
                Ok(0) => {}
                Ok(code) => {
                    return StepOutcome::Failed(
                        format!("{} exited with code {code}.", program_name(&tool)),
                        Some(code),
                    );
                }
                Err(SpawnError::Cancelled) => return StepOutcome::Cancelled,
                Err(SpawnError::Start(message)) => return StepOutcome::Failed(message, None),
                Err(SpawnError::TimedOut(limit)) => {
                    return StepOutcome::Failed(
                        format!(
                            "It ran longer than {} and was stopped.",
                            human_duration(limit)
                        ),
                        None,
                    );
                }
            }
            if self.cancelled() {
                return StepOutcome::Cancelled;
            }
        }
        let outputs = self.outputs_of(step);
        if let Some((pattern, _)) = cache::expand(&root, &outputs)
            .into_iter()
            .find(|(_, files)| files.is_empty())
        {
            return StepOutcome::Failed(
                format!("It finished but wrote nothing to {pattern}."),
                Some(0),
            );
        }
        let record = CacheRecord {
            key,
            outputs: cache::stamp_outputs(&root, &outputs),
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        };
        let id = if outputs.is_empty() {
            Self::last_id(step)
        } else {
            step.id.clone()
        };
        if let Err(err) = cache::save_record(&self.setup.cache_dir, &id, &record) {
            tracing::warn!(error = %err, "could not save the cache record");
        }
        StepOutcome::Done
    }

    fn outputs_of(&self, step: &StepDef) -> Vec<String> {
        step.outputs.clone()
    }

    /// The presets, material tweaks and toktx availability, for the
    /// compress step's key.
    fn cache_extra(&self, step: &StepDef) -> Option<String> {
        step.compress.as_ref()?;
        let presets = serde_json::to_string(&self.setup.def.presets).unwrap_or_default();
        let names = serde_json::to_string(&self.setup.shipped_names).unwrap_or_default();
        let materials = serde_json::to_string(&self.setup.materials).unwrap_or_default();
        Some(format!(
            "presets {presets} defaults {} {:?} {:?} toktx {} names {names} materials {materials}",
            self.setup.defaults.texture_size,
            self.setup.defaults.codec,
            self.setup.defaults.meshopt,
            self.setup.tools.has(ToolId::Toktx)
        ))
    }

    /// The args lists to run, each with an optional log label. Most steps
    /// have one; the compress step has a chain per model.
    fn invocations(&self, step: &StepDef, root: &Path) -> Result<Vec<Invocation>, String> {
        let Some(spec) = &step.compress else {
            return Ok(vec![(
                None,
                step.args.iter().map(|a| self.fill(a, step, root)).collect(),
            )]);
        };
        let models = self.compress_models(spec, root);
        if models.is_empty() {
            return Err(format!(
                "There are no GLB files in {} to compress.",
                spec.from
            ));
        }
        let tmp = self.setup.cache_dir.join("tmp");
        fs::create_dir_all(&tmp).map_err(|e| format!("Could not create the temp folder. {e}"))?;
        let toktx = self.setup.tools.has(ToolId::Toktx);
        let mut out = Vec::new();
        for model in models {
            // The preset follows the model file; only the output is renamed.
            let preset = resolve(&self.setup.def.presets, &model.stem, self.setup.defaults);
            let output = root.join(&spec.to).join(format!("{}.glb", model.shipped));
            let tmp_file = tmp.join(format!("{}.tex.glb", model.stem));
            let input = self.tweaked_input(&model, &tmp);
            let groups = map_groups(&model.input);
            for pass in compress_passes(&input, &output, &tmp_file, &preset, toktx, groups) {
                out.push((Some(pass.label), pass.args));
            }
        }
        Ok(out)
    }

    /// The models the compress step reads, with the stem each ships under.
    /// A name taken by another model or an earlier rename keeps the stem.
    pub(super) fn compress_models(&self, spec: &CompressSpec, root: &Path) -> Vec<CompressModel> {
        let inputs: Vec<(PathBuf, String)> = cache::expand(root, &[format!("{}/*.glb", spec.from)])
            .into_iter()
            .flat_map(|(_, files)| files)
            .map(|input| {
                let stem = input
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (input, stem)
            })
            .collect();
        let mut taken: BTreeSet<String> =
            inputs.iter().map(|(_, stem)| stem.to_lowercase()).collect();
        inputs
            .into_iter()
            .map(|(input, stem)| {
                let shipped = self
                    .setup
                    .shipped_names
                    .get(&stem.to_lowercase())
                    .filter(|name| taken.insert(name.to_lowercase()))
                    .cloned()
                    .unwrap_or_else(|| stem.clone());
                CompressModel {
                    input,
                    stem,
                    shipped,
                }
            })
            .collect()
    }

    /// Removes the files an earlier run left under a name this run no longer
    /// uses, so a renamed model does not ship twice. Only names the pipeline
    /// could have written are touched: a model file's stem, a name from the
    /// rename table, or a versioned stem the last run left behind (an asset
    /// renamed twice).
    fn drop_stale_outputs(&self, spec: &CompressSpec, root: &Path, last: &[PathBuf]) {
        let models = self.compress_models(spec, root);
        let wanted: BTreeSet<String> = models.iter().map(|m| m.shipped.to_lowercase()).collect();
        let ours: BTreeSet<String> = models
            .iter()
            .map(|m| m.stem.to_lowercase())
            .chain(self.setup.shipped_names.values().map(|n| n.to_lowercase()))
            .collect();
        let dir = root.join(&spec.to);
        let Ok(read) = fs::read_dir(&dir) else { return };
        for path in read.filter_map(Result::ok).map(|e| e.path()) {
            let is_glb = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("glb"));
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let versioned = stem
                .rsplit_once("_v")
                .is_some_and(|(_, n)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
            let left = versioned && last.contains(&path);
            if is_glb && (ours.contains(&stem) || left) && !wanted.contains(&stem) {
                let _ = fs::remove_file(&path);
            }
        }
    }
}
