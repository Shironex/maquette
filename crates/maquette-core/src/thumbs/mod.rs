//! Model thumbnails: a square studio render of a `.glb`, `.gltf` or
//! `.blend`, made by a headless Blender job and cached by the file's
//! content hash as `<workspace cache>/thumbs/<hash>.png`.
//!
//! Renders go through the same [`PipelineRunner`] as the pipeline, on a
//! lane of their own, so they queue one at a time and never block a
//! command. Blender cannot read KTX2 textures or meshopt geometry, so a
//! compressed model renders from its uncompressed export when there is one
//! ([`source::pick`] has the order). When Blender is missing, nothing it can
//! read exists, or a render fails, the item is marked
//! [`ThumbState::Fallback`] and the web view stores a snapshot from its own
//! model preview instead.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

pub mod source;

use crate::error::{CoreError, CoreResult};
use crate::pipeline::cache::FileHasher;
use crate::pipeline::history::RunState;
use crate::pipeline::runner::{JobRunner, PipelineEvent, PipelineRunner, RunSetup, RunSink};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec, WorkspaceConfig};

/// The edge of every thumbnail, in pixels.
pub const THUMB_SIZE: u32 = 256;
/// The Blender script, written next to the renders before a job runs.
pub const SCRIPT: &str = include_str!("render_thumb.py");
pub const SCRIPT_FILE: &str = "render_thumb.py";
/// The runner lane every render shares.
pub const LANE: &str = "thumbnails";
/// The most a stored snapshot may weigh.
pub const MAX_PNG_BYTES: usize = 4 * 1024 * 1024;
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Where a model's thumbnail stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ThumbState {
    /// The PNG exists.
    Ready,
    /// A Blender render is queued or running.
    Rendering,
    /// No render will come (Blender is missing or the render failed): the
    /// web view snapshots its own preview and stores that.
    Fallback,
}

/// A Board asset version that should show the thumbnail once it exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbTarget {
    pub project: String,
    pub asset: String,
    pub version: u32,
}

/// What [`Thumbnails::request`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Requested {
    Ready(PathBuf),
    Rendering,
    Fallback(String),
}

impl Requested {
    pub fn state(&self) -> ThumbState {
        match self {
            Requested::Ready(_) => ThumbState::Ready,
            Requested::Rendering => ThumbState::Rendering,
            Requested::Fallback(_) => ThumbState::Fallback,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Requested::Ready(path) => Some(path),
            _ => None,
        }
    }
}

/// A file's size and modified time as the inbox lists them. A render comes
/// back with the one its file had when it was asked for, so a late result
/// for an old file under the same name can be told from the new one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fingerprint {
    pub bytes: f64,
    pub modified_ms: f64,
}

impl Fingerprint {
    pub fn new(len: u64, modified_ns: u128) -> Self {
        #[expect(
            clippy::cast_precision_loss,
            reason = "sizes and epoch ms stay far below 2^53"
        )]
        Self {
            bytes: len as f64,
            modified_ms: (modified_ns / 1_000_000) as f64,
        }
    }

    /// None when the file cannot be read.
    pub fn of(path: &Path) -> Option<Self> {
        let (len, modified_ns) = crate::inbox::stamp(&fs::metadata(path).ok()?);
        Some(Self::new(len, modified_ns))
    }
}

/// A render that finished, with everyone who asked for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Finished {
    pub file: PathBuf,
    /// The file as it was when the render was asked for.
    pub fingerprint: Option<Fingerprint>,
    pub png: Option<PathBuf>,
    pub error: Option<String>,
    pub targets: Vec<ThumbTarget>,
}

pub fn thumbs_dir(root: &Path, config: &WorkspaceConfig) -> PathBuf {
    root.join(&config.layout.cache).join("thumbs")
}

/// True for the files a thumbnail can be made of.
pub fn is_renderable(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        ["glb", "gltf", "blend"]
            .iter()
            .any(|x| e.eq_ignore_ascii_case(x))
    })
}

/// `<dir>/<hash>.png` for the content of `file`. Hashes are memoised by
/// size and modified time in `<dir>/hashes.json`.
pub fn png_for(dir: &Path, file: &Path) -> CoreResult<PathBuf> {
    let mut hasher = FileHasher::with_memo(dir);
    let hash = hasher.hash(file).map_err(|e| CoreError::io(file, e))?;
    hasher.flush();
    Ok(dir.join(format!("{}.png", &hash[..32])))
}

/// Saves a PNG someone else made (the web view's preview snapshot) as the
/// thumbnail of `file`, and returns its path.
pub fn store(dir: &Path, file: &Path, png: &[u8]) -> CoreResult<PathBuf> {
    if !is_renderable(file) {
        return Err(CoreError::Invalid(format!(
            "{} is not a model file.",
            file.display()
        )));
    }
    if png.len() > MAX_PNG_BYTES || !png.starts_with(PNG_MAGIC) {
        return Err(CoreError::Invalid(
            "The thumbnail must be a PNG under 4 MB.".to_string(),
        ));
    }
    fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
    let target = png_for(dir, file)?;
    crate::fs::write_atomic(&target, png).map_err(|e| CoreError::io(&target, e))?;
    Ok(target)
}

/// The one step pipeline that renders `model` to `out` (a file name inside
/// the thumbs folder, which is the pipeline root).
pub fn render_def(model: &Path, out: &str) -> PipelineDef {
    let model_arg = model.display().to_string();
    let mut args: Vec<String> = ["-b", "--factory-startup", "--python-exit-code", "1"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let is_blend = model
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("blend"));
    // A .blend is opened by Blender itself; a model is imported by the script.
    if is_blend {
        args.push(model_arg.clone());
    }
    args.extend(
        [
            "--python",
            "{script}",
            "--",
            "--model",
            &model_arg,
            "--out",
            out,
            "--size",
            &THUMB_SIZE.to_string(),
        ]
        .iter()
        .map(|s| (*s).to_string()),
    );
    PipelineDef {
        version: PIPELINE_VERSION,
        root: None,
        steps: vec![StepDef {
            id: "render".into(),
            name: "Render thumbnail".into(),
            kind: StepKind::Blender,
            command: None,
            script: Some(SCRIPT_FILE.into()),
            args,
            inputs: Vec::new(),
            outputs: vec![out.to_string()],
            depends_on: Vec::new(),
            compress: None,
        }],
        presets: Vec::new(),
    }
}

struct Pending {
    run_id: Option<String>,
    png: PathBuf,
    file: PathBuf,
    fingerprint: Option<Fingerprint>,
    targets: Vec<ThumbTarget>,
}

type Callback = Box<dyn Fn(Finished) + Send + Sync>;

struct Shared {
    pending: Mutex<HashMap<PathBuf, Pending>>,
    on_done: Callback,
}

impl Shared {
    fn pending(&self) -> MutexGuard<'_, HashMap<PathBuf, Pending>> {
        self.pending.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// Hears when a render run ends and hands its result to the callback.
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
        let ok = run.state == RunState::Done && done.png.is_file();
        let error = (!ok).then(|| {
            run.steps
                .iter()
                .find_map(|s| s.message.clone())
                .or(run.message.clone())
                .unwrap_or_else(|| "The render did not finish.".to_string())
        });
        (self.0.on_done)(Finished {
            file: done.file,
            fingerprint: done.fingerprint,
            png: ok.then_some(done.png),
            error,
            targets: done.targets,
        });
    }
}

/// The thumbnail service. Cheap to clone.
#[derive(Clone)]
pub struct Thumbnails {
    shared: Arc<Shared>,
    runner: PipelineRunner,
}

impl Thumbnails {
    pub fn new(on_done: impl Fn(Finished) + Send + Sync + 'static) -> Self {
        let shared = Arc::new(Shared {
            pending: Mutex::new(HashMap::new()),
            on_done: Box::new(on_done),
        });
        let runner = PipelineRunner::new(Arc::new(Sink(Arc::clone(&shared))));
        Self { shared, runner }
    }

    /// The thumbnail of `file`: the cached PNG, or a render queued (joined
    /// when one for the same content is already on its way). `target`, when
    /// given, comes back in [`Finished`] so the caller can update the asset.
    pub fn request(
        &self,
        dir: &Path,
        file: &Path,
        target: Option<ThumbTarget>,
        tools: &Toolbox,
    ) -> CoreResult<Requested> {
        if !is_renderable(file) {
            return Ok(Requested::Fallback(format!(
                "{} is not a model file.",
                file.display()
            )));
        }
        fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
        let png = png_for(dir, file)?;
        if png.is_file() {
            return Ok(Requested::Ready(png));
        }
        let mut pending = self.shared.pending();
        if let Some(entry) = pending.get_mut(&png) {
            entry.targets.extend(target);
            return Ok(Requested::Rendering);
        }
        if !tools.has(ToolId::Blender) {
            return Ok(Requested::Fallback(
                "Blender is not installed, so the preview stands in.".to_string(),
            ));
        }
        let render_from = match source::pick(file) {
            source::Source::Original => file.to_path_buf(),
            source::Source::Uncompressed(raw) => raw,
            source::Source::Snapshot(reason) => return Ok(Requested::Fallback(reason)),
        };
        let script = dir.join(SCRIPT_FILE);
        if fs::read_to_string(&script).ok().as_deref() != Some(SCRIPT) {
            crate::fs::write_atomic(&script, SCRIPT.as_bytes())
                .map_err(|e| CoreError::io(&script, e))?;
        }
        let out = png
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let setup = RunSetup {
            project: LANE.to_string(),
            project_dir: dir.to_path_buf(),
            cache_dir: dir.join("runs"),
            def: render_def(&render_from, &out),
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
        };
        pending.insert(
            png.clone(),
            Pending {
                run_id: None,
                png: png.clone(),
                file: file.to_path_buf(),
                fingerprint: Fingerprint::of(file),
                targets: target.into_iter().collect(),
            },
        );
        match self.runner.submit(setup) {
            Ok(record) => {
                if let Some(entry) = pending.get_mut(&png) {
                    entry.run_id = Some(record.id);
                }
                Ok(Requested::Rendering)
            }
            Err(err) => {
                pending.remove(&png);
                Ok(Requested::Fallback(err.to_string()))
            }
        }
    }

    /// Blocks until no render is queued or running. For tests and the CLI.
    pub fn wait(&self) {
        while !self.shared.pending().is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
    }
}

/// Copies a finished render into the project as `thumbs/<asset>_v<n>.png`
/// and points the asset at it, when that version is still the latest.
/// Returns the project when it changed.
pub fn apply(
    root: &Path,
    config: &WorkspaceConfig,
    target: &ThumbTarget,
    png: &Path,
) -> CoreResult<Option<crate::ProjectManifest>> {
    let project_dir = crate::project::dir(root, &config.layout, &target.project)?;
    let manifest = crate::project::load(root, &config.layout, &target.project)?;
    let asset = crate::assets::find(&manifest, &target.asset)?;
    if asset.latest().map(|v| v.version) != Some(target.version) {
        return Ok(None);
    }
    let relative = format!(
        "thumbs/{}_v{}.png",
        target.asset.replace('-', "_"),
        target.version
    );
    let dest = project_dir.join(&relative);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    fs::copy(png, &dest).map_err(|e| CoreError::io(&dest, e))?;
    let mut changed = false;
    let manifest = crate::project::modify(root, &config.layout, &target.project, |m| {
        changed = crate::assets::set_thumbnail(m, &target.asset, target.version, relative)?;
        Ok(())
    })?;
    Ok(changed.then_some(manifest))
}

#[cfg(test)]
mod tests;
