//! A download's front picture for the Downloads watcher: a model rendered
//! headless from the front on a plain ground, one at a time on a lane of
//! its own. The watcher reaches it through the picture port, which this
//! renderer implements.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use super::{SCRIPT, SCRIPT_FILE, render_def_with};
use crate::base::ports::PictureRender;
use crate::error::{CoreError, CoreResult};
use crate::pipeline::CompressDefaults;
use crate::pipeline::history::RunState;
use crate::pipeline::runner::{PipelineRunner, RunOptions, RunSetup};
use crate::pipeline::tools::Toolbox;
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec};

/// The runner lane every download picture shares.
const LANE: &str = "download-pictures";
const TIMEOUT: Duration = Duration::from_secs(5 * 60);

type ToolsFn = Arc<dyn Fn() -> Toolbox + Send + Sync>;

/// Renders a model from the front on a plain ground with headless Blender.
#[derive(Clone)]
pub struct PictureRenderer {
    runner: PipelineRunner,
    tools: ToolsFn,
}

impl PictureRenderer {
    /// `tools` is asked for the toolbox at each render, so a Blender found
    /// later is used.
    pub fn new(
        runner: PipelineRunner,
        tools: impl Fn() -> Toolbox + Send + Sync + 'static,
    ) -> Self {
        Self {
            runner,
            tools: Arc::new(tools),
        }
    }
}

impl PictureRender for PictureRenderer {
    fn render(&self, model: &Path, dir: &Path, key: &str) -> CoreResult<PathBuf> {
        let out = format!("{key}.png");
        let png = dir.join(&out);
        if png.is_file() {
            return Ok(png);
        }
        let tools = (self.tools)();
        if !tools.has(ToolId::Blender) {
            return Err(CoreError::Unavailable(
                "Blender is not installed, so the download is matched by name only.".to_string(),
            ));
        }
        std::fs::create_dir_all(dir).map_err(|e| CoreError::io(dir, e))?;
        let script = dir.join(SCRIPT_FILE);
        crate::fs::write_atomic(&script, SCRIPT.as_bytes())
            .map_err(|e| CoreError::io(&script, e))?;
        let record = self.runner.run_blocking_with(
            RunSetup {
                project: LANE.to_string(),
                project_dir: dir.to_path_buf(),
                cache_dir: dir.join("runs"),
                def: render_def_with(model, &out, &["--view", "front", "--plain"]),
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
            RunOptions {
                timeout: Some(TIMEOUT),
                ..RunOptions::default()
            },
        )?;
        if record.state != RunState::Done || !png.is_file() {
            return Err(CoreError::Invalid(
                "The download's picture could not be rendered.".to_string(),
            ));
        }
        Ok(png)
    }
}
