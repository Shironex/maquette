//! Softening the baked highlights of a written VRM, with a script runner
//! standing in for Blender.

use std::cell::Cell;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::*;
use crate::error::CoreResult;
use crate::inbox::fixture::{self, HumanoidSpec};

const SOFT: &[u8] = b"\x89PNG softened";

/// Stands in for the script when a fake runner meets it (`file`): writes a
/// report that leaves every image as it is. False for other scripts.
pub(crate) fn leaves_as_is(work: &Path, file: &str) -> bool {
    if file != SCRIPT_FILE {
        return false;
    }
    write_report(work, None);
    true
}

/// Writes the script's report for the job in `work`: each image softened
/// to `soft` when given, else skipped.
fn write_report(work: &Path, soft: Option<&[u8]>) {
    let job: Value =
        serde_json::from_str(&fs::read_to_string(work.join(JOB_FILE)).expect("job")).expect("json");
    let images: Vec<Value> = job["images"]
        .as_array()
        .expect("images")
        .iter()
        .map(|image| {
            assert!(work.join(image["input"].as_str().expect("input")).is_file());
            let output = image["output"].as_str().expect("output");
            match soft {
                Some(bytes) => {
                    fs::write(work.join(output), bytes).expect("output");
                    json!({ "output": output, "width": 16, "height": 16,
                        "coverage": 0.04, "darkCoverage": 0.05, "touched": 0.07 })
                }
                None => json!({ "output": null, "width": 16, "height": 16,
                    "coverage": 0.0, "darkCoverage": 0.0, "touched": 0.0,
                    "skipped": "too few baked highlights to change" }),
            }
        })
        .collect();
    fs::write(
        work.join(REPORT_FILE),
        json!({ "strength": job["strength"], "images": images }).to_string(),
    )
    .expect("report");
}

/// A runner that softens every image to [`SOFT`], or fails.
struct Softener {
    runs: Cell<u32>,
    fail: bool,
}

impl Softener {
    fn new(fail: bool) -> Self {
        Self {
            runs: Cell::new(0),
            fail,
        }
    }
}

impl ScriptRunner for Softener {
    fn run_script(
        &self,
        _tools: &Toolbox,
        work: &Path,
        (file, script): (&str, &str),
        extra: &[String],
    ) -> CoreResult<()> {
        self.runs.set(self.runs.get() + 1);
        assert_eq!((file, script), (SCRIPT_FILE, SCRIPT));
        assert_eq!(extra, ["--job", JOB_FILE]);
        if self.fail {
            return Err(invalid("Blender is not installed."));
        }
        write_report(work, Some(SOFT));
        Ok(())
    }
}

/// The box humanoid as written, whose base color is a small PNG.
fn written() -> Written {
    Written {
        bytes: fixture::humanoid(&HumanoidSpec::default()),
        unresolved: Vec::new(),
    }
}

fn apply_with(
    runner: &Softener,
    dir: &Path,
    material: MaterialMode,
    mode: Highlights,
    written: Written,
) -> (Written, Option<HighlightsReport>) {
    apply(runner, &Toolbox::default(), dir, material, mode, written)
}

#[test]
fn the_vrm_shades_from_the_softened_copy_and_keeps_the_original() {
    let dir = tempfile::tempdir().expect("tempdir");
    let runner = Softener::new(false);
    let before = written();
    let original = vrm::base_color_images(&before.bytes).expect("images");
    let (after, report) = apply_with(
        &runner,
        dir.path(),
        MaterialMode::MtoonSoft,
        Highlights::Soften,
        before.clone(),
    );
    let report = report.expect("report");
    assert!(report.softened() && report.note.is_none());
    assert_eq!(report.strength, STRENGTH);
    assert_eq!(report.images.len(), 1);
    assert!((report.images[0].coverage - 0.04).abs() < 1e-9);
    let now = vrm::base_color_images(&after.bytes).expect("images");
    assert_eq!(now[0].bytes, SOFT);
    assert_ne!(
        now[0].image, original[0].image,
        "a new image, the old one kept"
    );
    assert_eq!(after.unresolved, before.unresolved);
    assert!(
        dir.path()
            .join(HIGHLIGHTS_DIR)
            .join("image_0.png")
            .is_file()
    );
    // The same images again read the last run.
    let (again, _) = apply_with(
        &runner,
        dir.path(),
        MaterialMode::MtoonSoft,
        Highlights::Soften,
        before,
    );
    assert_eq!(runner.runs.get(), 1);
    assert_eq!(again.bytes, after.bytes);
}

#[test]
fn keep_and_pbr_write_the_texture_as_it_is_without_blender() {
    let dir = tempfile::tempdir().expect("tempdir");
    let runner = Softener::new(false);
    let before = written();
    let (kept, report) = apply_with(
        &runner,
        dir.path(),
        MaterialMode::MtoonSoft,
        Highlights::Keep,
        before.clone(),
    );
    assert_eq!(kept.bytes, before.bytes);
    let report = report.expect("report");
    assert_eq!(report.mode, Highlights::Keep);
    assert!(!report.softened() && report.images.is_empty());
    let (pbr, report) = apply_with(
        &runner,
        dir.path(),
        MaterialMode::Pbr,
        Highlights::Soften,
        before.clone(),
    );
    assert_eq!(pbr.bytes, before.bytes);
    assert!(report.is_none());
    assert_eq!(runner.runs.get(), 0);
}

#[test]
fn a_texture_without_highlights_is_left_as_it_is() {
    let dir = tempfile::tempdir().expect("tempdir");
    struct Nothing;
    impl ScriptRunner for Nothing {
        fn run_script(
            &self,
            _tools: &Toolbox,
            work: &Path,
            (file, _): (&str, &str),
            _extra: &[String],
        ) -> CoreResult<()> {
            assert!(leaves_as_is(work, file));
            Ok(())
        }
    }
    let before = written();
    let (after, report) = apply(
        &Nothing,
        &Toolbox::default(),
        dir.path(),
        MaterialMode::MtoonSoft,
        Highlights::Soften,
        before.clone(),
    );
    assert_eq!(after.bytes, before.bytes);
    let report = report.expect("report");
    assert!(!report.softened());
    assert_eq!(
        report.note.as_deref(),
        Some("No strong baked highlights were found.")
    );
    assert_eq!(
        report.images[0].note.as_deref(),
        Some("too few baked highlights to change")
    );
}

#[test]
fn a_failed_run_keeps_the_texture_and_says_why() {
    let dir = tempfile::tempdir().expect("tempdir");
    let runner = Softener::new(true);
    let before = written();
    let (after, report) = apply_with(
        &runner,
        dir.path(),
        MaterialMode::MtoonSoft,
        Highlights::Soften,
        before.clone(),
    );
    assert_eq!(after.bytes, before.bytes);
    let note = report.expect("report").note.expect("note");
    assert!(
        note.contains("kept as it is") && note.contains("not installed"),
        "{note}"
    );
}

#[test]
fn a_model_without_a_base_color_texture_runs_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let runner = Softener::new(false);
    let mut before = written();
    let (mut json, bin) = split(&before.bytes);
    for material in json["materials"].as_array_mut().expect("materials") {
        material["pbrMetallicRoughness"]
            .as_object_mut()
            .expect("pbr")
            .remove("baseColorTexture");
    }
    before.bytes = join(&json, &bin);
    let (after, report) = apply_with(
        &runner,
        dir.path(),
        MaterialMode::MtoonSoft,
        Highlights::Soften,
        before.clone(),
    );
    assert_eq!(after.bytes, before.bytes);
    assert_eq!(runner.runs.get(), 0);
    assert_eq!(
        report.expect("report").note.as_deref(),
        Some("The model has no base color texture to soften.")
    );
}

#[test]
fn the_key_follows_the_images() {
    let image = |bytes: &[u8]| BaseColorImage {
        image: 1,
        mime: "image/png".to_string(),
        bytes: bytes.to_vec(),
    };
    assert_eq!(run_key(&[image(b"a")]), run_key(&[image(b"a")]));
    assert_ne!(run_key(&[image(b"a")]), run_key(&[image(b"b")]));
    assert_eq!(
        names(&[BaseColorImage {
            mime: "image/jpeg".to_string(),
            ..image(b"a")
        }]),
        [("image_1.jpg".to_string(), "soft_1.jpg".to_string())]
    );
}

/// The JSON and BIN chunks of a GLB.
fn split(bytes: &[u8]) -> (Value, Vec<u8>) {
    let len = u32::from_le_bytes(bytes[12..16].try_into().expect("length")) as usize;
    let json = serde_json::from_slice(&bytes[20..20 + len]).expect("json");
    let at = 20 + len;
    let bin_len = u32::from_le_bytes(bytes[at..at + 4].try_into().expect("length")) as usize;
    (json, bytes[at + 8..at + 8 + bin_len].to_vec())
}

/// A GLB from its chunks, padded as the container asks.
fn join(json: &Value, bin: &[u8]) -> Vec<u8> {
    let mut text = serde_json::to_vec(json).expect("json");
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    let total = 12 + 8 + text.len() + 8 + bin.len();
    let mut out = b"glTF".to_vec();
    for word in [2, total, text.len()] {
        out.extend_from_slice(&u32::try_from(word).expect("u32").to_le_bytes());
    }
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    out.extend_from_slice(&u32::try_from(bin.len()).expect("u32").to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(bin);
    out
}
