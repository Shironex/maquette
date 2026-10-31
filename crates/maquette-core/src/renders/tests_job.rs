//! The render service without Blender: what it refuses, a record that says
//! what it left out, and a render the app closed on.

use std::path::Path;
use std::sync::Arc;

use super::plan::RenderPlan;
use super::record::{self, RECORD_VERSION};
use super::*;
use crate::packs::PackSource;
use crate::pipeline::runner::{NullSink, PipelineRunner};
use crate::pipeline::tools::Toolbox;

fn service() -> Renders {
    Renders::new(
        PipelineRunner::new(Arc::new(NullSink)),
        |_: &RenderRecord| {},
    )
}

fn request(styles: Vec<RenderStyle>) -> RenderRequest {
    RenderRequest {
        source: PackSource::Entries {
            entries: vec!["m".into()],
        },
        rows: vec![RenderRow {
            key: "m".into(),
            styles,
        }],
        settings: RenderSettings::default(),
        replace: true,
        sheet: false,
    }
}

fn empty_plan(root: &Path) -> RenderPlan {
    RenderPlan {
        label: "from the library".into(),
        dir: root.join("exports").join("renders").display().to_string(),
        targets: Vec::new(),
        skipped: Vec::new(),
    }
}

#[test]
fn a_render_needs_blender_and_says_where_to_set_it() {
    let dir = tempfile::tempdir().expect("dir");
    let err = service()
        .start(
            dir.path(),
            Toolbox::default(),
            &request(vec![RenderStyle::Clay]),
            empty_plan(dir.path()),
        )
        .expect_err("no blender");
    let said = err.to_string();
    assert!(said.contains("Blender was not found"), "{said}");
    assert!(said.contains("Settings, Toolchain"), "{said}");
}

#[test]
fn bad_settings_are_refused_first() {
    let dir = tempfile::tempdir().expect("dir");
    let mut bad = request(vec![RenderStyle::Clay]);
    bad.settings.size = 7;
    let err = service()
        .start(dir.path(), Toolbox::default(), &bad, empty_plan(dir.path()))
        .expect_err("bad size");
    assert!(err.to_string().contains("Pick a size"), "{err}");
}

#[test]
fn a_render_the_app_closed_on_reads_as_stopped_and_counts_its_files() {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    fn file(style: RenderStyle, state: RenderFileState) -> RenderFile {
        RenderFile {
            style,
            file: "x.png".into(),
            state,
            bytes: 0.0,
            seconds: 0.0,
            problem: None,
            moving: None,
        }
    }
    let target = RenderTarget {
        key: "m".into(),
        name: "Model".into(),
        folder: "model".into(),
        version: 1,
        file: "m.glb".into(),
        thumbnail: None,
        height_m: None,
        triangles: 10,
        has_colour: true,
        unreadable: None,
    };
    let saved = RenderRecord {
        version: RECORD_VERSION,
        id: "render-1".into(),
        source: PackSource::Entries {
            entries: vec!["m".into()],
        },
        label: "from the library".into(),
        dir: "d".into(),
        settings: RenderSettings::default(),
        state: RenderState::Running,
        items: vec![RenderItem {
            target,
            files: vec![
                file(RenderStyle::Clay, RenderFileState::Done),
                file(RenderStyle::Toon, RenderFileState::Running),
                file(RenderStyle::Shaded, RenderFileState::Queued),
            ],
        }],
        sheet: None,
        created_at: "2026-10-30T10:00:00Z".into(),
        finished_at: None,
        message: None,
    };
    record::save(root, &saved).expect("save");
    let renders = service();
    let read = renders.get(root, "render-1").expect("get");
    assert_eq!(read.state, RenderState::Stopped);
    assert_eq!(
        read.items[0].files[1].state,
        RenderFileState::Queued,
        "goes again"
    );
    assert_eq!((read.done(), read.total()), (1, 3));
    assert_eq!(renders.recent(root).len(), 1);
    assert!(renders.stop("render-1").is_err(), "not running");
    assert!(
        renders.get(root, "../render-1").is_err(),
        "no way out of the folder"
    );
    let err = renders
        .resume(root, Toolbox::default(), "render-1")
        .expect_err("no blender");
    assert!(err.to_string().contains("Blender was not found"));
}
