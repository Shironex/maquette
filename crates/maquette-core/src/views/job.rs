//! The views service: one headless Blender run per set, on a runner lane of
//! its own. A set that is in its folder with the same key answers at once,
//! a set on its way is joined, and a run that fails says why the next time
//! it is asked for.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use super::spec::{self, OUT_DIR, SPEC_FILE};
use super::{
    Asked, LANE, RECORD_FILE, SCRIPT, SCRIPT_FILE, SIZE, SIZES, ViewSet, ViewsRequest, stored,
};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::{self, RunRecord, RunState};
use crate::pipeline::runner::{
    JobRunner, PipelineEvent, PipelineRunner, RunOptions, RunSetup, RunSink,
};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec};

/// A set still rendering after this long is stopped: a dense scan takes a
/// few minutes at most.
const TIMEOUT: Duration = Duration::from_secs(20 * 60);
/// The runner's records and logs, inside a run's scratch folder.
const RUNS_DIR: &str = "runs";

struct Pending {
    run_id: Option<String>,
    key: String,
    request: ViewsRequest,
    cancelled: bool,
}

#[derive(Default)]
struct Shared {
    /// The sets on their way, by the folder they go to.
    pending: Mutex<HashMap<PathBuf, Pending>>,
    /// Why the last run for a folder failed, with its key, until asked.
    failed: Mutex<HashMap<PathBuf, (String, String)>>,
}

impl Shared {
    fn pending(&self) -> MutexGuard<'_, HashMap<PathBuf, Pending>> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn failed(&self) -> MutexGuard<'_, HashMap<PathBuf, (String, String)>> {
        self.failed.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Moves a finished run's pictures into their folder, then writes the
/// record, so a folder never shows half a set as a whole one.
fn land(request: &ViewsRequest, key: &str) -> CoreResult<ViewSet> {
    let set = spec::read_set(request, key, &request.work)?;
    fs::create_dir_all(&request.dir).map_err(|e| CoreError::io(&request.dir, e))?;
    let out = request.work.join(OUT_DIR);
    for target in spec::files_of(&set) {
        let Some(name) = target.file_name() else {
            continue;
        };
        let made = out.join(name);
        fs::copy(&made, &target).map_err(|e| CoreError::io(&made, e))?;
    }
    crate::json::write_json(&request.dir.join(RECORD_FILE), &set)?;
    let _ = fs::remove_dir_all(&out);
    Ok(set)
}

/// Why a run failed: the script's own `Error:` line when its log has one,
/// else what the runner says of the step.
fn why(run: &RunRecord, cache_dir: &Path) -> String {
    let said = history::load_log(cache_dir, run.number)
        .iter()
        .rev()
        .find_map(|line| {
            let (_, error) = line.text.split_once("[render_views] Error: ")?;
            Some(error.trim().to_string())
        });
    said.or_else(|| run.steps.iter().find_map(|s| s.message.clone()))
        .or(run.message.clone())
        .unwrap_or_else(|| "Blender stopped without saying why.".to_string())
}

/// Hears when a run ends and lands its set.
struct Sink(Arc<Shared>);

impl RunSink for Sink {
    fn event(&self, event: PipelineEvent) {
        let PipelineEvent::Run { run } = event else {
            return;
        };
        // Only finished runs take the lock: `request` holds it while it
        // submits, and the runner reports the queued run from that call.
        if !run.state.is_finished() {
            return;
        }
        let done = {
            let mut pending = self.0.pending();
            let key = pending
                .iter()
                .find(|(_, p)| p.run_id.as_deref() == Some(run.id.as_str()))
                .map(|(k, _)| k.clone());
            key.and_then(|k| pending.remove(&k))
        };
        let Some(done) = done else {
            return;
        };
        if done.cancelled {
            return;
        }
        let trouble = if run.state == RunState::Done {
            land(&done.request, &done.key).err().map(|e| e.to_string())
        } else {
            let said = why(&run, &done.request.work.join(RUNS_DIR));
            Some(format!("The views did not render. {said}"))
        };
        if let Some(message) = trouble {
            self.0
                .failed()
                .insert(done.request.dir.clone(), (done.key, message));
        }
    }
}

fn step(request: &ViewsRequest) -> StepDef {
    let args = [
        "-b",
        "--factory-startup",
        "--python-exit-code",
        "1",
        "--python",
        "{script}",
        "--",
        "--spec",
        SPEC_FILE,
    ];
    StepDef {
        id: "views".into(),
        name: format!(
            "Render {} views",
            request.views.len() * request.models.len()
        ),
        kind: StepKind::Blender,
        command: None,
        script: Some(SCRIPT_FILE.into()),
        args: args.iter().map(|s| (*s).to_string()).collect(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        depends_on: Vec::new(),
        compress: None,
    }
}

fn setup(request: &ViewsRequest, tools: &Toolbox) -> RunSetup {
    RunSetup {
        project: LANE.to_string(),
        project_dir: request.work.clone(),
        cache_dir: request.work.join(RUNS_DIR),
        def: PipelineDef {
            version: PIPELINE_VERSION,
            root: None,
            steps: vec![step(request)],
            presets: Vec::new(),
        },
        tools: tools.clone(),
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
    }
}

fn check(request: &ViewsRequest) -> CoreResult<()> {
    let bad = |message: String| Err(CoreError::Invalid(message));
    if request.models.is_empty() || request.models.len() > 2 {
        return bad("A set shows one model, or two side by side.".to_string());
    }
    if request.models.len() == 2 && request.models[0].id == request.models[1].id {
        return bad("A side by side takes two different versions.".to_string());
    }
    if request.views.is_empty() {
        return bad("A set needs at least one view.".to_string());
    }
    if !SIZES.contains(&request.size) {
        return bad(format!(
            "A picture's edge is {} to {} px, not {}.",
            SIZES.start(),
            SIZES.end(),
            request.size
        ));
    }
    for model in &request.models {
        let readable = model
            .file
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| {
                ["glb", "gltf", "obj"]
                    .iter()
                    .any(|x| e.eq_ignore_ascii_case(x))
            });
        if !readable {
            return bad(format!(
                "{} is not a .glb, .gltf or .obj model, so its views cannot be rendered.",
                model.file.display()
            ));
        }
        if !model.file.is_file() {
            return Err(CoreError::NotFound(format!(
                "the model file {}",
                model.file.display()
            )));
        }
    }
    Ok(())
}

/// Writes the scripts and the spec of a run into its scratch folder.
fn stage(request: &ViewsRequest) -> CoreResult<()> {
    let work = &request.work;
    fs::create_dir_all(work).map_err(|e| CoreError::io(work, e))?;
    let _ = fs::remove_dir_all(work.join(OUT_DIR));
    let _ = fs::remove_file(work.join(spec::REPORT_FILE));
    for (name, text) in [
        (SCRIPT_FILE, SCRIPT),
        (crate::thumbs::SCRIPT_FILE, crate::thumbs::SCRIPT),
        (crate::thumbs::RIG_FILE, crate::thumbs::RIG_SCRIPT),
    ] {
        let path = work.join(name);
        crate::fs::write_atomic(&path, text.as_bytes()).map_err(|e| CoreError::io(&path, e))?;
    }
    crate::json::write_json(&work.join(SPEC_FILE), &spec::spec_json(request, work))
}

/// The set in `dir` when it was made by `key` and every picture is there.
fn fresh(dir: &Path, key: &str) -> Option<ViewSet> {
    let set = stored(dir).filter(|s| s.key == key)?;
    spec::files_of(&set)
        .iter()
        .all(|f| f.is_file())
        .then_some(set)
}

/// The views service. Cheap to clone.
#[derive(Clone)]
pub struct Views {
    shared: Arc<Shared>,
    runner: PipelineRunner,
    size: u32,
}

impl Default for Views {
    fn default() -> Self {
        Self::sized(SIZE)
    }
}

impl Views {
    /// A service whose pictures are `size` pixels square: [`SIZE`] in the
    /// app, something small where a test has to stay quick.
    pub fn sized(size: u32) -> Self {
        let shared = Arc::new(Shared::default());
        let runner = PipelineRunner::new(Arc::new(Sink(Arc::clone(&shared))));
        Self {
            shared,
            runner,
            size,
        }
    }

    /// The edge of the pictures this service is asked for.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// The set of `request`: the one in its folder when nothing it is made
    /// from changed, else a render started (or joined when one for the same
    /// folder is on its way). A run that failed for this very request says
    /// so once; asking again after that tries again.
    ///
    /// # Errors
    ///
    /// Fails for a request that cannot be rendered, without Blender, and
    /// with what Blender said when the last run of this request failed.
    pub fn request(&self, request: &ViewsRequest, tools: &Toolbox) -> CoreResult<Asked> {
        check(request)?;
        let key = spec::key_of(request)?;
        if let Some(set) = fresh(&request.dir, &key) {
            return Ok(Asked::Ready(Box::new(set)));
        }
        let mut pending = self.shared.pending();
        if pending.contains_key(&request.dir) {
            return Ok(Asked::Rendering);
        }
        self.take_failure(&request.dir, &key)?;
        if !tools.has(ToolId::Blender) {
            return Err(CoreError::Unavailable(
                "Blender was not found. The views are rendered by headless Blender. Set its path in Settings, Toolchain, then render again."
                    .to_string(),
            ));
        }
        stage(request)?;
        pending.insert(
            request.dir.clone(),
            Pending {
                run_id: None,
                key,
                request: request.clone(),
                cancelled: false,
            },
        );
        let options = RunOptions {
            timeout: Some(TIMEOUT),
            ..RunOptions::default()
        };
        match self.runner.submit_with(setup(request, tools), options) {
            Ok(record) => {
                if let Some(entry) = pending.get_mut(&request.dir) {
                    entry.run_id = Some(record.id);
                }
                Ok(Asked::Rendering)
            }
            Err(err) => {
                pending.remove(&request.dir);
                Err(err)
            }
        }
    }

    /// Says why the last run for `dir` failed, once, when it was made by `key`.
    fn take_failure(&self, dir: &Path, key: &str) -> CoreResult<()> {
        match self.shared.failed().remove(dir) {
            Some((failed, message)) if failed == key => Err(CoreError::Invalid(message)),
            _ => Ok(()),
        }
    }

    /// How the last run of `request` ended, without starting another: its
    /// set, or `None` when it was stopped before it finished.
    ///
    /// # Errors
    ///
    /// Fails with what Blender said when the run failed.
    pub fn outcome(&self, request: &ViewsRequest) -> CoreResult<Option<ViewSet>> {
        let key = spec::key_of(request)?;
        if let Some(set) = fresh(&request.dir, &key) {
            return Ok(Some(set));
        }
        self.take_failure(&request.dir, &key)?;
        Ok(None)
    }

    /// True while a set for `dir` is queued or rendering.
    pub fn is_running(&self, dir: &Path) -> bool {
        self.shared.pending().contains_key(dir)
    }

    /// Stops the render of the set for `dir`. False when none is running.
    pub fn cancel(&self, dir: &Path) -> bool {
        let run = {
            let mut pending = self.shared.pending();
            let Some(entry) = pending.get_mut(dir) else {
                return false;
            };
            entry.cancelled = true;
            entry.run_id.clone()
        };
        if let Some(id) = run
            && let Err(err) = self.runner.cancel(&id)
        {
            tracing::info!(error = %err, "the views run was already over");
        }
        true
    }

    /// Blocks until no set is queued or rendering. For tests and the CLI.
    pub fn wait(&self) {
        while !self.shared.pending().is_empty() {
            std::thread::sleep(Duration::from_millis(40));
        }
    }
}
