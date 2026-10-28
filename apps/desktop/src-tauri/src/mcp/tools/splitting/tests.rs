use super::*;

fn args(value: serde_json::Value) -> SplitArgs {
    serde_json::from_value(value).expect("args")
}

#[test]
fn a_tripo_split_takes_the_defaults_it_is_not_given() {
    let request = args(json!({
        "project": "frostveil",
        "asset": "Frost lantern",
        "provider": "tripo",
        "model": "semantic",
        "complete": "quick-cap",
    }))
    .request("frost-lantern".into());
    assert_eq!(request.provider, Provider::Tripo);
    assert_eq!(request.asset, "frost-lantern");
    assert!(request.meshy.is_none());
    let tripo = request.tripo.expect("tripo settings");
    assert_eq!(tripo.model, SegmentModel::Semantic);
    assert_eq!(tripo.granularity, Granularity::Balanced);
    assert!(tripo.split_by_connectivity);
    assert_eq!(tripo.complete, Some(CompletionMode::QuickCap));
}

#[test]
fn a_meshy_split_carries_its_part_names() {
    let request = args(json!({
        "project": "frostveil",
        "asset": "frost-lantern",
        "provider": "meshy",
        "version": 2,
        "mode": "by-parts",
        "parts": ["lid", "cage"],
    }))
    .request("frost-lantern".into());
    assert_eq!(request.provider, Provider::Meshy);
    assert_eq!(request.version, Some(2));
    assert!(request.tripo.is_none());
    let meshy = request.meshy.expect("meshy settings");
    assert_eq!(meshy.mode, MeshySplitMode::ByParts);
    assert_eq!(meshy.parts, ["lid", "cage"]);
}

#[test]
fn a_part_turns_about_its_bottom_centre_unless_told() {
    assert_eq!(pivot(None), Pivot::BottomCentre);
    assert_eq!(pivot(Some(PivotArg::LeftHinge)), Pivot::LeftHinge);
    let part: PartArgs = serde_json::from_value(json!({
        "asset": "Sigil gate door",
        "name": "door_leaf",
        "pivot": "left-hinge",
        "at": [-0.6, 0.0, 0.0],
    }))
    .expect("part");
    assert_eq!(part.at, Some([-0.6, 0.0, 0.0]));
    assert!(part.turn.abs() < f32::EPSILON);
}
