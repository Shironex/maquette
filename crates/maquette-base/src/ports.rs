//! Ports: the Blender work a layer above the pipeline asks for without
//! depending on the pipeline. The pipeline implements them (its runner runs
//! the script, its thumbnail script draws the picture, its bake script
//! bakes) and the app hands the implementation in where it wires things up.
//! Each port is one call, and an implementation does exactly what the code
//! behind it did before the port existed.

use std::path::{Path, PathBuf};

use crate::error::CoreResult;
use crate::tools::Toolbox;

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
}

/// Renders a model from the front on a plain ground.
pub trait PictureRender: Send + Sync {
    /// The front picture of `model` as `<dir>/<key>.png`, rendered unless it
    /// is there already.
    ///
    /// # Errors
    ///
    /// Fails without Blender, or when the render does.
    fn render(&self, model: &Path, dir: &Path, key: &str) -> CoreResult<PathBuf>;
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
