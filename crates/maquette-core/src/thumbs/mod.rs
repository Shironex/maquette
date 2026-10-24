//! Model thumbnails: a square studio render of a `.glb`, `.gltf` or
//! `.blend`, made by a headless Blender job and cached by the file's
//! content hash as `<workspace cache>/thumbs/<hash>.png`.
//!
//! Renders go through the same [`PipelineRunner`] as the pipeline, on a
//! lane of their own, so they queue one at a time and never block a
//! command. When Blender is missing, or a render fails (Blender cannot read
//! KTX2 textures, for example), the item is marked [`ThumbState::Fallback`]
//! and the web view stores a snapshot from its own model preview instead.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

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

/// A render that finished, with everyone who asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub file: PathBuf,
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
            def: render_def(file, &out),
            tools: tools.clone(),
            defaults: CompressDefaults {
                texture_size: 2048,
                codec: TextureCodec::Etc1s,
                meshopt: MeshoptLevel::Medium,
            },
            vars: Default::default(),
            from: None,
            force: true,
        };
        pending.insert(
            png.clone(),
            Pending {
                run_id: None,
                png: png.clone(),
                file: file.to_path_buf(),
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
mod tests {
    use super::*;
    use crate::pipeline::graph::blender_headless;
    use crate::pipeline::validate;

    #[test]
    fn render_jobs_are_headless_and_put_a_blend_before_the_script() {
        let glb = render_def(Path::new("/m/crate.glb"), "a.png");
        let args = &glb.steps[0].args;
        assert_eq!(args[0], "-b");
        assert!(blender_headless(args));
        assert!(!args.iter().any(|a| a.ends_with(".blend")));
        assert!(validate(&glb).iter().all(|i| !i.is_error()));

        let blend = render_def(Path::new("/m/scene.blend"), "b.png");
        let args = &blend.steps[0].args;
        assert!(blender_headless(args));
        let open = args
            .iter()
            .position(|a| a == "/m/scene.blend")
            .expect("blend");
        let script = args.iter().position(|a| a == "--python").expect("script");
        assert!(open < script);
        assert_eq!(blend.steps[0].outputs, vec!["b.png".to_string()]);
    }

    #[test]
    fn the_key_follows_the_content_not_the_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.glb");
        let b = dir.path().join("b.glb");
        fs::write(&a, b"same").expect("a");
        fs::write(&b, b"same").expect("b");
        let thumbs = dir.path().join("thumbs");
        fs::create_dir_all(&thumbs).expect("thumbs");
        assert_eq!(
            png_for(&thumbs, &a).expect("a"),
            png_for(&thumbs, &b).expect("b")
        );
        fs::write(&b, b"other").expect("b2");
        assert_ne!(
            png_for(&thumbs, &a).expect("a"),
            png_for(&thumbs, &b).expect("b")
        );
    }

    #[test]
    fn store_takes_only_pngs_for_model_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let model = dir.path().join("crate.glb");
        fs::write(&model, b"glb").expect("model");
        let thumbs = dir.path().join("thumbs");
        let mut png = PNG_MAGIC.to_vec();
        png.extend_from_slice(b"rest");
        let saved = store(&thumbs, &model, &png).expect("store");
        assert_eq!(fs::read(&saved).expect("read"), png);
        assert_eq!(
            store(&thumbs, &model, b"GIF89a").expect_err("gif").code(),
            "invalid"
        );
        assert_eq!(
            store(&thumbs, &dir.path().join("notes.txt"), &png)
                .expect_err("txt")
                .code(),
            "invalid"
        );
    }

    #[test]
    fn without_blender_the_preview_stands_in_and_cached_pngs_are_ready() {
        let dir = tempfile::tempdir().expect("tempdir");
        let model = dir.path().join("crate.glb");
        fs::write(&model, b"glb").expect("model");
        let thumbs_dir = dir.path().join("thumbs");
        let service = Thumbnails::new(|_| {});
        let tools = Toolbox::default();
        let first = service
            .request(&thumbs_dir, &model, None, &tools)
            .expect("request");
        assert_eq!(first.state(), ThumbState::Fallback);
        let mut png = PNG_MAGIC.to_vec();
        png.push(0);
        let saved = store(&thumbs_dir, &model, &png).expect("store");
        assert_eq!(
            service
                .request(&thumbs_dir, &model, None, &tools)
                .expect("again"),
            Requested::Ready(saved)
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_render_reports_every_target_once_it_finishes() {
        use crate::pipeline::tools::ResolvedTool;
        use std::os::unix::fs::PermissionsExt;
        use std::sync::mpsc;

        let dir = tempfile::tempdir().expect("tempdir");
        // A stand-in for Blender: writes the file named after --out.
        let fake = dir.path().join("blender");
        fs::write(
            &fake,
            "#!/bin/sh\nwhile [ \"$1\" != \"--out\" ]; do shift; done\nprintf '\\211PNG' > \"$2\"\n",
        )
        .expect("fake");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("chmod");
        let mut tools = Toolbox {
            path_env: std::env::var_os("PATH").unwrap_or_default(),
            os: Some(crate::toolchain::HostOs::current()),
            ..Toolbox::default()
        };
        tools.tools.insert(
            ToolId::Blender,
            ResolvedTool {
                program: fake,
                prefix: Vec::new(),
                version: "test".into(),
            },
        );
        let model = dir.path().join("crate.glb");
        fs::write(&model, b"glb").expect("model");
        let thumbs_dir = dir.path().join("thumbs");
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let service = Thumbnails::new(move |done| {
            let _ = tx.lock().expect("tx").send(done);
        });
        let target = |version| ThumbTarget {
            project: "moon-island".into(),
            asset: "crate".into(),
            version,
        };
        let first = service
            .request(&thumbs_dir, &model, Some(target(1)), &tools)
            .expect("first");
        assert_eq!(first, Requested::Rendering);
        let joined = service
            .request(&thumbs_dir, &model, Some(target(2)), &tools)
            .expect("joined");
        assert_eq!(joined, Requested::Rendering);
        let done = rx
            .recv_timeout(std::time::Duration::from_secs(20))
            .expect("finished");
        service.wait();
        assert_eq!(done.error, None);
        let png = done.png.expect("png");
        assert!(png.is_file());
        assert_eq!(done.targets, vec![target(1), target(2)]);
        assert!(thumbs_dir.join(SCRIPT_FILE).is_file());
        assert_eq!(
            service
                .request(&thumbs_dir, &model, None, &tools)
                .expect("cached"),
            Requested::Ready(png)
        );
    }
}
