//! Edge padding in a real, headless Blender (skipped without one): the
//! fix-up script fills a scan's light UV gaps with its island colours.

use crate::test_blender::{blender, run};

fn pad_report(output: &str) -> serde_json::Value {
    let line = output
        .lines()
        .find_map(|l| l.strip_prefix("PAD:"))
        .expect("pad report");
    serde_json::from_str(line).expect("json")
}

#[test]
fn padding_fills_the_gaps_with_the_island_colour_before_the_shrink() {
    let Some(blender) = blender() else { return };
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("pad_edges.py");
    let script = dir.path().join("fixups.py");
    let model = dir.path().join("card.glb");
    std::fs::write(&fixture, include_str!("../pipeline/fixtures/pad_edges.py")).expect("fixture");
    std::fs::write(&script, super::SCRIPT).expect("script");
    let path = |p: &std::path::Path| p.display().to_string();
    run(
        &blender,
        &["--python", &path(&fixture), "--", "make", &path(&model)],
    );
    let mut gaps = Vec::new();
    for pad in [false, true] {
        let plan = dir.path().join(format!("plan_{pad}.json"));
        let out = dir.path().join(format!("out_{pad}.glb"));
        let entry = serde_json::json!({
            "fixups": [{ "op": "shrink-textures", "maxSize": 32 }],
            "padEdges": pad,
        });
        std::fs::write(&plan, entry.to_string()).expect("plan");
        let log = run(
            &blender,
            &[
                "--python",
                &path(&script),
                "--",
                "--input",
                &path(&model),
                "--plan",
                &path(&plan),
                "--output",
                &path(&out),
                "--stats",
                &path(&dir.path().join(format!("stats_{pad}.json"))),
            ],
        );
        assert_eq!(log.contains("pad-edges: padded 1 textures"), pad, "{log}");
        let report = pad_report(&run(
            &blender,
            &["--python", &path(&fixture), "--", "read", &path(&out)],
        ));
        assert_eq!(report["size"], 32, "shrunk after padding");
        let island = report["island"][0].as_f64().expect("red");
        assert!(
            (island - 0.5).abs() < 0.05,
            "the island keeps its colour: {report}"
        );
        gaps.push(report["gap"][1].as_f64().expect("green"));
    }
    assert!(
        gaps[0] > 0.9,
        "without padding the gap stays white: {gaps:?}"
    );
    assert!(
        gaps[1] < 0.1,
        "padded, the gap takes the island colour: {gaps:?}"
    );
}
