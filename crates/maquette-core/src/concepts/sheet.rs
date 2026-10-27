//! Contact sheets: every image of a direction tiled into one PNG with
//! ffmpeg, saved next to the concepts as `contact_sheet.png`.

use std::path::Path;
use std::sync::Arc;

use super::{Aspect, Direction, direction_dir};
use crate::error::{CoreError, CoreResult};
use crate::pipeline::history::RunState;
use crate::pipeline::runner::{NullSink, PipelineRunner, RunSetup};
use crate::pipeline::tools::Toolbox;
use crate::pipeline::{CompressDefaults, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::toolchain::ToolId;
use crate::workspace::{MeshoptLevel, TextureCodec};

pub const SHEET_FILE: &str = "contact_sheet.png";
const CELL_WIDTH: u32 = 640;
const GAP: u32 = 12;
/// The app's dark canvas colour, so a sheet reads like the Lab grid.
const BACKGROUND: &str = "0x1b1916";

fn cell(aspect: Aspect) -> (u32, u32) {
    let (w, h) = match aspect {
        Aspect::Wide => (16, 9),
        Aspect::Landscape => (3, 2),
        Aspect::Square => (1, 1),
        Aspect::Portrait => (3, 4),
        Aspect::Tall => (9, 16),
    };
    // Even sizes, as some encoders want.
    let height = (CELL_WIDTH * h / w) & !1;
    (CELL_WIDTH, height)
}

/// Columns for `n` images: up to four, as square a grid as fits.
pub fn columns(n: usize) -> usize {
    match n {
        0 | 1 => 1,
        2..=4 => 2,
        5..=9 => 3,
        _ => 4,
    }
}

/// The ffmpeg arguments that tile `files` (names inside the direction
/// folder) into [`SHEET_FILE`].
pub fn args(files: &[String], aspect: Aspect) -> Vec<String> {
    let (w, h) = cell(aspect);
    let cols = columns(files.len());
    let rows = files.len().div_ceil(cols).max(1);
    let mut args: Vec<String> = vec!["-y".into(), "-loglevel".into(), "error".into()];
    for file in files {
        args.push("-i".into());
        args.push(file.clone());
    }
    let fit = format!(
        "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color={BACKGROUND},setsar=1,format=rgb24"
    );
    let mut graph = String::new();
    for i in 0..files.len() {
        graph.push_str(&format!("[{i}:v]{fit}[v{i}];"));
    }
    for i in 0..files.len() {
        graph.push_str(&format!("[v{i}]"));
    }
    graph.push_str(&format!(
        "concat=n={}:v=1:a=0,tile={cols}x{rows}:padding={GAP}:margin={GAP}:color={BACKGROUND}[out]",
        files.len()
    ));
    args.extend(
        [
            "-filter_complex",
            &graph,
            "-map",
            "[out]",
            "-frames:v",
            "1",
            "-update",
            "1",
            SHEET_FILE,
        ]
        .iter()
        .map(|s| (*s).to_string()),
    );
    args
}

/// Renders the sheet of `direction` and returns its path relative to the
/// project folder. `cache_dir` keeps the run history.
pub fn render(
    project_dir: &Path,
    direction: &Direction,
    cache_dir: &Path,
    tools: &Toolbox,
) -> CoreResult<String> {
    if !tools.has(ToolId::Ffmpeg) {
        return Err(CoreError::Unavailable(
            "ffmpeg is not installed, so there is no contact sheet. Install it, then check the tools in Settings.".into(),
        ));
    }
    let dir = direction_dir(project_dir, &direction.id)?;
    let files: Vec<String> = direction
        .images
        .iter()
        .filter_map(|i| Path::new(&i.file).file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| dir.join(n).is_file())
        .collect();
    if files.is_empty() {
        return Err(CoreError::Invalid(
            "This direction has no images yet.".into(),
        ));
    }
    let def = PipelineDef {
        version: PIPELINE_VERSION,
        root: None,
        steps: vec![StepDef {
            id: "sheet".into(),
            name: "Contact sheet".into(),
            kind: StepKind::Ffmpeg,
            command: None,
            script: None,
            args: args(&files, direction.aspect),
            inputs: Vec::new(),
            outputs: vec![SHEET_FILE.into()],
            depends_on: Vec::new(),
            compress: None,
        }],
        presets: Vec::new(),
    };
    let runner = PipelineRunner::new(Arc::new(NullSink));
    let run = runner.run_blocking(RunSetup {
        project: "contact-sheet".into(),
        project_dir: dir.clone(),
        cache_dir: cache_dir.to_path_buf(),
        def,
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
    })?;
    if run.state != RunState::Done {
        let why = run
            .steps
            .iter()
            .find_map(|s| s.message.clone())
            .or(run.message)
            .unwrap_or_else(|| "ffmpeg did not finish.".into());
        return Err(CoreError::Unavailable(format!(
            "The contact sheet failed. {why}"
        )));
    }
    Ok(format!(
        "{}/{}/{SHEET_FILE}",
        super::CONCEPTS_DIR,
        direction.id
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_stays_close_to_square() {
        assert_eq!(columns(1), 1);
        assert_eq!(columns(3), 2);
        assert_eq!(columns(6), 3);
        assert_eq!(columns(12), 4);
        let a = args(
            &["a.png".into(), "b.png".into(), "c.png".into()],
            Aspect::Wide,
        );
        let graph = &a[a
            .iter()
            .position(|x| x == "-filter_complex")
            .expect("graph")
            + 1];
        assert!(graph.contains("scale=640:360"));
        assert!(graph.contains("concat=n=3:v=1:a=0,tile=2x2"));
        assert_eq!(a.last().map(String::as_str), Some(SHEET_FILE));
        assert_eq!(cell(Aspect::Portrait), (640, 852));
    }

    /// Renders a real sheet when ffmpeg is on PATH (skipped otherwise).
    #[test]
    fn renders_a_sheet_next_to_the_concepts() {
        let probe = crate::toolchain::ProbeEnv::from_system();
        let Some(ffmpeg) = crate::toolchain::find_on_path(&["ffmpeg"], &probe) else {
            eprintln!("ffmpeg not on PATH, skipped");
            return;
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let new = crate::concepts::NewConcepts {
            project: "moon".into(),
            direction: "Sheet".into(),
            prompt: "x".into(),
            refs: Vec::new(),
            aspect: Aspect::Wide,
            count: 3,
        };
        let (_, files) = crate::concepts::prepare(dir.path(), &new).expect("prepare");
        let png = crate::inbox::images::write_png(32, 18, &[150; 32 * 18 * 3]);
        let mut direction = None;
        for (index, file) in &files {
            std::fs::write(dir.path().join(file), &png).expect("png");
            direction = Some(
                crate::concepts::add_image(dir.path(), "sheet", *index, file, None).expect("add"),
            );
        }
        let mut tools = Toolbox::default();
        tools.tools.insert(
            ToolId::Ffmpeg,
            crate::pipeline::tools::ResolvedTool {
                program: ffmpeg,
                prefix: Vec::new(),
                version: "test".into(),
            },
        );
        tools.path_env = probe.child_path(None);
        let rel = render(
            dir.path(),
            &direction.expect("direction"),
            &dir.path().join("cache"),
            &tools,
        )
        .expect("sheet");
        assert_eq!(rel, "concepts/sheet/contact_sheet.png");
        let bytes = std::fs::read(dir.path().join(&rel)).expect("read");
        let (w, h) = crate::inbox::images::dimensions(&bytes).expect("png");
        assert_eq!((w, h), (12 + 2 * (640 + 12), 12 + 2 * (360 + 12)));
    }
}
