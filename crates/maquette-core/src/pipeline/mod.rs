//! The pipeline: a `pipeline.json` per project with the steps that turn the
//! Blender files into the shipped web models, the compression presets, the
//! content cache that skips unchanged steps and the runner that spawns them.

pub mod bake;
pub mod cache;
pub mod export_one;
pub mod graph;
pub mod presets;
pub mod progress;
pub mod runner;
pub mod script;
pub mod starter;
pub mod template;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::json::write_json;
use crate::workspace::{MeshoptLevel, TextureCodec};

pub use crate::base::tools::StepKind;
pub use crate::base::{history, tools};
pub use graph::{PipelineIssue, validate};
pub use presets::{CompressPreset, ResolvedPreset};

pub const PIPELINE_FILE: &str = "pipeline.json";
pub const PIPELINE_VERSION: u32 = 1;

/// Where the compress step reads raw GLBs and writes the compressed ones.
/// Each `<from>/<model>.glb` goes through the texture and meshopt passes its
/// preset asks for and lands at `<to>/<model>.glb`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompressSpec {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StepDef {
    /// Unique within the pipeline, used by `dependsOn`.
    pub id: String,
    pub name: String,
    pub kind: StepKind,
    /// The program for shell steps, for example `sh`, `python3` or `node`.
    /// Other kinds use the detected tool.
    pub command: Option<String>,
    /// The script the step runs, relative to the pipeline root. It is part
    /// of the cache key and fills `{script}` in the args.
    pub script: Option<String>,
    /// One argument per entry, never joined into a shell string. Supports
    /// `{script}`, `{root}`, `{project}`, `{workspace}`, `{siteRepo}` and
    /// `{shipIndex}` and `{exportOrigin}`.
    pub args: Vec<String>,
    /// Globs relative to the pipeline root. Their contents are part of the
    /// cache key.
    pub inputs: Vec<String>,
    /// Globs relative to the pipeline root that must exist after the step.
    pub outputs: Vec<String>,
    pub depends_on: Vec<String>,
    /// Set on the compress step: the args come from the presets instead.
    pub compress: Option<CompressSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PipelineDef {
    pub version: u32,
    /// Where steps run and paths resolve. `None` means the project folder;
    /// an imported project points at its source folder.
    pub root: Option<String>,
    pub steps: Vec<StepDef>,
    /// Per model overrides for the compress step. Models without a row use
    /// the workspace defaults.
    pub presets: Vec<CompressPreset>,
}

/// The step that copies the build to the site: id or name "ship".
pub fn is_ship_step(step: &StepDef) -> bool {
    step.id.trim().eq_ignore_ascii_case("ship") || step.name.trim().eq_ignore_ascii_case("ship")
}

impl PipelineDef {
    pub fn step(&self, id: &str) -> Option<&StepDef> {
        self.steps.iter().find(|s| s.id == id)
    }

    /// The folder steps run in.
    pub fn root_dir(&self, project_dir: &Path) -> PathBuf {
        match self
            .root
            .as_deref()
            .map(str::trim)
            .filter(|r| !r.is_empty())
        {
            Some(root) => project_dir.join(root),
            None => project_dir.to_path_buf(),
        }
    }
}

/// The workspace defaults the presets fall back on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressDefaults {
    pub texture_size: u32,
    pub codec: TextureCodec,
    pub meshopt: MeshoptLevel,
}

pub fn path(project_dir: &Path) -> PathBuf {
    project_dir.join(PIPELINE_FILE)
}

/// Reads `pipeline.json`, or `None` when the project has none yet.
pub fn load(project_dir: &Path) -> CoreResult<Option<PipelineDef>> {
    let file = path(project_dir);
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(CoreError::io(&file, err)),
    };
    let def: PipelineDef = serde_json::from_str(&text).map_err(|e| CoreError::json(&file, e))?;
    if def.version > PIPELINE_VERSION {
        return Err(CoreError::UnsupportedVersion {
            found: def.version,
            supported: PIPELINE_VERSION,
        });
    }
    Ok(Some(def))
}

/// Validates the graph and writes pretty JSON atomically. A pipeline with
/// errors is refused so the file on disk always runs.
pub fn save(project_dir: &Path, def: &PipelineDef) -> CoreResult<()> {
    if let Some(issue) = validate(def).into_iter().find(PipelineIssue::is_error) {
        return Err(CoreError::Invalid(issue.message));
    }
    write_json(&path(project_dir), def)
}

/// The project's pipeline, writing the starting one (its template, or the
/// proposal for an imported folder) the first time. A template's starter
/// scripts go into `tools/` at the same moment, skipping files that exist,
/// so a new project runs end to end on day one.
///
/// # Errors
///
/// Fails when `pipeline.json` exists but cannot be read.
pub fn ensure(
    project_dir: &Path,
    manifest: &crate::project::ProjectManifest,
) -> CoreResult<PipelineDef> {
    if let Some(mut def) = load(project_dir)? {
        if let Ok(settings) = crate::game::settings_of(manifest) {
            if template::fit_game(&mut def, settings)
                && let Err(err) = save(project_dir, &def)
            {
                tracing::warn!(error = %err, "the fitted game pipeline was not saved");
            }
            return Ok(def);
        }
        let presets = template::fit_presets(&mut def, manifest);
        let layout = template::fit_build_inputs(&mut def, project_dir);
        let ship = template::fit_ship_args(&mut def, project_dir);
        let origin = template::fit_export_origin(&mut def, project_dir);
        if presets || layout || ship || origin {
            match save(project_dir, &def) {
                Ok(()) => {
                    tracing::info!(
                        presets,
                        layout,
                        ship,
                        origin,
                        "fitted the pipeline to the project"
                    );
                }
                Err(err) => tracing::warn!(error = %err, "the fitted pipeline was not saved"),
            }
        }
        return Ok(def);
    }
    let mut def = template::initial(manifest);
    template::fit_presets(&mut def, manifest);
    // A proposal with problems is still shown, so it can be fixed; it is
    // only written once it validates.
    match save(project_dir, &def) {
        Ok(()) => match starter::write(project_dir, manifest.template) {
            Ok(written) if !written.is_empty() => {
                tracing::info!(scripts = ?written, "wrote the starter scripts");
            }
            Ok(_) => {}
            Err(err) => tracing::warn!(error = %err, "the starter scripts were not written"),
        },
        Err(err) => tracing::warn!(error = %err, "the starting pipeline was not saved"),
    }
    Ok(def)
}

/// Registers this module's IPC types (the pipeline definition, its issues, its events and the
/// starters) with the shell's TypeScript export; `maquette_core::types` folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<CompressSpec>()
        .register::<StepDef>()
        .register::<PipelineDef>()
        .register::<presets::CompressPreset>()
        .register::<graph::IssueLevel>()
        .register::<graph::PipelineIssue>()
        .register::<runner::PipelineEvent>()
        .register::<starter::StarterState>()
        .register::<starter::StarterStatus>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectTemplate;

    #[test]
    fn save_and_load_round_trip_and_refuse_broken_graphs() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(load(dir.path()).expect("missing"), None);
        let def = template::for_template(ProjectTemplate::FloatingDiorama);
        save(dir.path(), &def).expect("save");
        let text = fs::read_to_string(path(dir.path())).expect("read");
        assert!(text.contains("\"dependsOn\""));
        assert!(text.contains("\"kind\": \"gltf-transform\""));
        assert_eq!(load(dir.path()).expect("load"), Some(def.clone()));

        let mut broken = def;
        broken.steps[0].depends_on = vec!["nowhere".into()];
        assert_eq!(
            save(dir.path(), &broken).expect_err("broken").code(),
            "invalid"
        );
    }

    #[test]
    fn ensure_writes_the_starter_scripts_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest = crate::project::ProjectManifest {
            version: crate::project::PROJECT_VERSION,
            id: "moon-island".into(),
            name: "Moon Island".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            site_repo_name: None,
            site_repo_state: Default::default(),
            source_folder: None,
            created_at: "2026-10-24T09:30:00.000Z".into(),
            assets: Vec::new(),
        };
        let def = ensure(dir.path(), &manifest).expect("ensure");
        for script in def.steps.iter().filter_map(|s| s.script.as_deref()) {
            assert!(dir.path().join(script).is_file(), "{script} missing");
        }
        // An edited script survives; a deleted one stays deleted, because
        // the pipeline exists now and ensure only reads it.
        let build = dir.path().join("tools/build_scene.py");
        fs::write(&build, "print('mine')\n").expect("edit");
        fs::remove_file(dir.path().join("tools/ship.mjs")).expect("remove");
        ensure(dir.path(), &manifest).expect("again");
        assert_eq!(fs::read_to_string(&build).expect("read"), "print('mine')\n");
        assert!(!dir.path().join("tools/ship.mjs").exists());
    }

    #[test]
    fn root_dir_defaults_to_the_project() {
        let mut def = template::for_template(ProjectTemplate::Room);
        let project = Path::new("/p");
        assert_eq!(def.root_dir(project), PathBuf::from("/p"));
        def.root = Some("/src/island".into());
        assert_eq!(def.root_dir(project), PathBuf::from("/src/island"));
    }
}
