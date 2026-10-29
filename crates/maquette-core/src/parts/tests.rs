use std::collections::BTreeMap;

use super::*;
use crate::inbox::fixture;
use crate::prepare::SplitBy;

fn names(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(a, b)| ((*a).to_string(), (*b).to_string()))
        .collect()
}

#[test]
fn rows_carry_the_names_joins_and_hidden_parts() {
    let parts = read_bytes(&fixture::messy_prop()).expect("parts");
    let plan = PartsPlan {
        names: names(&[("Shade", "lid"), ("Base", "post")]),
        merge: vec![vec!["Base".into(), "Speck".into()]],
        hidden: vec!["Shade".into()],
    };
    let list = rows(&parts, Some(&plan));
    assert_eq!(list.len(), 2, "the speck joined the base");
    assert_eq!(list[0].name, "lid");
    assert!(list[0].hidden);
    assert_eq!(list[1].id, "Base");
    assert_eq!(list[1].name, "post");
    assert_eq!(list[1].triangles, 24);
    assert_eq!(list[1].joined, ["Speck".to_string()]);
    let plain = rows(&parts, None);
    assert_eq!(plain.len(), 3);
    assert!(plain.iter().all(|r| r.id == r.name && !r.hidden));
}

#[test]
fn only_a_split_or_a_merge_changes_the_parts() {
    let mut prep = ModelPrep {
        fixups: vec![Fixup::Decimate { triangles: 500 }, Fixup::OriginBottom],
        ..ModelPrep::default()
    };
    assert!(!changes_parts(&prep));
    prep.fixups.push(Fixup::SplitParts {
        by: SplitBy::Loose,
        merge_under: 0.01,
    });
    assert!(changes_parts(&prep));
    prep.parts = Some(PartsPlan {
        names: names(&[("part_1", "lid")]),
        ..PartsPlan::default()
    });
    let listing = listing_plan(&prep);
    assert_eq!(listing.fixups, prep.fixups);
    assert!(listing.parts.is_none() && listing.size.is_none());
}

#[test]
fn numbers_count_parts_draw_calls_and_checks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = WorkspaceConfig::default();
    let one = dir.path().join("one.glb");
    std::fs::write(&one, fixture::merged_islands()).expect("one");
    let before = numbers(&one, AssetKind::Prop, &config, None, None).expect("before");
    assert!(before.parts.is_empty(), "one mesh moves only as a whole");
    assert_eq!(before.draw_calls, 1);
    assert!(before.checks.warning >= 1, "{before:?}");
    let split = dir.path().join("split.glb");
    std::fs::write(&split, fixture::messy_prop()).expect("split");
    let plan = PartsPlan {
        names: names(&[("Shade", "lid")]),
        ..PartsPlan::default()
    };
    let after = numbers(&split, AssetKind::Prop, &config, None, Some(&plan)).expect("after");
    assert_eq!(after.parts, ["lid", "Base", "Speck"]);
    assert_eq!(after.draw_calls, 3);
    assert_eq!(after.max_texture, 1024);
    assert!(after.bytes > 0.0);
}

#[test]
fn keeping_a_version_makes_it_the_newest_and_keeps_every_number() {
    use crate::assets::{AssetVersion, NewAsset, Stage};
    use crate::project::{PROJECT_VERSION, ProjectTemplate};
    let mut manifest = ProjectManifest {
        version: PROJECT_VERSION,
        id: "frostveil".into(),
        name: "Frostveil".into(),
        template: ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: None,
        site_repo_name: None,
        site_repo_state: Default::default(),
        source_folder: None,
        created_at: "2026-10-28T09:30:00.000Z".into(),
        assets: Vec::new(),
    };
    let id = assets::add(
        &mut manifest,
        NewAsset {
            name: "Frost lantern".into(),
            kind: AssetKind::Prop,
            stage: Stage::Blender,
        },
    )
    .expect("asset")
    .id
    .clone();
    for version in 1..=3 {
        manifest.assets[0].versions.push(AssetVersion {
            version,
            file: format!("models/frost_lantern_v{version}.glb"),
            created_at: "2026-10-28T10:00:00.000Z".into(),
            stats: None,
        });
    }
    keep_version(&mut manifest, &id, 2).expect("keep");
    let asset = &manifest.assets[0];
    assert_eq!(asset.latest().map(|v| v.version), Some(2));
    assert_eq!(asset.versions.len(), 3);
    assert_eq!(asset.next_version(), 4, "numbers carry on");
    assert!(keep_version(&mut manifest, &id, 9).is_err());
}
