use super::*;
use maquette_core::parts::PartRow;
use maquette_core::prepare::PartsPlan;

fn row(id: &str, name: &str, joined: &[&str]) -> PartRow {
    PartRow {
        id: id.into(),
        name: name.into(),
        triangles: 100,
        materials: vec!["stone".into()],
        hidden: false,
        joined: joined.iter().map(|j| (*j).to_string()).collect(),
    }
}

fn lantern() -> AssetParts {
    AssetParts {
        project: "frostveil".into(),
        asset: "frost-lantern".into(),
        name: "Frost lantern".into(),
        kind: maquette_core::AssetKind::Prop,
        version: Some(3),
        file: None,
        split: true,
        needs_preview: false,
        rows: vec![
            row("part_1", "lid", &[]),
            row("part_2", "part_2", &[]),
            row("part_4", "post", &["part_5"]),
        ],
        plan: PartsPlan::default(),
        game: false,
    }
}

#[test]
fn parts_are_found_by_id_by_shipped_name_or_inside_a_join() {
    let found = lantern();
    assert_eq!(part_id(&found, "lid"), Ok("part_1".to_string()));
    assert_eq!(part_id(&found, " part_1 "), Ok("part_1".to_string()));
    assert_eq!(part_id(&found, "part_5"), Ok("part_5".to_string()));
    let err = part_id(&found, "chain").expect_err("unknown");
    assert!(err.contains("lid, part_2, post"), "{err}");
}

#[test]
fn a_set_parts_call_becomes_a_patch_by_model_names() {
    let args: SetPartsArgs = serde_json::from_value(json!({
        "project": "frostveil",
        "asset": "Frost lantern",
        "rename": { "part_2": "cage", "lid": "lantern_lid" },
        "merge": [["part_2", "post"]],
        "hide": ["post"],
    }))
    .expect("args");
    let patch = args.patch(&lantern()).expect("patch");
    assert_eq!(
        patch.rename.get("part_1").map(String::as_str),
        Some("lantern_lid")
    );
    assert_eq!(patch.rename.get("part_2").map(String::as_str), Some("cage"));
    assert_eq!(
        patch.merge,
        vec![vec!["part_2".to_string(), "part_4".to_string()]]
    );
    assert_eq!(patch.hide, ["part_4".to_string()]);
    let bad: SetPartsArgs = serde_json::from_value(json!({
        "project": "frostveil",
        "asset": "Frost lantern",
        "hide": ["chain"],
    }))
    .expect("args");
    assert!(bad.patch(&lantern()).is_err(), "nothing half applied");
}
