//! The `codex exec` run behind a Codex job: one shell step on the concepts
//! lane, the prompt on its standard input. [`CodexRun`] is what that run
//! needs, so the same run can be built again from it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{LANE, Rerun};
use crate::pipeline::runner::{RunOptions, RunSetup};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::workspace::{MeshoptLevel, TextureCodec};

/// What one Codex job's run is made of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CodexRun {
    /// `<workspace cache>/concepts`, for the run history and logs.
    pub(super) cache_dir: PathBuf,
    /// The codex program.
    pub(super) program: Option<String>,
    pub(super) args: Vec<String>,
    /// The step's name, `<direction> #<index>`.
    pub(super) step: String,
    /// The prompt, written to its standard input.
    pub(super) prompt: String,
    pub(super) timeout_ms: u64,
}

impl CodexRun {
    /// The run for the job `id` in `project_dir`.
    pub(super) fn rerun(&self, id: &str, project_dir: &Path, tools: Toolbox) -> Rerun {
        Rerun {
            setup: RunSetup {
                project: LANE.to_string(),
                project_dir: project_dir.to_path_buf(),
                cache_dir: self.cache_dir.clone(),
                def: PipelineDef {
                    version: PIPELINE_VERSION,
                    root: None,
                    steps: vec![StepDef {
                        id: id.to_string(),
                        name: self.step.clone(),
                        kind: StepKind::Shell,
                        command: self.program.clone(),
                        script: None,
                        args: self.args.clone(),
                        inputs: Vec::new(),
                        outputs: Vec::new(),
                        depends_on: Vec::new(),
                        compress: None,
                    }],
                    presets: Vec::new(),
                },
                tools,
                defaults: CompressDefaults {
                    texture_size: 2048,
                    codec: TextureCodec::Etc1s,
                    meshopt: MeshoptLevel::Medium,
                },
                vars: Default::default(),
                shipped_names: Default::default(),
                materials: Default::default(),
                from: None,
                force: true,
                textures: Default::default(),
                quality: crate::ship::DEFAULT_QUALITY,
                build_only: false,
            },
            options: RunOptions {
                stdin: Some(self.prompt.clone()),
                timeout: Some(Duration::from_millis(self.timeout_ms)),
                gate: None,
            },
        }
    }

    /// What `rerun` was built from.
    pub(super) fn of(rerun: &Rerun) -> Self {
        let step = rerun.setup.def.steps.first();
        Self {
            cache_dir: rerun.setup.cache_dir.clone(),
            program: step.and_then(|s| s.command.clone()),
            args: step.map(|s| s.args.clone()).unwrap_or_default(),
            step: step.map(|s| s.name.clone()).unwrap_or_default(),
            prompt: rerun.options.stdin.clone().unwrap_or_default(),
            timeout_ms: rerun
                .options
                .timeout
                .map_or(0, |t| u64::try_from(t.as_millis()).unwrap_or(u64::MAX)),
        }
    }
}
