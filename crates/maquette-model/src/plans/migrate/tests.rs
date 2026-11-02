use std::fs;

use serde_json::json;

use super::*;
use crate::plans::{PlanPreset, create, get, list, rename, save};

/// A canvas exactly as a version 1 Maquette wrote it.
const V1_FILE: &str = r#"{
  "version": 1,
  "name": "Combat ideas",
  "preset": "brainstorm",
  "created": "2026-10-30T09:00:00.000Z",
  "edited": "2026-11-02T08:40:00.000Z",
  "nodes": [
    {
      "id": "n-01",
      "kind": "note",
      "x": 60,
      "y": 80,
      "width": null,
      "title": "Frost veil",
      "text": "Slow time around you.",
      "image": null,
      "url": null,
      "by": null
    },
    {
      "id": "n-02",
      "kind": "link",
      "x": 380,
      "y": 80,
      "width": null,
      "title": "Lighting",
      "text": "",
      "image": null,
      "url": "https://example.com/lighting",
      "by": null
    }
  ],
  "edges": [
    {
      "id": "e-01",
      "from": "n-01",
      "to": "n-02",
      "fromSide": "right",
      "toSide": "left",
      "kind": "relates",
      "label": null,
      "by": null
    }
  ]
}
"#;

struct Project {
    _dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

fn project_with_v1() -> Project {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("frostveil");
    fs::create_dir_all(root.join("plans")).expect("plans");
    fs::write(root.join("plans/combat-ideas.plan.json"), V1_FILE).expect("v1 file");
    Project { _dir: dir, root }
}

fn file_text(p: &Project) -> String {
    fs::read_to_string(p.root.join("plans/combat-ideas.plan.json")).expect("read")
}

#[test]
fn a_version_1_file_opens_as_version_2_without_being_rewritten() {
    let p = project_with_v1();
    let opened = get(&p.root, "combat-ideas").expect("a v1 file opens");
    assert_eq!(opened.plan.version, PLAN_VERSION);
    assert_eq!(opened.plan.nodes.len(), 2);
    assert!(
        opened
            .plan
            .nodes
            .iter()
            .all(|n| n.asset.is_none() && n.direction.is_none())
    );
    assert_eq!(list(&p.root).expect("list")[0].problem, None);

    // Sent back unchanged, nothing is written: the file still says version 1.
    save(&p.root, "combat-ideas", &opened.plan).expect("unchanged save");
    assert_eq!(file_text(&p), V1_FILE);

    // An edit writes it as version 2, with the new fields on every node.
    let mut edited = opened.plan.clone();
    edited.nodes[0].x = 100;
    save(&p.root, "combat-ideas", &edited).expect("edit");
    let text = file_text(&p);
    assert!(text.contains("\"version\": 2"), "{text}");
    assert_eq!(text.matches("\"asset\": null").count(), 2);
    assert_eq!(text.matches("\"direction\": null").count(), 2);
    let again = get(&p.root, "combat-ideas").expect("reopens");
    assert_eq!(again.plan.nodes[0].x, 100);
    assert_eq!(again.plan.edges, opened.plan.edges);
}

#[test]
fn a_rename_is_an_edit_and_writes_version_2() {
    let p = project_with_v1();
    rename(&p.root, "combat-ideas", "Combat").expect("rename");
    assert!(file_text(&p).contains("\"version\": 2"));
}

#[test]
fn the_migration_adds_the_link_fields_and_keeps_what_was_there() {
    let v1: Value = serde_json::from_str(V1_FILE).expect("json");
    let v2 = to_current(v1.clone()).expect("migrate");
    assert_eq!(v2["version"], json!(2));
    for (old, new) in v1["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .zip(v2["nodes"].as_array().expect("nodes"))
    {
        assert_eq!(new["asset"], Value::Null);
        assert_eq!(new["direction"], Value::Null);
        for (key, value) in old.as_object().expect("node") {
            assert_eq!(&new[key], value, "{key} kept");
        }
    }
    assert_eq!(v2["edges"], v1["edges"]);
    // A current file goes through as it is.
    assert_eq!(to_current(v2.clone()).expect("current"), v2);
}

#[test]
fn a_version_1_file_holding_a_newer_kind_is_refused() {
    let mut v1: Value = serde_json::from_str(V1_FILE).expect("json");
    v1["nodes"][0]["kind"] = json!("asset");
    let refused = to_current(v1).expect_err("an asset node in v1");
    assert!(refused.to_string().contains("version 1"), "{refused}");
}

#[test]
fn versions_that_are_missing_odd_or_newer_are_refused() {
    for odd in [
        json!({}),
        json!({"version": 0}),
        json!({"version": "1"}),
        json!({"version": -1}),
    ] {
        assert!(
            matches!(version_of(&odd), Err(CoreError::Invalid(_))),
            "{odd}"
        );
    }
    match version_of(&json!({"version": 3})) {
        Err(CoreError::UnsupportedVersion { found, supported }) => {
            assert_eq!((found, supported), (3, PLAN_VERSION));
        }
        other => panic!("expected a version error, got {other:?}"),
    }
    assert_eq!(version_of(&json!({"version": 1})).expect("v1"), 1);
}

#[test]
fn a_file_from_a_newer_maquette_is_never_written_back() {
    let dir = tempfile::tempdir().expect("temp dir");
    let made = create(dir.path(), "Ideas", PlanPreset::Brainstorm).expect("create");
    let file = dir.path().join(&made.file);
    let newer = fs::read_to_string(&file)
        .expect("read")
        .replace("\"version\": 2", "\"version\": 3")
        .replace(
            "\"nodes\": []",
            "\"nodes\": [{ \"id\": \"n-1\", \"kind\": \"sketch\" }]",
        );
    fs::write(&file, &newer).expect("newer");
    let mut plan = made.plan.clone();
    plan.nodes = vec![];
    assert!(save(dir.path(), &made.id, &plan).is_err(), "save refused");
    assert!(
        rename(dir.path(), &made.id, "Other").is_err(),
        "rename refused"
    );
    assert_eq!(fs::read_to_string(&file).expect("read"), newer);
    let listed = list(dir.path()).expect("list");
    assert!(
        listed[0]
            .problem
            .as_deref()
            .is_some_and(|m| m.contains('3')),
        "{:?}",
        listed[0].problem
    );
}
