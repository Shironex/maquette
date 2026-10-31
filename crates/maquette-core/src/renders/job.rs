//! The render service: runs each render on a thread of its own, one file at
//! a time through one runner lane, so renders queue behind each other beside
//! the pipeline. Keeps the running records in memory, written to disk after
//! every change, and tells the app through a callback. A person may stop
//! after the file running now and go on later from the next one.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::plan::{self, RenderPlan};
use super::preview::{self, RenderPreview, RenderShot};
use super::record::{
    self, ProblemKind, RECORD_VERSION, RenderFile, RenderFileState, RenderItem, RenderProblem,
    RenderRecord, RenderState, file_in,
};
use super::run::{self, FileRun};
use super::{LoopFormat, RenderRequest, RenderSettings, RenderStyle};
use crate::error::{CoreError, CoreResult};
use crate::packs::PackSource;
use crate::pipeline::runner::PipelineRunner;
use crate::pipeline::tools::Toolbox;
use crate::project::{self, now_rfc3339};
use crate::toolchain::ToolId;
use crate::workspace::WorkspaceConfig;

/// What the app hears: a render's record after every change.
pub type RenderEvent = RenderRecord;

struct Active {
    record: RenderRecord,
    stop: bool,
}

struct Inner {
    runner: PipelineRunner,
    on_change: Box<dyn Fn(&RenderRecord) + Send + Sync>,
    active: Mutex<HashMap<String, Active>>,
}

impl Inner {
    fn active(&self) -> MutexGuard<'_, HashMap<String, Active>> {
        self.active.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn tell(&self, root: &Path, record: &RenderRecord) {
        if let Err(err) = record::save(root, record) {
            tracing::warn!(error = %err, render = %record.id, "could not save a render record");
        }
        (self.on_change)(record);
    }
}

/// The render service. Cheap to clone.
#[derive(Clone)]
pub struct Renders {
    inner: Arc<Inner>,
}

fn extension_of(style: RenderStyle, settings: &RenderSettings) -> &'static str {
    match style {
        RenderStyle::Turntable => settings.format.extension(),
        _ => "png",
    }
}

/// A file's problem before anything runs, so a batch can say so and carry on.
fn left_out(style: RenderStyle, target: &plan::RenderTarget) -> Option<RenderProblem> {
    if let Some(why) = &target.unreadable {
        return Some(RenderProblem {
            kind: ProblemKind::Unreadable,
            message: why.clone(),
            webp_bytes: None,
        });
    }
    (style == RenderStyle::Toon && !target.has_colour).then(|| RenderProblem {
        kind: ProblemKind::NoTexture,
        message: format!(
            "{} has no texture, so toon would come out flat grey. Clay and blueprint still work.",
            target.name
        ),
        webp_bytes: None,
    })
}

/// The record of a new render of `plan`.
fn new_record(
    request: &RenderRequest,
    planned: RenderPlan,
    id: String,
    source_slug: &str,
) -> CoreResult<RenderRecord> {
    let dir = PathBuf::from(&planned.dir);
    let mut items = Vec::new();
    for row in &request.rows {
        let target = plan::target(&planned, &row.key)?.clone();
        let folder = dir.join(&target.folder);
        let mut files: Vec<RenderFile> = Vec::new();
        for &style in &row.styles {
            let extension = extension_of(style, &request.settings);
            let mut file = RenderFile::new(
                style,
                &file_in(&folder, style.stem(), extension, request.replace),
            );
            if let Some(problem) = left_out(style, &target) {
                file.state = RenderFileState::Skipped;
                file.problem = Some(problem);
            }
            files.push(file);
        }
        items.push(RenderItem { target, files });
    }
    if items.iter().all(|i| i.files.is_empty()) && !request.sheet {
        return Err(CoreError::Invalid(
            "Pick at least one style to render.".to_string(),
        ));
    }
    let sheet = request.sheet.then(|| {
        let name = format!("{source_slug}_contact");
        RenderFile::new(
            RenderStyle::ContactSheet,
            &file_in(&dir, &name, "png", request.replace),
        )
    });
    Ok(RenderRecord {
        version: RECORD_VERSION,
        id,
        source: request.source.clone(),
        label: planned.label,
        dir: planned.dir,
        settings: request.settings.clone(),
        state: RenderState::Running,
        items,
        sheet,
        created_at: now_rfc3339(),
        finished_at: None,
        message: None,
    })
}

fn source_slug(source: &PackSource) -> String {
    match source {
        PackSource::Project { project } => project::slugify(project),
        _ => "models".to_string(),
    }
}

enum Next {
    File(usize, usize, Box<RenderFile>),
    Sheet,
    Stop,
    Finish,
}

fn next_of(a: &mut Active) -> Next {
    if a.stop {
        return Next::Stop;
    }
    for (i, item) in a.record.items.iter_mut().enumerate() {
        for (f, file) in item.files.iter_mut().enumerate() {
            if file.state == RenderFileState::Queued {
                file.state = RenderFileState::Running;
                return Next::File(i, f, Box::new(file.clone()));
            }
        }
    }
    match &mut a.record.sheet {
        Some(sheet) if sheet.state == RenderFileState::Queued => {
            sheet.state = RenderFileState::Running;
            Next::Sheet
        }
        _ => Next::Finish,
    }
}

/// The pictures of a batch's sheet: each model's tile in the settings' look.
fn sheet_tiles(
    inner: &Inner,
    tools: &Toolbox,
    work: &Path,
    record: &RenderRecord,
) -> Vec<(PathBuf, String)> {
    let run = FileRun {
        runner: &inner.runner,
        tools,
        settings: &record.settings,
        work,
    };
    let tiles_dir = work.with_file_name("sheet-tiles");
    let mut tiles = Vec::new();
    for item in &record.items {
        let out = tiles_dir.join(format!("{}.png", item.target.folder));
        if item.target.unreadable.is_some() {
            continue;
        }
        match run::sheet_tile(&run, &item.target, &out) {
            Ok(true) if out.is_file() => tiles.push((out, item.target.name.clone())),
            Ok(_) => {}
            Err(err) => tracing::info!(error = %err, "a contact sheet tile failed"),
        }
    }
    tiles
}

fn run_render(inner: &Inner, root: &Path, tools: &Toolbox, id: &str) {
    let scratch = record::work_dir(root, id);
    loop {
        let (next, record) = {
            let mut active = inner.active();
            let Some(a) = active.get_mut(id) else {
                return;
            };
            let next = next_of(a);
            (next, a.record.clone())
        };
        let mut record = record;
        match next {
            Next::File(i, f, file) => {
                inner.tell(root, &record);
                let work = scratch.join("work");
                let run = FileRun {
                    runner: &inner.runner,
                    tools,
                    settings: &record.settings,
                    work: &work,
                };
                let done = run::run_file(&run, &record.items[i].target, &file);
                let snapshot = {
                    let mut active = inner.active();
                    let Some(a) = active.get_mut(id) else {
                        return;
                    };
                    a.record.items[i].files[f] = done;
                    a.record.clone()
                };
                inner.tell(root, &snapshot);
                continue;
            }
            Next::Sheet => {
                inner.tell(root, &record);
                let work = scratch.join("work");
                let tiles = sheet_tiles(inner, tools, &work, &record);
                let run = FileRun {
                    runner: &inner.runner,
                    tools,
                    settings: &record.settings,
                    work: &work,
                };
                let queued = record.sheet.clone();
                if let Some(queued) = queued {
                    let done = run::batch_sheet(&run, &tiles, &queued);
                    let snapshot = {
                        let mut active = inner.active();
                        let Some(a) = active.get_mut(id) else {
                            return;
                        };
                        a.record.sheet = Some(done);
                        a.record.clone()
                    };
                    inner.tell(root, &snapshot);
                }
                continue;
            }
            Next::Stop => {
                record.state = RenderState::Stopped;
                record.message = Some(format!(
                    "Stopped at {} of {} files. Go on picks up from the next one.",
                    record.done(),
                    record.total()
                ));
            }
            Next::Finish => {
                record.state = RenderState::Done;
                record.finished_at = Some(now_rfc3339());
                let _ = fs::remove_dir_all(&scratch);
            }
        }
        // Only now, with the last state saved, does the render stop running.
        inner.tell(root, &record);
        inner.active().remove(id);
        return;
    }
}

impl Renders {
    pub fn new(
        runner: PipelineRunner,
        on_change: impl Fn(&RenderRecord) + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                runner,
                on_change: Box::new(on_change),
                active: Mutex::new(HashMap::new()),
            }),
        }
    }

    fn launch(&self, root: &Path, tools: Toolbox, mut record: RenderRecord) {
        record.state = RenderState::Running;
        record.message = None;
        self.inner.tell(root, &record);
        self.inner.active().insert(
            record.id.clone(),
            Active {
                record: record.clone(),
                stop: false,
            },
        );
        let inner = Arc::clone(&self.inner);
        let root = root.to_path_buf();
        std::thread::spawn(move || run_render(&inner, &root, &tools, &record.id));
    }

    /// Starts a render of `plan`'s models; answers its record at once.
    ///
    /// # Errors
    ///
    /// Fails without Blender, with GIF or MP4 asked for and no ffmpeg, for
    /// bad settings and for a model the plan does not hold.
    pub fn start(
        &self,
        root: &Path,
        tools: Toolbox,
        request: &RenderRequest,
        planned: RenderPlan,
    ) -> CoreResult<RenderRecord> {
        request.settings.check()?;
        if !tools.has(ToolId::Blender) {
            return Err(CoreError::Unavailable(
                "Blender was not found. Render styles use headless Blender. Set its path in Settings, Toolchain, then render again."
                    .to_string(),
            ));
        }
        let looping = request
            .rows
            .iter()
            .any(|r| r.styles.contains(&RenderStyle::Turntable));
        if looping && request.settings.format != LoopFormat::Webp && !tools.has(ToolId::Ffmpeg) {
            return Err(CoreError::Unavailable(format!(
                "A {} loop needs ffmpeg, and it was not found. Set its path in Settings, Toolchain, or save a WebP loop, which needs only Blender.",
                request.settings.format.extension().to_uppercase()
            )));
        }
        let id = record::new_id();
        let slug = source_slug(&request.source);
        let record = new_record(request, planned, id, &slug)?;
        self.launch(root, tools, record.clone());
        Ok(record)
    }

    /// A render's record: the running one's from memory, else from disk.
    ///
    /// # Errors
    ///
    /// Fails for an unknown render.
    pub fn get(&self, root: &Path, id: &str) -> CoreResult<RenderRecord> {
        if let Some(active) = self.inner.active().get(id) {
            return Ok(active.record.clone());
        }
        record::load(root, id)
    }

    /// The newest renders.
    pub fn recent(&self, root: &Path) -> Vec<RenderRecord> {
        let active = self.inner.active();
        record::recent(root)
            .into_iter()
            .map(|r| match active.get(&r.id) {
                Some(a) => a.record.clone(),
                None => r,
            })
            .collect()
    }

    /// Stops a render once the file running now is done.
    ///
    /// # Errors
    ///
    /// Fails for a render that is not running.
    pub fn stop(&self, id: &str) -> CoreResult<RenderRecord> {
        let mut active = self.inner.active();
        let Some(a) = active.get_mut(id) else {
            return Err(CoreError::Invalid(
                "That render is not running.".to_string(),
            ));
        };
        a.stop = true;
        Ok(a.record.clone())
    }

    /// Goes on with a stopped render from the next file.
    ///
    /// # Errors
    ///
    /// Fails without Blender, and for a render that is running or done.
    pub fn resume(&self, root: &Path, tools: Toolbox, id: &str) -> CoreResult<RenderRecord> {
        if self.inner.active().contains_key(id) {
            return Err(CoreError::Invalid(
                "That render is still running.".to_string(),
            ));
        }
        if !tools.has(ToolId::Blender) {
            return Err(CoreError::Unavailable(
                "Blender was not found. Set its path in Settings, Toolchain, then go on."
                    .to_string(),
            ));
        }
        let record = record::load(root, id)?;
        if record.state != RenderState::Stopped {
            return Err(CoreError::Invalid(
                "Only a stopped render can go on.".to_string(),
            ));
        }
        self.launch(root, tools, record.clone());
        Ok(record)
    }

    /// The small pictures of `key` the Render tab shows, cached by their
    /// settings. Blocks while Blender runs.
    ///
    /// # Errors
    ///
    /// Fails without Blender, for bad settings and when Blender fails.
    pub fn preview(
        &self,
        root: &Path,
        config: &WorkspaceConfig,
        tools: &Toolbox,
        source: &PackSource,
        key: &str,
        settings: &RenderSettings,
        shots: &[RenderShot],
    ) -> CoreResult<RenderPreview> {
        preview::make(
            &self.inner.runner,
            root,
            config,
            tools,
            source,
            key,
            settings,
            shots,
        )
    }

    /// Blocks until no render runs. For tests and the CLI.
    pub fn wait(&self) {
        while !self.inner.active().is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
    }
}
