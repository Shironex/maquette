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
