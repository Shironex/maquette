//! Real renders in a real, headless Blender: every style of a textured
//! crate, on real pixels. Skipped where no Blender is found. Set
//! `MAQUETTE_KEEP_RENDERS` to a folder to keep the files there and look.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::packs::PackSource;
use crate::pipeline::runner::{NullSink, PipelineRunner};
use crate::refs::png;
use crate::workspace::{FolderLayout, WorkspaceConfig};

/// How much of the picture is not the paper (its corner), and how dark its
/// darkest big share is, so a black or empty render fails.
fn look_at(file: &Path) -> (f64, u8) {
    let pixels = png::decode(&fs::read(file).expect("read")).expect("png");
    let paper = pixels.at(1, 1);
    let mut differs = 0u64;
    let mut sum = 0u64;
    let total = (pixels.width * pixels.height) as u64;
    for y in 0..pixels.height {
        for x in 0..pixels.width {
            let p = pixels.at(x, y);
            let far = (0..3).any(|c| p[c].abs_diff(paper[c]) > 12);
            differs += u64::from(far);
            sum += u64::from(p[0]) + u64::from(p[1]) + u64::from(p[2]);
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "pixel counts")]
    let share = differs as f64 / total as f64;
    let mean = u8::try_from(sum / (total * 3)).expect("mean");
    (share, mean)
}

#[test]
fn every_style_of_a_real_model_comes_out_right() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    super::tests::entry(root, "crate", "Old crate");
    let source = PackSource::Entries {
        entries: vec!["crate".into()],
    };
    let settings = RenderSettings {
        size: 512,
        width: 240,
        length: 1,
        ..RenderSettings::default()
    };
    let request = RenderRequest {
        source: source.clone(),
        rows: vec![RenderRow {
            key: "crate".into(),
            styles: RenderStyle::ALL.to_vec(),
        }],
        settings: settings.clone(),
        replace: true,
        sheet: true,
    };
    let planned = plan(root, &FolderLayout::default(), &source).expect("plan");
    let renders = Renders::new(
        PipelineRunner::new(Arc::new(NullSink)),
        |_: &RenderRecord| {},
    );
    let tools = crate::test_blender::toolbox(&blender);
    let started = renders
        .start(root, tools.clone(), &request, planned)
        .expect("start");
    renders.wait();
    let record = renders.get(root, &started.id).expect("record");
    if let Some(keep) = std::env::var_os("MAQUETTE_KEEP_RENDERS") {
        let keep = Path::new(&keep);
        fs::create_dir_all(keep).expect("keep");
        for file in record
            .items
            .iter()
            .flat_map(|i| &i.files)
            .chain(&record.sheet)
        {
            if let Some(name) = Path::new(&file.file).file_name() {
                let _ = fs::copy(&file.file, keep.join(name));
            }
        }
    }
    assert_eq!(record.state, RenderState::Done, "{:?}", record.message);
    assert_eq!(record.done(), 7, "six styles and the batch's sheet");
    let dir_of = Path::new(&record.dir).join("old-crate");
    for file in &record.items[0].files {
        assert_eq!(
            file.state,
            RenderFileState::Done,
            "{:?}: {:?}",
            file.style,
            file.problem
        );
        assert!(Path::new(&file.file).starts_with(&dir_of), "{}", file.file);
        assert!(file.bytes > 500.0, "{:?} wrote {}", file.style, file.bytes);
    }
    let file_of = |style: RenderStyle| {
        let found = record.items[0].files.iter().find(|f| f.style == style);
        Path::new(&found.expect("file").file).to_path_buf()
    };
    for style in [
        RenderStyle::Blueprint,
        RenderStyle::Toon,
        RenderStyle::Clay,
        RenderStyle::Shaded,
    ] {
        let (share, mean) = look_at(&file_of(style));
        assert!(share > 0.03, "{style:?} drew too little: {share}");
        assert!(mean > 25, "{style:?} came out nearly black: {mean}");
    }
    // A blueprint is lines on its blue paper, not a filled shape.
    let blue = png::decode(&fs::read(file_of(RenderStyle::Blueprint)).expect("read")).expect("png");
    let corner = blue.at(4, 4);
    assert!(corner[2] > corner[0], "blue paper: {corner:?}");
    let sheet = record.sheet.as_ref().expect("sheet");
    assert_eq!(sheet.state, RenderFileState::Done, "{:?}", sheet.problem);
    assert!(Path::new(&sheet.file).ends_with("models_contact.png"));

    // The loop: a real animation, cropped to what moves.
    let webp = fs::read(file_of(RenderStyle::Turntable)).expect("webp");
    assert_eq!(&webp[8..12], b"WEBP");
    assert_eq!(webp.windows(4).filter(|w| w == b"ANMF").count(), 20);
    let turntable = record.items[0]
        .files
        .iter()
        .find(|f| f.style == RenderStyle::Turntable)
        .expect("turntable");
    let moving = turntable.moving.expect("moving box");
    assert!(moving.w < 240 && moving.h < 240, "cropped: {moving:?}");
    assert!(
        moving.w > 40 && moving.h > 40,
        "the model is in it: {moving:?}"
    );

    // Asking again for the pictures is cached.
    let shots = [
        RenderShot {
            style: RenderStyle::Clay,
            turn: 0,
            size: 160,
        },
        RenderShot {
            style: RenderStyle::Turntable,
            turn: 0,
            size: 120,
        },
    ];
    let config = WorkspaceConfig::default();
    let first = renders
        .preview(root, &config, &tools, &source, "crate", &settings, &shots)
        .expect("preview");
    assert_eq!(first.shots[1].frames.len(), 8);
    assert!(first.shots[1].estimate_bytes.is_some_and(|b| b > 0.0));
    assert!(first.shots[0].seconds > 0.0);
    let again = renders
        .preview(root, &config, &tools, &source, "crate", &settings, &shots)
        .expect("preview");
    assert!(again.shots.iter().all(|s| s.seconds == 0.0), "cached");
    assert!(Path::new(&again.shots[0].file).is_file());
}

/// A blueprint turntable keeps the sheet's grid and its height mark under
/// the turning model, where a plain loop has bare paper.
#[test]
fn a_blueprint_turntable_has_its_grid_and_marks() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    super::tests::entry(root, "crate", "Old crate");
    let source = PackSource::Entries {
        entries: vec!["crate".into()],
    };
    let tools = crate::test_blender::toolbox(&blender);
    let renders = Renders::new(
        PipelineRunner::new(Arc::new(NullSink)),
        |_: &RenderRecord| {},
    );
    let shots = [RenderShot {
        style: RenderStyle::Turntable,
        turn: 0,
        size: 240,
    }];
    let config = WorkspaceConfig::default();
    let frames_of = |marks: bool, look: RenderLook| {
        let settings = RenderSettings {
            look,
            marks,
            width: 240,
            length: 1,
            ..RenderSettings::default()
        };
        let got = renders
            .preview(root, &config, &tools, &source, "crate", &settings, &shots)
            .expect("preview");
        let frame = &got.shots[0].frames[0];
        if let Some(keep) = std::env::var_os("MAQUETTE_KEEP_RENDERS") {
            fs::create_dir_all(&keep).expect("keep");
            let name = format!("turn-{look:?}-marks-{marks}.png");
            let _ = fs::copy(frame, Path::new(&keep).join(name));
        }
        png::decode(&fs::read(frame).expect("read")).expect("png")
    };
    // The spread of one channel over the top left block, away from the model.
    let spread = |frame: &png::Pixels| {
        let mut lo = u8::MAX;
        let mut hi = 0u8;
        for y in 0..40 {
            for x in 0..40 {
                let v = frame.at(x, y)[2];
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        hi - lo
    };
    let plain = frames_of(true, RenderLook::Clay);
    assert_eq!(spread(&plain), 0, "a clay loop has bare paper");
    let drawn = frames_of(true, RenderLook::Blueprint);
    assert!(spread(&drawn) > 4, "the grid shows: {}", spread(&drawn));
    // The height mark is a line of ink beside the model: more of the frame
    // differs from its paper with the marks than without.
    let bare = frames_of(false, RenderLook::Blueprint);
    let ink = |f: &png::Pixels| {
        let paper = f.at(1, 1);
        (0..f.height)
            .flat_map(|y| (0..f.width).map(move |x| (x, y)))
            .filter(|&(x, y)| f.at(x, y)[2].abs_diff(paper[2]) > 40)
            .count()
    };
    assert!(ink(&drawn) > ink(&bare), "marks add ink");
}

/// GIF and MP4 through a real ffmpeg, where one is found.
#[test]
fn a_gif_keeps_the_rules_in_a_real_ffmpeg() {
    let Some(blender) = crate::test_blender::blender() else {
        return;
    };
    let tools = crate::test_blender::toolbox(&blender);
    let Some(tools) = with_ffmpeg(tools) else {
        eprintln!("no ffmpeg found, skipped");
        return;
    };
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    super::tests::entry(root, "crate", "Old crate");
    let source = PackSource::Entries {
        entries: vec!["crate".into()],
    };
    let settings = RenderSettings {
        format: LoopFormat::Gif,
        width: 200,
        length: 1,
        ..RenderSettings::default()
    };
    let request = RenderRequest {
        source: source.clone(),
        rows: vec![RenderRow {
            key: "crate".into(),
            styles: vec![RenderStyle::Turntable],
        }],
        settings,
        replace: true,
        sheet: false,
    };
    let planned = plan(root, &FolderLayout::default(), &source).expect("plan");
    let renders = Renders::new(
        PipelineRunner::new(Arc::new(NullSink)),
        |_: &RenderRecord| {},
    );
    let started = renders
        .start(root, tools, &request, planned)
        .expect("start");
    renders.wait();
    let record = renders.get(root, &started.id).expect("record");
    let file = &record.items[0].files[0];
    assert_eq!(file.state, RenderFileState::Done, "{:?}", file.problem);
    let gif = fs::read(&file.file).expect("gif");
    assert_eq!(&gif[0..6], b"GIF89a");
    // Every graphic control block says 5/100 s: delay bytes 5, 0.
    let delays: Vec<u16> = gif
        .windows(8)
        .filter(|w| w[0] == 0x21 && w[1] == 0xf9 && w[2] == 4 && w[7] == 0)
        .map(|w| u16::from_le_bytes([w[4], w[5]]))
        .collect();
    assert_eq!(delays.len(), 20, "one per frame");
    assert!(delays.iter().all(|d| *d == 5), "{delays:?}");
}

fn with_ffmpeg(tools: crate::pipeline::tools::Toolbox) -> Option<crate::pipeline::tools::Toolbox> {
    let found = [
        "/opt/homebrew/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/usr/bin/ffmpeg",
    ]
    .iter()
    .map(Path::new)
    .find(|p| p.is_file())?;
    let mut tools = tools;
    let blender = tools.tools.get(&crate::toolchain::ToolId::Blender)?.clone();
    tools.tools.insert(
        crate::toolchain::ToolId::Ffmpeg,
        crate::pipeline::tools::ResolvedTool {
            program: found.to_path_buf(),
            ..blender
        },
    );
    Some(tools)
}
