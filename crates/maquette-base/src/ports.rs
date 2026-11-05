//! Ports: the Blender work a layer above the pipeline asks for without
//! depending on the pipeline. The pipeline implements them (its runner runs
//! the script, its thumbnail script draws the picture, its bake script
//! bakes) and the app hands the implementation in where it wires things up.
//! Each port is one call, and an implementation does exactly what the code
//! behind it did before the port existed.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::error::CoreResult;
use crate::tools::Toolbox;

/// What a watched run checks while it waits: whether to stop, and what to
/// call now and then (to read a progress file the script writes, say).
pub struct Watch<'a> {
    /// Turned true, the run is stopped: Blender is killed where it can be.
    pub cancel: &'a AtomicBool,
    /// Called every few hundred milliseconds while the run goes.
    pub tick: &'a mut dyn FnMut(),
}

/// Runs a Blender script headless and waits for it.
pub trait ScriptRunner {
    /// Writes `script` (its file name and text) into `work` and runs it
    /// headless with `extra` args after `--`.
    ///
    /// # Errors
    ///
    /// Fails without Blender, or with the reason the run gave when it failed.
    fn run_script(
        &self,
        tools: &Toolbox,
        work: &Path,
        script: (&str, &str),
        extra: &[String],
    ) -> CoreResult<()>;

    /// As [`ScriptRunner::run_script`], with `name` for the step the run
    /// shows and fails as (`Springs chain`, `Reweight`), so a failure names
    /// what ran. An implementation that shows no steps ignores it.
    ///
    /// # Errors
    ///
    /// As [`ScriptRunner::run_script`].
    fn run_step(
        &self,
        tools: &Toolbox,
        work: &Path,
        name: &str,
        script: (&str, &str),
        extra: &[String],
    ) -> CoreResult<()> {
        let _ = name;
        self.run_script(tools, work, script, extra)
    }

    /// As [`ScriptRunner::run_step`], watched: `watch.cancel` stops the run
    /// part way (it then fails with [`CANCELLED`]) and `watch.tick` is called
    /// while it goes. An implementation that cannot stop a running script
    /// runs it to its end, checking `cancel` only before it starts.
    ///
    /// # Errors
    ///
    /// As [`ScriptRunner::run_script`], and [`CANCELLED`] once stopped.
    fn run_watched(
        &self,
        tools: &Toolbox,
        work: &Path,
        (name, script): (&str, (&str, &str)),
        extra: &[String],
        watch: &mut Watch<'_>,
    ) -> CoreResult<()> {
        if watch.cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(crate::error::CoreError::Invalid(CANCELLED.to_string()));
        }
        (watch.tick)();
        self.run_step(tools, work, name, script, extra)
    }
}

/// What a watched run that was stopped fails with.
pub const CANCELLED: &str = "The run was cancelled.";

/// Renders a model from the front on a plain ground.
pub trait PictureRender: Send + Sync {
    /// The front picture of `model` as `<dir>/<key>.png`, rendered unless it
    /// is there already.
    ///
    /// # Errors
    ///
    /// Fails without Blender, or when the render does.
    fn render(&self, model: &Path, dir: &Path, key: &str) -> CoreResult<PathBuf>;

    /// False while the tool it renders with is missing: the downloads wait
    /// for it instead of failing.
    fn ready(&self) -> bool {
        true
    }
}

/// Bakes the texture of a model back onto the parts it was split into.
pub trait TextureBake: Send + Sync {
    /// Bakes `source`'s texture onto the parts of `target` inside `dir` and
    /// returns the baked model. Blocks until Blender is done.
    ///
    /// # Errors
    ///
    /// Fails without Blender or when the bake fails.
    fn bake(
        &self,
        dir: &Path,
        source: &Path,
        target: &Path,
        tools: &Toolbox,
    ) -> CoreResult<PathBuf>;
}
